import { execFile } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, stat, symlink, unlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { expect, it } from 'vitest';

import { decisionActorField } from '../core/decision-record.ts';

const run = promisify(execFile);
const script = fileURLToPath(
	new URL('../../scripts/migrate-lean-design-records.ts', import.meta.url),
);
const cli = fileURLToPath(new URL('../../bin/demistify.ts', import.meta.url));
const exampleLibrary =
	'<link rel="stylesheet" href="https://demistify.getagentstudio.dev/v1/theme.css"><script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script>';
const plannedCases = [
	{ given: 'Result exists', expect: 'The result is visible.' },
	{ given: 'Result was updated', expect: 'The latest result is visible.' },
	{ given: 'No result exists', expect: 'The reader sees the empty state.' },
];

const oldDesign = {
	pages: [{ id: 'home', title: 'Old design', href: 'index.html', icon: 'file' }],
	mapGroups: [],
	records: [
		{
			id: 'N1',
			kind: 'need',
			title: 'Reader need',
			statement: 'The reader sees the result.',
			destination: { page: 'index.html', fragment: 'N1' },
			sources: [{ href: 'index.html#N1', excerpt: 'Reader need.' }],
		},
		{
			id: 'R1',
			kind: 'rule',
			title: 'Visible result',
			statement: 'Always display the result.',
			destination: { page: 'index.html', fragment: 'R1' },
			sources: [{ href: 'index.html#evidence', excerpt: 'Read the observed output.' }],
			relationships: [{ relation: 'serves', targetId: 'N1' }],
		},
		{
			id: 'R2',
			kind: 'rule',
			title: 'Detailed result',
			statement: 'Explain the current result.',
			destination: { page: 'index.html', fragment: 'R2' },
			sources: [{ href: 'index.html#evidence', excerpt: 'Read the observed output.' }],
			relationships: [{ relation: 'serves', targetId: 'N1' }],
		},
		...[
			{ id: 'V1', title: 'Observe the result' },
			{ id: 'V2', title: 'Contextual link' },
			{ id: 'V3', title: 'Inline check title' },
		].map(({ id, title }) => ({
			id,
			kind: 'planned-check',
			title,
			status: 'planned',
			destination: { page: 'index.html', fragment: id },
			sources: [{ href: 'index.html#' + id, excerpt: title }],
			relationships: [{ relation: 'checks', targetId: 'R1' }],
			cases: plannedCases,
		})),
	],
};
const oldHtml = `<!doctype html><html lang="en"><head><title>Old design</title>
<link rel="stylesheet" href="https://demistify.getagentstudio.dev/v1/theme.css">
<script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script></head>
<body><ex-nav for="content" src="navigation.generated.json"></ex-nav><ex-document id="content"><h1>Old design</h1>
<p><a href="#evidence">Evidence index</a></p>
<ex-callout title="Reader context"><h3 id="N1">Reader need</h3><p>The reader needs the current result.</p></ex-callout>
<h3 id="R1">Visible result</h3><p>Explain where the result appears.</p>
<h3 id="R2">Detailed result</h3><p>See <a href="#evidence">the observed output</a>.</p>
<table><tbody><tr id="V1"><td>V1 · Read the output record</td><td>Check the current result.</td></tr></tbody></table>
<h3 id="V2">Contextual link</h3><p>Observe the result. Checks R1. Read <a href="#evidence">the evidence</a> for the next case.</p>
<h3 id="V3">Inline check title</h3><p>Compare the <a href="#evidence">saved record</a> before proceeding.</p>
<h3 id="evidence">Observed output</h3><p>Read the observed output.</p>
<h3 id="DEL-01">Delivery</h3><p>Verify the output.</p>
<h3 id="D1">Owner decision</h3><p>Keep the matching result available.</p>
<a href="#R1">Read the rule</a><a href="#D1">Read the decision</a></ex-document></body></html>`;
const oldDecision = [
	{
		id: 'D1',
		kind: 'choice',
		title: 'Owner decision',
		destination: { page: 'index.html', fragment: 'D1' },
		sources: [
			{ href: 'index.html#D1', excerpt: 'Keep the matching result available.', verbatim: true },
		],
		authority: {
			state: 'owner-decided',
			owner: 'owner',
			sourceKind: 'owner-words',
			decidedOn: '2026-11-22',
			decision: 'Keep the matching result available.',
		},
	},
];

