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
			sources: [{ href: 'index.html#R1', excerpt: 'Visible result.' }],
			relationships: [{ relation: 'serves', targetId: 'N1' }],
		},
		{
			id: 'V1',
			kind: 'planned-check',
			title: 'Observe the result',
			status: 'planned',
			destination: { page: 'index.html', fragment: 'V1' },
			sources: [{ href: 'index.html#V1', excerpt: 'Observe the result.' }],
			relationships: [{ relation: 'checks', targetId: 'R1' }],
			cases: [
				{ given: 'Result exists', expect: 'The result is visible.' },
				{ given: 'Result was updated', expect: 'The latest result is visible.' },
				{ given: 'No result exists', expect: 'The reader sees the empty state.' },
			],
		},
	],
};
const oldHtml = `<!doctype html><html lang="en"><head><title>Old design</title>
<link rel="stylesheet" href="https://demistify.getagentstudio.dev/v1/theme.css">
<script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script></head>
<body><ex-nav for="content" src="navigation.generated.json"></ex-nav><ex-document id="content"><h1>Old design</h1>
<h3 id="N1">Reader need</h3><p>The reader needs the current result.</p>
<h3 id="R1">Visible result</h3><p>Explain where the result appears.</p>
<table><tbody><tr id="V1"><td>V1 · Read the output record</td><td>Check the current result.</td></tr></tbody></table>
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
		const dryRun = await run(process.execPath, [...command, '--dry-run']);
		expect(JSON.parse(dryRun.stdout)).toMatchObject({ dryRun: true, unchangedRuleStatements: 1 });
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

		await run(process.execPath, command);
		const outProject = join(output, 'project');
		const outSet = join(outProject, 'phase');
		const html = await readFile(join(outSet, 'index.html'), 'utf8');
		expect(html).toContain('<p data-design-record="R1"></p>');
		expect(html).toContain('<p data-design-record="V1"></p>');
		expect(html).toContain('<h3 id="D1">Owner decision</h3>');
		expect(html).toContain('<p data-design-record="phase/D1"></p>');
		expect(await readFile(join(outSet, 'design.ts'), 'utf8')).toContain('Read the output record');
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
