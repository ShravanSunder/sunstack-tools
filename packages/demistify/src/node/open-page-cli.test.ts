import { execFile, spawn } from 'node:child_process';
import { once } from 'node:events';
import { chmod, mkdir, mkdtemp, readdir, rm, stat, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { expect, it } from 'vitest';

const cli = fileURLToPath(new URL('../../bin/demistify.ts', import.meta.url));
const run = promisify(execFile);

async function withPreview(
	page: string,
	root: string,
	observe: (url: URL) => Promise<void>,
): Promise<void> {
	const processHandle = spawn(
		process.execPath,
		[cli, 'open', page, '--site-root', root, '--no-browser'],
		{ stdio: ['ignore', 'pipe', 'inherit'] },
	);
	const stopped = once(processHandle, 'close');
	const lines = createInterface({ input: processHandle.stdout });
	try {
		const output: unknown = await Promise.race([
			once(lines, 'line', { signal: AbortSignal.timeout(10_000) }),
			stopped.then(() => {
				throw new Error('Preview exited before publishing an address.');
			}),
		]);
		if (!Array.isArray(output) || typeof output[0] !== 'string')
			throw new Error('Expected the preview address on stdout.');
		expect(output[0]).toMatch(/^Open http:\/\/127\.0\.0\.1:\d+\//);
		await observe(new URL(output[0].slice('Open '.length)));
	} finally {
		lines.close();
		processHandle.kill();
		await stopped;
	}
}

it('opens authored pages and their local includes within the selected site root', async (): Promise<void> => {
	const scratch = await mkdtemp(join(tmpdir(), 'demistify-open-'));
	const root = join(scratch, 'site');
	const pages = join(root, 'project');
	const page = join(pages, 'review notes.html');
	await mkdir(pages, { recursive: true });
	const html =
		'<p>The guide mentions <code>navigation.generated.json</code>.</p><ex-markdown src="../notes.md"></ex-markdown>';
	await Promise.all([
		writeFile(page, html),
		writeFile(join(pages, 'index.html'), '<ex-markdown src="notes.md"></ex-markdown>'),
		writeFile(join(pages, 'notes.md'), 'Nested include'),
		writeFile(join(root, 'notes.md'), '## Reviewed passage'),
		writeFile(join(root, 'project.generated.json'), '{"version":1}'),
		writeFile(join(root, '.hidden.json'), '{}'),
		writeFile(join(pages, 'project.ts'), 'throw new Error("Not a Demistify project.");'),
		writeFile(join(scratch, 'outside.md'), 'Outside'),
	]);
	await symlink(join(scratch, 'outside.md'), join(root, 'outside.md'), 'file');
	try {
		await withPreview(page, root, async (url) => {
			expect(url.pathname).toBe('/project/review%20notes.html');
			const [document, markdown, data, outside, hidden] = await Promise.all([
				fetch(url),
				fetch(new URL('../notes.md', url)),
				fetch(new URL('../project.generated.json', url)),
				fetch(new URL('../outside.md', url)),
				fetch(new URL('../.hidden.json', url)),
			]);
			expect(await document.text()).toBe(html);
			expect(await markdown.text()).toBe('## Reviewed passage');
			expect(data.headers.get('content-type')).toBe('application/json');
			expect(await data.text()).toBe('{"version":1}');
			expect(outside.status).toBe(404);
			expect(hidden.status).toBe(404);
			const nested = await fetch(new URL('/project?mode=review', url));
			expect(new URL(nested.url).pathname).toBe('/project/');
			expect(new URL(nested.url).search).toBe('?mode=review');
			expect(await (await fetch(new URL('./notes.md', nested.url))).text()).toBe('Nested include');
		});
	} finally {
		await rm(scratch, { recursive: true, force: true });
	}
}, 10_000);

async function createExternalPageProject(
	directory: string,
	href = '../../Reviewed%20Guide.html',
): Promise<void> {
	const milestone = join(directory, 'milestone');
	await mkdir(milestone, { recursive: true });
	await Promise.all([
		writeFile(
			join(directory, 'project.ts'),
			"import notes from './milestone/design.ts'; export default { designs: { notes } };",
		),
		writeFile(
			join(milestone, 'design.ts'),
			`export default {
			pages: [{ id: 'guide', title: 'Reviewed guide', href: ${JSON.stringify(href)}, icon: 'file' }],
			mapGroups: [], records: [{ kind: 'need', id: 'N1', statement: 'Readers can find the approved scope.' }],
		};`,
		),
	]);
}

it('opens a declared external page without generated files and refuses competing project claims', async (): Promise<void> => {
	const site = await mkdtemp(join(tmpdir(), 'demistify-open-external-'));
	const page = join(site, 'Reviewed Guide.html');
	const project = join(site, 'project');
	const html = '<h1>Reviewed guide</h1><h2 data-design-record="notes/N1"></h2>';

	try {
		await Promise.all([createExternalPageProject(project), writeFile(page, html)]);
		for (const source of [
			'',
			'<ex-project-overview src="project/project.generated.json"></ex-project-overview>',
		]) {
			// oxlint-disable-next-line no-await-in-loop -- exercise the same declared page with and without its project data binding.
			await writeFile(page, `${html}${source}`);
			// oxlint-disable-next-line no-await-in-loop -- the preview validates the page written in this iteration.
			await withPreview(page, site, async (url) => {
				const response = await fetch(url);
				expect(response.status).toBe(200);
				expect(await response.text()).toContain(
					'<h2 data-design-record="notes/N1" id="notes/N1">Readers can find the approved scope.</h2>',
				);
				const view = await fetch(new URL('./project/project.generated.json', url));
				expect(view.status).toBe(200);
			});
		}
		await expect(stat(join(project, 'project.generated.json'))).rejects.toMatchObject({
			code: 'ENOENT',
		});
		await writeFile(page, html);
		await createExternalPageProject(join(site, 'other-project'));
		await expect(
			run(process.execPath, [cli, 'open', page, '--site-root', site, '--no-browser'], {
				timeout: 10_000,
			}),
		).rejects.toThrow(/multiple.*own|ambiguous/i);
	} finally {
		await rm(site, { recursive: true, force: true });
	}
}, 30_000);

it('serves a declared HTML alias at its authored URL and refuses aliases claimed by another project', async (): Promise<void> => {
	const root = await mkdtemp(join(tmpdir(), 'demistify-open-alias-'));
	const site = join(root, 'site');
	const project = join(site, 'project');
	const aliasDirectory = join(site, 'published');
	const alias = join(aliasDirectory, 'Reviewed Guide.html');
	const physicalDirectory = join(site, 'source');
	const physical = join(physicalDirectory, 'guide.html');
	const html =
		'<h1>Guide</h1><h2 data-design-record="notes/N1"></h2><a href="./notes.md">Read notes</a>';
	try {
		await Promise.all([
			mkdir(aliasDirectory, { recursive: true }),
			mkdir(physicalDirectory, { recursive: true }),
		]);
		await Promise.all([
			createExternalPageProject(project, '../../published/Reviewed%20Guide.html'),
			writeFile(physical, html),
			writeFile(join(aliasDirectory, 'notes.md'), 'The notes beside the declared page'),
			writeFile(join(physicalDirectory, 'notes.md'), 'The notes beside the symlink target'),
			symlink('../source/guide.html', alias, 'file'),
		]);
		await run(process.execPath, [cli, 'project', 'check', project, '--site-root', site]);
		await withPreview(alias, site, async (url) => {
			expect(url.pathname).toBe('/published/Reviewed%20Guide.html');
			expect(await (await fetch(url)).text()).toContain(
				'<h2 data-design-record="notes/N1" id="notes/N1">Readers can find the approved scope.</h2>',
			);
			expect(await (await fetch(new URL('./notes.md', url))).text()).toBe(
				'The notes beside the declared page',
			);
		});
		await withPreview(physical, site, async (url) => {
			expect(url.pathname).toBe('/published/Reviewed%20Guide.html');
			expect(await (await fetch(url)).text()).toContain(
				'id="notes/N1">Readers can find the approved scope.',
			);
		});

		const alternate = join(site, 'alternate.html');
		await symlink('source/guide.html', alternate, 'file');
		await createExternalPageProject(join(site, 'other-project'), '../../alternate.html');
		await expect(
			run(process.execPath, [cli, 'open', alias, '--site-root', site, '--no-browser'], {
				timeout: 5_000,
			}),
		).rejects.toThrow(/multiple.*own|ambiguous/i);

		const outside = join(root, 'outside.html');
		const escape = join(site, 'escape.html');
		await writeFile(outside, html);
		await symlink('../outside.html', escape, 'file');
		await expect(
			run(process.execPath, [cli, 'open', escape, '--site-root', site, '--no-browser'], {
				timeout: 5_000,
			}),
		).rejects.toThrow(/outside the site root/i);
	} finally {
		await rm(root, { recursive: true, force: true });
	}
}, 30_000);

it.skipIf(process.platform === 'win32' || process.getuid?.() === 0)(
	'opens despite an unreadable unrelated branch and reports an unreadable bound project',
	async (): Promise<void> => {
		const site = await mkdtemp(join(tmpdir(), 'demistify-open-unreadable-'));
		const page = join(site, 'Reviewed Guide.html');
		const project = join(site, 'project');
		const unrelated = join(site, 'archives');
		const html =
			'<h2 data-design-record="notes/N1"></h2><ex-project-overview src="project/project.generated.json"></ex-project-overview>';
		try {
			await Promise.all([
				createExternalPageProject(project),
				mkdir(unrelated),
				writeFile(page, html),
			]);
			await chmod(unrelated, 0o000);
			await expect(readdir(unrelated)).rejects.toHaveProperty('code', 'EACCES');
			await withPreview(page, site, async (url) => {
				expect(await (await fetch(url)).text()).toContain(
					'id="notes/N1">Readers can find the approved scope.',
				);
			});

			await chmod(project, 0o000);
			await expect(
				run(process.execPath, [cli, 'open', page, '--site-root', site, '--no-browser'], {
					timeout: 5_000,
				}),
			).rejects.toThrow(/cannot read.*project|EACCES.*project/i);
		} finally {
			await Promise.all([chmod(project, 0o700), chmod(unrelated, 0o700)]);
			await rm(site, { recursive: true, force: true });
		}
	},
	30_000,
);
