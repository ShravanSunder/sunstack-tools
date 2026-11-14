// HTML and Markdown document conventions come only from configuration: standard defaults,
// the explicit 0.14.1 restore and unrelated custom names, observed in outlines, raw fields,
// model views and the requests the model receives; and the refusals Rust owns.
import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import { test } from 'node:test';

import type {
	CheckOutcome,
	Configuration,
	HtmlEvidenceSettings,
	MarkdownEvidenceSettings,
	SdkReport,
} from '../src/index.ts';
import { check, run } from '../src/index.ts';
import { isCheckOutcome } from '../src/reply-decoding.ts';
import type { ControlledBackend } from './support/controlled-backend.ts';
import { withAnsweringBackend } from './support/controlled-backend.ts';
import { anchorState, headingOf, tokensOf } from './support/document-observations.ts';
import { ownedDirectory, writeFiles } from './support/sdk-fixtures.ts';

delete process.env['OPENROUTER_API_KEY'];
process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';

/** The 0.14.1 conventions, restored explicitly as the 0.15.0 changelog gives them. */
const restored014 = {
	html: { skip_tags: ['ex-markdown'], text_attributes: { 'ex-mermaid': ['label'] } },
	markdown: { heading_anchors: 'github' },
} as const satisfies { html: HtmlEvidenceSettings; markdown: MarkdownEvidenceSettings };

/** Unrelated custom names: behavior follows configuration, not built-in tag names. */
const unrelatedCustom = {
	html: { skip_tags: ['example-source'], text_attributes: { 'custom-diagram': ['caption'] } },
} as const satisfies { html: HtmlEvidenceSettings };

const conventionsPage = [
	'<h2>Intro</h2><p>Opening</p>',
	'<ex-markdown><h2>Old example</h2><p>Old sample</p></ex-markdown>',
	'<example-source><h2>New example</h2><p>New sample</p></example-source>',
	'<template><h2>Template heading</h2></template>',
	"<ex-mermaid label='Old flow' aria-label='Old aria'>Old graph</ex-mermaid>",
	"<custom-diagram caption='New flow' alt='New picture'>New graph</custom-diagram>",
	"<h2 id='explicit'>Last</h2><p>End</p>",
].join('\n');

/** Markdown links into the HTML page: an explicit id and a heading text with no id. */
const linksPage = '# Links\n\n[x](page.html#explicit) [y](page.html#intro)\n';

const headingStarts: Readonly<Record<string, string>> = {
	Intro: '<h2>Intro',
	'Old example': '<h2>Old example',
	'New example': '<h2>New example',
	Last: "<h2 id='explicit'>Last",
};

/** The raw source text of each expected section, from its heading to the next one. */
function sectionTexts(page: string, headings: readonly string[]): string[] {
	const starts = headings.map((heading) => page.indexOf(headingStarts[heading] ?? '\u0000'));
	assert.ok(starts.every((start) => start >= 0));
	return starts.map((start, index) => page.slice(start, starts[index + 1] ?? page.length));
}

