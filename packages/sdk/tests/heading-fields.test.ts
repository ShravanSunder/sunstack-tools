// Native section heading fields authored inline and executed through the addon: own and
// enclosing heading values, their source spans and order for HTML and Markdown, and the
// admission refusals Rust owns for references, same_file context and target nesting.
import assert from 'node:assert/strict';
import { test } from 'node:test';

import type {
	Configuration,
	EvidenceKindDefinition,
	FieldSource,
	SdkReport,
} from '../src/index.ts';
import { check, run } from '../src/index.ts';
import { ownedDirectory, writeFiles } from './support/sdk-fixtures.ts';

delete process.env['OPENROUTER_API_KEY'];
process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';

const headingFields = {
	heading: { from: 'section', text: 'heading' },
	heading_path: { from: 'enclosing', kind: 'outline', text: 'heading' },
	raw_body: { from: 'section', text: 'full' },
} as const satisfies Readonly<Record<string, FieldSource>>;

function headingConfiguration(
	base: string,
	file: string,
	kinds: Readonly<Record<string, EvidenceKindDefinition>>,
	subject = 'subject',
): Configuration {
	return {
		id: 'heading-fields',
		base_directory: base,
		marker_prefix: 'K',
		check: { files: [file] },
		decision_model: { model: 'jev-latest' },
		evidence: { kind: 'inline', utilities: {}, matchers: {}, kinds },
		rulebooks: [
			{
				kind: 'inline',
				id: 'heading-rules',
				markdown: 'Each section has a heading. <!-- K001 -->\n',
				entries: {
					K001: {
						name: 'has_heading',
						evidence: subject,
						severity: 'warn',
						decision: {
							start: 'complete',
							steps: {
								complete: {
									mechanical_check: { tool: 'complete', field: 'heading' },
									yes: 'pass',
									no: { finding: 'No heading.' },
								},
							},
						},
					},
				},
			},
		],
	};
}

/** A native section kind pair: `outline` is referenced, `subject` carries heading fields. */
function nativeSectionKinds(
	units: 'html_units' | 'markdown_units',
): Readonly<Record<string, EvidenceKindDefinition>> {
	const selection = { mode: 'unit', units: ['section'] } as const;
	return units === 'html_units'
		? {
				outline: { html_units: selection, fields: {} },
				subject: { html_units: selection, fields: headingFields },
			}
		: {
				outline: { markdown_units: selection, fields: {} },
				subject: { markdown_units: selection, fields: headingFields },
			};
}

/** The UTF-8 byte range of `fragment` inside `page`. */
function byteRange(page: string, fragment: string): readonly [bigint, bigint] {
	const start = Buffer.from(page).indexOf(Buffer.from(fragment));
	assert.ok(start >= 0, `${fragment} is in the page`);
	return [BigInt(start), BigInt(start + Buffer.byteLength(fragment))];
}

/** Each subject piece's heading fields: stored values, span byte ranges and asking view. */
function headingObservations(report: SdkReport): unknown[] {
	return Object.values(report.piece_evidence)
		.filter((evidence) => evidence.piece.kind === 'subject')
		.map(({ piece }) => {
			const { heading, heading_path: path } = piece.fields;
			const view = piece.model_view;
			return {
				heading:
					heading?.state === 'text'
						? [heading.value, heading.spans.map((span) => [span.byte_start, span.byte_end])]
						: heading?.state,
				path:
					path?.state === 'texts'
						? [path.values, path.spans.map((span) => [span.byte_start, span.byte_end])]
						: path?.state,
				view:
					view?.state === 'bounded'
						? [view.fields['heading'], view.fields['heading_path'], view.cut]
						: view?.state,
			};
		});
}

async function cleanRun(configuration: Configuration, root: string): Promise<SdkReport> {
	const outcome = await run(configuration, { root, scope: { kind: 'all' } });
	assert.equal(outcome.kind, 'completed');
	if (outcome.kind !== 'completed') throw new Error('unreachable');
	assert.equal(outcome.report.status.kind, 'clean');
	return outcome.report;
}

