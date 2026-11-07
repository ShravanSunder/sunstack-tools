/** O6 selected-case budget and development-source refusal through the real CLI. */
import filesystem from 'node:fs';
import path from 'node:path';

import {
	ordinaryProjection,
	requireBars,
	requireCaseScope,
	requireCasesReport,
	requireInvalidBarSource,
} from './rulebook-case-checks.ts';
import { casesConfigName, unrelatedDescription } from './rulebook-case-fixtures.ts';
import {
	arrayValue,
	equal,
	objectValue,
	parseObject,
	readObject,
	requireCondition,
	runSequence,
	textValue,
	writeJson,
	type JsonObject,
} from './rulebook-fixture-support.ts';
import { type RulebookProcess } from './rulebook-process.ts';

export async function runCaseControls(options: {
	observer: RulebookProcess;
	root: string;
	evidence: string;
	prepared: JsonObject;
	definitions: readonly JsonObject[];
}): Promise<void> {
	const { observer, root, evidence, prepared, definitions } = options;
	const original = definitions.find((definition) => definition['id'] === 'current-pass');
	requireCondition(original, 'Missing selected budget case');
	const selected: JsonObject = { ...original, id: 'budget-selected', set: 'held-out' };
	const selectedPath = textValue(selected['file']);
	const configPath = path.join(root, casesConfigName);
	const configBytes = filesystem.readFileSync(configPath);
	const config = readObject(configPath);
	config['limits'] = { ...objectValue(config['limits'] ?? {}), max_model_requests: 2 };
	writeJson(configPath, config);
	try {
		const direct = await observer.report({
			root,
			name: 'budget-direct',
			arguments: ['lint', '--config', casesConfigName, '--format', 'json', selectedPath],
			credential: true,
			expectedExit: 0,
		});
		equal(
			direct.captures.length,
			2,
			'Direct named lint did not need one filter and one rule request',
		);
		await runSequence(
			[
				['valid', undefined],
				['held-out', 'budget-selected'],
				['wrong-owner', 'filter-right'],
				['unknown', 'missing-case-id'],
			] as const,
			async ([name, source]) => {
				const bars = arrayValue(prepared['bars']).map((bar) => structuredClone(objectValue(bar)));
				const affected = bars.find(
					(bar) => bar['for'] === 'asking_gate' && bar['bound'] === 'yes_at_or_above',
				);
				requireCondition(affected, 'Missing provenance control bar');
				if (source) affected['set_from'] = [source];
				const input = path.join(evidence, `budget-${name}.json`);
				writeJson(input, { repository: root, cases: [...definitions, selected], bars });
				const run = await observer.run({
					root,
					name: `budget-${name}`,
					arguments: ['lint', '--config', casesConfigName, 'cases', input, '--set', 'held-out'],
					credential: true,
					expectedExit: source ? 1 : 0,
				});
				const report = parseObject(run.stdout.toString('utf8'));
				requireCasesReport({
					report,
					definitions: [selected],
					expected: { 'budget-selected': 'right' },
					notSelected: definitions.map((definition) => textValue(definition['id'])),
				});
				const runs = arrayValue(report['runs']).map(objectValue);
				equal(runs.length, 1, 'Unselected development cases started revision runs');
				equal(runs[0]?.['revision'], selected['revision'], 'Selected case read the wrong revision');
				const lint = objectValue(runs[0]?.['report']);
				requireCaseScope(lint, [selectedPath]);
				equal(
					ordinaryProjection(lint),
					ordinaryProjection(direct.report),
					'Bar provenance changed ordinary lint behavior',
				);
				equal(
					run.captures.map((capture) => capture.body),
					direct.captures.map((capture) => capture.body),
					'Cases differ from named lint or spent their two-request budget on unrelated files',
				);
				requireCondition(
					!JSON.stringify(run.captures).includes(unrelatedDescription),
					'Unrelated folder consumed model allowance',
				);
				if (source) requireInvalidBarSource(report, [source]);
				else requireBars(report, false);
			},
		);
	} finally {
		filesystem.writeFileSync(configPath, configBytes);
	}
}
