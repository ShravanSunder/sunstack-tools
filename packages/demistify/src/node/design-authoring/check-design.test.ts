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
<body><ex-nav for="content" src="./navigation.generated.json"></ex-nav><ex-document id="content">
<h1>Design</h1><h2 id="NeedOverview">Read the overview</h2><p><a href="../overview.html#scope">Overview</a> and <a href="../overview.html#details--from-markdown">details</a></p>
</ex-document></body></html>`;
const overview = `<!doctype html><html lang="en">
<head><title>Overview</title><script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script></head>
<body><ex-nav for="overview-details"></ex-nav>
<ex-document id="overview"><h1>Overview</h1><h2 id="scope">Site scope</h2>
<ex-choice id="shared-decision" data-decision="project/D1" title="Keep the overview?">
<ex-option id="keep-overview" title="Keep it"><template slot="summary">Keep the shared page.</template></ex-option>
</ex-choice></ex-document>
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
		const navigationPath = join(deliverable, 'navigation.generated.json');
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
		await writeFile(join(deliverable, 'navigation.json'), originalNavigation);
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
		const output = await readFile(join(deliverable, 'design.generated.json'), 'utf8');
		expect(JSON.parse(output)).toMatchObject({
			recordDestinations: { NeedOverview: 'index.html#NeedOverview' },
		});
		expect(await readFile(navigationPath, 'utf8')).toBe(originalNavigation);
		expect(await readFile(definition, 'utf8')).toBe(withRecord);
		expect(await readFile(join(deliverable, 'index.html'), 'utf8')).toBe(page);
		const legacyEntry = join(deliverable, 'delivery.ts');
		await writeFile(legacyEntry, 'export default [];');
		await expect(run(process.execPath, check)).rejects.toThrow(
			'Rename delivery.ts to design-delivery.ts before checking this design.',
		);
		await unlink(legacyEntry);
		await writeFile(
			definition,
			withRecord.replace("id: 'NeedOverview', kind: 'need'", "id: 'NeedOverview', kind: 'choice'"),
		);
		await expect(run(process.execPath, check)).rejects.toThrow(/Invalid discriminator value/);
		expect(await readFile(join(deliverable, 'design.generated.json'), 'utf8')).toBe(output);
		await writeFile(definition, withRecord);
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
		await writeFile(
			definition,
			withRecord.replace(
				"page: 'index.html', fragment: 'NeedOverview'",
				"page: 'alias.html', fragment: 'scope'",
			),
		);
		await expect(run(process.execPath, check)).rejects.toThrow(
			'Record NeedOverview must point to a declared HTML page.',
		);
		expect(await readFile(navigationPath, 'utf8')).toBe(originalNavigation);
		expect(await readFile(join(deliverable, 'design.generated.json'), 'utf8')).toBe(output);
		await writeFile(definition, withRecord.replace("'Shared scope'", "'Updated shared scope'"));
		const lock = join(deliverable, '.demistify-design.lock');
		await writeFile(lock, '');
		await expect(run(process.execPath, check)).rejects.toThrow(
			'Another design check has the output lock',
		);
		expect(await readFile(join(deliverable, 'design.generated.json'), 'utf8')).toBe(output);
		await unlink(lock);
		await rename(navigationPath, join(deliverable, 'navigation.saved'));
		await mkdir(navigationPath);
		await expect(run(process.execPath, check)).rejects.toThrow('Expected a generated file');
		expect(await readFile(join(deliverable, 'design.generated.json'), 'utf8')).toBe(output);
	} finally {
		await rm(directory, { recursive: true, force: true });
	}
}, 90_000);