await test('HTML heading fields decode entities once and keep the heading element spans, outermost first', async () => {
	const repository = ownedDirectory('html-headings');
	try {
		// A multi-byte prefix makes byte offsets differ from string indexes.
		const page =
			'猫<h2>Review &lt;Widget&gt; &amp;lt;</h2><h3>1. <em>Read</em> addressed source</h3><p>Body</p>';
		writeFiles(repository.path, { 'docs/page.html': page });
		const report = await cleanRun(
			headingConfiguration(repository.path, 'docs/page.html', nativeSectionKinds('html_units')),
			repository.path,
		);
		const parent = byteRange(page, '<h2>Review &lt;Widget&gt; &amp;lt;</h2>');
		const child = byteRange(page, '<h3>1. <em>Read</em> addressed source</h3>');
		// Decoded once: "<Widget>" stays literal text and "&amp;lt;" becomes "&lt;", not "<".
		const parentHeading = 'Review <Widget> &lt;';
		const childHeading = '1. Read addressed source';
		assert.deepEqual(headingObservations(report), [
			{
				heading: [parentHeading, [parent]],
				// The enclosing outline includes the equal-range section itself.
				path: [[parentHeading], [parent]],
				view: [parentHeading, [parentHeading], []],
			},
			{
				heading: [childHeading, [child]],
				path: [
					[parentHeading, childHeading],
					[parent, child],
				],
				view: [childHeading, [parentHeading, childHeading], []],
			},
		]);
		// An ordinary section field stays the raw source.
		const first = Object.values(report.piece_evidence).find(
			(evidence) => evidence.piece.kind === 'subject',
		);
		const rawBody = first?.piece.fields['raw_body'];
		assert.deepEqual(
			rawBody?.state === 'text' ? rawBody.value : rawBody?.state,
			page.slice(page.indexOf('<h2>')),
		);
	} finally {
		repository.remove();
	}
});

await test('Markdown heading fields keep inline source without ATX or setext markers', async () => {
	const repository = ownedDirectory('markdown-headings');
	try {
		const page = 'Café *inline* &amp;\n===\n\n## Child `code` ###\nbody\n';
		writeFiles(repository.path, { 'docs/page.md': page });
		const report = await cleanRun(
			headingConfiguration(repository.path, 'docs/page.md', nativeSectionKinds('markdown_units')),
			repository.path,
		);
		// Spans cover the whole heading block: the setext underline and the ATX closing marks.
		const parent = byteRange(page, 'Café *inline* &amp;\n===\n');
		const child = byteRange(page, '## Child `code` ###\n');
		const parentHeading = 'Café *inline* &amp;';
		const childHeading = 'Child `code`';
		assert.deepEqual(headingObservations(report), [
			{
				heading: [parentHeading, [parent]],
				path: [[parentHeading], [parent]],
				view: [parentHeading, [parentHeading], []],
			},
			{
				heading: [childHeading, [child]],
				path: [
					[parentHeading, childHeading],
					[parent, child],
				],
				view: [childHeading, [parentHeading, childHeading], []],
			},
		]);
	} finally {
		repository.remove();
	}
});

/** A check's diagnostics as [code, json_path, message]. */
async function admission(configuration: Configuration): Promise<unknown> {
	const outcome = await check(configuration);
	return outcome.kind === 'invalid'
		? outcome.diagnostics.map((diagnostic) => [
				diagnostic.code,
				diagnostic.json_path,
				diagnostic.message,
			])
		: outcome.kind;
}

const enclosingRequiresSection =
	'kind candidate field heading_path: enclosing heading requires a native section kind (html_units or markdown_units with unit mode and units exactly [section])';

