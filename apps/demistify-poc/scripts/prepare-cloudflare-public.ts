/** Prepare this frozen public trial only; no login, upload, account IDs or credential access. */
import { createHash } from 'node:crypto';
import { copyFile, mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { z } from 'zod';

const repositoryRoot = fileURLToPath(new URL('../../../', import.meta.url));
const configurationRoot = resolve(
	repositoryRoot,
	'apps/demistify-poc/experiments/cloudflare-public-assets',
);
const manifestSchema = z.object({
	sourceCommit: z.string().regex(/^[a-f0-9]{40}$/),
	releaseRoot: z.literal('demistify/experimental/e3a92d2'),
	examplesRoot: z.literal('examples/2026-10-16-sol-luna-v1'),
	totalBytes: z.number().int().positive(),
	files: z.array(
		z.object({
			path: z.string(),
			bytes: z.number().int().positive(),
			sha256: z.string().regex(/^[a-f0-9]{64}$/),
		}),
	),
});
const manifest = manifestSchema.parse(
	JSON.parse(await readFile(resolve(configurationRoot, 'release-manifest.json'), 'utf8')),
);
const libraryRoot = resolve(repositoryRoot, 'packages/demistify/dist');
const publicRoot = resolve(repositoryRoot, 'tmp/demistify-cloudflare-public/public');

async function copyFrozenFile(
	source: string,
	target: string,
	expectedHash?: string,
): Promise<void> {
	const bytes = await readFile(source);
	const hash = createHash('sha256').update(bytes).digest('hex');
	if (expectedHash && hash !== expectedHash)
		throw new Error(
			'Built library differs from the pinned trial. Use the correct build or prepare a separately named release.',
		);
	const existing = await readFile(target).catch((error: unknown): undefined => {
		if (error instanceof Error && 'code' in error && error.code === 'ENOENT') return undefined;
		throw error;
	});
	if (existing && !existing.equals(bytes))
		throw new Error(`Refusing to overwrite a frozen trial file: ${target}`);
	await mkdir(dirname(target), { recursive: true });
	await copyFile(source, target);
}

for (const entry of manifest.files) {
	if (entry.path.includes('..') || entry.path.startsWith('/'))
		throw new Error('Invalid release asset path.');
	// oxlint-disable-next-line no-await-in-loop -- one writer prepares and verifies each frozen artifact.
	await copyFrozenFile(
		resolve(libraryRoot, entry.path),
		resolve(publicRoot, manifest.releaseRoot, entry.path),
		entry.sha256,
	);
}
const exampleRoot = resolve(
	repositoryRoot,
	'apps/demistify-poc/experiments/agent-written/round-12-public-assets',
);
const examples = (await readdir(exampleRoot)).filter((name): boolean =>
	/\.(html|json)$/.test(name),
);
for (const name of examples) {
	// oxlint-disable-next-line no-await-in-loop -- preserve one-writer publication and refuse changed frozen files.
	await copyFrozenFile(
		resolve(exampleRoot, name),
		resolve(publicRoot, manifest.examplesRoot, name),
	);
}
await writeFile(
	resolve(publicRoot, 'index.html'),
	`<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Demistify public examples</title></head><body><h1>Demistify public examples</h1><p><a href="/${manifest.examplesRoot}/index.html">Open the Sol and Luna examples</a></p></body></html>\n`,
);
process.stdout.write(
	`Prepared ${manifest.files.length} pinned library assets and ${examples.length} sample files. No upload performed.\n`,
);
