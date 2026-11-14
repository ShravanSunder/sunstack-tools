import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Configuration, EvidenceKindDefinition } from '../src/index.ts';
import { check, deserialize, inspect, serialize } from '../src/index.ts';
import { ownedDirectory, writeFiles } from './support/sdk-fixtures.ts';

delete process.env['OPENROUTER_API_KEY'];
process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';

function configuration(base: string, kind: EvidenceKindDefinition): Configuration {
	return {
		id: 'section-levels',
		base_directory: base,
		marker_prefix: 'K',
		check: { files: ['docs/page.*'] },
		decision_model: { model: 'jev-latest' },
		evidence: {
			kind: 'inline',
			utilities: {},
			matchers: {},
			html: { embedded_markdown: [{ tag: 'script', attributes: { type: 'text/markdown' } }] },
			kinds: { section: kind },
		},
		rulebooks: [
			{
				kind: 'inline',
				id: 'sections',
				markdown: 'Section content is complete.<!-- K001 -->\n',
				entries: {
					K001: {
						name: 'section',
						evidence: 'section',
						decision: {
							start: 'complete',
							steps: {
								complete: {
									mechanical_check: { tool: 'complete', field: 'body' },
									yes: 'pass',
									no: { finding: 'Incomplete.' },
								},
							},
						},
					},
				},
			},
		],
	};
}

await test('section level predicates preserve complete source spans through the addon and codec', async () => {
	await Promise.all(
		['html', 'markdown', 'island'].map(async (surface) => {
			const repository = ownedDirectory(`section-levels-${surface}`);
			try {
				const html = surface === 'html';
				const raw = html
					? '<h3>Keep wrong-level</h3><h2>Keep correct-level</h2><h3>Child</h3><p>Child body</p><h2>Other</h2>'
					: '### Keep wrong-level\n\nKeep correct-level\n------------------\n\n### Child\n\nChild body\n\nOther\n=====\n';
				const page = surface === 'island' ? `<script type='text/markdown'>\n${raw}</script>` : raw;
				writeFiles(repository.path, {
					[surface === 'markdown' ? 'docs/page.md' : 'docs/page.html']: page,
				});
				const selector = {
					mode: 'unit',
					units: ['section'],
					levels: [2],
					starts_with: 'Keep',
				} as const;
				const fields = {
					body: { from: 'section', text: 'full' },
					own_body: { from: 'section', text: 'own' },
				} as const;
				const kind: EvidenceKindDefinition = html
					? { html_units: selector, fields }
					: { markdown_units: selector, fields };
				const config = configuration(repository.path, kind);
				assert.equal((await check(config)).kind, 'valid');
				const result = await inspect(config, { root: repository.path, scope: { kind: 'all' } });
				assert.equal(result.kind, 'inspected');
				if (result.kind !== 'inspected') throw new Error('inspection failed');
				assert.equal(result.report.pieces.length, 1);
				const piece = result.report.pieces[0];
				assert.ok(piece !== undefined);
				const start = page.indexOf(html ? '<h2>Keep correct' : 'Keep correct-level');
				const end = page.indexOf(html ? '<h2>Other' : 'Other\n');
				assert.equal(piece.text, page.slice(start, end));
				assert.equal(piece.where.span.byte_start, BigInt(Buffer.byteLength(page.slice(0, start))));
				assert.equal(piece.where.span.byte_end, BigInt(Buffer.byteLength(page.slice(0, end))));
				const own = piece.fields['own_body'];
				assert.equal(own?.state, 'text');
				if (own?.state !== 'text') throw new Error('own field unavailable');
				assert.doesNotMatch(
					own.value,
					/Child body/u,
					'filtering emitted sections must not erase child boundaries',
				);
				const saved = serialize({ kind: 'inspection_outcome', value: result });
				assert.equal(saved.kind, 'serialized');
				if (saved.kind !== 'serialized') throw new Error('serialization failed');
				const restored = await deserialize(saved.json);
				assert.equal(restored.kind, 'decoded');
				if (restored.kind !== 'decoded') throw new Error('decoding failed');
				assert.deepEqual(restored.value, { kind: 'inspection_outcome', value: result });
				const refused = await Promise.all(
					[[], [0], [7], [2, 2]].map((levels) => {
						const selection = { ...selector, levels };
						const invalidKind: EvidenceKindDefinition = html
							? { html_units: selection, fields }
							: { markdown_units: selection, fields };
						return check(configuration(repository.path, invalidKind));
					}),
				);
				assert.deepEqual(
					refused.map((outcome) => outcome.kind),
					['invalid', 'invalid', 'invalid', 'invalid'],
				);
			} finally {
				repository.remove();
			}
		}),
	);
});
