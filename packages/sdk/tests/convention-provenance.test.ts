// Loaded-project provenance of document conventions: held A after disk B, an edited typed
// C and an explicit reload B, and each setting edited alone; every case checks the actual
// outline, asking-text and anchor effects and the root source's origin, content and hash.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';

import type {
	Configuration,
	ConfigurationInput,
	HtmlEvidenceSettings,
	LoadedConfiguration,
	MarkdownEvidenceSettings,
	RuleEntry,
} from '../src/index.ts';
import { load, run } from '../src/index.ts';
import { anchorState, headingOf, tokensOf } from './support/document-observations.ts';
import type { OwnedDirectory } from './support/sdk-fixtures.ts';
import { ownedDirectory, writeFiles } from './support/sdk-fixtures.ts';

delete process.env['OPENROUTER_API_KEY'];
process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';

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
