import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Configuration } from '../src/index.ts';
import { check, deserialize, inspect, serialize } from '../src/index.ts';
import { ownedDirectory, writeFiles } from './support/sdk-fixtures.ts';

function configuration(base: string, maxSiblings: number): Configuration {
	return {
		id: 'captures-attached-blocks',
		base_directory: base,
		marker_prefix: 'K',
		check: { files: ['docs/requirements.md'] },
		decision_model: { model: 'jev-latest' },
		evidence: {
			kind: 'inline',
			utilities: {},
			matchers: {
				subject: { language: 'Markdown', rule: { kind: 'paragraph', regex: '^U2' } },
				obligation: { language: 'Markdown', rule: { kind: 'paragraph', regex: '^R' } },
				lead: { language: 'Markdown', rule: { kind: 'paragraph', regex: '^\\*\\*D' } },
			},
			kinds: {
				subject: {
					rule: 'subject',
					fields: {
						ids: { from: 'captures', regex: '\\bU[0-9]+\\b' },
						obligations: {
							from: 'document',
							path: ['specification.md'],
							rule: 'obligation',
							containing: 'ids',
						},
						linked_ids: { from: 'captures', of: 'obligations', regex: '\\bR[0-9]+\\b' },
						blocks: {
							from: 'document',
							path: ['program-design.md'],
							rule: 'lead',
							containing: 'linked_ids',
							attach: { until: 'lead', max_siblings: maxSiblings },
						},
						names: { from: 'captures', of: 'blocks', regex: 'AttemptOutcome' },
					},
				},
			},
		},
		rulebooks: [
			{
				kind: 'inline',
				id: 'coverage',
				markdown: 'Blocks are complete.<!-- K001 -->\n',
				entries: {
					K001: {
						name: 'coverage',
						evidence: 'subject',
						decision: {
							start: 'complete',
							steps: {
								complete: {
									mechanical_check: { tool: 'complete', field: 'blocks' },
									yes: 'pass',
									no: 'uncertain',
								},
							},
						},
					},
				},
			},
		],
	};
}

await test('captures and attached blocks preserve cuts and spans across the native addon and codec', async () => {
	const repository = ownedDirectory('captures-attached-blocks');
	try {
		const block =
			'**D10 R2.** classifier\n\n```rust\nenum AttemptOutcome { NoResponse }\n```\n\nClosing paragraph.\n';
		writeFiles(repository.path, {
			'docs/requirements.md': 'U2, U2 need\n',
			'docs/specification.md': 'R2 covers U2\n\nR20 covers U20\n',
			'docs/program-design.md': `${block}\n**D11 R20.** unrelated\n`,
		});
		await Promise.all(
			(
				[
					[12, false],
					[1, true],
				] as const
			).map(async ([cap, expectedCut]) => {
				const config = configuration(repository.path, cap);
				assert.equal((await check(config)).kind, 'valid');
				const outcome = await inspect(config, { root: repository.path, scope: { kind: 'all' } });
				assert.equal(outcome.kind, 'inspected');
				if (outcome.kind !== 'inspected') throw new Error('inspection failed');
				const piece = outcome.report.pieces[0];
				assert.ok(piece !== undefined);
				const ids = piece.fields['ids'];
				assert.ok(ids?.state === 'texts');
				assert.deepEqual(ids.values, ['U2']);
				const blocks = piece.fields['blocks'];
				assert.ok(blocks?.state === 'document');
				assert.equal(blocks.cut ?? false, expectedCut);
				assert.match(blocks.values[0] ?? '', /enum AttemptOutcome/u);
				assert.doesNotMatch(blocks.values[0] ?? '', /D11/u);
				if (!expectedCut) assert.equal(blocks.values[0]?.trimEnd(), block.trimEnd());
				const names = piece.fields['names'];
				assert.ok(names?.state === 'texts');
				assert.deepEqual(names.values, ['AttemptOutcome']);
				assert.equal(names.cut ?? false, expectedCut);
				assert.equal(piece.model_view.state, 'bounded');
				if (piece.model_view.state === 'bounded')
					assert.equal(piece.model_view.cut.includes('names'), expectedCut);
				const saved = serialize({ kind: 'inspection_outcome', value: outcome });
				assert.equal(saved.kind, 'serialized');
				if (saved.kind !== 'serialized') throw new Error('serialization failed');
				const restored = await deserialize(saved.json);
				assert.equal(restored.kind, 'decoded');
				if (restored.kind !== 'decoded') throw new Error('decode failed');
				assert.deepEqual(restored.value, { kind: 'inspection_outcome', value: outcome });
			}),
		);
		assert.equal((await check(configuration(repository.path, 0))).kind, 'invalid');
	} finally {
		repository.remove();
	}
});
