/** Pending evidence selection is accounted separately from absent execution owners. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';

import { prepareConfig, prepareSelectionFixture } from '../prepare/prepare-fixtures.ts';
import { jsonSlice } from '../prepare/raw-json.ts';
import {
	arrayValue,
	gitCommand,
	objectValue,
	parseObject,
	textValue,
	writeJson,
} from '../rulebook/rulebook-fixture-support.ts';
import { type RulebookProcess } from '../rulebook/rulebook-process.ts';
import { oracleRows, researchFile } from '../selection/selection-fixture.ts';
import { requireCaseAccounting } from './case-dispatch-checks.ts';

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

export async function observePendingCaseSelection(options: {
	readonly observer: RulebookProcess;
	readonly evidence: string;
	readonly sourceRoot: string;
}): Promise<void> {
	const root = path.join(options.evidence, 'pending-case-fixture');
	prepareSelectionFixture(root, options.sourceRoot);
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
		expectedExit: 6,
	});
	assert.equal(result.captures.length, 0, 'Pending prepare dispatched HTTP');
	const raw = filesystem.readFileSync(output, 'utf8');
	const report = parseObject(raw);
	requireCaseAccounting({
		selectedIds: ['pending-step'],
		dispositions: arrayValue(report['cases'])
			.map(objectValue)
			.map((item) => ({
				id: textValue(item['id']),
				kind: textValue(objectValue(item['outcome'])['kind']),
			})),
		expectedKinds: new Map([['pending-step', 'selection_pending']]),
	});
	assert.equal(arrayValue(report['runs']).length, 1);
	const nested = parseObject(jsonSlice(raw, ['runs', 0, 'report']));
	const requests = arrayValue(nested['requests']).map(objectValue);
	assert.ok(requests.length > 0, 'Pending selection has no prerequisite request');
	assert.ok(
		requests.every((request) => objectValue(request['owner'])['kind'] === 'selection'),
		'Pending case fabricated a final rule/filter owner',
	);
}
