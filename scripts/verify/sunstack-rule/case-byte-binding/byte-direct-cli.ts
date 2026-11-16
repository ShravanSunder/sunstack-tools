/** Draft direct CLI case-selection geometry controls through the owned loopback. */
import assert from 'node:assert/strict';
import path from 'node:path';

import {
	arrayValue,
	objectValue,
	parseObject,
	writeJson,
} from '../rulebook/rulebook-fixture-support.ts';
import type { RulebookProcess } from '../rulebook/rulebook-process.ts';
import { byteFixtureCases, byteFixturePath } from './byte-fixture.ts';

export async function observeDirectByteCli(options: {
	readonly observer: RulebookProcess;
	readonly root: string;
	readonly evidence: string;
}): Promise<void> {
	const first = byteFixtureCases[0];
	const binding = {
		id: 'selected',
		kind: 'rule',
		rule: 'approval',
		file: byteFixturePath,
		lines: [1, 1],
		bytes: [0, 16],
		text_sha256: first.text_sha256,
	};
	const run = async (
		name: string,
		selected: object,
		expectedExit: number,
	): Promise<{ report: ReturnType<typeof parseObject>; captures: number }> => {
		const inputPath = path.join(options.evidence, `direct-${name}.json`);
		writeJson(inputPath, [selected]);
		const result = await options.observer.run({
			root: options.root,
			name: `direct-${name}`,
			arguments: [
				'lint',
				'--config',
				'sunstack.lint.json',
				'--all',
				'--format',
				'json',
				'--case-selection',
				inputPath,
			],
			credential: true,
			expectedExit,
		});
		return {
			report: parseObject(result.stdout.toString('utf8')),
			captures: result.captures.length,
		};
	};
	const selected = await run('selected', binding, 0);
	assert.equal(selected.captures, 1, 'Direct CLI positive did not witness a request');
	const requireInvalid = async (name: string, bytes: readonly number[]): Promise<void> => {
		const result = await run(name, { ...binding, id: name, bytes }, 2);
		assert.equal(result.captures, 0);
		assert.equal(objectValue(result.report['status'])['kind'], 'invalid');
		assert.ok(
			arrayValue(result.report['diagnostics'])
				.map(objectValue)
				.some((item) => item['code'] === 'unsupported_inputs'),
		);
	};
	await requireInvalid('reversed', [16, 0]);
	await requireInvalid('out-of-file', [0, 9999]);
	const stale = await run('stale', { ...binding, id: 'stale', text_sha256: '0'.repeat(64) }, 0);
	assert.equal(stale.captures, 0);
	const rows = arrayValue(objectValue(stale.report['case_selection'])['bindings']).map(objectValue);
	assert.equal(rows.length, 1);
	assert.equal(objectValue(rows[0]?.['outcome'])['kind'], 'stale');
}
