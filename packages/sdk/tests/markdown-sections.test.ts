// Native Markdown units authored inline and executed through the addon: section field
// text and the absent parent of a top-level section, paragraph and list-item units,
// kind-boundary refusals, and Markdown heading anchors in target fields.
import assert from 'node:assert/strict';
import { test } from 'node:test';

import type {
	CheckOutcome,
	Configuration,
	EvidenceKindDefinition,
	MarkdownEvidenceSettings,
	SdkReport,
	TargetAnchor,
} from '../src/index.ts';
import { check, run } from '../src/index.ts';
import { isCheckOutcome } from '../src/reply-decoding.ts';
import { ownedDirectory, writeFiles } from './support/sdk-fixtures.ts';

delete process.env['OPENROUTER_API_KEY'];
process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';

const page = [
	'# Top',
	'',
	'Intro para.',
	'',
	'## Child One',
	'',
	'Child body.',
	'',
	'- item a',
	'- item b',
	'',
	'## Child One',
	'',
	'Duplicate.',
	'',
	'## Pic ![alt text](x.png) Here',
	'',
	'<a id="custom-id"></a>',
	'',
	'Links: [a](#top) [b](#child-one-1) [c](#missing) [d](#pic--here) [e](#custom-id) [f](other.md#caf%C3%A9) [g](notes.txt#part)',
	'',
].join('\n');

