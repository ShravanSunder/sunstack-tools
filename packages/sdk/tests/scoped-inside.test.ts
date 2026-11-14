// Released scoped collection through generated types and the real native SDK.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { inspect as formatValue } from 'node:util';

import type { Configuration, FieldSource, FieldValue, InspectionReport } from '../src/index.ts';
import { check, deserialize, inspect, run, serialize } from '../src/index.ts';
import { isCheckOutcome } from '../src/reply-decoding.ts';
import { ownedDirectory, writeFiles } from './support/sdk-fixtures.ts';

delete process.env['OPENROUTER_API_KEY'];
process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';

const page =
	"文<h2>Earlier</h2><img src='old'>\n<h2>Own</h2><img src='before'>\n<script type='text/markdown'>\n```mermaid\ngraph LR\n A --> B\n```\n</script>\n<ex-choice id='own'>Choose</ex-choice><img src='after'>\n<h2>Empty</h2><ex-choice id='empty'>Choose</ex-choice>\n<h2>Later</h2><img src='later'>";
const fields = {
	html_before: { from: 'inside', kind: 'html_picture', scope: { section_prefix: 'outline' } },
	markdown_before: {
		from: 'inside',
		kind: 'markdown_picture',
		scope: { section_prefix: 'outline' },
	},
	html_section: { from: 'inside', kind: 'html_picture', scope: { section: 'outline' } },
	markdown_section: { from: 'inside', kind: 'markdown_picture', scope: { section: 'outline' } },
	body: { from: 'remainder' },
} as const satisfies Readonly<Record<string, FieldSource>>;

function configuration(
	base: string,
	choiceFields: Readonly<Record<string, FieldSource>> = fields,
): Configuration {
	return {
		id: 'scoped-inside',
		base_directory: base,
		marker_prefix: 'K',
		check: { files: ['docs/*.html'] },
		decision_model: { model: 'jev-latest' },
		evidence: {
			kind: 'inline',
			utilities: {},
			html: { embedded_markdown: [{ tag: 'script', attributes: { type: 'text/markdown' } }] },
			matchers: {
				choice: { language: 'Html', rule: { kind: 'element', regex: '^<ex-choice(?: |>)' } },
				image: { language: 'Html', rule: { kind: 'element', regex: '^<img(?: |>)' } },
				fence: { language: 'Markdown', rule: { kind: 'fenced_code_block' } },
			},
			kinds: {
				outline: { html_units: { mode: 'unit', units: ['section'] }, fields: {} },
				html_picture: { rule: 'image', fields: {} },
				markdown_picture: { rule: 'fence', fields: {} },
				choice: { rule: 'choice', fields: choiceFields },
			},
		},
		rulebooks: [
			{
				kind: 'inline',
				id: 'rules',
				markdown: 'Sources are complete. <!-- K001 -->\n',
				entries: {
					K001: {
						name: 'sources',
						evidence: 'choice',
						severity: 'error',
						decision: {
							start: 'complete',
							steps: {
								complete: {
									mechanical_check: { tool: 'complete', field: 'markdown_before' },
									yes: 'pass',
									no: { finding: 'Incomplete source.' },
								},
							},
						},
					},
				},
			},
		],
	};
}

function assertTexts(
	value: FieldValue | undefined,
	expected: readonly string[],
	source: InspectionReport['pieces'][number]['where']['span']['source'],
): void {
	assert.equal(value?.state, 'texts');
	if (value?.state !== 'texts') throw new Error('expected text collection');
	assert.deepEqual(value.values, expected);
	assert.equal(value.spans.length, expected.length);
	expected.forEach((text, index) => {
		const span = value.spans[index];
		assert.ok(span !== undefined);
		const start = page.indexOf(text);
		assert.ok(start >= 0);
		assert.equal(span.byte_start, BigInt(Buffer.byteLength(page.slice(0, start))));
		assert.equal(span.byte_end, BigInt(Buffer.byteLength(page.slice(0, start + text.length))));
		assert.deepEqual(span.source, source);
	});
}

