/** Source-derived heading values and spans, joined through immutable candidate identities. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';

import { z } from 'zod';

import {
	arrayValue,
	numberValue,
	objectValue,
	readObject,
	textValue,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';

export const headingSchema = z.object({
	file: z.string(),
	source_sha256: z.string(),
	byte_start: z.number(),
	byte_end: z.number(),
	text: z.string(),
});
export type Heading = z.infer<typeof headingSchema>;
export function headings(kind: 'html' | 'markdown'): Heading[] {
	return z
		.object({ headings: z.array(headingSchema) })
		.parse(readObject(path.join(import.meta.dirname, `${kind}-headings.json`))).headings;
}
function identity(candidate: JsonObject): string {
	return JSON.stringify(candidate['id']);
}
interface Candidate {
	descriptor: JsonObject;
	state: JsonObject;
	shared: JsonObject | undefined;
}
function candidates(report: JsonObject): Map<string, Candidate> {
	const output = new Map<string, Candidate>();
	for (const value of arrayValue(report['requests'])) {
		const record = objectValue(value);
		assert.equal(record['status'], 'prepared');
		const request = objectValue(record['request']);
		assert.equal(
			record['input_sha256'],
			createHash('sha256').update(JSON.stringify(request)).digest('hex'),
		);
		const state = objectValue(request['state']);
		const states = objectValue(state['candidates']);
		const evidence = objectValue(record['selection_evidence']);
		const descriptors = arrayValue(evidence['candidates']).map(objectValue);
		const keys = descriptors.map((descriptor) => textValue(descriptor['key']));
		assert.ok(keys.length > 0);
		assert.equal(new Set(keys).size, keys.length, 'Duplicate local candidate key');
		assert.deepEqual(
			Object.keys(states).toSorted(),
			keys.toSorted(),
			'Serialized candidate keys differ from descriptors',
		);
		const shared = evidence['shared'] === undefined ? undefined : objectValue(evidence['shared']);
		if (shared) {
			assert.deepEqual(Object.keys(state).toSorted(), ['candidates', 'shared', 'where']);
			assert.deepEqual(state['shared'], objectValue(shared['view'])['fields']);
			assert.deepEqual(state['where'], { file: shared['file'] ?? null });
		} else assert.deepEqual(Object.keys(state), ['candidates']);
		for (const descriptor of descriptors) {
			const key = identity(descriptor);
			assert.ok(!output.has(key), 'Duplicate candidate');
			const requested = objectValue(states[textValue(descriptor['key'])]);
			assert.deepEqual(
				requested,
				objectValue(descriptor['selection_view'])['fields'],
				'Serialized candidate fields differ from its offered view',
			);
			output.set(key, { descriptor, state: requested, shared });
		}
	}
	return output;
}
function sourcePosition(raw: Buffer, offset: number): { line: number; column: number } {
	const prefix = raw.subarray(0, offset).toString('utf8');
	const lines = prefix.split('\n');
	return { line: lines.length, column: Array.from(lines.at(-1) ?? '').length + 1 };
}
export function headingSpan(heading: Heading, source: JsonObject, raw: Buffer): JsonObject {
	assert.equal(createHash('sha256').update(raw).digest('hex'), heading.source_sha256);
	assert.equal(source['content_identity'], heading.source_sha256);
	assert.equal(source['path'], heading.file);
	const start = sourcePosition(raw, heading.byte_start);
	const end = sourcePosition(raw, heading.byte_end);
	return {
		source,
		byte_start: heading.byte_start,
		byte_end: heading.byte_end,
		start_line: start.line,
		start_column: start.column,
		end_line: end.line,
		end_column: end.column,
	};
}
export function requireHeadingReport(options: {
	before: JsonObject;
	after: JsonObject;
	headings: readonly Heading[];
	root: string;
	expectedCandidates: number;
	expectedEntries: number;
}): JsonObject {
	const original = candidates(options.before);
	const actual = candidates(options.after);
	assert.equal(original.size, options.expectedCandidates);
	assert.deepEqual([...actual.keys()].toSorted(), [...original.keys()].toSorted());
	const index = new Map(
		options.headings.map((heading) => [`${heading.file}:${heading.byte_start}`, heading]),
	);
	let entries = 0;
	let changed = 0;
	const affected = new Set<string>();
	for (const [key, previous] of original) {
		const candidate = actual.get(key);
		assert.ok(candidate);
		assert.deepEqual(candidate.shared, previous.shared, 'Unrelated shared input changed');
		for (const name of ['id', 'location', 'first_line'])
			assert.deepEqual(candidate.descriptor[name], previous.descriptor[name]);
		const beforeFields = objectValue(structuredClone(previous.descriptor['fields']));
		const afterFields = objectValue(structuredClone(candidate.descriptor['fields']));
		const field = objectValue(beforeFields['heading_path']);
		const actualField = objectValue(afterFields['heading_path']);
		assert.equal(field['state'], 'texts');
		assert.equal(actualField['state'], 'texts');
		const expectedText: string[] = [];
		const expectedSpans: JsonObject[] = [];
		const file = textValue(objectValue(previous.descriptor['location'])['file']);
		const raw = filesystem.readFileSync(path.join(options.root, file));
		for (const [position, value] of arrayValue(field['spans']).entries()) {
			const span = objectValue(value);
			const heading = index.get(`${file}:${numberValue(span['byte_start'])}`);
			assert.ok(heading, `Missing heading oracle ${file}`);
			expectedText.push(heading.text);
			expectedSpans.push(headingSpan(heading, objectValue(span['source']), raw));
			entries++;
			if (arrayValue(previous.state['heading_path'])[position] !== heading.text) {
				changed++;
				affected.add(file);
			}
		}
		assert.deepEqual(actualField['values'], expectedText);
		assert.deepEqual(actualField['spans'], expectedSpans);
		assert.deepEqual(
			candidate.state['heading_path'],
			expectedText,
			'Request must carry exact heading text',
		);
		const view = objectValue(candidate.descriptor['selection_view']);
		assert.equal(view['state'], 'bounded');
		assert.deepEqual(objectValue(view['fields'])['heading_path'], expectedText);
		assert.ok(!arrayValue(view['cut']).includes('heading_path'), 'Short heading path was cut');
		delete beforeFields['heading_path'];
		delete afterFields['heading_path'];
		assert.deepEqual(afterFields, beforeFields, 'Unrelated raw fields changed');
		const previousView = objectValue(previous.descriptor['selection_view']);
		for (const [name, value] of Object.entries(objectValue(previousView['fields']))) {
			if (
				name !== 'heading_path' &&
				!arrayValue(previousView['cut']).includes(name) &&
				!arrayValue(view['cut']).includes(name)
			)
				assert.deepEqual(
					objectValue(view['fields'])[name],
					value,
					'Uncut unrelated projection changed',
				);
		}
	}
	assert.equal(entries, options.expectedEntries);
	assert.equal(objectValue(options.after['usage'])['model_requests'], 0);
	return {
		candidates: actual.size,
		heading_path_entries: entries,
		changed_scalar_entries: changed,
		affected_files: [...affected].toSorted(),
	};
}
