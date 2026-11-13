/** C6: parser-located whole attributes only; byte positions and newlines are immutable. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

import {
	arrayValue,
	numberValue,
	objectValue,
	textValue,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';

export interface AttributeSpan {
	readonly start: number;
	readonly end: number;
	readonly text: string;
}
export function labelSpans(report: JsonObject, file: string, original: Buffer): AttributeSpan[] {
	const sha = createHash('sha256').update(original).digest('hex');
	const selected = arrayValue(report['pieces'])
		.map(objectValue)
		.filter((piece) => objectValue(piece['where'])['file'] === file);
	const spans = selected.map((piece) => {
		const span = objectValue(objectValue(piece['where'])['span']);
		assert.equal(objectValue(span['source'])['content_identity'], sha);
		return {
			start: numberValue(span['byte_start']),
			end: numberValue(span['byte_end']),
			text: textValue(piece['text']),
		};
	});
	return [...new Map(spans.map((span) => [`${span.start}:${span.end}`, span])).values()].toSorted(
		(left, right) => left.start - right.start,
	);
}

export function neutralizeLabels(original: Buffer, spans: readonly AttributeSpan[]): Buffer {
	const output = Buffer.from(original);
	let previous = 0;
	for (const span of spans) {
		assert.ok(Number.isInteger(span.start) && Number.isInteger(span.end));
		assert.ok(
			span.start >= previous && span.end > span.start && span.end <= original.length,
			'Invalid/overlapping parser span',
		);
		const selected = original.subarray(span.start, span.end);
		assert.equal(selected.toString('utf8'), span.text, 'Parser span points at different source');
		assert.match(
			span.text,
			/^label\s*=/iu,
			'Expected a whole label attribute, never just its value',
		);
		for (let offset = span.start; offset < span.end; offset++) {
			if (original[offset] !== 10 && original[offset] !== 13) output[offset] = 32;
		}
		previous = span.end;
	}
	assert.equal(output.length, original.length);
	return output;
}

export const labelLocatorRule = `id: label-attribute
language: Html
rule:
  kind: attribute
  all:
    - has: {kind: attribute_name, regex: '(?i)^label$'}
    - has: {kind: attribute_value, stopBy: end}
  inside:
    stopBy: end
    any:
      - {kind: start_tag, has: {kind: tag_name, regex: '(?i)^ex-mermaid$'}}
      - {kind: self_closing_tag, has: {kind: tag_name, regex: '(?i)^ex-mermaid$'}}
`;