function conventionsConfiguration(
	base: string,
	endpoint: string,
	conventions: { html?: HtmlEvidenceSettings; markdown?: MarkdownEvidenceSettings },
): Configuration {
	return {
		id: 'document-conventions',
		base_directory: base,
		marker_prefix: 'K',
		check: { files: ['docs/page.html', 'docs/links.md'] },
		decision_model: { model: 'jev-latest', endpoint },
		evidence: {
			kind: 'inline',
			utilities: {},
			matchers: {},
			kinds: {
				section: {
					html_units: { mode: 'unit', units: ['section'] },
					fields: { own_body: { from: 'section', text: 'own' } },
				},
				link: {
					markdown_units: { mode: 'unit', units: ['paragraph'] },
					fields: { links: { from: 'target', regex: '\\(([^)]+)\\)', resolve: 'path' } },
				},
			},
			...conventions,
		},
		rulebooks: [
			{
				kind: 'inline',
				id: 'conventions-rules',
				markdown:
					'Each section reads clearly. <!-- K001 -->\n\nLinks are complete. <!-- K002 -->\n',
				entries: {
					K001: {
						name: 'clear_section',
						evidence: 'section',
						severity: 'warn',
						questions: {
							clear: { type: 'noul', instructions: { check: 'own_body', task: 'Is it clear?' } },
						},
						decision: {
							start: 'clear',
							steps: {
								clear: {
									ask: 'clear',
									yes_at_or_above: 0.8,
									no_at_or_below: 0.2,
									yes: 'pass',
									no: { finding: 'Unclear.' },
									uncertain: 'uncertain',
								},
							},
						},
					},
					K002: {
						name: 'complete_links',
						evidence: 'link',
						severity: 'warn',
						decision: {
							start: 'complete',
							steps: {
								complete: {
									mechanical_check: { tool: 'complete', field: 'links' },
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

/** What one completed run shows of the conventions, from the report and the requests. */
interface ConventionsObservation {
	readonly headings: readonly (string | undefined)[];
	readonly rawSections: readonly string[];
	readonly views: readonly string[];
	readonly viewTokens: readonly (readonly string[])[];
	readonly anchors: readonly (readonly [string, unknown])[];
}

function requestBodyText(body: Readonly<Record<string, unknown>>): string {
	const state = body['state'];
	const value: unknown =
		typeof state === 'object' && state !== null ? Reflect.get(state, 'own_body') : '';
	return typeof value === 'string' ? value : '';
}

function observe(report: SdkReport, requests: readonly string[]): ConventionsObservation {
	const sections = Object.values(report.piece_evidence).filter((evidence) =>
		evidence.piece.where.file.endsWith('.html'),
	);
	const views = sections.map((evidence) => {
		const view = evidence.piece.model_view;
		const own = view?.state === 'bounded' ? view.fields['own_body'] : undefined;
		return typeof own === 'string' ? own : '';
	});
	// The model receives exactly the projected views, once each.
	assert.deepEqual(requests.toSorted(), views.toSorted());
	const links = Object.values(report.piece_evidence).flatMap((evidence) => {
		const field = evidence.piece.fields['links'];
		return field?.state === 'targets'
			? field.values.map((target) => [target.name, anchorState(target.anchor)] as const)
			: [];
	});
	return {
		headings: sections.map((evidence) => headingOf(evidence.piece.text)),
		rawSections: sections.map((evidence) => {
			const own = evidence.piece.fields['own_body'];
			return own?.state === 'text' ? own.value : String(own?.state);
		}),
		views,
		viewTokens: views.map(tokensOf),
		anchors: links,
	};
}

async function runConventions(
	configuration: Configuration,
	root: string,
	backend: ControlledBackend,
): Promise<ConventionsObservation> {
	const first = backend.received.length;
	const outcome = await run(configuration, { root, scope: { kind: 'all' } });
	assert.equal(outcome.kind, 'completed');
	if (outcome.kind !== 'completed') throw new Error('unreachable');
	assert.equal(outcome.report.status.kind, 'clean');
	assert.deepEqual([outcome.report.findings, outcome.report.incomplete], [[], []]);
	const requests = backend.received.slice(first).map((request) => requestBodyText(request.body));
	return observe(outcome.report, requests);
}

/** Attribute tokens across a run's section views, without cut markers. */
function diagramTokens(observed: ConventionsObservation): string[] {
	return observed.viewTokens.flat().filter((token) => !token.startsWith('[cut'));
}

/** The most model requests the three convention runs may send; they send one per section. */
const conventionsResponseLimit = 16;

/** Run the three conventions over the shared page, in order. */
async function runThreeConventions(
	backend: ControlledBackend,
	root: string,
): Promise<readonly [ConventionsObservation, ConventionsObservation, ConventionsObservation]> {
	writeFiles(root, { 'docs/page.html': conventionsPage, 'docs/links.md': linksPage });
	const configure = (conventions: Parameters<typeof conventionsConfiguration>[2]): Configuration =>
		conventionsConfiguration(root, backend.endpoint, conventions);
	const standard = await runConventions(configure({}), root, backend);
	const restored = await runConventions(configure(restored014), root, backend);
	const custom = await runConventions(configure(unrelatedCustom), root, backend);
	return [standard, restored, custom];
}

await test('HTML conventions follow configuration: standard defaults, the explicit 0.14.1 restore and unrelated custom names', async () => {
	const { value, answered } = await withAnsweringBackend(
		'conventions',
		conventionsResponseLimit,
		runThreeConventions,
	);
	const [standard, restored, custom] = value;
	// Outline: <template> is always excluded; a skipped tag's headings leave the outline.
	const cases = [
		[standard, ['Intro', 'Old example', 'New example', 'Last']],
		[restored, ['Intro', 'New example', 'Last']],
		[custom, ['Intro', 'Old example', 'Last']],
	] as const;
	for (const [observed, headings] of cases) {
		assert.deepEqual(observed.headings, headings);
		// Raw section fields keep the exact source range, markup included.
		assert.deepEqual(observed.rawSections, sectionTexts(conventionsPage, headings));
	}
	// Asking text: alt and aria-label always; only configured attributes join them, in
	// source order. The section holding the diagrams is the one before "Last".
	assert.deepEqual(diagramTokens(standard), ['[aria-label: Old aria]', '[alt: New picture]']);
	assert.deepEqual(diagramTokens(restored), [
		'[label: Old flow]',
		'[aria-label: Old aria]',
		'[alt: New picture]',
	]);
	assert.deepEqual(diagramTokens(custom), [
		'[aria-label: Old aria]',
		'[caption: New flow]',
		'[alt: New picture]',
	]);
	// Skipping a tag from the outline does not remove its text from asking: the enclosing
	// section's view keeps the skipped heading and body.
	assert.match(restored.views[0] ?? '', /Old example Old sample/u);
	assert.match(custom.views[1] ?? '', /New example New sample/u);
	assert.doesNotMatch(standard.views[0] ?? '', /Old sample/u);
	// HTML targets resolve explicit ids only, under every convention.
	for (const observed of [standard, restored, custom]) {
		assert.deepEqual(observed.anchors, [
			['page.html#explicit', ['found', '']],
			['page.html#intro', ['missing', '']],
		]);
	}
	// The responder answered exactly one request per section.
	assert.equal(
		answered,
		[standard, restored, custom].reduce((total, observed) => total + observed.headings.length, 0),
	);
});

await test('a responder failure still stops the responder, closes the backend and removes the repository', async () => {
	let endpoint = '';
	let root = '';
	// The standard run sends four requests; a limit of two makes the responder fail.
	await assert.rejects(
		withAnsweringBackend('conventions', 2, async (backend, repositoryRoot) => {
			endpoint = backend.endpoint;
			root = repositoryRoot;
			writeFiles(root, { 'docs/page.html': conventionsPage, 'docs/links.md': linksPage });
			return runConventions(conventionsConfiguration(root, endpoint, {}), root, backend);
		}),
		{ message: '4 model requests; at most 2 expected' },
	);
	assert.equal(existsSync(root), false, 'the repository was removed');
	await assert.rejects(fetch(endpoint, { method: 'POST', body: '{}' }), 'the backend is closed');
});

/** Check a value as a plain-JavaScript caller would, outside the declarations. */
async function checkDynamically(value: unknown): Promise<CheckOutcome> {
	const reply: unknown = await Reflect.apply(check, undefined, [value]);
	if (!isCheckOutcome(reply)) throw new Error('check replied outside its closed outcome');
	return reply;
}

await test('blank names, null or mistyped values, unknown schemes and unknown keys are refused by Rust', async () => {
	const base = ownedDirectory('conventions-refusals');
	try {
		const valid = conventionsConfiguration(base.path, 'http://127.0.0.1:1/v1/systemone', {});
		assert.equal((await check(valid)).kind, 'valid');
		// Each refusal is Rust's own: its code and message, without serde's input position.
		const refused: readonly (readonly [Readonly<Record<string, unknown>>, string, string])[] = [
			[{ html: { skip_tags: [' '] } }, 'syntax', 'HTML tag name must be nonempty'],
			[{ html: { skip_tags: [''] } }, 'syntax', 'HTML tag name must be nonempty'],
			[
				{ html: { text_attributes: { '': ['caption'] } } },
				'syntax',
				'HTML tag name must be nonempty',
			],
			[
				{ html: { text_attributes: { figure: [' '] } } },
				'syntax',
				'HTML attribute name must be nonempty',
			],
			[
				{ markdown: { heading_anchors: 'gitlab' } },
				'syntax',
				'unknown variant `gitlab`, expected `github`',
			],
			[
				{ html: { skip: ['aside'] } },
				'unknown_field',
				'unknown field `skip`, expected one of `skip_tags`, `text_attributes`, `embedded_markdown`',
			],
			[
				{ markdown: { anchors: 'github' } },
				'unknown_field',
				'unknown field `anchors`, expected `heading_anchors`',
			],
			// Settings objects and collections are defaulted when omitted, never null; names
			// are strings.
			[{ html: null }, 'syntax', 'invalid type: null, expected struct HtmlEvidenceSettings'],
			[
				{ markdown: null },
				'syntax',
				'invalid type: null, expected struct MarkdownEvidenceSettings',
			],
			[{ html: { skip_tags: null } }, 'syntax', 'invalid type: null, expected a sequence'],
			[{ html: { text_attributes: null } }, 'syntax', 'invalid type: null, expected a map'],
			[{ html: { skip_tags: [null] } }, 'syntax', 'invalid type: null, expected a string'],
			[{ html: { skip_tags: [1] } }, 'syntax', 'invalid type: integer `1`, expected a string'],
			[
				{ html: { text_attributes: { figure: null } } },
				'syntax',
				'invalid type: null, expected a sequence',
			],
			[
				{ html: { text_attributes: { figure: [null] } } },
				'syntax',
				'invalid type: null, expected a string',
			],
			[
				{ html: { text_attributes: { figure: [1] } } },
				'syntax',
				'invalid type: integer `1`, expected a string',
			],
			[
				{ html: { text_attributes: { figure: 'caption' } } },
				'syntax',
				'invalid type: string "caption", expected a sequence',
			],
		];
		// Valid controls: a null heading scheme is the omitted default, as is an empty object.
		const accepted = await Promise.all(
			[{ markdown: { heading_anchors: null } }, { html: {}, markdown: {} }].map(
				async (conventions) =>
					(
						await checkDynamically({
							...valid,
							evidence: { ...valid.evidence, ...conventions },
						})
					).kind,
			),
		);
		assert.deepEqual(accepted, ['valid', 'valid']);

		const outcomes = await Promise.all(
			refused.map(async ([conventions]) =>
				checkDynamically({ ...valid, evidence: { ...valid.evidence, ...conventions } }),
			),
		);
		assert.deepEqual(
			outcomes.map((outcome) =>
				outcome.kind === 'invalid'
					? outcome.diagnostics.map((diagnostic) => [
							diagnostic.code,
							diagnostic.json_path,
							diagnostic.message.replace(/ at line \d+ column \d+$/u, ''),
						])
					: outcome.kind,
			),
			refused.map(([, code, message]) => [[code, ['evidence'], message]]),
		);
	} finally {
		base.remove();
	}
});