it('resolves Map and Walkthrough record references to the same checked Choice', async (): Promise<void> => {
	const directory = await mkdtemp(join(tmpdir(), 'demistify-decision-navigation-'));
	try {
		await run(process.execPath, [cli, 'design', 'init', directory]);
		const designFile = join(directory, 'design.ts');
		const pageFile = join(directory, 'index.html');
		const choice =
			'<ex-choice id="cancel-policy" title="Stop claimed work?" data-decision="D1"><ex-option id="only-queued" title="Stop queued work"><template slot="summary">Claimed work continues.</template></ex-option><ex-option id="also-running" title="Request a running stop"><template slot="summary">The worker receives a stop request.</template></ex-option></ex-choice>';
		const design = (await readFile(designFile, 'utf8')).replace(
			'"mapGroups": []',
			`"mapGroups": [
			{ title: 'Cancellation', description: 'See what a choice would change.', icon: 'file', target: { record: 'D1' } }
		], "walkthroughs": [
			{ id: 'cancel-route', title: 'Cancellation', description: 'Follow a queued job.',
			  steps: [{ target: { record: 'D1' }, narration: 'Decide what to do after the job is claimed.' }] }
		]`,
		);
		const designWithSharedPassage = design.replace(
			'"records": []',
			`"records": [{
				id: 'N1', kind: 'need', title: 'Stop work', statement: 'The caller can request cancellation.',
				destination: { page: 'index.html', fragment: 'cancel-policy' },
				sources: [{ href: 'index.html#NeedOverview', excerpt: 'Read the overview' }]
			}]`,
		);
		const html = page.replace(/<p><a href="\.\.\/overview[\s\S]*?<\/p>/, choice);
		await Promise.all([
			writeFile(designFile, designWithSharedPassage),
			writeFile(pageFile, html),
			writeFile(
				join(directory, 'design-decisions.ts'),
				`export default [{
				id: 'D1', kind: 'choice', title: 'Cancellation after claim',
				authority: { state: 'owner-open', owner: 'owner', question: 'Stop claimed work?' },
				destination: { page: 'index.html', fragment: 'cancel-policy' },
				sources: [{ href: 'index.html#NeedOverview', excerpt: 'Read the overview' }]
			}];`,
			),
		]);
		const check = [cli, 'design', 'check', directory];
		await run(process.execPath, check);
		const outputFile = join(directory, 'navigation.generated.json');
		const generated = await readFile(outputFile, 'utf8');
		const target = { page: 'home', fragment: 'cancel-policy' };
		expect(JSON.parse(generated)).toMatchObject({
			map: [{ target }],
			walkthroughs: [{ steps: [{ target }] }],
		});

		await writeFile(pageFile, html.replace('data-decision="D1"', 'data-decision="D2"'));
		await expect(run(process.execPath, check)).rejects.toThrow(/unknown design record D2/);
		await writeFile(pageFile, html.replace('Stop claimed work?', 'A different question?'));
		await expect(run(process.execPath, check)).rejects.toThrow(
			/must ask the question recorded in D1/,
		);
		await writeFile(pageFile, html);
		await writeFile(designFile, designWithSharedPassage.replace("record: 'D1'", "record: 'D2'"));
		await expect(run(process.execPath, check)).rejects.toThrow(/unknown design record D2/);
		expect(await readFile(outputFile, 'utf8')).toBe(generated);
	} finally {
		await rm(directory, { recursive: true, force: true });
	}
}, 90_000);

it('checks explicitly imported record arrays and rejects invalid split inputs without replacing snapshots', async (): Promise<void> => {
	const directory = await mkdtemp(join(tmpdir(), 'demistify-design-records-'));
	const milestone = join(directory, 'milestone');
	const check = [cli, 'design', 'check', milestone, '--site-root', directory];
	try {
		await run(process.execPath, [cli, 'design', 'init', milestone]);
		await mkdir(join(milestone, 'records'));
		await writeFile(
			join(milestone, 'index.html'),
			page.replace(/<p><a href[\s\S]*?<\/p>/, '<h2 id="RuleOverview">Keep the overview</h2>'),
		);
		const designPath = join(milestone, 'design.ts');
		const recordPath = join(milestone, 'records', 'overview.ts');
		const initial = await readFile(designPath, 'utf8');
		const authored = `import overview from './records/overview.ts';\n${initial.replace(
			'"records": []',
			`"records": [...overview, {
				id: 'RuleOverview', kind: 'rule', title: 'Keep the overview', statement: 'Retain the overview.',
				destination: { page: 'index.html', fragment: 'RuleOverview' },
				sources: [{ href: 'index.html#NeedOverview', excerpt: 'Overview' }],
				relationships: [{ relation: 'serves', targetId: 'NeedOverview' }],
			}]`,
		)}`;
		const records = `import type { DesignRecord } from '../design-schema.d.ts';
export default [{
	id: 'NeedOverview', kind: 'need', title: 'Shared scope', statement: 'Readers can open the overview.',
	destination: { page: 'index.html', fragment: 'NeedOverview' },
	sources: [{ href: 'index.html#NeedOverview', excerpt: 'Overview' }],
}] as const satisfies readonly DesignRecord[];`;
		await Promise.all([writeFile(designPath, authored), writeFile(recordPath, records)]);
		await run(process.execPath, check);
		const snapshot = join(milestone, 'design.generated.json');
		const checked = await readFile(snapshot, 'utf8');
		expect(JSON.parse(checked)).toMatchObject({
			recordDestinations: {
				NeedOverview: 'index.html#NeedOverview',
				RuleOverview: 'index.html#RuleOverview',
			},
			records: [
				{ id: 'NeedOverview' },
				{ id: 'RuleOverview', relationships: [{ relation: 'serves', targetId: 'NeedOverview' }] },
			],
		});

		const inlineRecord = records.slice(
			records.indexOf('[{') + 1,
			records.indexOf('}] as const') + 1,
		);
		const equivalentInline = authored
			.replace("import overview from './records/overview.ts';\n", '')
			.replace('...overview', inlineRecord);
		await writeFile(designPath, equivalentInline);
		await run(process.execPath, check);
		expect(await readFile(snapshot, 'utf8')).toBe(checked);
		await writeFile(designPath, authored);

		await writeFile(
			join(directory, 'project.ts'),
			`import milestone from './milestone/design.ts';
export default { designs: { milestone } };`,
		);
		await run(process.execPath, [cli, 'project', 'check', directory, '--write']);
		const project = await readFile(join(directory, 'project.generated.json'), 'utf8');
		expect(JSON.parse(project)).toMatchObject({
			records: [
				{ id: 'milestone/NeedOverview' },
				{
					id: 'milestone/RuleOverview',
					relationships: [{ relation: 'serves', targetId: 'milestone/NeedOverview' }],
				},
			],
		});

		await writeFile(recordPath, records.replace("kind: 'need'", "kind: 'choice'"));
		await expect(run(process.execPath, check)).rejects.toThrow(
			/records\/overview\.ts record NeedOverview.*Invalid discriminator/,
		);
		await writeFile(
			recordPath,
			records.replace('satisfies readonly DesignRecord[]', 'satisfies readonly never[]'),
		);
		await expect(run(process.execPath, check)).rejects.toThrow(/records\/overview\.ts.*TS2322/s);
		await writeFile(recordPath, 'export default (() => [])();');
		await expect(run(process.execPath, check)).rejects.toThrow(
			/records\/overview\.ts.*expected a data literal/,
		);
		await writeFile(recordPath, records);
		await writeFile(designPath, authored.replace('...overview', '...overview.slice()'));
		await expect(run(process.execPath, check)).rejects.toThrow(/spread.*imported record array/i);
		await writeFile(designPath, authored.replace('...overview,', '...overview, ...overview,'));
		await expect(run(process.execPath, check)).rejects.toThrow(/Duplicate design record ID/);

		const outside = join(directory, 'outside.ts');
		await writeFile(outside, 'export default [];');
		await writeFile(designPath, authored.replace('./records/overview.ts', '../outside.ts'));
		await expect(run(process.execPath, check)).rejects.toThrow(/escapes the design set/);
		await symlink(outside, join(milestone, 'records', 'external.ts'));
		await writeFile(designPath, authored.replace('./records/overview.ts', './records/external.ts'));
		await expect(run(process.execPath, check)).rejects.toThrow(
			/must remain inside this design set/,
		);
		expect(await readFile(snapshot, 'utf8')).toBe(checked);
	} finally {
		await rm(directory, { recursive: true, force: true });
	}
}, 60_000);