it('plans without writing and migrates a copy with row and verbatim evidence anchors intact', async (): Promise<void> => {
	const scratch = await mkdtemp(join(tmpdir(), 'demistify-lean-migration-'));
	try {
		const input = join(scratch, 'input');
		const output = join(scratch, 'output');
		const project = join(input, 'project');
		const set = join(project, 'phase');
		const oldSource = `export default ${JSON.stringify(oldDesign)};\n`;
		await mkdir(set, { recursive: true });
		await Promise.all([
			writeFile(
				join(project, 'project.ts'),
				"import phase from './phase/design.ts'; export default { designs: { phase } };",
			),
			writeFile(join(set, 'index.html'), oldHtml),
			writeFile(join(set, 'design.ts'), oldSource),
			writeFile(
				join(set, 'design-decisions.ts'),
				`export default ${JSON.stringify(oldDecision)};\n`,
			),
			writeFile(join(set, 'navigation.generated.json'), '{"stale":true}'),
		]);
		const command = [script, '--source', input, '--project', 'project', '--output', output];
		const ownership = {
			milestones: [{ name: 'phase', directory: 'project/phase' }],
			requiredBeforeRegeneration:
				'Update the listed generators for lean records before regenerating.',
		};
		await expect(
			run(process.execPath, [...command, '--generator-owned', 'unknown']),
		).rejects.toThrow(/Unknown generator-owned milestone namespace: unknown/);
		await expect(stat(output)).rejects.toMatchObject({ code: 'ENOENT' });
		const generatorArgs = ['--generator-owned', 'phase', '--generator-owned', 'phase'];
		const dryRun = await run(process.execPath, [...command, ...generatorArgs, '--dry-run']);
		expect(JSON.parse(dryRun.stdout)).toMatchObject({
			dryRun: true,
			unchangedRuleStatements: 2,
			generatorFollowUp: ownership,
		});
		await expect(stat(output)).rejects.toMatchObject({ code: 'ENOENT' });
		expect(await readFile(join(set, 'index.html'), 'utf8')).toBe(oldHtml);
		expect(await readFile(join(set, 'design.ts'), 'utf8')).toBe(oldSource);
		await writeFile(
			join(set, 'design.ts'),
			`throw new Error('This module must never run.');\n${oldSource}`,
		);
		await expect(run(process.execPath, command)).rejects.toThrow(
			/expected a legacy default literal/,
		);
		await expect(stat(output)).rejects.toMatchObject({ code: 'ENOENT' });
		await writeFile(join(set, 'design.ts'), oldSource);

		const pageFile = join(set, 'index.html');
		const unsupportedPage = oldHtml.replace(
			'<h3 id="R1">Visible result</h3>',
			'<div id="R1">Visible result</div>',
		);
		await writeFile(pageFile, unsupportedPage);
		const placementError = /index\.html#R1: no safe placement for phase\/R1 at <div>/;
		await expect(run(process.execPath, command)).rejects.toThrow(placementError);
		await expect(run(process.execPath, [...command, '--dry-run'])).rejects.toThrow(placementError);
		await expect(stat(output)).rejects.toMatchObject({ code: 'ENOENT' });
		expect(await readFile(pageFile, 'utf8')).toBe(unsupportedPage);
		await writeFile(pageFile, oldHtml);

		const outside = join(scratch, 'outside-ignore');
		const ignoreFile = join(project, '.gitignore');
		await symlink(outside, ignoreFile);
		await expect(run(process.execPath, [...command, '--dry-run'])).rejects.toThrow(
			/A migration input is a symlink: .*\.gitignore/,
		);
		await expect(run(process.execPath, command)).rejects.toThrow(
			/A migration input is a symlink: .*\.gitignore/,
		);
		await expect(stat(output)).rejects.toMatchObject({ code: 'ENOENT' });
		await expect(stat(outside)).rejects.toMatchObject({ code: 'ENOENT' });
		await unlink(ignoreFile);

		const migrated = await run(process.execPath, [...command, ...generatorArgs]);
		const outProject = join(output, 'project');
		const outSet = join(outProject, 'phase');
		const html = await readFile(join(outSet, 'index.html'), 'utf8');
		expect(html).toContain('<p data-design-record="R1"></p>');
		expect(html).toMatch(
			/<ex-callout title="Reader context">[\s\S]*?<p data-design-record="N1"><\/p>[\s\S]*?<\/ex-callout>/u,
		);
		expect(html).toContain('<p data-design-record="V1"></p>');
		expect(html).toContain('<h3 id="D1">Owner decision</h3>');
		expect(html).toContain('<p data-design-record="phase/D1"></p>');
		expect
			.soft(html)
			.toContain('<p>Read <a href="#evidence">the evidence</a> for the next case.</p>');
		expect
			.soft(html)
			.toContain('<p>Compare the <a href="#evidence">saved record</a> before proceeding.</p>');
		expect(html).toContain('<p><a href="#evidence">Evidence index</a></p>');
		expect
			.soft(html)
			.toMatch(
				/data-design-record="R1"><\/p>\s*<p><a href="index.html#evidence">Read the observed output\.<\/a>/,
			);
		expect(html).toMatch(
			/data-design-record="R2"><\/p>\s*<p>See <a href="#evidence">the observed output<\/a>/,
		);
		const design = await readFile(join(outSet, 'design.ts'), 'utf8');
		expect(design).toContain('Read the output record');
		expect(design).toContain('"statement": "Observe the result."');
		expect(design).toContain('"statement": "Inline check title"');
		const report: unknown = JSON.parse(migrated.stdout);
		const fallback: unknown = expect.arrayContaining([
			{ id: 'phase/V3', field: 'statement', source: 'old title' },
		]);
		expect(report).toMatchObject({
			dryRun: false,
			defaults: fallback,
			generatorFollowUp: ownership,
		});
		await expect(stat(join(outSet, 'navigation.generated.json'))).rejects.toMatchObject({
			code: 'ENOENT',
		});
		const checked = await run(process.execPath, [cli, 'project', 'check', outProject]);
		expect(checked.stdout).toContain('Checked 1 project design(s).');
		expect(await readFile(join(set, 'index.html'), 'utf8')).toBe(oldHtml);
		await expect(run(process.execPath, command)).rejects.toThrow(/Output already exists/);
	} finally {
		await rm(scratch, { recursive: true, force: true });
	}
}, 30_000);

