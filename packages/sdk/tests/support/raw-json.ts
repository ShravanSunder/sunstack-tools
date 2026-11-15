// Select original JSON token bytes without reserializing exact integers or strings.
import assert from 'node:assert/strict';

type JsonPath = readonly (string | number)[];
function stringEnd(raw: string, start: number): number {
	assert.equal(raw[start], '"');
	for (let position = start + 1; position < raw.length; position += 1) {
		if (raw[position] === '\\') position += 1;
		else if (raw[position] === '"') return position + 1;
	}
	throw new Error('Unterminated JSON string');
}
/** The input is SDK-serialized data; JSON parsing validates syntax only, never the slice. */
export function jsonSlice(raw: string, selected: JsonPath): string {
	JSON.parse(raw);
	const spans = new Map<string, { readonly start: number; readonly end: number }>();
	let cursor = 0;
	function whitespace(): void {
		while (/[ \r\n\t]/u.test(raw[cursor] ?? '') && cursor < raw.length) cursor += 1;
	}
	function visit(location: JsonPath): void {
		whitespace();
		const start = cursor;
		const token = raw[cursor];
		if (token === '{') {
			cursor += 1;
			whitespace();
			while (raw[cursor] !== '}') {
				const end = stringEnd(raw, cursor);
				const key: unknown = JSON.parse(raw.slice(cursor, end));
				assert.equal(typeof key, 'string');
				if (typeof key !== 'string') throw new Error('Non-string JSON key');
				cursor = end;
				whitespace();
				assert.equal(raw[cursor], ':');
				cursor += 1;
				visit([...location, key]);
				whitespace();
				if (raw[cursor] !== ',') break;
				cursor += 1;
				whitespace();
			}
			assert.equal(raw[cursor], '}');
			cursor += 1;
		} else if (token === '[') {
			cursor += 1;
			whitespace();
			let index = 0;
			while (raw[cursor] !== ']') {
				visit([...location, index]);
				index += 1;
				whitespace();
				if (raw[cursor] !== ',') break;
				cursor += 1;
			}
			assert.equal(raw[cursor], ']');
			cursor += 1;
		} else if (token === '"') cursor = stringEnd(raw, cursor);
		else {
			while (cursor < raw.length && !/[\s,}\]]/u.test(raw[cursor] ?? '')) cursor += 1;
			assert.ok(cursor > start);
		}
		const key = JSON.stringify(location);
		assert.ok(!spans.has(key), `Repeated JSON path ${key}`);
		spans.set(key, { start, end: cursor });
	}
	visit([]);
	whitespace();
	assert.equal(cursor, raw.length);
	const span = spans.get(JSON.stringify(selected));
	assert.ok(span !== undefined, `Missing JSON path ${JSON.stringify(selected)}`);
	return raw.slice(span.start, span.end);
}
