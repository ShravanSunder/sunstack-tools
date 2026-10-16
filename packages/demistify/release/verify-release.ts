import { readFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

import { immutableAssetPath } from './asset-policy.ts';
import {
	hashBytes,
	publicOrigin,
	releaseCatalogSchema,
	releaseManifestSchema,
} from './release-schema.ts';

const origin = publicOrigin(process.env['DEMISTIFY_ASSET_ORIGIN'] ?? '');
const directory = resolve(
	process.env['DEMISTIFY_RELEASE_OUTPUT'] ?? 'tmp/demistify-release/public',
);
const catalog = releaseCatalogSchema.parse(
	JSON.parse(await readFile(join(directory, 'release-catalog.json'), 'utf8')),
);
const retainedPaths = await Promise.all(
	catalog.majors.map(async (release): Promise<string[]> => {
		const path = `v${release.major}/release.json`;
		const bytes = await readFile(join(directory, path));
		if (hashBytes(bytes) !== release.manifestSha256)
			throw new Error('Staged manifest does not match catalog.');
		const manifest = releaseManifestSchema.parse(JSON.parse(bytes.toString('utf8')));
		return [path, ...manifest.files.map((file): string => `v${release.major}/${file.path}`)];
	}),
);
if (catalog.majors.length === 0) throw new Error('No supported release was staged.');
const files = ['release-catalog.json', ...retainedPaths.flat()];
const mimeTypes: Readonly<Record<string, string>> = {
	'.js': 'text/javascript',
	'.css': 'text/css',
	'.json': 'application/json',
	'.md': 'text/markdown',
	'.woff2': 'font/woff2',
};
await Promise.all(
	files.map(async (path): Promise<void> => {
		const response = await fetch(`${origin}/${path}`, {
			cache: 'no-store',
			redirect: 'error',
			signal: AbortSignal.timeout(20_000),
		});
		const actual = Buffer.from(await response.arrayBuffer());
		if (response.status !== 200 || !actual.equals(await readFile(join(directory, path))))
			throw new Error(`Published body mismatch: ${path}.`);
		const mime = mimeTypes[path.slice(path.lastIndexOf('.'))];
		if (mime && !response.headers.get('content-type')?.startsWith(mime))
			throw new Error(`Incorrect MIME: ${path}.`);
		if (response.headers.get('access-control-allow-origin') !== '*')
			throw new Error(`Missing CORS: ${path}.`);
		const immutable = immutableAssetPath(path);
		const cache = response.headers.get('cache-control') ?? '';
		if (immutable ? !cache.includes('immutable') : !cache.includes('must-revalidate'))
			throw new Error(`Incorrect cache policy: ${path}.`);
	}),
);
const missing = await fetch(`${origin}/v${catalog.majors.at(-1)?.major ?? 1}/missing-script.js`);
if (missing.status !== 404 || missing.headers.get('cache-control') !== 'no-store')
	throw new Error('Missing assets must return 404/no-store.');
process.stdout.write(
	`Verified ${files.length} published files: exact bytes, MIME, CORS, cache policy; missing path 404/no-store.\n`,
);