it.each(['design.ts', 'design-delivery.ts'])(
	'converts a nested deliverable when the manifest imports %s',
	async (manifestEntry): Promise<void> => {
		const scratch = await mkdtemp(join(tmpdir(), 'demistify-lean-delivery-'));
		try {
			const input = join(scratch, 'input');
			const output = join(scratch, 'output');
			const project = join(input, 'project');
			const set = join(project, 'phase');
			const deliverables = join(set, 'deliverables', 'topic');
			const baseDeliverable = {
				id: 'DEL-01',
				title: 'Delivery',
				destination: { page: 'index.html', fragment: 'DEL-01' },
				scope: { rules: ['R1'], implementationTasks: ['task'], verificationChecks: ['V1'] },
			};
			const coreSource =
				"import { defineDeliverable } from '../../design-sdk.generated.ts';\n" +
				'export default defineDeliverable(' +
				JSON.stringify(baseDeliverable) +
				');';
			const entryFile = join(set, 'design-delivery.ts');
			const entryImports =
				"import design from './design.ts';\n" +
				"import core from './deliverables/topic/core.ts';\n" +
				"import { defineDelivery } from './design-sdk.generated.ts';\n";
			await mkdir(deliverables, { recursive: true });
			await Promise.all([
				writeFile(
					join(project, 'project.ts'),
					"import phase from './phase/" +
						manifestEntry +
						"'; export default { designs: { phase } };",
				),
				writeFile(join(set, 'index.html'), oldHtml),
				writeFile(join(set, 'design.ts'), 'export default ' + JSON.stringify(oldDesign)),
				writeFile(
					entryFile,
					entryImports + 'export default defineDelivery(design, [{ deliverable: core }]);',
				),
				writeFile(join(deliverables, 'core.ts'), coreSource),
			]);
			const command = [script, '--source', input, '--project', 'project', '--output', output];
			await run(process.execPath, command);
			const converted = join(output, 'project', 'phase', 'deliverables', 'topic', 'core.ts');
			expect(await readFile(converted, 'utf8')).toContain('"kind": "core"');
			expect(await readFile(join(deliverables, 'core.ts'), 'utf8')).toBe(coreSource);
			const checked = await run(process.execPath, [
				cli,
				'project',
				'check',
				join(output, 'project'),
			]);
			expect(checked.stdout).toContain('phase/DEL-01 · planned · Delivery');
			const unsupportedOutput = join(scratch, 'unsupported');
			await writeFile(
				entryFile,
				entryImports +
					'export default defineDelivery(design, [{ deliverable: ' +
					JSON.stringify(baseDeliverable) +
					' }]);',
			);
			await expect(
				run(process.execPath, [...command.slice(0, -1), unsupportedOutput]),
			).rejects.toThrow(/expected all legacy deliverables to be declared in imported modules/);
			await expect(stat(unsupportedOutput)).rejects.toMatchObject({ code: 'ENOENT' });
		} finally {
			await rm(scratch, { recursive: true, force: true });
		}
	},
	30_000,
);

