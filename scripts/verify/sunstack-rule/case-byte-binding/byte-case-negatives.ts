/** Archived-byte negative cases through preparation and scoring. */
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
import { byteFixturePath } from './byte-fixture.ts';

export async function observeArchivedByteNegatives(options: {
	readonly observer: RulebookProcess;
	readonly root: string;
	readonly evidence: string;
	readonly revision: string;
}): Promise<void> {
	const observe = async (
		name: string,
		bytes: readonly number[],
		digest: string,
		expected: string,
	): Promise<void> => {
		const cases = path.join(options.evidence, `negative-${name}.json`);
		writeJson(cases, {
			repository: options.root,
			cases: [
				{
					id: name,
					kind: 'rule',
					rule: 'approval',
					file: byteFixturePath,
					revision: options.revision,
					lines: [1, 1],
					bytes,
					text_sha256: digest,
					expected: 'pass',
					set: 'development',
					label_from: 'blind-agents',
				},
			],
			bars: [],
		});
		const caseArguments = [
			'lint',
			'--config',
			'sunstack.lint.json',
			'cases',
			cases,
			'--set',
			'development',
		];
		const verify = (raw: string): void => {
			const rows = arrayValue(parseObject(raw)['cases']).map(objectValue);
			assert.equal(rows.length, 1);
			assert.equal(rows[0]?.['id'], name);
			assert.equal(objectValue(rows[0]?.['outcome'])['kind'], expected);
			assert.equal(rows[0]?.['result_id'], null);
		};
		const preparedPath = path.join(options.evidence, `negative-${name}-prepared.json`);
		const prepared = await options.observer.run({
			root: options.root,
			name: `negative-${name}-prepare`,
			arguments: [...caseArguments, '--prepare', preparedPath],
			expectedExit: 1,
		});
		assert.equal(prepared.captures.length, 0);
		verify(filesystem.readFileSync(preparedPath, 'utf8'));
		const scoredPath = path.join(options.evidence, `negative-${name}-scored.json`);
		const scored = await options.observer.run({
			root: options.root,
			name: `negative-${name}-score`,
			arguments: [
				'lint',
				'--config',
				'sunstack.lint.json',
				'--report',
				scoredPath,
				...caseArguments.slice(3),
			],
			credential: true,
			expectedExit: 1,
		});
		assert.equal(scored.captures.length, 0);
		verify(filesystem.readFileSync(scoredPath, 'utf8'));
	};
	await observe('wrong-hash', [0, 16], '0'.repeat(64), 'stale');
	await observe('wrong-hash-no-piece', [1, 16], '0'.repeat(64), 'stale');
	await observe(
		'one-byte-off',
		[1, 16],
		'f64b2130d1c3df72db6b12d7f3379df716cb662c5d752cb3769cbe70f9550494',
		'no_matching_piece',
	);
	await observe(
		'partial-codepoint',
		[6, 7],
		'ae3f4619b0413d70d3004b9131c3752153074e45725be13b9a148978895e359e',
		'no_matching_piece',
	);
}
