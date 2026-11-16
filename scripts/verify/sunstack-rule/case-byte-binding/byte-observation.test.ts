import assert from 'node:assert/strict';

import { test } from 'vitest';

import { requirePreparedByteLinks } from './byte-observation.ts';

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
