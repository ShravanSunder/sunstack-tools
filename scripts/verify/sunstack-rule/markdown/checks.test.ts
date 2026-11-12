import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

import { test } from 'vitest';

import {
	arrayValue,
	objectValue,
	textValue,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import { requireLinks, requireSections } from './checks.ts';
import { controlsMarkdown, repositories, sectionFiles, type Link } from './oracles.ts';

const raw = Buffer.from(controlsMarkdown);
const control = sectionFiles.find((file) => file.file === 'controls.md');
assert.ok(control);
const sections = control.sections;
const source = {
	kind: 'document',
	repository: 'checked_repository',
	path: 'input.md',
	content_identity: createHash('sha256').update(raw).digest('hex'),
};
function field(ranges: readonly (readonly [number, number])[]): JsonObject {
	return {
		state: 'text',
		value: ranges.map(([start, end]) => raw.subarray(start, end).toString()).join(''),
		spans: ranges.map(([start, end]) => ({ source, byte_start: start, byte_end: end })),
	};
}
function pieces(): JsonObject[] {
	return sections.map((section) => {
		const parent = section.parent === null ? undefined : sections[section.parent];
		return {
			where: {
				file: 'input.md',
				span: { source, byte_start: section.start, byte_end: section.end },
			},
			text: raw.subarray(section.start, section.end).toString(),
			fields: {
				full: field([[section.start, section.end]]),
				own: field(section.own),
				parent: parent ? field(parent.own) : { state: 'absent' },
			},
		};
	});
}
test('accepts separated list-own segments and quote sections extending beyond their quote', () => {
	assert.ok(sections.some((section) => section.own.length > 1));
	assert.ok(sections.some((section) => section.quote));
	requireSections(pieces(), sections, raw);
});
test('rejects discarded post-list own prose, duplicate owners and a quote-end clamp', () => {
	const discarded = pieces();
	const first = discarded[0];
	assert.ok(first);
	const root = sections[0];
	assert.ok(root);
	objectValue(first['fields'])['own'] = field(root.own.slice(0, 1));
	assert.throws(() => requireSections(discarded, sections, raw));
	const duplicate = pieces();
	duplicate[1] = duplicate[0] ?? {};
	assert.throws(() => requireSections(duplicate, sections, raw));
	const clamped = pieces();
	const quote = clamped.find((piece) => textValue(piece['text']).startsWith('### Quote'));
	assert.ok(quote);
	objectValue(objectValue(quote['where'])['span'])['byte_end'] = 123;
	assert.throws(() => requireSections(clamped, sections, raw));
});
function linkFixture(): {
	links: Link[];
	originals: ReadonlyMap<string, Buffer>;
	report: JsonObject;
} {
	const all = repositories.flatMap((repository) => repository.links);
	const selected = ['found', 'missing', 'missing_file'].map((expected) => {
		const link = all.find((item) => item.expected === expected);
		assert.ok(link);
		return link;
	});
	const original = Buffer.from(`文 ${selected.map((link) => link.raw).join(' plus ')} tail`);
	const digest = createHash('sha256').update(original).digest('hex');
	const links = selected.map((link) => {
		const start = original.indexOf(link.raw);
		return Object.assign({}, link, {
			source: 'input.md',
			source_sha256: digest,
			start,
			end: start + Buffer.byteLength(link.raw),
		});
	});
	const report: JsonObject = {
		usage: { model_requests: 0 },
		incomplete: [],
		results: [{ id: 1, outcome: 'finding' }],
		piece_evidence: {
			'1': {
				piece: {
					where: {
						file: 'input.md',
						span: {
							byte_start: 0,
							byte_end: original.length,
							source: { path: 'input.md', content_identity: digest },
						},
					},
					text: original.toString(),
					fields: {
						links: {
							state: 'targets',
							values: links.map((link) => ({
								name: link.url,
								path: link.target,
								exists: link.expected !== 'missing_file',
								...(link.expected === 'missing_file'
									? {}
									: { anchor: { status: link.expected, fragment: link.fragment } }),
							})),
						},
					},
				},
			},
		},
	};
	return { links, originals: new Map([['input.md', original]]), report };
}
test('accepts all three link states in one real inline block; rejects incomplete and duplicate accounting', () => {
	const fixture = linkFixture();
	requireLinks(fixture.report, fixture.links, fixture.originals);
	const unsupported = structuredClone(fixture.report);
	const piece = objectValue(objectValue(objectValue(unsupported['piece_evidence'])['1'])['piece']);
	const target = objectValue(
		arrayValue(objectValue(objectValue(piece['fields'])['links'])['values'])[0],
	);
	objectValue(target['anchor'])['status'] = 'unsupported';
	assert.throws(() => requireLinks(unsupported, fixture.links, fixture.originals));
	const duplicate = structuredClone(fixture.report);
	arrayValue(duplicate['results']).push({ id: 2, outcome: 'finding' });
	const retained = objectValue(duplicate['piece_evidence']);
	retained['2'] = retained['1'] ?? {};
	assert.throws(() => requireLinks(duplicate, fixture.links, fixture.originals));
	const dropped = structuredClone(fixture.report);
	const owner = objectValue(objectValue(objectValue(dropped['piece_evidence'])['1'])['piece']);
	arrayValue(objectValue(objectValue(owner['fields'])['links'])['values']).pop();
	assert.throws(() => requireLinks(dropped, fixture.links, fixture.originals));
});
