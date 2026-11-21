import { execFile } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rename, rm, symlink, unlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { expect, it } from 'vitest';

const run = promisify(execFile);
const cli = fileURLToPath(new URL('../../../bin/demistify.ts', import.meta.url));
const page = `<!doctype html><html lang="en">
<head><title>Design</title><link rel="stylesheet" href="https://demistify.getagentstudio.dev/v1/theme.css">
<script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script></head>
<body><ex-nav for="content" src="./navigation.json"></ex-nav><ex-document id="content">
<h1>Design</h1><h2 id="NeedOverview">Read the overview</h2><p><a href="../overview.html#scope">Overview</a> and <a href="../overview.html#details--from-markdown">details</a></p>
</ex-document></body></html>`;
const overview = `<!doctype html><html lang="en">
<head><title>Overview</title><script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script></head>
<body><ex-nav for="overview-details"></ex-nav>
<ex-document id="overview"><h1>Overview</h1><h2 id="scope">Site scope</h2></ex-document>
<main id="overview-details"><h2>Example</h2><h2>Example</h2>
<template><h2 id="heading-example-2">Inert template</h2></template>
<ex-markdown id="details"><script type="text/markdown">## From Markdown</script></ex-markdown>
</main></body></html>`;

it('checks a real adjacent page and source inside an explicitly selected site root', async (): Promise<void> => {
	const directory = await mkdtemp(join(tmpdir(), 'demistify-design-check-'));
	try {
		const site = join(directory, 'site');
		const deliverable = join(site, 'deliverable');
		const definition = join(deliverable, 'design.ts');
		const navigationPath = join(deliverable, 'navigation.json');
		await mkdir(deliverable, { recursive: true });
		await writeFile(join(deliverable, 'index.html'), page);
		await writeFile(join(site, 'overview.html'), overview);
		const navigation = {
			version: 1,
			pages: [
				{ id: 'design', title: 'Design', href: 'index.html', icon: 'file' },
				{ id: 'overview', title: 'Overview', href: '../overview.html', icon: 'file' },
			],
			map: [
				{
					title: 'Second example',
					description: 'Open a generated fragment for an HTML heading.',
					icon: 'file',
					target: { page: 'overview', fragment: 'heading-example-2' },
				},
				{
					title: 'Markdown details',
					description: 'Open the generated heading.',
					icon: 'file',
					target: { page: 'overview', fragment: 'details--from-markdown' },
				},
				{
					title: 'Scope',
					description: 'Read the shared design scope.',
					icon: 'file',
					target: { page: 'overview', fragment: 'scope' },
				},
			],
		};
		const originalNavigation = JSON.stringify(navigation);
		await writeFile(navigationPath, originalNavigation);
		await run(process.execPath, [cli, 'design', 'init', deliverable]);
		const original = await readFile(definition, 'utf8');
		const withRecord = original.replace(
			'"records": []',
			`"records": [{
				id: 'NeedOverview', kind: 'need', title: 'Shared scope', statement: 'Readers can open the site overview.',
				destination: { page: 'index.html', fragment: 'NeedOverview' },
				sources: [{ href: '../overview.html#scope', excerpt: 'Site scope' }]
			}]`,
		);
		await writeFile(definition, withRecord);
		const check = [cli, 'design', 'check', deliverable, '--site-root', site];
		const result = await run(process.execPath, check);
		expect(result.stdout).toContain(
			'Checked 1 authored page(s), 2 navigation page(s), and 1 design record(s).',
		);
		const output = await readFile(join(deliverable, 'design.json'), 'utf8');
		expect(JSON.parse(output)).toMatchObject({
			recordDestinations: { NeedOverview: 'index.html#NeedOverview' },
		});
		expect(await readFile(navigationPath, 'utf8')).toBe(originalNavigation);
		expect(await readFile(definition, 'utf8')).toBe(withRecord);
		expect(await readFile(join(deliverable, 'index.html'), 'utf8')).toBe(page);
		await expect(run(process.execPath, check.slice(0, 4))).rejects.toThrow(
			'escapes the site root: ../overview.html',
		);
		await writeFile(
			join(site, 'overview.html'),
			overview.replace(
				'<h2 id="scope">Site scope</h2>',
				'<script>const copy = \'id="scope"\';</script>',
			),
		);
		await expect(run(process.execPath, check)).rejects.toThrow(
			'Missing HTML target: overview.html#scope',
		);
		await writeFile(join(site, 'overview.html'), overview);
		await writeFile(join(directory, 'outside.html'), overview);
		await symlink(join(directory, 'outside.html'), join(site, 'escaped.html'));
		await writeFile(
			definition,
			withRecord.replace('../overview.html#scope', '../escaped.html#scope'),
		);
		await expect(run(process.execPath, check)).rejects.toThrow(
			'Local target resolves outside the site root: escaped.html',
		);
		await symlink(join(site, 'overview.html'), join(deliverable, 'alias.html'));
		const aliasPage = withRecord.replace('"href": "../overview.html"', '"href": "alias.html"');
		await writeFile(
			definition,
			aliasPage.replace(
				"page: 'index.html', fragment: 'NeedOverview'",
				"page: 'alias.html', fragment: 'scope'",
			),
		);
		await expect(run(process.execPath, check)).rejects.toThrow(
			'Record NeedOverview must point to a page owned by this design set.',
		);
		expect(await readFile(navigationPath, 'utf8')).toBe(originalNavigation);
		expect(await readFile(join(deliverable, 'design.json'), 'utf8')).toBe(output);
		await writeFile(definition, withRecord.replace("'Shared scope'", "'Updated shared scope'"));
		const lock = join(deliverable, '.demistify-design.lock');
		await writeFile(lock, '');
		await expect(run(process.execPath, check)).rejects.toThrow(
			'Another design check has the output lock',
		);
		expect(await readFile(join(deliverable, 'design.json'), 'utf8')).toBe(output);
		await unlink(lock);
		await rename(navigationPath, join(deliverable, 'navigation.saved'));
		await mkdir(navigationPath);
		await expect(run(process.execPath, check)).rejects.toThrow('Expected a generated file');
		expect(await readFile(join(deliverable, 'design.json'), 'utf8')).toBe(output);
	} finally {
		await rm(directory, { recursive: true, force: true });
	}
}, 15_000);