await test('scoped HTML and Markdown collections retain exact order and spans through inspect, run and codec', async () => {
	const repository = ownedDirectory('scoped-inside');
	try {
		writeFiles(repository.path, { 'docs/page.html': page });
		const config = configuration(repository.path);
		assert.equal((await check(config)).kind, 'valid');
		const options = { root: repository.path, scope: { kind: 'all' as const } };
		const observed = await inspect(config, options);
		assert.equal(observed.kind, 'inspected', formatValue(observed, { depth: null }));
		if (observed.kind !== 'inspected') throw new Error('inspection failed');
		const choices = observed.report.pieces.filter((piece) => piece.kind === 'choice');
		assert.equal(choices.length, 2);
		const own = choices[0];
		assert.ok(own !== undefined);
		const fence = '```mermaid\ngraph LR\n A --> B\n```\n';
		assertTexts(own.fields['html_before'], ["<img src='before'>\n"], own.where.span.source);
		assertTexts(
			own.fields['html_section'],
			["<img src='before'>\n", "<img src='after'>\n"],
			own.where.span.source,
		);
		assertTexts(own.fields['markdown_before'], [fence], own.where.span.source);
		assertTexts(own.fields['markdown_section'], [fence], own.where.span.source);
		for (const name of ['html_before', 'html_section', 'markdown_before', 'markdown_section']) {
			assertTexts(choices[1]?.fields[name], [], own.where.span.source);
		}
		const ran = await run(config, options);
		assert.equal(ran.kind, 'completed');
		if (ran.kind !== 'completed') throw new Error('run failed');
		assert.equal(ran.report.status.kind, 'clean');
		assert.equal(ran.report.results.length, 2);
		assert.deepEqual(
			Object.values(ran.report.piece_evidence).map((item) => item.piece.fields),
			choices.map((piece) => piece.fields),
		);
		const saved = serialize({ kind: 'inspection_outcome', value: observed });
		assert.equal(saved.kind, 'serialized');
		if (saved.kind !== 'serialized') throw new Error('save failed');
		const restored = await deserialize(saved.json);
		assert.equal(restored.kind, 'decoded');
		if (restored.kind !== 'decoded') throw new Error('restore failed');
		assert.deepEqual(restored.value, { kind: 'inspection_outcome', value: observed });
	} finally {
		repository.remove();
	}
});

await test('unscoped forms retain wire identity and scoped configs admit after exact codec readback', async () => {
	const repository = ownedDirectory('scoped-wire');
	try {
		const sources = {
			...fields,
			unscoped_rule: { from: 'inside', rule: 'image' },
			unscoped_kind: { from: 'inside', kind: 'html_picture' },
		} as const satisfies Readonly<Record<string, FieldSource>>;
		const value = configuration(repository.path, sources);
		const saved = serialize({ kind: 'configuration', value });
		assert.equal(saved.kind, 'serialized');
		if (saved.kind !== 'serialized') throw new Error('save failed');
		const restored = await deserialize(saved.json);
		assert.equal(restored.kind, 'decoded');
		if (restored.kind !== 'decoded' || restored.value.kind !== 'configuration')
			throw new Error('restore failed');
		assert.deepEqual(restored.value.value.evidence.kinds['choice']?.fields, sources);
		assert.equal((await check(restored.value.value)).kind, 'valid');
	} finally {
		repository.remove();
	}
});

await test('Rust owns scoped null, shape and dependency refusals at admission and codec boundaries', async () => {
	const repository = ownedDirectory('scoped-refusals');
	try {
		const config = configuration(repository.path);
		const refused = [
			{ from: 'inside', rule: 'image', scope: { section: 'outline' } },
			{ from: 'inside', scope: { section: 'outline' } },
			{ from: 'inside', kind: 'html_picture', scope: null },
			{
				from: 'inside',
				kind: 'html_picture',
				scope: { section: 'outline', section_prefix: 'outline' },
			},
			{ from: 'inside', kind: 'html_picture', scope: { unknown: 'outline' } },
			{ from: 'inside', kind: 'html_picture', scope: { section: 'outline' }, text: 'full' },
		];
		await Promise.all(
			refused.map(async (field) => {
				const value = {
					...config,
					evidence: {
						...config.evidence,
						kinds: {
							...config.evidence.kinds,
							choice: { rule: 'choice', fields: { source: field } },
						},
					},
				};
				const checked: unknown = await Reflect.apply(check, undefined, [value]);
				assert.ok(isCheckOutcome(checked));
				assert.equal(checked.kind, 'invalid');
				const decoded = await deserialize(
					JSON.stringify({ contract_revision: 13, payload: { kind: 'configuration', value } }),
				);
				assert.equal(decoded.kind, 'invalid');
				if (decoded.kind !== 'invalid') throw new Error('invalid field decoded');
				assert.equal(decoded.reason, 'invalid_value');
			}),
		);
		await Promise.all(
			['missing', 'choice'].map(async (section) => {
				const invalid = configuration(repository.path, {
					...fields,
					source: { from: 'inside', kind: 'html_picture', scope: { section } },
				});
				assert.equal((await check(invalid)).kind, 'invalid');
			}),
		);
		await Promise.all(
			[11, 12].map(async (revision) => {
				const result = await deserialize(
					JSON.stringify({ contract_revision: revision, payload: {} }),
				);
				assert.equal(result.kind, 'incompatible');
			}),
		);
	} finally {
		repository.remove();
	}
});