it('retains quoted container evidence and keeps public project record links at the placement', async (): Promise<void> => {
	const scratch = await mkdtemp(join(tmpdir(), 'demistify-evidence-container-'));
	try {
		const input = join(scratch, 'input');
		const project = join(input, 'project');
		const output = join(scratch, 'output');
		const page = join(project, 'index.html');
		const html = `<!doctype html><html lang="en"><head><title>Decisions</title>
<link rel="stylesheet" href="https://demistify.getagentstudio.dev/v1/theme.css">
<script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script></head>
<body><ex-document id="content"><h1>Decisions</h1>
<details id="D1"><summary>Owner decision</summary>
<ex-callout title="Owner wording"><p>Keep the matching result available.</p></ex-callout></details>
<ex-callout id="A1" title="Confirmed assumption"><p>The reader has permission.</p>
<p>Confirmed on the primary reader.</p></ex-callout>
<p><a href="#D1">Read the decision</a> and <a href="#A1">read the assumption</a>.</p>
<ex-project-overview src="./project.generated.json"></ex-project-overview></ex-document></body></html>`;
		await mkdir(project, { recursive: true });
		await Promise.all([
			writeFile(page, html),
			writeFile(
				join(project, 'project.ts'),
				"import decisions from './project-decisions.ts'; import assumptions from './project-assumptions.ts'; export default { decisions, assumptions };",
			),
			writeFile(
				join(project, 'project-decisions.ts'),
				`export default ${JSON.stringify(oldDecision)};`,
			),
			writeFile(
				join(project, 'project-assumptions.ts'),
				'export default ' +
					JSON.stringify([
						{
							id: 'A1',
							kind: 'assumption',
							title: 'Reader permission',
							statement: 'The reader has permission.',
							destination: { page: 'index.html', fragment: 'A1' },
							status: 'confirmed',
							sources: [
								{ href: './index.html#A1', excerpt: 'The reader has permission.', verbatim: true },
							],
							resolution: {
								href: 'index.html#A1',
								excerpt: 'Confirmed on the primary reader.',
								verbatim: true,
							},
						},
					]) +
					';',
			),
		]);
		const recordInput = join(scratch, 'required-record-input.json');
		await writeFile(
			recordInput,
			JSON.stringify({
				'project/A1': {
					[decisionActorField]: { alias: 'Reader Lead', role: 'lead' },
					settledBy: 'Reader Verifier',
				},
			}),
		);
		const command = [
			script,
			'--source',
			input,
			'--project',
			'project',
			'--record-input',
			recordInput,
			'--output',
			output,
		];
		const { stdout } = await run(process.execPath, command);
		const outProject = join(output, 'project');
		const converted = await readFile(join(outProject, 'index.html'), 'utf8');
		expect(converted).toContain('<details id="D1-source">');
		expect(converted).toMatch(
			/<\/details>\s*<span id="D1"><\/span>\s*<p data-design-record="project\/D1"><\/p>/,
		);
		expect(converted).toContain('<ex-callout id="A1-source"');
		expect(converted).toMatch(
			/<\/ex-callout>\s*<span id="A1"><\/span>\s*<p data-design-record="project\/A1"><\/p>/,
		);
		expect(converted).toContain('<a href="#D1">Read the decision</a>');
		expect(converted).toContain('<a href="#A1">read the assumption</a>');
		expect(await readFile(join(outProject, 'project-decisions.ts'), 'utf8')).toContain(
			'"href": "index.html#D1-source"',
		);
		const assumptions = await readFile(join(outProject, 'project-assumptions.ts'), 'utf8');
		expect(assumptions).toContain('"href": "./index.html#A1-source"');
		expect(assumptions).toContain('"href": "index.html#A1-source"');
		expect((await run(process.execPath, [cli, 'project', 'check', outProject])).stdout).toContain(
			'Checked 0 project design(s).',
		);
		const report: unknown = JSON.parse(stdout);
		expect(report).toMatchObject({
			passages: [
				{
					id: 'project/D1',
					sourceSplit: {
						anchor: 'D1-source',
						retargeted: [{ from: 'index.html#D1', to: 'index.html#D1-source' }],
					},
				},
				{ id: 'project/A1', sourceSplit: { anchor: 'A1-source' } },
			],
		});
		expect(await readFile(page, 'utf8')).toBe(html);
		const dryRun = [...command.slice(0, -2), '--dry-run'];
		const nested = html
			.replace('<details id="D1">', '<ex-callout title="Enclosing callout"><details id="D1">')
			.replace('</details>', '</details></ex-callout>');
		await writeFile(page, nested);
		await expect(run(process.execPath, dryRun)).rejects.toThrow(
			/cannot place a record inside <ex-callout>/,
		);
		await writeFile(page, html.replace('<h1>Decisions</h1>', '<h1 id="D1-source">Decisions</h1>'));
		await expect(run(process.execPath, dryRun)).rejects.toThrow(
			/source anchor #D1-source already exists/,
		);
	} finally {
		await rm(scratch, { recursive: true, force: true });
	}
}, 30_000);

