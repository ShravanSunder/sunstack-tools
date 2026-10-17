import { cp, mkdir, mkdtemp, readFile, readdir, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { extname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import type { Plugin } from 'vite';

import { embedMarkdownFiles } from '../../../../packages/demistify/src/node/markdown-files.ts';

const repositoryRoot = fileURLToPath(new URL('../../../../', import.meta.url));
const contentTypes: Readonly<Record<string, string>> = {
	'.html': 'text/html',
	'.js': 'text/javascript',
	'.css': 'text/css',
	'.md': 'text/markdown',
	'.woff2': 'font/woff2',
	'.json': 'application/json',
	'.svg': 'image/svg+xml',
};

/** Freeze built assets and authored pages; serve bytes without Vite rewriting worker imports. */
export async function builtFixtures(): Promise<Plugin> {
	const snapshot = await mkdtemp(join(tmpdir(), '2026-10-demistify-browser-'));
	await cp(
		process.env['DEMISTIFY_BROWSER_ASSETS'] ?? resolve(repositoryRoot, 'packages/demistify/dist'),
		join(snapshot, 'library'),
		{ recursive: true },
	);
	await Promise.all(
		[
			['navigation', 'navigation-markdown-fix'],
			['layout', 'layout-regression-fix'],
			['gallery', 'integrated-review'],
			['explorer', 'round-13-inspection'],
			['ui', 'review-ui'],
		].map(async ([name, source]): Promise<void> => {
			if (!name || !source) throw new Error('Missing fixture name.');
			await cp(
				resolve(repositoryRoot, 'apps/demistify-poc/experiments/agent-written', source),
				join(snapshot, name),
				{ recursive: true },
			);
			await Promise.all(
				(await readdir(join(snapshot, name)))
					.filter((file): boolean => file.endsWith('.html'))
					.map(async (filename): Promise<void> => {
						const file = join(snapshot, name, filename);
						const html = await readFile(file, 'utf8');
						await writeFile(
							file,
							html
								.replaceAll('/experiments/islands/', '/__demistify/library/')
								.replaceAll('../../../../../packages/demistify/dist/', '/__demistify/library/'),
						);
					}),
			);
		}),
	);
	const navigationHtml = await readFile(join(snapshot, 'navigation/index.html'), 'utf8');
	const notesSource = await readFile(join(snapshot, 'navigation/notes.md'), 'utf8');
	await mkdir(join(snapshot, 'embedded'));
	await writeFile(
		join(snapshot, 'embedded/index.html'),
		embedMarkdownFiles(navigationHtml, { 'notes.md': notesSource }),
	);
	return {
		name: 'demistify-built-fixtures',
		configureServer(server): void {
			process.stdout.write(`Demistify browser snapshot: ${snapshot}\n`);
			server.middlewares.use('/__demistify/', (request, response): void => {
				const path = decodeURIComponent((request.url ?? '').split('?')[0] ?? '');
				if (path.split('/').some((part): boolean => part === '..') || path.includes('\\')) {
					response.writeHead(400).end();
					return;
				}
				const file = join(snapshot, path);
				void readFile(file).then(
					(content): void => {
						response.setHeader(
							'Content-Type',
							contentTypes[extname(file)] ?? 'application/octet-stream',
						);
						response.setHeader('Cache-Control', 'no-store');
						response.end(content);
					},
					(): void => {
						response.writeHead(404).end();
					},
				);
			});
		},
	};
}
