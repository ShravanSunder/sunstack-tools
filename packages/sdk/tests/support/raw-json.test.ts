import assert from 'node:assert/strict';
import { test } from 'node:test';

import { jsonSlice } from './raw-json.ts';

await test('raw slices retain wide numbers, numeric spelling, Unicode, escapes and key order', () => {
	const request = String.raw`{"questions":{"clear":"café \" } ]"},"state":{"wide":9007199254740993,"ratio":1.00,"nested":[{"value":18446744073709551615}]}}`;
	const report = `{"requests":[{"request":${request}}],"unused":0}`;
	assert.equal(jsonSlice(report, ['requests', 0, 'request']), request);
	assert.equal(jsonSlice(report, ['requests', 0, 'request', 'state', 'wide']), '9007199254740993');
	assert.equal(jsonSlice(report, ['requests', 0, 'request', 'state', 'ratio']), '1.00');
	assert.equal(
		jsonSlice(report, ['requests', 0, 'request', 'state', 'nested', 0, 'value']),
		'18446744073709551615',
	);
	assert.notEqual(
		JSON.stringify(JSON.parse(request)),
		request,
		'parse/stringify would erase the claimed tokens',
	);
});
await test('a missing or repeated selected path and malformed input are refused', () => {
	assert.throws(() => jsonSlice('{"request":1}', ['absent']), /Missing JSON path/u);
	assert.throws(() => jsonSlice('{"request":1,"request":2}', ['request']), /Repeated JSON path/u);
	assert.throws(() => jsonSlice('{"request":', ['request']));
});
