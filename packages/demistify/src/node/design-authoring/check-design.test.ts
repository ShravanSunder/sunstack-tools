import { execFile } from 'node:child_process';
import {
	mkdir,
	mkdtemp,
	readFile,
	rename,
	rm,
	stat,
	symlink,
	unlink,
	writeFile,
} from 'node:fs/promises';
import { createRequire } from 'node:module';
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
<h1>Design</h1><h2 data-design-record="NeedOverview"></h2><p><a href="../overview.html#scope">Overview</a> and <a href="../overview.html#details--from-markdown">details</a></p>
</ex-document></body></html>`;
const overview = `<!doctype html><html lang="en">
<head><title>Overview</title><script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script></head>
<body><ex-nav for="overview-details"></ex-nav>
<ex-document id="overview"><h1>Overview</h1><h2 id="scope">Site scope</h2>
<ex-choice id="shared-decision" data-decision="initiative/D1" title="Keep the overview?">
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

			}]`,
		);
		await writeFile(definition, withRecord);
		const check = [cli, 'design', 'check', deliverable, '--site-root', site, '--write'];
		const result = await run(process.execPath, check);
		expect(result.stdout).toContain(
			'Checked 1 authored page(s), 2 navigation page(s), and 1 design record(s).',
		);
		const output = await readFile(join(deliverable, 'design.generated.json'), 'utf8');
		expect(JSON.parse(output)).toMatchObject({
			recordDestinations: { NeedOverview: { page: 'index.html', fragment: 'NeedOverview' } },
		});
		expect(JSON.parse(await readFile(navigationPath, 'utf8'))).toMatchObject(navigation);
		expect(await readFile(definition, 'utf8')).toBe(withRecord);
		expect(await readFile(join(deliverable, 'index.html'), 'utf8')).toBe(page);
		const advisory =
			'Generated views are missing or stale. Run demistify design check --write to refresh them.';
		const readOnly = check.slice(0, -1);
		expect((await run(process.execPath, readOnly)).stdout).not.toContain(advisory);
		const recordScript = join(deliverable, 'records.generated.js');
		await unlink(recordScript);
		expect((await run(process.execPath, readOnly)).stdout.split(advisory)).toHaveLength(2);
		await expect(stat(recordScript)).rejects.toMatchObject({ code: 'ENOENT' });
		await run(process.execPath, check);
		await expect(
			run(process.execPath, [cli, 'initiative', 'check', deliverable, '--site-root', site]),
		).rejects.toThrow(/initiative\.ts/);
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
			join(deliverable, 'index.html'),
			page.replaceAll('../overview.html', '../escaped.html'),
		);
		await expect(run(process.execPath, check)).rejects.toThrow(
			'Local target resolves outside the site root: escaped.html',
		);
		await writeFile(join(deliverable, 'index.html'), page);
		expect(JSON.parse(await readFile(navigationPath, 'utf8'))).toMatchObject(navigation);
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
			'<ex-choice data-design-record="D1"><ex-option data-option="only-queued"><template slot="summary">Claimed work continues.</template></ex-option><ex-option data-option="also-running"><template slot="summary">The worker receives a stop request.</template></ex-option></ex-choice>';
		const design = (await readFile(designFile, 'utf8')).replace(
			'"mapGroups": []',
			`"mapGroups": [
			{ title: 'Cancellation', description: 'See what a choice would change.', icon: 'file', target: { record: 'D1' } }
		], "walkthroughs": [
			{ id: 'cancel-route', title: 'Cancellation', description: 'Follow a queued job.',
			  steps: [{ target: { record: 'D1' }, narration: 'Decide what to do after the job is claimed.' },
				    { target: { record: 'N1' }, narration: 'Remember the need even without a placement.' }] }
		]`,
		);
		const designWithSharedPassage = design.replace(
			'"records": []',
			`"records": [{
				id: 'N1', kind: 'need', title: 'Stop work', statement: 'The caller can request cancellation.',

			}]`,
		);
		const html = page
			.replace('<h2 data-design-record="NeedOverview"></h2>', '<h2>Read the overview</h2>')
			.replace(/<p><a href="\.\.\/overview[\s\S]*?<\/p>/, choice);
		await Promise.all([
			writeFile(designFile, designWithSharedPassage),
			writeFile(pageFile, html),
			writeFile(
				join(directory, 'design-decisions.ts'),
				`export default [{
				id: 'D1', kind: 'choice', title: 'Cancellation after claim',
				question: 'Stop claimed work?', recommended: 'none', options: [{ id: 'only-queued', title: 'Stop queued work' }, { id: 'also-running', title: 'Request a running stop' }], authority: { state: 'owner-open' },

			}];`,
			),
		]);
		const check = [cli, 'design', 'check', directory, '--write'];
		await run(process.execPath, check);
		const outputFile = join(directory, 'navigation.generated.json');
		const generated = await readFile(outputFile, 'utf8');
		const written = await stat(outputFile);
		await run(process.execPath, check);
		expect(await readFile(outputFile, 'utf8')).toBe(generated);
		expect(await stat(outputFile)).toMatchObject({ ino: written.ino, mtimeMs: written.mtimeMs });
		const target = { page: 'home', fragment: 'D1' };
		const source = await readFile(join(directory, 'design.generated.json'), 'utf8');
		expect(JSON.parse(source)).toMatchObject({ records: [{ id: 'N1' }, { id: 'D1' }] });
		expect(JSON.parse(source)).toMatchObject({
			recordDestinations: { ['D1']: { fragment: 'D1' } },
		});
		expect(source).not.toContain('"N1": {');
		expect(JSON.parse(generated)).toMatchObject({
			map: [{ target }],
			walkthroughs: [
				{ steps: [{ target }, { narration: 'Remember the need even without a placement.' }] },
			],
		});

		await writeFile(pageFile, html.replace('data-design-record="D1"', 'data-design-record="D2"'));
		await expect(run(process.execPath, check)).rejects.toThrow(/unknown design record D2/);
		await writeFile(
			pageFile,
			html.replace('</ex-document>', '<p data-design-record="D1"></p></ex-document>'),
		);
		await expect(run(process.execPath, check)).rejects.toThrow(
			/Duplicate HTML id or record placement.*D1/,
		);
		await writeFile(pageFile, html);
		await writeFile(designFile, designWithSharedPassage.replace("record: 'D1'", "record: 'D2'"));
		await expect(run(process.execPath, check)).rejects.toThrow(/TS2322.*D2/s);
		await writeFile(
			designFile,
			designWithSharedPassage.replace("record: 'D1'", "record: 'D2' as 'D1'"),
		);
		await expect(run(process.execPath, check)).rejects.toThrow(/unknown design record D2/);
		expect(await readFile(outputFile, 'utf8')).toBe(generated);
	} finally {
		await rm(directory, { recursive: true, force: true });
	}
}, 90_000);

it('checks Map and Walkthrough IDs from a widened default record module in TypeScript and the CLI', async (): Promise<void> => {
	const directory = await mkdtemp(join(tmpdir(), 'demistify-widened-records-'));
	try {
		const designFile = join(directory, 'design.ts');
		const source = `import { defineDesign, defineNeed } from './design-sdk.generated.ts';
