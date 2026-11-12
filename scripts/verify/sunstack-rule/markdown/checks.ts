/** Exact source owners, target states and non-contiguous section fields; counts cannot pass alone. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import path from 'node:path';

import { savedPiece } from '../html-evidence/html-checks.ts';
import {
	arrayValue,
	objectValue,
	numberValue,
	textValue,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import type { Link, Section } from './oracles.ts';

export function requireLinks(
	report: JsonObject,
	links: readonly Link[],
	originals: ReadonlyMap<string, Buffer>,
): void {
	assert.equal(objectValue(report['usage'])['model_requests'], 0);
	assert.deepEqual(report['incomplete'], []);
	const rows = arrayValue(report['results']).map(objectValue);
	assert.ok(rows.length > 0, 'No original link blocks were checked');
	const seen = new Set<string>();
	for (const row of rows) {
		const piece = savedPiece(report, row);
		const location = objectValue(piece['where']);
		const span = objectValue(location['span']);
		const file = textValue(location['file']);
		const start = numberValue(span['byte_start']);
		const end = numberValue(span['byte_end']);
		const expected = links
			.filter((link) => link.source === file && link.start >= start && link.end <= end)
			.toSorted((left, right) => left.start - right.start);
		assert.ok(expected.length > 0, `Unexpected original link block ${JSON.stringify(location)}`);
		const original = originals.get(file);
		assert.ok(original);
		assert.equal(piece['text'], original.subarray(start, end).toString('utf8'));
		const source = objectValue(span['source']);
		assert.equal(source['path'], file);
		assert.equal(source['content_identity'], createHash('sha256').update(original).digest('hex'));
		assert.equal(
			row['outcome'],
			expected.every((link) => link.expected === 'found') ? 'pass' : 'finding',
			file,
		);
		const field = objectValue(objectValue(piece['fields'])['links']);
		assert.equal(field['state'], 'targets');
		const values = arrayValue(field['values']);
		assert.equal(
			values.length,
			expected.length,
			'Every original link needs exactly one target state',
		);
		for (const [index, link] of expected.entries()) {
			const key = `${link.source}:${link.start}`;
			assert.ok(!seen.has(key), 'Duplicate original link owner');
			seen.add(key);
			assert.equal(original.subarray(link.start, link.end).toString('utf8'), link.raw);
			assert.equal(source['content_identity'], link.source_sha256);
			const target = objectValue(values[index]);
			assert.equal(target['name'], link.url);
			assert.equal(path.posix.normalize(textValue(target['path'])), link.target);
			assert.equal(target['exists'], link.expected !== 'missing_file');
			if (link.expected === 'missing_file') assert.equal(target['anchor'], undefined);
			else {
				const anchor = objectValue(target['anchor']);
				assert.equal(anchor['status'], link.expected);
				assert.equal(anchor['fragment'], link.fragment);
			}
		}
	}
	assert.equal(seen.size, links.length, 'Original link denominator changed');
}
function requireField(
	field: JsonObject,
	raw: Buffer,
	ranges: readonly (readonly [number, number])[],
	source: JsonObject,
): void {
	assert.equal(field['state'], 'text');
	assert.equal(
		field['value'],
		ranges.map(([start, end]) => raw.subarray(start, end).toString('utf8')).join(''),
	);
	const spans = arrayValue(field['spans']).map(objectValue);
	assert.deepEqual(
		spans.map((span) => [span['byte_start'], span['byte_end']]),
		ranges,
	);
	for (const span of spans) assert.deepEqual(span['source'], source);
}
export function requireSections(
	pieces: readonly JsonObject[],
	sections: readonly Section[],
	raw: Buffer,
): void {
	assert.equal(pieces.length, sections.length);
	const seen = new Set<number>();
	for (const piece of pieces) {
		const location = objectValue(piece['where']);
		const span = objectValue(location['span']);
		const start = numberValue(span['byte_start']);
		const expected = sections.find((section) => section.start === start);
		assert.ok(expected, 'Unexpected section opening');
		assert.ok(!seen.has(start), 'Duplicate section owner');
		seen.add(start);
		assert.equal(span['byte_end'], expected.end, expected.label);
		assert.equal(piece['text'], raw.subarray(start, expected.end).toString('utf8'));
		const source = objectValue(span['source']);
		assert.equal(source['path'], location['file']);
		assert.equal(source['content_identity'], createHash('sha256').update(raw).digest('hex'));
		const fields = objectValue(piece['fields']);
		requireField(objectValue(fields['full']), raw, [[start, expected.end]], source);
		requireField(objectValue(fields['own']), raw, expected.own, source);
		if (expected.parent === null) assert.deepEqual(fields['parent'], { state: 'absent' });
		else {
			const parent = sections[expected.parent];
			assert.ok(parent);
			requireField(objectValue(fields['parent']), raw, parent.own, source);
		}
	}
}
