import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

import { test } from 'vitest';

import { objectValue, type JsonObject } from '../rulebook/rulebook-fixture-support.ts';
import { requireCut, requireSections } from './checks.ts';
import type { Section } from './corpus.ts';

const raw = Buffer.from('<h1>A 文</h1>Intro<h2>B</h2>Parent<h4>C</h4>Child<h1>D</h1>End');
const source = {
	kind: 'document',
	repository: 'checked_repository',
	path: 'page.html',
	content_identity: createHash('sha256').update(raw).digest('hex'),
};
const sections: Section[] = [
	{ level: 1, start: 0, end: 50, own_end: 19, parent: null, heading: 'A' },
	{ level: 2, start: 19, end: 50, own_end: 35, parent: 0, heading: 'B' },
	{ level: 4, start: 35, end: 50, own_end: 50, parent: 1, heading: 'C' },
	{ level: 1, start: 50, end: raw.length, own_end: raw.length, parent: null, heading: 'D' },
];
function textField(start: number, end: number): JsonObject {
	return {
		state: 'text',
		value: raw.subarray(start, end).toString(),
		spans: [{ source, byte_start: start, byte_end: end }],
	};
}
function pieces(): JsonObject[] {
	return sections.map((section) => {
		const parent = section.parent === null ? undefined : sections[section.parent];
		return {
			where: {
				file: 'page.html',
				span: { source, byte_start: section.start, byte_end: section.end },
			},
			text: raw.subarray(section.start, section.end).toString(),
			fields: {
				full: textField(section.start, section.end),
				own: textField(section.start, section.own_end),
				parent: parent ? textField(parent.start, parent.own_end) : { state: 'absent' },
			},
		};
	});
}
test('accepts raw nested sections with skipped levels and exact UTF8 source ranges', () => {
	requireSections({ pieces: pieces(), sections, raw });
});
test('rejects repeated headings despite the correct total', () => {
	const actual = pieces();
	actual[1] = actual[0] ?? {};
	assert.throws(() => requireSections({ pieces: actual, sections, raw }));
});
test('rejects own text that retains a child section', () => {
	const actual = pieces();
	const parent = actual[1];
	assert.ok(parent);
	objectValue(parent['fields'])['own'] = textField(19, 50);
	assert.throws(() => requireSections({ pieces: actual, sections, raw }));
});
test('rejects a grandparent intro used as the direct enclosing intro', () => {
	const actual = pieces();
	const child = actual[2];
	assert.ok(child);
	objectValue(child['fields'])['parent'] = textField(0, 19);
	assert.throws(() => requireSections({ pieces: actual, sections, raw }));
});
test('rejects correct field text with a false original span', () => {
	const actual = pieces();
	const child = actual[2];
	assert.ok(child);
	objectValue(objectValue(child['fields'])['own'])['spans'] = [{ byte_start: 33, byte_end: 48 }];
	assert.throws(() => requireSections({ pieces: actual, sections, raw }));
});
test('rejects a top-level absence mislabeled as an unavailable field', () => {
	const actual = pieces();
	const top = actual[0];
	assert.ok(top);
	objectValue(top['fields'])['parent'] = { state: 'unavailable', reason: 'no_enclosing_section' };
	assert.throws(() => requireSections({ pieces: actual, sections, raw }));
});
test('rejects matching text and byte offsets attributed to another source', () => {
	const actual = pieces();
	const child = actual[2];
	assert.ok(child);
	objectValue(objectValue(child['fields'])['own'])['spans'] = [
		{ source: { ...source, path: 'other.html' }, byte_start: 35, byte_end: 50 },
	];
	assert.throws(() => requireSections({ pieces: actual, sections, raw }));
});
test('requires cuts in the request as well as the projected view', () => {
	const piece = {
		model_view: { state: 'bounded', cut: ['full'] },
		fields: { full: { value: 'Long original text' } },
	};
	requireCut({ piece, state: { full: 'Long [cut]', 'cut-fields': ['full'] }, field: 'full' });
	assert.throws(() =>
		requireCut({ piece, state: { full: 'Long [cut]', 'cut-fields': [] }, field: 'full' }),
	);
	assert.throws(() =>
		requireCut({
			piece: { ...piece, model_view: { state: 'bounded', cut: [] } },
			state: { full: 'Long [cut]', 'cut-fields': ['full'] },
			field: 'full',
		}),
	);
});