import notes from './notes.ts';
export default defineDesign({
  pages: [{ id: 'home', title: 'Home', href: 'index.html', icon: 'file' }],
  records: [...notes, defineNeed({ id: 'N2', statement: 'Read the current state.' })],
  mapGroups: [{ description: 'Review the scope.', icon: 'file', target: { record: 'N1' } }],
  walkthroughs: [{ id: 'start', title: 'Start', description: 'Review the scope.',
    steps: [{ target: { record: 'N1' }, narration: 'Read the note.' }] }],
});`;
		await Promise.all([
			writeFile(designFile, source),
			writeFile(
				join(directory, 'notes.ts'),
				`import type { DesignRecord } from './design-schema.d.ts';
export default [{ id: 'N1', kind: 'need', statement: 'Readers can find the scope.' }] satisfies readonly DesignRecord[];`,
			),
			writeFile(
				join(directory, 'index.html'),
				page.replace(/<h2 data-design-record=[\s\S]*?<\/p>/, '<h2 data-design-record="N1"></h2>'),
			),
		]);
		await run(process.execPath, [cli, 'design', 'types', directory]);
		const compiler = createRequire(import.meta.url).resolve('typescript/lib/tsc.js');
		const compile = [
			compiler,
			'--noEmit',
			'--strict',
			'--skipLibCheck',
			'--allowImportingTsExtensions',
			'--module',
			'nodenext',
			designFile,
		];
		const check = [cli, 'design', 'check', directory];
		await run(process.execPath, compile);
		await run(process.execPath, check);
		const diagnostic = /TS(?:2322|2820)[^\n]*N9/;
		const invalidMap = source.replace("record: 'N1'", "record: 'N9'");
		const invalidWalkthrough = source.replace(
			"record: 'N1' }, narration",
			"record: 'N9' }, narration",
		);
		for (const invalid of [invalidMap, invalidWalkthrough]) {
			// oxlint-disable-next-line no-await-in-loop -- verify each independently mistyped authored design.
			await writeFile(designFile, invalid);
			// oxlint-disable-next-line no-await-in-loop -- TypeScript writes diagnostics to stdout.
			await expect(run(process.execPath, compile)).rejects.toHaveProperty(
				'stdout',
				expect.stringMatching(diagnostic),
			);
			// oxlint-disable-next-line no-await-in-loop -- the CLI must enforce the same ID type.
			await expect(run(process.execPath, check)).rejects.toThrow(diagnostic);
		}
	} finally {
		await rm(directory, { recursive: true, force: true });
	}
}, 30_000);

it('checks explicitly imported record arrays and rejects invalid split inputs without replacing snapshots', async (): Promise<void> => {
	const directory = await mkdtemp(join(tmpdir(), 'demistify-design-records-'));
	const milestone = join(directory, 'milestone');
	const check = [cli, 'design', 'check', milestone, '--site-root', directory, '--write'];
	try {
		await run(process.execPath, [cli, 'design', 'init', milestone]);
		await mkdir(join(milestone, 'records'));
		await writeFile(
			join(milestone, 'index.html'),
			page.replace(/<p><a href[\s\S]*?<\/p>/, '<h2 data-design-record="RuleOverview"></h2>'),
		);
		const designPath = join(milestone, 'design.ts');
		const recordPath = join(milestone, 'records', 'overview.ts');
		const initial = await readFile(designPath, 'utf8');
		const authored = `import overview from './records/overview.ts';\n${initial.replace(
			'"records": []',
			`"records": [...overview, {
				id: 'RuleOverview', kind: 'rule', title: 'Keep the overview', statement: 'Retain the overview.',
				serves: 'NeedOverview',
			}]`,
		)}`;
		const records = `import type { DesignRecord } from '../design-schema.d.ts';
