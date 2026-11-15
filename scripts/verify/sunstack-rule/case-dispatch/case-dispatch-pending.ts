/** Pending evidence selection is accounted separately from absent execution owners. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';

import { prepareConfig, prepareSelectionFixture } from '../prepare/prepare-fixtures.ts';
import { compactJson, jsonSlice } from '../prepare/raw-json.ts';
import {
	arrayValue,
	configureEndpoint,
	gitCommand,
	objectValue,
	parseObject,
	runSequence,
	textValue,
	writeJson,
} from '../rulebook/rulebook-fixture-support.ts';
import { type RulebookProcess } from '../rulebook/rulebook-process.ts';
import { oracleRows, researchFile } from '../selection/selection-fixture.ts';
import { scriptedSelectionReply } from '../selection/selection-reply.ts';
import {
	attributeControlledCaptures,
	requireCaseAccounting,
	requireCaseRunStatus,
	requireUnassessedBars,
} from './case-dispatch-checks.ts';

export function candidateLines(source: string, text: string): [number, number] {
	const start = source.indexOf(text);
	assert.ok(
		start >= 0 && source.indexOf(text, start + 1) < 0,
		'Fixture step is missing or repeated',
	);
	assert.ok(start === 0 || source[start - 1] === '\n', 'Fixture candidate starts inside a line');
	assert.ok(text.endsWith('\n'), 'Fixture candidate needs a complete final line');
	const firstLine = source.slice(0, start).split('\n').length;
	return [firstLine, firstLine + text.split('\n').length - 2];
}

export function requirePendingCasePreparation(raw: string): void {
	const report = parseObject(raw);
	assert.equal(report['mode'], 'prepare');
	assert.equal(objectValue(report['status'])['kind'], 'findings');
	assert.deepEqual(objectValue(report['errors']), {});
	const cases = arrayValue(report['cases']).map(objectValue);
	assert.equal(cases.length, 1);
	assert.equal(cases[0]?.['id'], 'pending-step');
	assert.deepEqual(cases[0]?.['outcome'], {
		kind: 'selection_pending',
		evidence_kind: 'step',
		file: researchFile,
	});
	assert.equal(cases[0]?.['result_id'], null);
	assert.equal(arrayValue(report['runs']).length, 1);
	const nestedRaw = jsonSlice(raw, ['runs', 0, 'report']);
	const nested = parseObject(nestedRaw);
	assert.equal(nested['mode'], 'prepare');
	assert.equal(objectValue(nested['status'])['kind'], 'prepared');
	assert.deepEqual(arrayValue(nested['results']), []);
	const requests = arrayValue(nested['requests']).map(objectValue);
	assert.equal(requests.length, 1, 'Expected exactly one prerequisite and no final owner');
	const request = requests[0];
	assert.ok(request);
	assert.deepEqual(request['owner'], {
		kind: 'selection',
		selection_kind: 'workflow_section',
		file: researchFile,
	});
	assert.equal(request['state'], 'prepared');
	assert.equal(request['status'], 'prepared');
	assert.equal(
		request['input_sha256'],
		createHash('sha256')
			.update(compactJson(jsonSlice(nestedRaw, ['requests', 0, 'request'])))
			.digest('hex'),
	);
	const needs = arrayValue(nested['needs_answer'])
		.map(objectValue)
		.filter((entry) => entry['kind'] === 'step' && entry['file'] === researchFile);
	assert.equal(needs.length, 1, 'Expected one pending step prerequisite');
	assert.deepEqual(needs[0]?.['waits_on'], {
		selection_kind: 'workflow_section',
		request_id: request['id'],
	});
}

export async function observePendingCaseSelection(options: {
	readonly observer: RulebookProcess;
	readonly evidence: string;
	readonly sourceRoot: string;
}): Promise<void> {
	const root = path.join(options.evidence, 'pending-case-fixture');
	prepareSelectionFixture(root, options.sourceRoot);
	configureEndpoint(root, options.observer.endpoint);
	const source = filesystem.readFileSync(path.join(root, researchFile), 'utf8');
	const row = oracleRows(root, 'step', researchFile)[0];
	assert.ok(row, 'No independently recorded step candidate');
	const text = textValue(row['text']);
	const lines = candidateLines(source, text);
	const revision = gitCommand(root, ['rev-parse', 'HEAD']);
	const cases = path.join(options.evidence, 'pending-case-input.json');
	writeJson(cases, {
		repository: root,
		cases: [
			{
				id: 'pending-step',
				kind: 'rule',
				rule: 'selected_step_asks',
				file: researchFile,
				revision,
				lines,
				text_sha256: createHash('sha256').update(text).digest('hex'),
				expected: 'pass',
				set: 'development',
				label_from: 'blind-agents',
			},
		],
		bars: [],
	});
	const output = path.join(options.evidence, 'pending-case-prepared.json');
	const result = await options.observer.run({
		root,
		name: 'pending-case-prepare',
		arguments: [
			'lint',
			'--config',
			prepareConfig,
			'cases',
			cases,
			'--set',
			'development',
			'--case',
			'pending-step',
			'--prepare',
			output,
		],
		expectedExit: 1,
	});
	assert.equal(result.captures.length, 0, 'Pending prepare dispatched HTTP');
	const raw = filesystem.readFileSync(output, 'utf8');
	requirePendingCasePreparation(raw);
	await runSequence(
		[
			['positive', 0.95, 1, 'right'],
			['negative', 0.05, 1, 'no_matching_piece'],
		] as const,
		async ([label, probability, exit, disposition]) => {
			options.observer.reply = scriptedSelectionReply({ probability: () => probability });
			const ordinaryFile = path.join(options.evidence, `pending-${label}-ordinary.json`);
			const selectedFile = path.join(options.evidence, `pending-${label}-selected.json`);
			const ordinary = await options.observer.run({
				root,
				name: `pending-${label}-ordinary`,
				arguments: ['lint', '--config', prepareConfig, '--all', '--report', ordinaryFile],
				credential: true,
				expectedExit: 0,
			});
			const selected = await options.observer.run({
				root,
				name: `pending-${label}-selected`,
				arguments: [
					'lint',
					'--config',
					prepareConfig,
					'--report',
					selectedFile,
					'cases',
					cases,
					'--set',
					'development',
					'--case',
					'pending-step',
				],
				credential: true,
				expectedExit: exit,
			});
			const selectedRaw = filesystem.readFileSync(selectedFile, 'utf8');
			const selectedReport = parseObject(selectedRaw);
			requireUnassessedBars(selectedReport);
			requireCaseRunStatus(
				selectedReport,
				label === 'positive' ? 'clean' : 'nothing_selected',
				label === 'positive' ? 1 : 0,
			);
			requireCaseAccounting({
				selectedIds: ['pending-step'],
				dispositions: arrayValue(selectedReport['cases'])
					.map(objectValue)
					.map((item) => ({
						id: textValue(item['id']),
						kind: textValue(objectValue(item['outcome'])['kind']),
					})),
				expectedKinds: new Map([['pending-step', disposition]]),
			});
			const ordinaryCaptures = attributeControlledCaptures(
				filesystem.readFileSync(ordinaryFile, 'utf8'),
				ordinary.captures.map((capture) => capture.raw.toString('utf8')),
			);
			const selectedCaptures = attributeControlledCaptures(
				jsonSlice(selectedRaw, ['runs', 0, 'report']),
				selected.captures.map((capture) => capture.raw.toString('utf8')),
			);
			const selector = JSON.stringify(['selection', 'workflow_section', researchFile]);
			const selectedSelector = selectedCaptures.find((capture) => capture.owner === selector);
			const ordinarySelector = ordinaryCaptures.find((capture) => capture.owner === selector);
			assert.ok(
				selectedSelector && ordinarySelector,
				'Selection settlement has no matching prerequisite witness',
			);
			assert.equal(
				selectedSelector.raw,
				ordinarySelector.raw,
				'Selection prerequisite body differs',
			);
			const ruleOwner = JSON.stringify([
				'rule',
				'selected_step_asks',
				researchFile,
				String(lines[0]),
				String(lines[1]),
			]);
			const selectedRule = selectedCaptures.find((capture) => capture.owner === ruleOwner);
			if (label === 'positive') {
				const ordinaryRule = ordinaryCaptures.find((capture) => capture.owner === ruleOwner);
				assert.ok(selectedRule && ordinaryRule, 'Positive settlement did not reach selected rule');
				assert.equal(
					selectedRule.raw,
					ordinaryRule.raw,
					'Positive settlement changed selected request',
				);
			} else assert.equal(selectedRule, undefined, 'Negative settlement dispatched rejected rule');
		},
	);
}
