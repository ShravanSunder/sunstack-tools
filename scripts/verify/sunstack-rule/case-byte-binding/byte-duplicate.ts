/** Distinct case IDs for one exact byte owner share a single execution. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';

import {
	arrayValue,
	objectValue,
	parseObject,
	writeJson,
} from '../rulebook/rulebook-fixture-support.ts';
import type { RulebookProcess } from '../rulebook/rulebook-process.ts';
import { byteFixtureCases, byteFixturePath } from './byte-fixture.ts';

function checkDuplicateReport(reportPath: string, kind: string): void {
	const report = parseObject(filesystem.readFileSync(reportPath, 'utf8'));
	const rows = arrayValue(report['cases']).map(objectValue);
	assert.deepEqual(
		rows.map((row) => [row['id'], objectValue(row['outcome'])['kind']]),
		[
			['first', kind],
			['first-copy', kind],
		],
	);
	assert.equal(typeof rows[0]?.['result_id'], 'number');
	assert.equal(
		rows[0]?.['result_id'],
		rows[1]?.['result_id'],
		'Duplicate binding did not share an owner',
	);
	const runs = arrayValue(report['runs']);
	assert.equal(runs.length, 1);
	assert.equal(arrayValue(objectValue(objectValue(runs[0])['report'])['requests']).length, 1);
}

export async function observeDuplicateByteBinding(options: {
	readonly observer: RulebookProcess;
	readonly root: string;
	readonly evidence: string;
	readonly revision: string;
}): Promise<void> {
	const caseFile = path.join(options.evidence, 'duplicate-byte-cases.json');
	const first = {
		...byteFixtureCases[0],
		kind: 'rule',
		rule: 'approval',
		file: byteFixturePath,
		revision: options.revision,
		lines: [1, 1],
		set: 'development',
		label_from: 'blind-agents',
	};
	writeJson(caseFile, {
		repository: options.root,
		cases: [first, { ...first, id: 'first-copy' }],
		bars: [],
	});
	const preparedPath = path.join(options.evidence, 'duplicate-prepared.json');
	const prepared = await options.observer.run({
		root: options.root,
		name: 'duplicate-prepare',
		arguments: [
			'lint',
			'--config',
			'sunstack.lint.json',
			'cases',
			caseFile,
			'--prepare',
			preparedPath,
		],
		expectedExit: 6,
	});
	assert.equal(prepared.captures.length, 0);
	checkDuplicateReport(preparedPath, 'prepared');
	const scoredPath = path.join(options.evidence, 'duplicate-scored.json');
	const scored = await options.observer.run({
		root: options.root,
		name: 'duplicate-score',
		arguments: [
			'lint',
			'--config',
			'sunstack.lint.json',
			'--report',
			scoredPath,
			'cases',
			caseFile,
		],
		credential: true,
		expectedExit: 1,
	});
	assert.equal(scored.captures.length, 1);
	checkDuplicateReport(scoredPath, 'right');
}
