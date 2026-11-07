/** O6 real lint cases: revision context, complete accounting and no adoption gate. */
import { spawnSync } from 'node:child_process';
import filesystem from 'node:fs';
import path from 'node:path';
import { isDeepStrictEqual } from 'node:util';

import { requireDebugSignature } from './rulebook-amendment.ts';
import {
	requireCasesReport,
	requireRevisionContext,
	requireBars,
	ordinaryProjection,
	selectedFixturePaths,
	requireCaseScope,
} from './rulebook-case-checks.ts';
import { runCaseControls } from './rulebook-case-controls.ts';
import {
	casesOracle,
	casesConfigName,
	prepareCasesFixture,
	historicalDescription,
	unrelatedDescription,
} from './rulebook-case-fixtures.ts';
import {
	arrayValue,
	equal,
	gitCommand,
	journeyArguments,
	objectValue,
	parseObject,
	readObject,
	requireCondition,
	rulebookName,
	rulesName,
	runSequence,
	textValue,
	writeJson,
	type JsonObject,
	type JourneyArguments,
} from './rulebook-fixture-support.ts';
import {
	declaredTarget,
	responseBody,
	RulebookProcess,
	type CapturedRequest,
	type ControlledReply,
} from './rulebook-process.ts';
import { reportReply, requireSavedRequest } from './rulebook-report.ts';

