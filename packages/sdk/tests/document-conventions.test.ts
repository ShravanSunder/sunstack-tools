// HTML and Markdown document conventions come only from configuration: standard defaults,
// the explicit 0.14.1 restore, unrelated custom names, isolated edits of each setting,
// loaded/held/edited/reloaded provenance and the refusals Rust owns.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';

import type {
	CheckOutcome,
	Configuration,
	ConfigurationInput,
	HtmlEvidenceSettings,
	LoadedConfiguration,
	MarkdownEvidenceSettings,
	RuleEntry,
	SdkReport,
	TargetAnchor,
} from '../src/index.ts';
import { check, load, run } from '../src/index.ts';
import { isCheckOutcome } from '../src/reply-decoding.ts';
import type { ControlledBackend } from './support/controlled-backend.ts';
import { answerNoul, startControlledBackend } from './support/controlled-backend.ts';
import type { OwnedDirectory } from './support/sdk-fixtures.ts';
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

function headingOf(text: string): string | undefined {
	return /^<h2[^>]*>([^<]*)<\/h2>/u.exec(text)?.[1];
}

/** `[name: value]` tokens of asking text, in order. */
function tokensOf(text: string): string[] {
	return text.match(/\[[a-z-]+: [^\]]*\]/gu) ?? [];
}

