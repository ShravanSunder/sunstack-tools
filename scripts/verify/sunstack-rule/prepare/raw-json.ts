/** Compare persisted JSON payload bytes without parsing away their order or spelling. */
import assert from 'node:assert/strict';

import { z } from 'zod';

import { jsonValueSchema } from '../rulebook/rulebook-fixture-support.ts';

type JsonPath = readonly (string | number)[];
interface Span {
	start: number;
	end: number;
}
function isSpace(value: string | undefined): boolean {
	return value !== undefined && /[ \r\n\t]/.test(value);
}
function stringEnd(raw: string, start: number): number {
	assert.equal(raw[start], '"', 'Expected JSON string');
	for (let position = start + 1; position < raw.length; position++) {
		if (raw[position] === '\\') position++;
		else if (raw[position] === '"') return position + 1;
	}
	throw new Error('Unterminated JSON string');
}
/** Remove presentation whitespace only: strings, numeric spelling and object order survive. */
export function compactJson(raw: string): string {
	jsonValueSchema.parse(JSON.parse(raw));
	let compact = '';
	for (let position = 0; position < raw.length; position++) {
		if (raw[position] === '"') {
			const end = stringEnd(raw, position);
			compact += raw.slice(position, end);
			position = end - 1;
		} else if (!isSpace(raw[position])) compact += raw[position];
	}
	return compact;
}
/** Select original value slices; decoded values are never reserialized into the comparison. */
export function jsonSlice(raw: string, selected: JsonPath): string {
	jsonValueSchema.parse(JSON.parse(raw));
	const spans = new Map<string, Span>();
	let cursor = 0;
	function whitespace(): void {
		while (isSpace(raw[cursor])) cursor++;
	}
	function visit(location: JsonPath): void {
		whitespace();
		const start = cursor;
		const token = raw[cursor];
		if (token === '{') {
			cursor++;
			whitespace();
			while (raw[cursor] !== '}') {
				const end = stringEnd(raw, cursor);
				const key = z.string().parse(JSON.parse(raw.slice(cursor, end)));
				cursor = end;
				whitespace();
				assert.equal(raw[cursor++], ':');
				visit([...location, key]);
				whitespace();
				if (raw[cursor] !== ',') break;
				cursor++;
				whitespace();
			}
			assert.equal(raw[cursor++], '}');
		} else if (token === '[') {
			cursor++;
			whitespace();
			let index = 0;
			while (raw[cursor] !== ']') {
				visit([...location, index++]);
				whitespace();
				if (raw[cursor] !== ',') break;
				cursor++;
			}
			assert.equal(raw[cursor++], ']');
		} else if (token === '"') cursor = stringEnd(raw, cursor);
		else {
			while (cursor < raw.length && !/[\s,}\]]/.test(raw[cursor] ?? '')) cursor++;
			assert.ok(cursor > start, 'Missing JSON value');
		}
		const key = JSON.stringify(location);
		assert.ok(!spans.has(key), `Repeated JSON path ${key}`);
		spans.set(key, { start, end: cursor });
	}
	visit([]);
	whitespace();
	assert.equal(cursor, raw.length, 'Trailing JSON bytes');
	const span = spans.get(JSON.stringify(selected));
	assert.ok(span, `Missing JSON path ${JSON.stringify(selected)}`);
	return raw.slice(span.start, span.end);
}
export function requirePayloadBytes(options: {
	left: string;
	leftPath: JsonPath;
	right: string;
	rightPath: JsonPath;
	label: string;
}): void {
	assert.equal(
		compactJson(jsonSlice(options.left, options.leftPath)),
		compactJson(jsonSlice(options.right, options.rightPath)),
		options.label,
	);
}
