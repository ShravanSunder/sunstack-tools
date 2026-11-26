import { execFile } from 'node:child_process';
import { mkdir, mkdtemp, readFile, realpath, rm, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { expect, it } from 'vitest';

import { routeCheckTargets } from './check-routing.ts';

const run = promisify(execFile);
const cli = fileURLToPath(new URL('../../bin/demistify.ts', import.meta.url));

const decisions = `import { defineChoice } from './design-sdk.generated.ts';
export default [
  defineChoice({
    id: 'D2',
    question: 'Should an expired row stay visible until the reader scrolls?',
    options: [ { id: 'hide-at-once', title: 'Hide at once' }, { id: 'grey-until-scroll', title: 'Grey until scroll' } ],
    recommended: 'none',
    authority: { state: 'owner-open' },
  }),
];
`;

const typedPage = `<!doctype html><html lang="en"><head><title>Design</title>
<link rel="stylesheet" href="https://demistify.getagentstudio.dev/v1/theme.css">
<script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script></head><body>
<ex-nav for="content" src="./navigation.generated.json"></ex-nav><ex-document id="content"><h1>Design</h1>
<ex-section id="open" title="For you to decide" holds="needs-owner"></ex-section>
<ex-section-item record="D2"><ex-choice><ex-option data-option="hide-at-once"><template slot="summary">Hide.</template></ex-option><ex-option data-option="grey-until-scroll"><template slot="summary">Grey.</template></ex-option></ex-choice></ex-section-item>
<ex-section id="agent" title="Decided without you" holds="decided-by-agent"></ex-section>
<ex-section id="settled" title="Settled" holds="settled"></ex-section>
</ex-document></body></html>`;

const plainPage = `<!doctype html><html lang="en"><head><title>Notes</title>
<link rel="stylesheet" href="https://demistify.getagentstudio.dev/v1/theme.css">
<script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script></head><body>
<ex-document id="notes"><h1>Notes</h1><ex-callout tone="note"><p>A plain page.</p></ex-callout></ex-document></body></html>`;

it('one check runs a typed page through its design set and a plain page through the page check', async (): Promise<void> => {
	const root = await mkdtemp(join(tmpdir(), 'demistify-one-check-'));
	const project = join(root, 'design');
	try {
		await run(process.execPath, [cli, 'design', 'init', project]);
		await Promise.all([
			writeFile(join(project, 'design-decisions.ts'), decisions),
			writeFile(join(project, 'index.html'), typedPage),
			writeFile(join(root, 'notes.html'), plainPage),
		]);

		const checked = await run(process.execPath, [
			cli,
			'check',
			join(project, 'index.html'),
			join(root, 'notes.html'),
			'--write',
		]);

		expect(checked.stdout).toContain(`(design check for ${join(project, 'index.html')}):`);
		expect(checked.stdout).toContain(
			'Checked 1 authored page(s), 1 navigation page(s), and 1 design record(s).',
		);
		expect(checked.stdout).toContain('1 page and 1 typed tree: 0 errors');
		await expect(stat(join(project, 'navigation.generated.json'))).resolves.toBeTruthy();
		const byDirectory = await run(process.execPath, [cli, 'check', project]);
		expect(byDirectory.stdout).toContain('1 typed tree: 0 errors');

		await expect(
			run(process.execPath, [cli, 'check', join(root, 'mistyped.html')]),
		).rejects.toThrow('mistyped.html: no such page or folder.');
		const published = join(root, 'published');
		await mkdir(published);
		await Promise.all(
			['index.html', 'navigation.generated.json', 'records.generated.js'].map(async (file) =>
				writeFile(join(published, file), await readFile(join(project, file), 'utf8')),
			),
		);
		const copy = await run(process.execPath, [cli, 'check', join(published, 'index.html')]);
		expect(copy.stdout).toContain(
			`${join(published, 'index.html')}: no initiative.ts or design.ts encloses it, so it gets the page check as a published copy.`,
		);
		expect(copy.stdout).toContain('1 file: 0 errors');
	} finally {
		await rm(root, { recursive: true, force: true });
	}
}, 30_000);

it('a project page routes to its enclosing initiative, once per tree', async (): Promise<void> => {
	const root = await realpath(await mkdtemp(join(tmpdir(), 'demistify-check-routes-')));
	const project = join(root, 'projects/2026-11-notes');
	try {
		await mkdir(project, { recursive: true });
		await Promise.all([
			writeFile(join(root, 'initiative.ts'), 'export default {};'),
			writeFile(join(project, 'design.ts'), 'export default {};'),
			writeFile(join(project, 'index.html'), typedPage),
			writeFile(
				join(root, 'index.html'),
				'<meta name="demistify-records" content="records.generated.js">',
			),
			writeFile(join(root, 'plain.html'), plainPage),
		]);

		const routes = await routeCheckTargets([
			join(project, 'index.html'),
			join(root, 'index.html'),
			join(root, 'plain.html'),
		]);

		expect(routes).toEqual({
			pages: [join(root, 'plain.html')],
			trees: [
				{
					kind: 'initiative',
					directory: root,
					inputs: [join(project, 'index.html'), join(root, 'index.html')],
				},
			],
			published: [],
		});
	} finally {
		await rm(root, { recursive: true, force: true });
	}
});