it('copies a typed record project with explicit assumption roles and a single option carrier', async (): Promise<void> => {
	const scratch = await mkdtemp(join(tmpdir(), 'demistify-record-authorship-'));
	try {
		const source = join(scratch, 'input');
		const project = join(source, 'project');
		const design = join(project, 'example');
		const output = join(scratch, 'converted');
		const recordInput = join(scratch, 'authors.json');
		await mkdir(design, { recursive: true });
		const choiceSource = `import { defineDecision } from './design-sdk.generated.ts';
export default [defineDecision({ id: 'D1', question: 'Keep the fixture?', recommended: 'none',
options: [{ id: 'yes', title: 'Keep it' }, { id: 'no', title: 'Remove it' }], authority: { state: 'owner-open' } })];`;
		const pageSource = `<!doctype html><head><title>Example</title>${exampleLibrary}</head><body>
<ex-nav for="content" src="./navigation.generated.json"></ex-nav><ex-document id="content">
<h1>Example</h1>
<ex-section id="open" title="Open" holds="needs-owner"></ex-section>
<ex-section id="agent" title="Agent" holds="decided-by-agent"></ex-section>
<ex-section id="settled" title="Settled" holds="settled"></ex-section>
<ex-section-item record="D1"><ex-choice>
<ex-option data-option="yes"><template slot="summary">Keep the fixture.</template><div slot="details"><p>Retain the link.</p></div></ex-option>
<ex-option data-option="no"><template slot="summary">Remove the fixture.</template></ex-option>
</ex-choice></ex-section-item>
<ex-choice id="ordinary" title="Prefer a fixture?"><ex-option id="existing" title="Existing"><template slot="summary">An ordinary choice.</template><div slot="details">Ordinary choice.</div></ex-option></ex-choice>
</ex-document></body>`;
		await Promise.all([
			writeFile(
				join(project, 'project.ts'),
				"import example from './example/design.ts'; export default { designs: { example } };",
			),
			writeFile(
				join(design, 'design.ts'),
				"import { defineDesign } from './design-sdk.generated.ts'; export default defineDesign({ pages: [{id:'page',href:'index.html',title:'Example',icon:'file'}], mapGroups: [], records: [] });",
			),
			writeFile(join(design, 'design-decisions.ts'), choiceSource),
			writeFile(
				join(design, 'design-assumptions.ts'),
				"import { defineAssumption } from './design-sdk.generated.ts'; export default [defineAssumption({ id: 'A1', statement: 'The fixture can restart.', by: 'Reader Lead', weight: 'design', status: 'open', settles: 'Run the restart fixture.' })];",
			),
			writeFile(join(design, 'index.html'), pageSource),
			writeFile(
				recordInput,
				JSON.stringify({
					'example/A1': { [decisionActorField]: { alias: 'Reader Lead', role: 'lead' } },
				}),
			),
		]);
		const command = [script, '--source', source, '--project', 'project', '--record-authorship'];
		await expect(run(process.execPath, [...command, '--dry-run'])).rejects.toHaveProperty(
			'stderr',
			expect.stringContaining('explicit role'),
		);
		for (const variant of [
			{ details: '<div slot="details">Duplicate.</div>', message: /more than one Details child/ },
			{
				details:
					'<div slot="details"><ex-option data-option="inner"><div slot="details">Inner.</div></ex-option></div>',
				message: /Overlapping record authoring edits/,
			},
		]) {
			// oxlint-disable-next-line no-await-in-loop -- one authored page, one deciding malformed input per run.
			await writeFile(
				join(design, 'index.html'),
				pageSource.replace(
					'<div slot="details"><p>Retain the link.</p></div>',
					variant.details.includes('Duplicate')
						? '<div slot="details"><p>Retain the link.</p></div>' + variant.details
						: variant.details,
				),
			);
			// oxlint-disable-next-line no-await-in-loop -- the next fixture would otherwise replace this input before it is read.
			await expect(
				run(process.execPath, [...command, '--record-input', recordInput, '--dry-run']),
			).rejects.toThrow(variant.message);
		}
		await writeFile(join(design, 'index.html'), pageSource);
		await run(process.execPath, [...command, '--record-input', recordInput, '--output', output]);
		const migrated = join(output, 'project', 'example');
		expect(await readFile(join(migrated, 'design-decisions.ts'), 'utf8')).toContain(
			'defineChoice({',
		);
		expect(await readFile(join(design, 'design-decisions.ts'), 'utf8')).toBe(choiceSource);
		expect(await readFile(join(migrated, 'design-assumptions.ts'), 'utf8')).toContain(
			'{"alias":"Reader Lead","role":"lead"}',
		);
		const page = await readFile(join(migrated, 'index.html'), 'utf8');
		expect(page).toContain('<template slot="details"><p>Retain the link.</p></template>');
		expect(page).toContain('<div slot="details">Ordinary choice.</div>');
		const checked = await run(process.execPath, [
			cli,
			'project',
			'check',
			join(output, 'project'),
			'--site-root',
			output,
		]);
		expect(checked.stdout).toContain('and 2 design record(s).');
	} finally {
		await rm(scratch, { recursive: true, force: true });
	}
});

