// Copied by the permanent install gate; imports resolve only from the packed SDK.
import assert from 'node:assert/strict';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import type { Configuration, HtmlMarkdownIsland } from '@sunstack/sdk';
import { check, deserialize, inspect, run, serialize } from '@sunstack/sdk';

const root = join(process.cwd(), 'island-checked');
mkdirSync(root);
const page =
	'猫<p>Before</p><custom-guide>## Guide\n\nCafé instructions.\n</custom-guide><p>After</p>';
writeFileSync(join(root, 'page.html'), page);
const selector = { tag: 'custom-guide' } satisfies HtmlMarkdownIsland;
const configuration = {
	id: 'installed-islands',
	base_directory: root,
	marker_prefix: 'K',
	check: { files: ['*.html'] },
	decision_model: { model: 'jev-latest' },
	evidence: {
		kind: 'inline',
		html: { embedded_markdown: [selector] },
		utilities: {},
		matchers: {},
		kinds: {
			paragraph: {
				markdown_units: { mode: 'unit', units: ['paragraph'] },
				fields: { body: { from: 'remainder' } },
			},
		},
	},
	rulebooks: [
		{
			kind: 'inline',
			id: 'rules',
			markdown: 'Body is complete. <!-- K001 -->\n',
			entries: {
				K001: {
					name: 'body',
					evidence: 'paragraph',
					severity: 'error',
					decision: {
						start: 'complete',
						steps: {
							complete: {
								mechanical_check: { tool: 'complete', field: 'body' },
								yes: 'pass',
								no: { finding: 'Missing body.' },
							},
						},
					},
				},
			},
		},
	],
} as const satisfies Configuration;
assert.equal((await check(configuration)).kind, 'valid');
const options = { root, scope: { kind: 'all' as const } };
const observed = await inspect(configuration, options);
assert.equal(observed.kind, 'inspected');
if (observed.kind !== 'inspected') throw new Error('inspection failed');
assert.deepEqual(
	observed.report.pieces.map((piece) => piece.text),
	['Café instructions.\n'],
);
const piece = observed.report.pieces[0];
assert.ok(piece !== undefined);
assert.equal(
	piece.where.span.byte_start,
	BigInt(Buffer.byteLength(page.slice(0, page.indexOf('Café')))),
);
const ran = await run(configuration, options);
assert.equal(ran.kind, 'completed');
if (ran.kind !== 'completed') throw new Error('run failed');
assert.equal(ran.report.status.kind, 'clean');
assert.equal(ran.report.results.length, 1);
const saved = serialize({ kind: 'configuration', value: configuration });
assert.equal(saved.kind, 'serialized');
if (saved.kind !== 'serialized') throw new Error('save failed');
const restored = await deserialize(saved.json);
assert.equal(restored.kind, 'decoded');
if (restored.kind !== 'decoded' || restored.value.kind !== 'configuration')
	throw new Error('restore failed');
assert.deepEqual(restored.value.value.evidence.html?.embedded_markdown, [
	{ tag: 'custom-guide', attributes: {} },
]);
assert.equal((await check(restored.value.value)).kind, 'valid');
const repeated = await inspect(restored.value.value, options);
assert.equal(repeated.kind, 'inspected');
if (repeated.kind !== 'inspected') throw new Error('restored inspection failed');
assert.deepEqual(repeated.report.pieces, observed.report.pieces);
process.stdout.write(
	'Installed configured Markdown island: strict types, inspect/run, UTF-8 span and exact config readback passed.\n',
);