export default [{
	id: 'NeedOverview', kind: 'need', title: 'Shared scope', statement: 'Readers can open the overview.',
}] as const satisfies readonly DesignRecord[];`;
		await Promise.all([writeFile(designPath, authored), writeFile(recordPath, records)]);
		await run(process.execPath, check);
		const snapshot = join(milestone, 'design.generated.json');
		const checked = await readFile(snapshot, 'utf8');
		expect(JSON.parse(checked)).toMatchObject({
			recordDestinations: {
				NeedOverview: { page: 'index.html', fragment: 'NeedOverview' },
				RuleOverview: { page: 'index.html', fragment: 'RuleOverview' },
			},
			records: [{ id: 'NeedOverview' }, { id: 'RuleOverview', serves: 'NeedOverview' }],
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
			join(directory, 'initiative.ts'),
			`import milestone from './milestone/design.ts';
export default { projects: { milestone: { design: milestone } } };`,
		);
		await run(process.execPath, [cli, 'initiative', 'check', directory, '--write']);
		const project = await readFile(join(directory, 'initiative.generated.json'), 'utf8');
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
		const sentinel = join(directory, 'authored-code-ran.txt');
		await writeFile(
			recordPath,
			`import { writeFileSync } from 'node:fs';
export default (() => { writeFileSync(${JSON.stringify(sentinel)}, 'ran'); return []; })();`,
		);
		await expect(run(process.execPath, check)).rejects.toThrow(/records\/overview\.ts/);
		await expect(stat(sentinel)).rejects.toMatchObject({ code: 'ENOENT' });
		await writeFile(recordPath, 'export default (() => [])();');
		await expect(run(process.execPath, check)).rejects.toThrow(
			/records\/overview\.ts.*export a literal array of records/,
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
