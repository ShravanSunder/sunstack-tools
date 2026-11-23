import { execFile } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, stat, symlink, unlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { expect, it } from 'vitest';

const run = promisify(execFile);
const script = fileURLToPath(
	new URL('../../scripts/migrate-lean-design-records.ts', import.meta.url),
);
const cli = fileURLToPath(new URL('../../bin/demistify.ts', import.meta.url));
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
<h3 id="N1">Reader need</h3><p>The reader needs the current result.</p>
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
		const command = [script, '--source', input, '--project', 'project', '--output', output];
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
		await writeFile(page, html.replace('<h1>Decisions</h1>', '<h1 id="D1-source">Decisions</h1>'));
		await expect(run(process.execPath, [...command.slice(0, -2), '--dry-run'])).rejects.toThrow(
			/source anchor #D1-source already exists/,
		);
	} finally {
		await rm(scratch, { recursive: true, force: true });
	}
}, 30_000);
