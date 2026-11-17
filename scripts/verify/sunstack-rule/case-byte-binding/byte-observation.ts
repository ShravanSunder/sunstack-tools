/** Compare case links to their own saved request owner, including original byte offsets. */
import assert from 'node:assert/strict';

import {
	arrayValue,
	jsonObjectSchema,
	objectValue,
	textValue,
} from '../rulebook/rulebook-fixture-support.ts';
import { byteFixtureCases, byteFixturePath } from './byte-fixture.ts';

export function requireByteCaseDetail(detail: unknown, id: string, revision: string): void {
	const expected = byteFixtureCases.find((item) => item.id === id);
	assert.ok(expected, `Unknown fixture case ${id}`);
	const shown = jsonObjectSchema.parse(detail);
	const caseRow = objectValue(shown['case']);
	assert.equal(caseRow['id'], id);
	assert.equal(shown['revision'], revision);
	const saved = objectValue(shown['detail']);
	const evidence = objectValue(saved['evidence']);
	const piece = objectValue(evidence['piece']);
	const location = objectValue(piece['where']);
	assert.equal(location['file'], byteFixturePath);
	const span = objectValue(location['span']);
	assert.deepEqual(
		[span['byte_start'], span['byte_end']],
		expected.bytes,
		`Wrong readback span for ${id}`,
	);
	assert.equal(
		objectValue(span['source'])['content_identity'],
		'0f0d416333509a1f132e311aac7aeab87886428bbce0b20b0538a1281582ab34',
	);
	const linked = objectValue(saved['request'] ?? saved['result']);
	assert.equal(linked['id'], caseRow['result_id'], 'Readback borrowed another result');
}

export function requirePreparedByteLinks(report: unknown, selectedIds: readonly string[]): void {
	const outer = jsonObjectSchema.parse(report);
	const runs = arrayValue(outer['runs']);
	assert.equal(runs.length, 1, 'Expected exactly one revision');
	const nested = objectValue(objectValue(runs[0])['report']);
	const requests = arrayValue(nested['requests']).map(objectValue);
	const rows = arrayValue(outer['cases']).map(objectValue);
	assert.equal(rows.length, selectedIds.length, 'Wrong selected case count');
	assert.equal(requests.length, selectedIds.length, 'Wrong prepared request count');
	const seen = new Set<number>();
	for (const id of selectedIds) {
		const expected = byteFixtureCases.find((item) => item.id === id);
		assert.ok(expected, `Unknown fixture case ${id}`);
		const row = rows.find((item) => item['id'] === id);
		assert.ok(row, `Missing case ${id}`);
		assert.equal(objectValue(row['outcome'])['kind'], 'prepared');
		const matching = requests.filter((request) => request['id'] === row['result_id']);
		assert.equal(matching.length, 1, `Case ${id} does not link one request`);
		const request = matching[0];
		assert.ok(request);
		const requestId = request['id'];
		assert.ok(typeof requestId === 'number');
		assert.ok(!seen.has(requestId), 'Distinct pieces borrowed a shared request');
		seen.add(requestId);
		const owner = objectValue(request['owner']);
		assert.equal(owner['kind'], 'rule');
		assert.equal(textValue(owner['rule']), 'approval');
		const piece = objectValue(owner['piece']);
		assert.equal(piece['file'], byteFixturePath);
		assert.deepEqual([piece['first_line'], piece['last_line']], [1, 1]);
		const span = objectValue(piece['span']);
		assert.deepEqual(
			[span['byte_start'], span['byte_end']],
			expected.bytes,
			`Wrong span for ${id}`,
		);
	}
}