it('upgrades released lean sidecars and wraps sections without changing their authored content', async (): Promise<void> => {
	const scratch = await mkdtemp(join(tmpdir(), 'demistify-released-lean-'));
	try {
		const source = join(scratch, 'input');
		const project = join(source, 'project');
		const design = join(project, 'example');
		const output = join(scratch, 'converted');
		const recordInput = join(scratch, 'records.json');
		await mkdir(design, { recursive: true });
		const decisionSource = `import { defineDecision } from './design-sdk.generated.ts';
export default [
  defineDecision({ id: 'D1', title: 'Expiry', authority: { state: 'owner-decided', owner: 'owner', decision: 'Use expiry.  ', decidedOn: '2026-11-24', sourceKind: 'owner-words' }, sources: [{ href: 'index.html#evidence', excerpt: 'Use expiry.', verbatim: true }, { href: 'index.html#evidence', excerpt: 'Approved by me.' }] }),
  // weight: process
  defineDecision({ id: 'D2', /* preserve this rationale note */ title: 'Review order', authority: { state: 'lead-decided', lead: 'reader-lead', decision: 'Run the narrow test first.', rationale: 'It is faster.', decidedOn: '2026-11-24', sourceKind: 'author' } }),
  defineDecision({ id: 'D3', title: 'Reopen', authority: { state: 'owner-open', owner: 'owner', question: 'Should the fixture reopen?' } }),
];`;
		const assumptionSource = `import { defineAssumption } from './design-sdk.generated.ts';
export default [
  // weight: design
  // by: reader-lead lead
  // settles: unknown
  defineAssumption({ id: 'A1', statement: 'The fixture can restart.', status: 'open' }),
  defineAssumption({ id: 'A2', statement: 'Expiry is visible.', status: 'confirmed', resolution: { href: 'index.html#evidence', excerpt: 'Use expiry.' } }),
];`;
		const opening = '<ex-section title="Evidence" id="details">';
		const content =
			'\n  <p id="evidence">Use expiry.</p>\n  <a href="#D1">Keep the existing link</a>\n';
		const prefix = `<!doctype html><head><title>Example</title>${exampleLibrary}</head><ex-nav for="content" src="./navigation.generated.json"></ex-nav><ex-document id="content"><h1>Example</h1><p data-design-record="D1"></p><div data-design-record="D2"></div><ex-choice data-design-record="D3"><ex-option id="yes" title="Reopen"><template slot="summary">Reopen the fixture.</template><div slot="details"><p id="explainer">The evidence stays here.</p></div></ex-option><ex-option id="no" title="Keep closed"><template slot="summary">Leave it closed.</template></ex-option></ex-choice><a href="#yes">Read the option</a><a href="#yes--choice-details">Read its Details</a><a href="#explainer">Read the evidence</a><p data-design-record="A1"></p><p data-design-record="A2"></p>`;
		const pageSource = `${prefix}${opening}${content}</ex-section></ex-document>`;
		await Promise.all([
			writeFile(
				join(project, 'project.ts'),
				"import example from './example/design.ts'; export default { designs: { example } };",
			),
			writeFile(
				join(design, 'design.ts'),
				"import { defineDesign } from './design-sdk.generated.ts'; export default defineDesign({ pages: [{ id: 'page', href: 'index.html', title: 'Example', icon: 'file' }], mapGroups: [], records: [] });",
			),
			writeFile(join(design, 'design-decisions.ts'), decisionSource),
			writeFile(join(design, 'design-assumptions.ts'), assumptionSource),
			writeFile(join(design, 'index.html'), pageSource),
		]);
		const command = [
			script,
			'--source',
			source,
			'--project',
			'project',
			'--released-lean',
			'--output',
			output,
		];
		let blockedReport: unknown;
		try {
			await run(process.execPath, command);
		} catch (error: unknown) {
			if (!(error instanceof Error) || !('stdout' in error) || typeof error.stdout !== 'string')
				throw error;
			blockedReport = JSON.parse(error.stdout);
		}
		expect(blockedReport).toMatchObject({
			blocked: true,
			changedFiles: 0,
			missingCount: 4,
			sourceCandidates: {
				'example/D1': [{ excerpt: 'Use expiry.' }, { excerpt: 'Approved by me.' }],
			},
			recordInput: {
				'example/D1': { source: null },
				'example/D3': { question: 'Should the fixture reopen?', recommended: null },
				'example/A1': { settles: null },
				'example/A2': { [decisionActorField]: null, weight: null, settledBy: null },
			},
		});
		await expect(stat(output)).rejects.toMatchObject({ code: 'ENOENT' });
		const author = { alias: 'reader-lead', role: 'lead' };
		await writeFile(
			recordInput,
			JSON.stringify({
				'example/D1': { source: { href: 'index.html#evidence', excerpt: 'Approved by me.' } },
				'example/D2': { weight: 'minor' },
				'example/D3': {
					question: 'Should the fixture reopen?',
					recommended: 'none',
				},
				'example/A1': { settles: 'Run the restart fixture.' },
				'example/A2': { [decisionActorField]: author, weight: 'design', settledBy: 'reader-lead' },
			}),
		);
		const withInput = [...command, '--record-input', recordInput];
		const pageFile = join(design, 'index.html');
		await writeFile(pageFile, pageSource.replace('href="#explainer"', 'href="missing.html"'));
		await expect(run(process.execPath, [...withInput, '--dry-run'])).rejects.toThrow(
			/Missing local link/,
		);
		await expect(run(process.execPath, withInput)).rejects.toHaveProperty(
			'stdout',
			expect.stringContaining('"validationError"'),
		);
		await expect(stat(output)).rejects.toMatchObject({ code: 'ENOENT' });
		await writeFile(pageFile, pageSource);
		const dryRun = await run(process.execPath, [...withInput, '--dry-run']);
		expect(dryRun.stdout).toContain('and 5 design record(s).');
		await expect(stat(output)).rejects.toMatchObject({ code: 'ENOENT' });
		const result = await run(process.execPath, withInput);
		const observed: unknown = JSON.parse(result.stdout);
		expect(observed).toMatchObject({
			changedFiles: 3,
			missingCount: 0,
			pages: [{ file: 'project/example/index.html', sections: ['details'], choices: ['D3'] }],
			conflicts: [
				{ id: 'example/D2', field: 'weight', comment: 'process', input: 'minor', used: 'input' },
			],
			records: [
				{ id: 'example/D1', kind: 'ruling' },
				{ id: 'example/D2', kind: 'ruling' },
				{ id: 'example/D3', kind: 'choice' },
				{ id: 'example/A1', kind: 'assumption' },
				{ id: 'example/A2', kind: 'assumption' },
			],
		});
		const migrated = join(output, 'project', 'example');
		const migratedPage = await readFile(join(migrated, 'index.html'), 'utf8');
		const migratedPrefix = prefix
			.replace('id="yes" title="Reopen"', 'data-option="yes" data-option-anchor="yes" ')
			.replace('id="no" title="Keep closed"', 'data-option="no" data-option-anchor="no" ')
			.replace(
				'<div slot="details"><p id="explainer">The evidence stays here.</p></div>',
				'<template slot="details"><p id="explainer">The evidence stays here.</p></template>',
			);
		expect(migratedPage).toBe(
			`${migratedPrefix}${opening}</ex-section><ex-section-item section="details">${content}</ex-section-item></ex-document>`,
		);
		expect(await readFile(join(design, 'index.html'), 'utf8')).toBe(pageSource);
		const convertedDecisions = await readFile(join(migrated, 'design-decisions.ts'), 'utf8');
		expect(convertedDecisions).toContain('"statement": "Use expiry.  "');
		expect(convertedDecisions).toContain('"weight": "minor"');
		expect(convertedDecisions).toContain('/* preserve this rationale note */');
		expect(convertedDecisions).toMatch(
			/"source": \{\s*"href": "index.html#evidence",\s*"excerpt": "Approved by me\."/,
		);
		const checked = await run(process.execPath, [
			cli,
			'project',
			'check',
			join(output, 'project'),
			'--site-root',
			output,
			'--write',
		]);
		expect(checked.stdout).toContain('and 5 design record(s).');
	} finally {
		await rm(scratch, { recursive: true, force: true });
	}
}, 30_000);
