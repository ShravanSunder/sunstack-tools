import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Configuration, CountBound, SdkDocumentValue } from '../src/index.ts';
import { check, deserialize, run, serialize } from '../src/index.ts';
import { ownedDirectory, writeFiles } from './support/sdk-fixtures.ts';

delete process.env['OPENROUTER_API_KEY'];
process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';

function configuration(base: string, bound: CountBound): Configuration {
	return {
		id: 'section-measures',
		base_directory: base,
		marker_prefix: 'K',
		check: { files: ['docs/*.html'] },
		decision_model: { model: 'jev-latest' },
		evidence: {
			kind: 'inline',
			html: { embedded_markdown: [{ tag: 'script', attributes: { type: 'text/markdown' } }] },
			matchers: {
				html: { language: 'Html', rule: { kind: 'element', regex: '^<p\\b' } },
				markdown: { language: 'Markdown', rule: { kind: 'paragraph' } },
			},
			kinds: {
				outline: { html_units: { mode: 'unit', units: ['section'] }, fields: {} },
				markdown_outline: { markdown_units: { mode: 'unit', units: ['section'] }, fields: {} },
				html: { rule: 'html', fields: {} },
				markdown: { rule: 'markdown', fields: {} },
				section: {
					html_units: { mode: 'unit', units: ['section'] },
					fields: {
						html: {
							from: 'inside',
							kind: 'html',
							scope: { section_own: ['outline', 'markdown_outline'] },
						},
						markdown: {
							from: 'inside',
							kind: 'markdown',
							scope: { section_own: ['outline', 'markdown_outline'] },
						},
					},
				},
			},
		},
		rulebooks: [
			{
				kind: 'inline',
				id: 'measures',
				markdown: 'Measure section content. <!-- K001 -->\n',
				entries: {
					K001: {
						name: 'section',
						evidence: 'section',
						severity: 'warn',
						decision: {
							start: 'words',
							steps: {
								words: {
									mechanical_check: { tool: 'word_count', field: ['html', 'markdown'], bound },
									yes: 'pieces',
									no: 'pass',
								},
								pieces: {
									mechanical_check: {
										tool: 'piece_count',
										field: ['html', 'markdown'],
										bound: { greater_than: 1n },
									},
									yes: {
										finding:
											'{{word_count:html,markdown}} words in {{piece_count:html,markdown}} pieces.',
									},
									no: 'pass',
								},
							},
						},
					},
				},
			},
		],
	};
}

async function roundTrip(value: SdkDocumentValue): Promise<SdkDocumentValue> {
	const saved = serialize(value);
	assert.equal(saved.kind, 'serialized');
	if (saved.kind !== 'serialized') throw new Error('serialization failed');
	const decoded = await deserialize(saved.json);
	assert.equal(decoded.kind, 'decoded');
	if (decoded.kind !== 'decoded') throw new Error('measurement document was not decoded');
	if (value.kind !== 'configuration') assert.deepEqual(decoded.value, value);
	return decoded.value;
}

await test('real addon measures mixed content and preserves full-width bounds and finding text', async () => {
	const repository = ownedDirectory('section-measures');
	try {
		writeFiles(repository.path, {
			'docs/page.html':
				'<h2>Mixed</h2><p data-note="ignore these attribute words"><b>One</b> two three.</p><script type="text/markdown">\n**Four** [five](https://example.com/destination) `six`.\n</script><h2>Empty</h2>',
		});
		const config = configuration(repository.path, { at_least: 6n });
		assert.equal((await check(config)).kind, 'valid');
		const restored = await roundTrip({ kind: 'configuration', value: config });
		assert.equal(restored.kind, 'configuration');
		if (restored.kind !== 'configuration') throw new Error('wrong document root');
		const rulebook = restored.value.rulebooks[0];
		assert.equal(rulebook?.kind, 'inline');
		if (rulebook?.kind !== 'inline') throw new Error('wrong rulebook shape');
		assert.deepEqual(rulebook.entries['K001']?.decision.steps['words'], {
			mechanical_check: {
				tool: 'word_count',
				field: ['html', 'markdown'],
				bound: { at_least: 6n },
			},
			yes: 'pieces',
			no: 'pass',
		});
		const observed = await run(config, { root: repository.path, scope: { kind: 'all' } });
		assert.equal(observed.kind, 'completed');
		if (observed.kind !== 'completed') throw new Error('measurement run did not complete');
		assert.equal(observed.report.results.length, 2);
		assert.equal(observed.report.counts.warn, 1);
		assert.equal(observed.report.findings[0]?.message, '6 words in 2 pieces.');
		assert.equal(observed.report.usage.model_requests, 0);
		await roundTrip({ kind: 'run_outcome', value: observed });
		const wide = await roundTrip({
			kind: 'configuration',
			value: configuration(repository.path, { at_least: 9007199254740993n }),
		});
		if (wide.kind !== 'configuration' || wide.value.rulebooks[0]?.kind !== 'inline')
			throw new Error('wrong wide document shape');
		const wideStep = wide.value.rulebooks[0].entries['K001']?.decision.steps['words'];
		if (wideStep === undefined || !('mechanical_check' in wideStep))
			throw new Error('missing wide check');
		assert.deepEqual(wideStep.mechanical_check, {
			tool: 'word_count',
			field: ['html', 'markdown'],
			bound: { at_least: 9007199254740993n },
		});
		const old = await deserialize('{"contract_revision":14,"payload":{}}');
		assert.equal(old.kind, 'incompatible');
	} finally {
		repository.remove();
	}
});