await test('Rust refuses heading text from non-section references, same_file context and target nesting', async () => {
	const base = ownedDirectory('heading-refusals');
	try {
		const section = { mode: 'unit', units: ['section'] } as const;
		const enclosingHeading = {
			heading: { from: 'section', text: 'heading' },
			heading_path: { from: 'enclosing', kind: 'outline', text: 'heading' },
		} as const;
		const configure = (kinds: Readonly<Record<string, EvidenceKindDefinition>>): Configuration => {
			const configuration = headingConfiguration(base.path, 'docs/*.md', kinds, 'candidate');
			return {
				...configuration,
				evidence: {
					kind: 'inline',
					utilities: {},
					matchers: { paragraph: { language: 'Markdown', rule: { kind: 'paragraph' } } },
					kinds,
				},
			};
		};
		const cases: readonly (readonly [
			string,
			Readonly<Record<string, EvidenceKindDefinition>>,
			unknown,
		])[] = [
			[
				'native section reference (control)',
				{
					outline: { markdown_units: section, fields: {} },
					candidate: { markdown_units: section, fields: enclosingHeading },
				},
				'valid',
			],
			[
				'AST rule kind reference',
				{
					outline: { rule: 'paragraph', fields: {} },
					candidate: { markdown_units: section, fields: enclosingHeading },
				},
				[['syntax', ['evidence.kinds.candidate.fields.heading_path'], enclosingRequiresSection]],
			],
			[
				'paragraph units reference',
				{
					outline: { markdown_units: { mode: 'unit', units: ['paragraph'] }, fields: {} },
					candidate: { markdown_units: section, fields: enclosingHeading },
				},
				[['syntax', ['evidence.kinds.candidate.fields.heading_path'], enclosingRequiresSection]],
			],
			[
				'mixed units reference',
				{
					outline: {
						markdown_units: { mode: 'unit', units: ['section', 'paragraph'] },
						fields: {},
					},
					candidate: { markdown_units: section, fields: enclosingHeading },
				},
				[['syntax', ['evidence.kinds.candidate.fields.heading_path'], enclosingRequiresSection]],
			],
			[
				'same_file first_line (control)',
				{
					outline: { markdown_units: section, fields: {} },
					candidate: {
						markdown_units: section,
						fields: {
							heading: { from: 'section', text: 'heading' },
							context: { from: 'same_file', kind: 'outline', text: 'first_line' },
						},
					},
				},
				'valid',
			],
			[
				'same_file heading',
				{
					outline: { markdown_units: section, fields: {} },
					candidate: {
						markdown_units: section,
						fields: {
							heading: { from: 'section', text: 'heading' },
							context: { from: 'same_file', kind: 'outline', text: 'heading' },
						},
					},
				},
				[
					[
						'syntax',
						['evidence.kinds.candidate.fields.context'],
						'kind candidate field context: same_file does not support heading text',
					],
				],
			],
			[
				'target as_kind naming a heading-free kind (control)',
				{
					nested: { markdown_units: section, fields: { legacy: { from: 'section', text: 'own' } } },
					candidate: {
						markdown_units: section,
						fields: {
							heading: { from: 'section', text: 'heading' },
							linked: { from: 'target', regex: '(.+)', resolve: 'path', as_kind: 'nested' },
						},
					},
				},
				'valid',
			],
			[
				'target as_kind naming a heading kind',
				{
					nested: {
						markdown_units: section,
						fields: { context: { from: 'section', text: 'heading' } },
					},
					candidate: {
						markdown_units: section,
						fields: {
							heading: { from: 'section', text: 'heading' },
							linked: { from: 'target', regex: '(.+)', resolve: 'path', as_kind: 'nested' },
						},
					},
				},
				[
					[
						'syntax',
						['evidence.kinds.candidate.fields.linked'],
						'as_kind must name a heading-free kind',
					],
				],
			],
		];
		const outcomes = await Promise.all(cases.map(async ([, kinds]) => admission(configure(kinds))));
		assert.deepEqual(
			outcomes,
			cases.map(([, , expected]) => expected),
		);
	} finally {
		base.remove();
	}
});
