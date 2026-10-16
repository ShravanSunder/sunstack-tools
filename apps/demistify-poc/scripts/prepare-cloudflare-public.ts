/** Prepare this frozen public trial only; no login, upload, account IDs or credential access. */
import { createHash } from 'node:crypto';
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
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
	releaseRoot: z.string().regex(/^demistify\/experimental\/[a-f0-9]{7}$/),
	examplesRoot: z.string().regex(/^examples\/2026-10-16-sol-luna-v[123]$/),
	totalBytes: z.number().int().positive(),
	files: z.array(
		z.object({
			path: z.string(),
			bytes: z.number().int().positive(),
			sha256: z.string().regex(/^[a-f0-9]{64}$/),
		}),
	),
});
const releases = [
	{ manifest: 'release-manifest.json', examples: 'round-12-public-assets', archived: true },
	{
		manifest: 'inspection-release-manifest.json',
		examples: 'round-13-inspection',
		archived: true,
	},
	{
		manifest: 'interactive-release-manifest.json',
		examples: 'round-14-inspection-graphs',
		archived: false,
	},
] as const;
const assetOrigin = 'https://demistify-assets-cli-experiment.askluna.workers.dev';
const libraryRoot = resolve(repositoryRoot, 'packages/demistify/dist');
const publicRoot = resolve(repositoryRoot, 'tmp/demistify-cloudflare-public/public');

async function copyFrozenFile(bytes: Buffer, target: string, expectedHash?: string): Promise<void> {
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
	await writeFile(target, bytes);
}

let latestExamplesRoot = '';
for (const release of releases) {
	// oxlint-disable-next-line no-await-in-loop -- frozen releases are assembled by one writer.
	const manifestText = await readFile(resolve(configurationRoot, release.manifest), 'utf8');
	const manifest = manifestSchema.parse(JSON.parse(manifestText));
	for (const entry of manifest.files) {
		if (entry.path.includes('..') || entry.path.startsWith('/'))
			throw new Error('Invalid release asset path.');
		// oxlint-disable-next-line no-await-in-loop -- verify each file before preserving or writing it.
		const sourcePath = entry.path.startsWith('authoring/')
			? resolve(repositoryRoot, 'packages/demistify/skill', entry.path.slice('authoring/'.length))
			: resolve(libraryRoot, entry.path);
		// oxlint-disable-next-line no-await-in-loop -- one writer reads each source before hash checking.
		let bytes = await readFile(sourcePath).catch((): Buffer => Buffer.alloc(0));
		if (createHash('sha256').update(bytes).digest('hex') !== entry.sha256 && release.archived) {
			// Prior immutable bytes come from their exact published URL and must match the saved hash.
			// oxlint-disable-next-line no-await-in-loop -- bounded historical asset retrieval, never credentials.
			const response = await fetch(`${assetOrigin}/${manifest.releaseRoot}/${entry.path}`);
			if (!response.ok) throw new Error(`Archived asset unavailable: HTTP ${response.status}.`);
			// oxlint-disable-next-line no-await-in-loop -- hash verification precedes publication.
			bytes = Buffer.from(await response.arrayBuffer());
		}
		// oxlint-disable-next-line no-await-in-loop -- one writer refuses changed frozen targets.
		await copyFrozenFile(
			bytes,
			resolve(publicRoot, manifest.releaseRoot, entry.path),
			entry.sha256,
		);
	}
	const exampleRoot = resolve(
		repositoryRoot,
		'apps/demistify-poc/experiments/agent-written',
		release.examples,
	);
	// oxlint-disable-next-line no-await-in-loop -- each frozen example folder is prepared with its release.
	const examples = (await readdir(exampleRoot)).filter((name): boolean =>
		/\.(html|json)$/.test(name),
	);
	for (const name of examples) {
		// oxlint-disable-next-line no-await-in-loop -- read and freeze each checked-in sample, never overwrite.
		const bytes = await readFile(resolve(exampleRoot, name));
		// oxlint-disable-next-line no-await-in-loop -- preserve one-writer immutable publication.
		await copyFrozenFile(bytes, resolve(publicRoot, manifest.examplesRoot, name));
	}
	latestExamplesRoot = manifest.examplesRoot;
	process.stdout.write(
		`Prepared ${manifest.files.length} pinned assets and ${examples.length} sample files for ${manifest.releaseRoot}.\n`,
	);
}
await writeFile(
	resolve(publicRoot, 'index.html'),
	`<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Demistify public examples</title></head><body><h1>Demistify public examples</h1><p><a href="/${latestExamplesRoot}/index.html">Open the Sol and Luna examples</a></p></body></html>\n`,
);
process.stdout.write('No upload performed. Previous immutable roots retained.\n');
