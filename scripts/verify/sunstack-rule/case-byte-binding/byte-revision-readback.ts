/** Saved case readback must resolve a colliding nested result inside its own revision. */
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

export async function observeRevisionReadback(options: {
	readonly observer: RulebookProcess;
	readonly root: string;
	readonly evidence: string;
	readonly firstRevision: string;
	readonly secondRevision: string;
}): Promise<void> {
	const definitions = [
		{
			id: 'revision-first',
			revision: options.firstRevision,
			source: '0f0d416333509a1f132e311aac7aeab87886428bbce0b20b0538a1281582ab34',
		},
		{
			id: 'revision-second',
			revision: options.secondRevision,
			source: 'db2cb4db436ae30b98f3cd3254c4891f7269b198cc94386614a97470288d7e8c',
		},
	] as const;
	assert.notEqual(definitions[0].revision, definitions[1].revision);
	const caseFile = path.join(options.evidence, 'revision-cases.json');
	writeJson(caseFile, {
		repository: options.root,
		cases: definitions.map((item) => ({
			...byteFixtureCases[0],
			id: item.id,
			kind: 'rule',
			rule: 'approval',
			file: byteFixturePath,
			revision: item.revision,
			lines: [1, 1],
			set: 'development',
			label_from: 'blind-agents',
		})),
		bars: [],
	});
	const observe = async (mode: 'prepare' | 'score'): Promise<void> => {
		const reportPath = path.join(options.evidence, `revision-${mode}.json`);
		const args =
			mode === 'prepare'
				? ['lint', '--config', 'sunstack.lint.json', 'cases', caseFile, '--prepare', reportPath]
				: ['lint', '--config', 'sunstack.lint.json', '--report', reportPath, 'cases', caseFile];
		const result = await options.observer.run({
			root: options.root,
			name: `revision-${mode}`,
			arguments: args,
			credential: mode === 'score',
			expectedExit: mode === 'prepare' ? 6 : 1,
		});
		assert.equal(result.captures.length, mode === 'prepare' ? 0 : 2);
		const report = parseObject(filesystem.readFileSync(reportPath, 'utf8'));
		const rows = arrayValue(report['cases']).map(objectValue);
		assert.equal(rows.length, 2);
		assert.equal(arrayValue(report['runs']).length, 2);
		const first = rows.find((row) => row['id'] === definitions[0].id);
		const second = rows.find((row) => row['id'] === definitions[1].id);
		assert.ok(first && second);
		assert.equal(typeof first['result_id'], 'number');
		assert.equal(
			first['result_id'],
			second['result_id'],
			'Fixture did not produce colliding nested IDs',
		);
		const show = async (item: (typeof definitions)[number]): Promise<void> => {
			const output = await options.observer.run({
				root: options.root,
				name: `revision-${mode}-show-${item.id}`,
				arguments: ['lint', 'show', reportPath, item.id],
				expectedExit: 0,
			});
			const detail = parseObject(output.stdout.toString('utf8'));
			assert.equal(detail['revision'], item.revision);
			const evidence = objectValue(objectValue(detail['detail'])['evidence']);
			const location = objectValue(objectValue(evidence['piece'])['where']);
			const span = objectValue(location['span']);
			assert.deepEqual([span['byte_start'], span['byte_end']], [0, 16]);
			assert.equal(objectValue(span['source'])['content_identity'], item.source);
		};
		await show(definitions[0]);
		await show(definitions[1]);
	};
	await observe('prepare');
	await observe('score');
}