function anchorState(anchor: TargetAnchor | null | undefined): readonly [string, string] | null {
	if (anchor === null || anchor === undefined) return null;
	return [anchor.status, anchor.status === 'unsupported' ? anchor.reason : ''];
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

/** Answer every request "yes" for one backend's lifetime. */
function answerEveryRequest(backend: ControlledBackend): void {
	const answerNext = async (): Promise<void> => {
		answerNoul(await backend.nextRequest(), 0.9);
		await answerNext();
	};
	void answerNext();
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

await test('HTML conventions follow configuration: standard defaults, the explicit 0.14.1 restore and unrelated custom names', async () => {
	const repository = ownedDirectory('conventions');
	const backend = await startControlledBackend();
	process.env['OPENROUTER_API_KEY'] = 'synthetic-controlled-key';
	answerEveryRequest(backend);
	try {
		writeFiles(repository.path, { 'docs/page.html': conventionsPage, 'docs/links.md': linksPage });
		const configure = (
			conventions: Parameters<typeof conventionsConfiguration>[2],
		): Configuration => conventionsConfiguration(repository.path, backend.endpoint, conventions);
		const standard = await runConventions(configure({}), repository.path, backend);
		const restored = await runConventions(configure(restored014), repository.path, backend);
		const custom = await runConventions(configure(unrelatedCustom), repository.path, backend);

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
		const diagramTokens = (observed: ConventionsObservation): string[] =>
			observed.viewTokens.flat().filter((token) => !token.startsWith('[cut'));
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
	} finally {
		delete process.env['OPENROUTER_API_KEY'];
		await backend.close();
		repository.remove();
	}
});

// Loaded-project provenance: one keyless mechanical project whose conventions are file data.

const provenancePage = [
	'<h2>One</h2><p>Body</p>',
	'<ex-a><h2>A hidden</h2></ex-a>',
	'<ex-b><h2>B hidden</h2></ex-b>',
	'<ex-c><h2>C hidden</h2></ex-c>',
	"<x-diagram a-note='A' b-note='B' c-note='C'>Graph</x-diagram>",
].join('\n');

interface ProjectConventions {
	readonly html?: HtmlEvidenceSettings;
	readonly markdown?: MarkdownEvidenceSettings;
}

const sentinelA: ProjectConventions = {
	html: { skip_tags: ['ex-a'], text_attributes: { 'x-diagram': ['a-note'] } },
};
const sentinelB: ProjectConventions = {
	html: { skip_tags: ['ex-b'], text_attributes: { 'x-diagram': ['b-note'] } },
	markdown: { heading_anchors: 'github' },
};
const sentinelC: ProjectConventions = {
	html: { skip_tags: ['ex-c'], text_attributes: { 'x-diagram': ['c-note'] } },
	markdown: { heading_anchors: 'github' },
};

function configText(conventions: ProjectConventions): string {
	return `${JSON.stringify(
		{
			rulebook: [{ markdown: 'rulebook.md', rules: 'rules.json' }],
			marker_prefix: 'K',
			check: { files: ['docs/*'] },
			evidence: {
				ast_grep: 'evidence/sgconfig.yml',
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
			decision_model: { model: 'jev-latest' },
		},
		null,
		2,
	)}\n`;
}

const complete = (field: string): RuleEntry['decision'] => ({
	start: 'complete',
	steps: {
		complete: {
			mechanical_check: { tool: 'complete', field },
			yes: 'pass',
			no: { finding: 'Incomplete.' },
		},
	},
});

function writeProvenanceProject(directory: string, conventions: ProjectConventions): void {
	writeFiles(directory, {
		'sunstack.lint.json': configText(conventions),
		'rulebook.md': 'Sections are complete. <!-- K001 -->\n\nLinks are complete. <!-- K002 -->\n',
		'rules.json': JSON.stringify({
			K001: {
				name: 'sections',
				evidence: 'section',
				severity: 'warn',
				decision: complete('own_body'),
			},
			K002: { name: 'links', evidence: 'link', severity: 'warn', decision: complete('links') },
		}),
		'evidence/sgconfig.yml': 'ruleDirs: [rules]\n',
		'evidence/rules/whole.yml': 'id: whole\nlanguage: Html\nrule: {kind: document}\n',
		'docs/page.html': provenancePage,
		'docs/links.md': '# Head\n\n[h](#head)\n',
	});
}

/** The conventions' effects and the root source, from one keyless run. */
interface ProvenanceObservation {
	readonly headings: readonly (string | undefined)[];
	readonly tokens: readonly string[];
	readonly anchor: unknown;
	readonly root: { readonly origin: string; readonly content: string };
}

async function observeProvenance(
	configuration: ConfigurationInput,
	root: string,
): Promise<ProvenanceObservation> {
	const outcome = await run(configuration, { root, scope: { kind: 'all' } });
	assert.equal(outcome.kind, 'completed');
	if (outcome.kind !== 'completed') throw new Error('unreachable');
	const { report } = outcome;
	assert.equal(report.status.kind, 'clean');
	const [rootSource] = report.sources;
	assert.ok(rootSource !== undefined);
	// Every reported source is digested as reported.
	for (const source of report.sources) {
		assert.equal(source.sha256, createHash('sha256').update(source.content).digest('hex'));
	}
	const pieces = Object.values(report.piece_evidence);
	const sections = pieces.filter((evidence) => evidence.piece.where.file.endsWith('.html'));
	const links = pieces.flatMap((evidence) => {
		const field = evidence.piece.fields['links'];
		return field?.state === 'targets' ? field.values : [];
	});
	return {
		headings: sections.map((evidence) => headingOf(evidence.piece.text)),
		tokens: sections.flatMap((evidence) => {
			const view = evidence.piece.model_view;
			const own = view?.state === 'bounded' ? view.fields['own_body'] : undefined;
			return typeof own === 'string' ? tokensOf(own) : [];
		}),
		anchor: anchorState(links[0]?.anchor),
		root: { origin: rootSource.origin.kind, content: rootSource.content },
	};
}

/** The expected effects of one set of conventions on the provenance project. */
function effectsOf(conventions: ProjectConventions): Omit<ProvenanceObservation, 'root'> {
	const skipped = new Set(conventions.html?.skip_tags ?? []);
	const notes = conventions.html?.text_attributes?.['x-diagram'] ?? [];
	return {
		headings: [
			'One',
			...(['a', 'b', 'c'] as const)
				.filter((letter) => !skipped.has(`ex-${letter}`))
				.map((letter) => `${letter.toUpperCase()} hidden`),
		],
		tokens: notes.map((note) => `[${note}: ${note.charAt(0).toUpperCase()}]`),
		anchor:
			conventions.markdown?.heading_anchors === 'github'
				? ['found', '']
				: ['unsupported', 'heading_scheme_not_configured'],
	};
}

/** The `document` member of a normalized root source. */
function documentOf(content: string): unknown {
	const root: unknown = JSON.parse(content);
	return typeof root === 'object' && root !== null ? Reflect.get(root, 'document') : undefined;
}

async function loadedProject(directory: string): Promise<LoadedConfiguration> {
	const outcome = await load({ directory });
	assert.equal(outcome.kind, 'loaded');
	if (outcome.kind !== 'loaded') throw new Error('unreachable');
	return outcome.value;
}

function withConventions(
	held: LoadedConfiguration,
	conventions: ProjectConventions,
): Configuration {
	const { configuration } = held;
	return { ...configuration, evidence: { ...configuration.evidence, ...conventions } };
}

const sharedProject: OwnedDirectory = ownedDirectory('conventions-provenance');

await test('held A, disk B, an edited typed C and an explicit reload each execute their own conventions', async () => {
	const directory = sharedProject.path;
	writeProvenanceProject(directory, sentinelA);
	const textA = readFileSync(join(directory, 'sunstack.lint.json'), 'utf8');
	const held = await loadedProject(directory);
	const fileA = { origin: 'file', content: textA };
	assert.deepEqual(await observeProvenance(held, directory), {
		...effectsOf(sentinelA),
		root: fileA,
	});

	// Disk changes to B take effect only on an explicit reload.
	writeFileSync(join(directory, 'sunstack.lint.json'), configText(sentinelB));
	const textB = readFileSync(join(directory, 'sunstack.lint.json'), 'utf8');
	assert.deepEqual(await observeProvenance(held, directory), {
		...effectsOf(sentinelA),
		root: fileA,
	});

	// A typed edit of the held value is visibly edited and executes C.
	const editedC = await observeProvenance(withConventions(held, sentinelC), directory);
	assert.deepEqual(
		{ ...editedC, root: editedC.root.origin },
		{ ...effectsOf(sentinelC), root: 'edited' },
	);
	assert.deepEqual(documentOf(editedC.root.content), {
		html: sentinelC.html,
		markdown: sentinelC.markdown,
	});

	// Editing only the informational receipt changes nothing.
	const receiptOnly: LoadedConfiguration = {
		...held,
		sources: held.sources.map((source, index) =>
			index === 0 ? Object.assign({}, source, { content: textB }) : source,
		),
	};
	assert.deepEqual(await observeProvenance(receiptOnly, directory), {
		...effectsOf(sentinelA),
		root: fileA,
	});
	assert.deepEqual(await observeProvenance(held, directory), {
		...effectsOf(sentinelA),
		root: fileA,
	});

	assert.deepEqual(await observeProvenance(await loadedProject(directory), directory), {
		...effectsOf(sentinelB),
		root: { origin: 'file', content: textB },
	});
});

await test('each convention edited alone changes its own behavior and the root source; default-equivalent values keep the file', async () => {
	const directory = sharedProject.path;
	writeProvenanceProject(directory, {});
	const text = readFileSync(join(directory, 'sunstack.lint.json'), 'utf8');
	const held = await loadedProject(directory);
	const standard = effectsOf({});
	assert.deepEqual(await observeProvenance(held, directory), {
		...standard,
		root: { origin: 'file', content: text },
	});

	// Explicit empty or null conventions equal the omitted defaults: still the file bytes.
	const equivalents: readonly ProjectConventions[] = [
		{ html: {}, markdown: {} },
		{ html: { skip_tags: [], text_attributes: {} }, markdown: { heading_anchors: null } },
	];
	const equivalentRuns = await Promise.all(
		equivalents.map(async (equivalent) =>
			observeProvenance(withConventions(held, equivalent), directory),
		),
	);
	for (const observed of equivalentRuns) {
		assert.deepEqual(observed, { ...standard, root: { origin: 'file', content: text } });
	}

	const isolated = [
		['skip_tags', { html: { skip_tags: ['ex-b'] } }, { headings: ['One', 'A hidden', 'C hidden'] }],
		[
			'text_attributes',
			{ html: { text_attributes: { 'x-diagram': ['b-note'] } } },
			{ tokens: ['[b-note: B]'] },
		],
		['heading_anchors', { markdown: { heading_anchors: 'github' } }, { anchor: ['found', ''] }],
	] as const satisfies readonly (readonly [string, ProjectConventions, object])[];
	const isolatedRuns = await Promise.all(
		isolated.map(async ([, conventions]) =>
			observeProvenance(withConventions(held, conventions), directory),
		),
	);
	for (const [index, [name, conventions, changed]] of isolated.entries()) {
		const observed = isolatedRuns[index];
		assert.ok(observed !== undefined);
		// Only this convention's effect differs from the standard run.
		assert.deepEqual(
			{ ...observed, root: undefined },
			{ ...standard, ...changed, root: undefined },
			name,
		);
		// The root is an edited, normalized source holding exactly this change.
		assert.equal(observed.root.origin, 'edited', name);
		assert.notEqual(observed.root.content, text, name);
		assert.deepEqual(
			documentOf(observed.root.content),
			{
				html: {
					skip_tags: [],
					text_attributes: {},
					...('html' in conventions ? conventions.html : {}),
				},
				markdown: {
					heading_anchors: null,
					...('markdown' in conventions ? conventions.markdown : {}),
				},
			},
			name,
		);
	}
	sharedProject.remove();
});

/** Check a value as a plain-JavaScript caller would, outside the declarations. */
async function checkDynamically(value: unknown): Promise<CheckOutcome> {
	const reply: unknown = await Reflect.apply(check, undefined, [value]);
	if (!isCheckOutcome(reply)) throw new Error('check replied outside its closed outcome');
	return reply;
}

await test('blank names, unknown schemes and unknown convention keys are refused by Rust', async () => {
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
				'unknown field `skip`, expected `skip_tags` or `text_attributes`',
			],
			[
				{ markdown: { anchors: 'github' } },
				'unknown_field',
				'unknown field `anchors`, expected `heading_anchors`',
			],
		];

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
