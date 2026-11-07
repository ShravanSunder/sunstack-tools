import { expect, test } from 'vitest';

import { compactJson, jsonSlice, requirePayloadBytes } from './raw-json.ts';

const report = String.raw`{
 "requests": [{"request": {"state": {"10":"ten","2":"two","text":"☃ \" quoted\n", "values": ["first", "second"]},"questions":{"verdict":{"type":"noul"}}}}]
}`;
const sent = String.raw`{"model":"synthetic/pin","state":{"10":"ten","2":"two","text":"☃ \" quoted\n","values":["first","second"]},"questions":{"verdict":{"type":"noul"}}}`;
function compare(right: string): void {
	requirePayloadBytes({
		left: report,
		leftPath: ['requests', 0, 'request', 'state'],
		right,
		rightPath: ['state'],
		label: 'Payload differs',
	});
}
test('persisted and captured values match despite presentation whitespace', () => {
	expect(() => compare(sent)).not.toThrow();
	expect(jsonSlice(report, ['requests', 0, 'request', 'questions'])).toBe(
		'{"verdict":{"type":"noul"}}',
	);
});
test.each([
	['key order', sent.replace('"10":"ten","2":"two"', '"2":"two","10":"ten"')],
	['list order', sent.replace('["first","second"]', '["second","first"]')],
	['evidence value', sent.replace('"ten"', '"changed"')],
	['missing field', sent.replace('"10":"ten",', '')],
])('byte comparison rejects changed %s', (_label, changed) => {
	expect(() => compare(changed)).toThrow('Payload differs');
});
test('compaction preserves numeric spelling, escapes and spaces inside strings', () => {
	expect(compactJson(' { "value" : 1.00, "text": "two  spaces" } ')).toBe(
		'{"value":1.00,"text":"two  spaces"}',
	);
	expect(compactJson(String.raw`{"text":"\u2603"}`)).not.toBe(compactJson('{"text":"☃"}'));
});
test('missing, repeated and malformed paths refuse instead of yielding empty matches', () => {
	expect(() => jsonSlice(report, ['requests', 1])).toThrow('Missing JSON path');
	expect(() => jsonSlice('{"state":1,"state":2}', ['state'])).toThrow('Repeated JSON path');
	expect(() => compactJson('{broken')).toThrow();
});
