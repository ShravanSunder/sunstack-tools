/** Demand the real same-range document/element ambiguity seen on the baseline. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';

import {
	arrayValue,
	gitCommand,
	objectValue,
	parseObject,
	writeJson,
} from '../rulebook/rulebook-fixture-support.ts';
import type { RulebookProcess } from '../rulebook/rulebook-process.ts';
import { byteFixtureCases, byteFixturePath, createByteFixture } from './byte-fixture.ts';

export async function observeByteAmbiguity(options: {
	readonly observer: RulebookProcess;
	readonly evidence: string;
}): Promise<void> {
	const root = path.join(options.evidence, 'ambiguous-fixture');
	const fixture = createByteFixture(root, options.observer.endpoint);
	filesystem.writeFileSync(path.join(root, byteFixturePath), '<p>café yes</p>');
	filesystem.writeFileSync(
		path.join(root, 'evidence', 'rules', 'paragraph.yml'),
		'id: paragraph\nlanguage: Html\nrule:\n  any:\n    - {kind: document}\n    - {kind: element}\n',
	);
	gitCommand(root, ['add', byteFixturePath, 'evidence/rules/paragraph.yml']);
	gitCommand(root, ['commit', '-qm', 'Genuine equal-range nodes']);
	const revision = gitCommand(root, ['rev-parse', 'HEAD']);
	writeJson(fixture.casesPath, {
		repository: root,
		cases: [
			{
				...byteFixtureCases[0],
				id: 'ambiguous',
				kind: 'rule',
				rule: 'approval',
				file: byteFixturePath,
				revision,
				lines: [1, 1],
				set: 'development',
				label_from: 'blind-agents',
			},
		],
		bars: [],
	});
	const check = await options.observer.run({
		root,
		name: 'ambiguous-evidence',
		arguments: ['lint', '--config', 'sunstack.lint.json', '--all', '--evidence', 'paragraph'],
		expectedExit: 0,
	});
	const pieces = arrayValue(parseObject(check.stdout.toString('utf8'))['pieces']).map(objectValue);
	assert.equal(pieces.length, 2);
	for (const piece of pieces) {
		const span = objectValue(objectValue(piece['where'])['span']);
		assert.deepEqual([span['byte_start'], span['byte_end']], [0, 16]);
	}
	const prepared = path.join(options.evidence, 'ambiguous-prepared.json');
	const result = await options.observer.run({
		root,
		name: 'ambiguous-prepare',
		arguments: [
			'lint',
			'--config',
			'sunstack.lint.json',
			'cases',
			fixture.casesPath,
			'--prepare',
			prepared,
		],
		expectedExit: 1,
	});
	assert.equal(result.captures.length, 0);
	const rows = arrayValue(parseObject(filesystem.readFileSync(prepared, 'utf8'))['cases']).map(
		objectValue,
	);
	assert.equal(rows.length, 1);
	assert.equal(objectValue(rows[0]?.['outcome'])['kind'], 'more_than_one_matching_piece');
	assert.equal(rows[0]?.['result_id'], null);
	const scored = path.join(options.evidence, 'ambiguous-scored.json');
	const scoredRun = await options.observer.run({
		root,
		name: 'ambiguous-score',
		arguments: [
			'lint',
			'--config',
			'sunstack.lint.json',
			'--report',
			scored,
			'cases',
			fixture.casesPath,
		],
		credential: true,
		expectedExit: 1,
	});
	assert.equal(scoredRun.captures.length, 0);
	const scoredRows = arrayValue(parseObject(filesystem.readFileSync(scored, 'utf8'))['cases']).map(
		objectValue,
	);
	assert.equal(scoredRows.length, 1);
	assert.equal(objectValue(scoredRows[0]?.['outcome'])['kind'], 'more_than_one_matching_piece');
	assert.equal(scoredRows[0]?.['result_id'], null);
}