function casesReply(body: JsonObject, alwaysUnsure: boolean): ControlledReply {
	const state = objectValue(body['state']);
	if (!('scope' in objectValue(body['questions'])) && alwaysUnsure)
		return { status: 200, body: responseBody(body, 0.5) };
	if (
		['CURRENT_FILTER_DESCRIPTION_SENTINEL.', unrelatedDescription].includes(
			textValue(state['description'] ?? ''),
		)
	)
		return reportReply({ ...body, state: { ...state, description: historicalDescription } });
	return reportReply(body);
}
function caseDefinitions(prepared: JsonObject): JsonObject[] {
	return [...arrayValue(prepared['rule_cases']), ...arrayValue(prepared['filter_cases'])].map(
		objectValue,
	);
}
function caseFile(
	prepared: JsonObject,
	root: string,
	definitions = caseDefinitions(prepared),
): JsonObject {
	return { repository: root, cases: definitions, bars: prepared['bars'] ?? [] };
}
function expectedDispositions(prepared: JsonObject, alwaysUnsure: boolean): Record<string, string> {
	const expected: Record<string, string> = {};
	for (const [id, value] of Object.entries(objectValue(prepared['expected_dispositions'])))
		expected[id] = textValue(value);
	if (alwaysUnsure)
		for (const id of [
			'pass',
			'must-pass',
			'must-warn',
			'unsure',
			'incomplete',
			'historical-reference',
			'current-pass',
			'current-reference',
		])
			expected[id] = 'unsure';
	return expected;
}
function requireCaseRequests(
	report: JsonObject,
	captures: readonly CapturedRequest[],
	alwaysUnsure: boolean,
): void {
	const records = arrayValue(report['runs'])
		.map(objectValue)
		.flatMap((run) => arrayValue(objectValue(run['report'])['requests']).map(objectValue));
	equal(records.length, captures.length, 'Case runs lost or invented captured requests');
	const consumed = new Set<number>();
	for (const record of records) {
		equal(record['state'], 'run', 'Fixture request did not dispatch');
		const request = objectValue(record['request']);
		const index = captures.findIndex(
			(capture, position) =>
				!consumed.has(position) &&
				capture.body['model'] === record['requested_model'] &&
				isDeepStrictEqual(capture.body['state'], request['state']) &&
				isDeepStrictEqual(capture.body['questions'], request['questions']),
		);
		requireCondition(index >= 0, 'Saved case request has no actual HTTP request');
		consumed.add(index);
		const captured = captures[index];
		requireCondition(captured, 'Capture missing');
		const outcome = objectValue(record['outcome']);
		const reply = casesReply(captured.body, alwaysUnsure);
		equal(
			outcome['kind'],
			reply.status === 200 ? 'completed' : 'provider_failure',
			'Case request outcome differs from controlled response',
		);
		if (reply.status === 400) {
			equal(outcome['failure_class'], 'provider_rejected_request', 'Case refusal class differs');
			equal(outcome['http_status'], 400, 'Case refusal status differs');
		}
		requireSavedRequest({
			sent: captured.body,
			savedState: request['state'],
			savedQuestions: request['questions'],
			savedAnswers: outcome['answers'],
			requestedModel: record['requested_model'],
			resolvedModel: record['resolved_model'],
			reply,
		});
		const state = objectValue(request['state']);
		for (const field of [
			'expected',
			'expected_label',
			'label_from',
			'set',
			'text_sha256',
			'reads_sha256',
			'lines',
		])
			requireCondition(!(field in state), 'Case metadata leaked into model state');
	}
}
function prepareOrdinaryCopy(root: string, revision: string, evidence: string): string {
	const archive = path.join(evidence, `${revision}.tar`);
	const destination = path.join(evidence, revision);
	gitCommand(root, ['archive', '--format=tar', '--output', archive, revision]);
	filesystem.mkdirSync(destination);
	const extracted = spawnSync('tar', ['-xf', archive, '-C', destination], {
		encoding: 'utf8',
		timeout: 15000,
	});
	requireCondition(
		extracted.status === 0,
		`Fixture archive extraction failed: ${extracted.stderr}`,
	);
	gitCommand(destination, ['init', '-q']);
	for (const file of [casesConfigName, rulebookName, rulesName, 'evidence/sgconfig.yml'])
		filesystem.copyFileSync(path.join(root, file), path.join(destination, file));
	return destination;
}
export async function runCasesJourney(options: JourneyArguments): Promise<void> {
	const target = options.prepare
		? undefined
		: declaredTarget({
				binary: options.binary ?? '',
				source: options['source-root'],
				commit: options.commit ?? '',
				...(options.sha256 ? { digest: options.sha256 } : {}),
			});
	const evidence = path.resolve(options.evidence);
	filesystem.mkdirSync(evidence);
	const root = path.join(evidence, 'fixture');
	const prepared = prepareCasesFixture(root);
	const definitions = caseDefinitions(prepared);
	const input = path.join(evidence, 'cases.json');
	writeJson(path.join(evidence, 'prepared.json'), prepared);
	writeJson(input, caseFile(prepared, root));
	if (options.prepare) {
		process.stdout.write('Prepared case revision fixtures; product unobserved\n');
		return;
	}
	requireCondition(target, 'Case observation needs declared candidate');
	requireDebugSignature(target.binary);
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	try {
		const currentConfigPath = path.join(root, casesConfigName);
		const currentConfig = readObject(currentConfigPath);
		objectValue(currentConfig['decision_model'])['endpoint'] = observer.endpoint;
		writeJson(currentConfigPath, currentConfig);
		observer.respond = ({ body }) => Promise.resolve(casesReply(body, false));
		const checked = await observer.run({
			root,
			name: 'check',
			arguments: ['lint', '--config', casesConfigName, '--check'],
			expectedExit: 0,
		});
		equal(checked.captures.length, 0, 'Keyless check dispatched');
		const before = await observer.run({
			root,
			name: 'lint-before',
			arguments: ['lint', '--config', casesConfigName, '--all'],
			credential: true,
			expectedExit: 3,
		});
		const positive = path.join(evidence, 'positive-cases.json');
		const positiveIds = [
			'pass',
			'must-pass',
			'must-warn',
			'filter-right',
			'historical-reference',
			'current-pass',
			'current-reference',
		];
		const positives = definitions.filter((item) => positiveIds.includes(textValue(item['id'])));
		writeJson(positive, caseFile(prepared, root, positives));
		const good = await observer.run({
			root,
			name: 'positive-cases',
			arguments: ['lint', '--config', casesConfigName, 'cases', positive, '--set', 'development'],
			credential: true,
			expectedExit: 0,
		});
		const goodReport = parseObject(good.stdout.toString('utf8'));
		const goodExpected: Record<string, string> = {};
		for (const id of positiveIds) goodExpected[id] = 'right';
		requireCasesReport({ report: goodReport, definitions: positives, expected: goodExpected });
		requireBars(goodReport, false);
		const command = ['lint', '--config', casesConfigName, 'cases', input, '--set', 'development'];
		const mixed = await observer.run({
			root,
			name: 'mixed-cases',
			arguments: command,
			credential: true,
			expectedExit: 1,
		});
		const mixedReport = parseObject(mixed.stdout.toString('utf8'));
		requireCasesReport({
			report: mixedReport,
			definitions,
			expected: expectedDispositions(prepared, false),
		});
		requireCaseRequests(mixedReport, mixed.captures, false);
		requireRevisionContext(mixedReport, prepared);
		requireBars(mixedReport, false);
		const runs = arrayValue(mixedReport['runs']).map(objectValue);
		await runSequence(runs, async (run) => {
			const revision = textValue(run['revision']);
			const selected = selectedFixturePaths(
				definitions,
				revision,
				arrayValue(prepared['checked_files']).map(textValue),
			);
			requireCondition(selected.length > 0, 'Empty selected case scope');
			requireCaseScope(objectValue(run['report']), selected);
			const copied = prepareOrdinaryCopy(root, revision, evidence);
			const result = await observer.report({
				root: copied,
				name: `ordinary-${revision}`,
				arguments: ['lint', '--config', casesConfigName, '--format', 'json', ...selected],
				credential: true,
				expectedExit: revision === prepared['historical'] ? 3 : 0,
			});
			equal(
				ordinaryProjection(result.report),
				ordinaryProjection(objectValue(run['report'])),
				'Lint and case run differ on the same text/context',
			);
		});
		const rulesPath = path.join(root, rulesName);
		const originalRules = filesystem.readFileSync(rulesPath);
		const rules = readObject(rulesPath);
		const askingRule = objectValue(rules['K002']);
		const decisionSteps = objectValue(objectValue(askingRule['decision'])['steps']);
		objectValue(decisionSteps['approved'])['yes_at_or_above'] = 0.8;
		writeJson(rulesPath, rules);
		try {
			const changed = await observer.run({
				root,
				name: 'changed-bar',
				arguments: command,
				credential: true,
				expectedExit: 1,
			});
			const report = parseObject(changed.stdout.toString('utf8'));
			requireCasesReport({ report, definitions, expected: expectedDispositions(prepared, false) });
			requireBars(report, true);
		} finally {
			filesystem.writeFileSync(rulesPath, originalRules);
		}
		observer.respond = ({ body }) => Promise.resolve(casesReply(body, true));
		const unsure = await observer.run({
			root,
			name: 'always-unsure',
			arguments: command,
			credential: true,
			expectedExit: 1,
		});
		const unsureReport = parseObject(unsure.stdout.toString('utf8'));
		requireCasesReport({
			report: unsureReport,
			definitions,
			expected: expectedDispositions(prepared, true),
		});
		requireCaseRequests(unsureReport, unsure.captures, true);
		observer.respond = ({ body }) => Promise.resolve(casesReply(body, false));
		await runCaseControls({ observer, root, evidence, prepared, definitions });
		const after = await observer.run({
			root,
			name: 'lint-after',
			arguments: ['lint', '--config', casesConfigName, '--all'],
			credential: true,
			expectedExit: 3,
		});
		equal(after.stdout, before.stdout, 'Case runs changed ordinary lint stdout bytes');
		equal(after.stderr, before.stderr, 'Case runs changed ordinary lint stderr bytes');
	} finally {
		await observer.close();
	}
	writeJson(path.join(evidence, 'pass.json'), {
		product_observed: true,
		oracle: casesOracle,
		commit: target.commit,
		binary_sha256: target.digest,
		unverified: ['real judgment quality', 'ai-tools held-out run', 'release publication'],
	});
	process.stdout.write('Revision cases, bars and lint independence journey passed\n');
}
if (process.argv[1] === import.meta.filename) {
	const options = journeyArguments(
		'O6 revision-bound cases, exact accounting, changed bars, always-unsure and lint independence through the real binary.',
	);
	if (options) await runCasesJourney(options);
}