function markdownConfiguration(
	base: string,
	kind: EvidenceKindDefinition,
	field: string,
	markdown?: MarkdownEvidenceSettings,
): Configuration {
	const conventions = markdown === undefined ? {} : { markdown };
	return {
		id: 'markdown-units',
		base_directory: base,
		marker_prefix: 'K',
		check: { files: ['docs/page.md'] },
		decision_model: { model: 'jev-latest' },
		evidence: {
			kind: 'inline',
			utilities: {},
			matchers: {},
			kinds: { unit: kind },
			...conventions,
		},
		rulebooks: [
			{
				kind: 'inline',
				id: 'markdown-rules',
				markdown: 'Each unit is complete. <!-- K001 -->\n',
				entries: {
					K001: {
						name: 'complete_unit',
						evidence: 'unit',
						severity: 'warn',
						decision: {
							start: 'complete',
							steps: {
								complete: {
									mechanical_check: { tool: 'complete', field },
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

/**
 * The report of a run that must complete clean: these mechanical `complete` rules pass on
 * every selected unit, and an unsupported anchor is a field value, not unfinished work.
 * main 0.14.0's CLI gives the same ending for these definitions (exit 0, clean, no
 * findings, incomplete parts or diagnostics).
 */
function cleanReport(outcome: Awaited<ReturnType<typeof run>>): SdkReport {
	assert.equal(outcome.kind, 'completed');
	if (outcome.kind !== 'completed') throw new Error('unreachable');
	const { report } = outcome;
	assert.equal(report.status.kind, 'clean');
	assert.deepEqual([report.findings, report.incomplete, report.diagnostics], [[], [], []]);
	return report;
}

function textOf(report: SdkReport, piece: number, field: string): string | undefined {
	const value = Object.values(report.piece_evidence)[piece]?.piece.fields[field];
	return value?.state === 'text' ? value.value : value?.state;
}

/** Check a value as a plain-JavaScript caller would, outside the declarations. */
async function checkDynamically(value: unknown): Promise<CheckOutcome> {
	const reply: unknown = await Reflect.apply(check, undefined, [value]);
	if (!isCheckOutcome(reply)) throw new Error('check replied outside its closed outcome');
	return reply;
}

const repository = ownedDirectory('markdown-units');
writeFiles(repository.path, {
	'docs/page.md': page,
	'docs/other.md': '# Café\n',
	'docs/notes.txt': 'part\n',
});

await test('Markdown sections expose full, own and enclosing own text; a top-level section has no parent', async () => {
	const report = cleanReport(
		await run(
			markdownConfiguration(
				repository.path,
				{
					markdown_units: { mode: 'unit', units: ['section'] },
					fields: {
						full: { from: 'section', text: 'full' },
						own: { from: 'section', text: 'own' },
						parent: { from: 'section', text: 'enclosing_own' },
					},
				},
				'own',
			),
			{ root: repository.path, scope: { kind: 'all' } },
		),
	);
	const lines = Object.values(report.piece_evidence).map(
		(evidence) => evidence.piece.where.first_line,
	);
	assert.deepEqual(lines, [1n, 5n, 12n, 16n]);
	assert.equal(textOf(report, 0, 'own'), '# Top\n\nIntro para.\n\n');
	assert.equal(textOf(report, 0, 'parent'), 'absent');
	assert.equal(textOf(report, 1, 'parent'), '# Top\n\nIntro para.\n\n');
	assert.ok(textOf(report, 1, 'own')?.startsWith('## Child One\n\nChild body.'));
	assert.ok(textOf(report, 0, 'full')?.includes('## Pic'), 'the top section contains its children');
	assert.equal(report.usage.model_requests, 0);
});

await test('paragraph and list-item units select their own pieces', async () => {
	const counts = await Promise.all(
		(['paragraph', 'list_item'] as const).map(async (unit) => {
			const report = cleanReport(
				await run(
					markdownConfiguration(
						repository.path,
						{
							markdown_units: { mode: 'unit', units: [unit] },
							fields: { body: { from: 'remainder' } },
						},
						'body',
					),
					{ root: repository.path, scope: { kind: 'all' } },
				),
			);
			return Object.keys(report.piece_evidence).length;
		}),
	);
	assert.deepEqual(counts, [7, 2]);
});

/** A target anchor as [status, reason]; the reason exists only when unsupported. */
function anchorState(anchor: TargetAnchor | null | undefined): readonly [string, string] | null {
	if (anchor === null || anchor === undefined) return null;
	return [anchor.status, anchor.status === 'unsupported' ? anchor.reason : ''];
}

const anchorKind: EvidenceKindDefinition = {
	markdown_units: { mode: 'unit', units: ['paragraph'] },
	fields: {
		links: { from: 'target', regex: '\\(([^)]+)\\)', resolve: 'path' },
		file_or_skill: {
			from: 'target',
			regex: '\\((other\\.md#[^)]+)\\)',
			resolve: 'path_or_skill',
		},
	},
};

async function anchorReport(markdown?: MarkdownEvidenceSettings): Promise<SdkReport> {
	return cleanReport(
		await run(markdownConfiguration(repository.path, anchorKind, 'links', markdown), {
			root: repository.path,
			scope: { kind: 'all' },
		}),
	);
}

function linkAnchors(report: SdkReport): readonly (readonly [string, unknown])[] {
	return Object.values(report.piece_evidence).flatMap((evidence) => {
		const links = evidence.piece.fields['links'];
		return links?.state === 'targets'
			? links.values.map((target) => [target.name, anchorState(target.anchor)] as const)
			: [];
	});
}

await test('Markdown fragment targets resolve heading slugs only under an explicit github scheme', async () => {
	const [configured, standard] = await Promise.all([
		anchorReport({ heading_anchors: 'github' }),
		anchorReport(),
	]);
	assert.deepEqual(linkAnchors(configured), [
		['#top', ['found', '']],
		// The second "Child One" heading gets GitHub's duplicate suffix.
		['#child-one-1', ['found', '']],
		['#missing', ['missing', '']],
		// Image alt text is not part of the heading slug.
		['#pic--here', ['found', '']],
		['#custom-id', ['found', '']],
		// The fragment is percent-decoded once before matching "Café".
		['other.md#caf%C3%A9', ['found', '']],
		// A plain-text target has no anchors to check.
		['notes.txt#part', ['unsupported', 'unsupported_document']],
	]);
	// The same definitions with no Markdown convention: heading fragments are not checked,
	// an explicit HTML id still is, and a plain-text target keeps its own reason.
	assert.deepEqual(linkAnchors(standard), [
		['#top', ['unsupported', 'heading_scheme_not_configured']],
		['#child-one-1', ['unsupported', 'heading_scheme_not_configured']],
		['#missing', ['unsupported', 'heading_scheme_not_configured']],
		['#pic--here', ['unsupported', 'heading_scheme_not_configured']],
		['#custom-id', ['found', '']],
		['other.md#caf%C3%A9', ['unsupported', 'heading_scheme_not_configured']],
		['notes.txt#part', ['unsupported', 'unsupported_document']],
	]);
	// Under path_or_skill, a Markdown link with a fragment is a file path, not a skill name.
	const fileOrSkill = Object.values(configured.piece_evidence)
		.map((evidence) => evidence.piece.fields['file_or_skill'])
		.find((field) => field?.state === 'targets' && field.values.length > 0);
	assert.ok(fileOrSkill?.state === 'targets');
	assert.deepEqual(
		fileOrSkill.values.map((target) => [target.name, target.path, anchorState(target.anchor)]),
		[['other.md#caf%C3%A9', 'docs/other.md', ['found', '']]],
	);
});

await test('kind-boundary refusals: conflicting matchers, null, call_site, unknown units, two filters, section fields on paragraphs', async () => {
	const valid = markdownConfiguration(
		repository.path,
		{
			markdown_units: { mode: 'unit', units: ['section'] },
			fields: { body: { from: 'remainder' } },
		},
		'body',
	);
	assert.equal((await check(valid)).kind, 'valid');
	const kinds: readonly (readonly [string, unknown])[] = [
		[
			'conflict',
			{ rule: 'paragraph', markdown_units: { mode: 'unit', units: ['section'] }, fields: {} },
		],
		['null', { markdown_units: null, fields: {} }],
		[
			'call_site',
			{ markdown_units: { mode: 'call_site', units: ['section'], starts_with: 'C' }, fields: {} },
		],
		['unknown unit', { markdown_units: { mode: 'unit', units: ['heading'] }, fields: {} }],
		[
			'two filters',
			{
				markdown_units: { mode: 'unit', units: ['section'], starts_with: 'A', pattern: 'B' },
				fields: {},
			},
		],
		[
			'section field on paragraphs',
			{
				markdown_units: { mode: 'unit', units: ['paragraph'] },
				fields: { body: { from: 'section', text: 'own' } },
			},
		],
	];
	const outcomes = await Promise.all(
		kinds.map(async ([, kind]) =>
			checkDynamically({ ...valid, evidence: { ...valid.evidence, kinds: { unit: kind } } }),
		),
	);
	for (const [index, outcome] of outcomes.entries()) {
		assert.equal(outcome.kind, 'invalid', `${kinds[index]?.[0]} is refused`);
	}
	const callSite = outcomes[2];
	assert.ok(
		callSite?.kind === 'invalid' && callSite.diagnostics[0]?.message.includes('requires unit mode'),
		'call_site is refused at the kind boundary',
	);
});

repository.remove();
