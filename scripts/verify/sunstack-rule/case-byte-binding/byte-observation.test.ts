import assert from 'node:assert/strict';

import { test } from 'vitest';

import { requireByteCaseDetail, requirePreparedByteLinks } from './byte-observation.ts';

function request(id: number, byteStart: number, byteEnd: number): object {
	return {
		id,
		owner: {
			kind: 'rule',
			rule: 'approval',
			piece: {
				file: 'docs/pieces.html',
				first_line: 1,
				last_line: 1,
				span: { byte_start: byteStart, byte_end: byteEnd },
			},
		},
	};
}
function preparedReport(): object {
	return {
		cases: [
			{ id: 'first', result_id: 1, outcome: { kind: 'prepared' } },
			{ id: 'second', result_id: 2, outcome: { kind: 'prepared' } },
		],
		runs: [{ report: { requests: [request(1, 0, 16), request(2, 16, 25)] } }],
	};
}

test('prepared case links reject same-line result borrowing and swapped spans', () => {
	const report = preparedReport();
	assert.doesNotThrow(() => requirePreparedByteLinks(report, ['first', 'second']));
	const swapped = structuredClone(report);
	const text = JSON.stringify(swapped).replace('"result_id":1', '"result_id":2');
	assert.throws(() => requirePreparedByteLinks(JSON.parse(text), ['first', 'second']));
	const wrongSpan = JSON.stringify(report).replace('"byte_start":16', '"byte_start":15');
	assert.throws(() => requirePreparedByteLinks(JSON.parse(wrongSpan), ['first', 'second']));
});

test('case readback rejects borrowed evidence and a changed source identity', () => {
	const detail = {
		case: { id: 'first', result_id: 7 },
		revision: 'revision-a',
		detail: {
			result: { id: 7 },
			evidence: {
				piece: {
					where: {
						file: 'docs/pieces.html',
						span: {
							byte_start: 0,
							byte_end: 16,
							source: {
								content_identity:
									'0f0d416333509a1f132e311aac7aeab87886428bbce0b20b0538a1281582ab34',
							},
						},
					},
				},
			},
		},
	};
	assert.doesNotThrow(() => requireByteCaseDetail(detail, 'first', 'revision-a'));
	const borrowed = structuredClone(detail);
	borrowed.detail.evidence.piece.where.span.byte_start = 16;
	assert.throws(() => requireByteCaseDetail(borrowed, 'first', 'revision-a'));
	const changed = structuredClone(detail);
	changed.detail.evidence.piece.where.span.source.content_identity = 'other';
	assert.throws(() => requireByteCaseDetail(changed, 'first', 'revision-a'));
});
