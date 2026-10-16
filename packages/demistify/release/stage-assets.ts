import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { dirname, join, relative, sep } from 'node:path';

import { immutableAssetPath } from './asset-policy.ts';
import {
	compareVersions,
	fetchReleaseBytes,
	hashBytes,
	jsonBytes,
	releaseCatalogSchema,
	releaseManifestSchema,
	versionNumbers,
	type ReleaseCatalog,
	type ReleaseFile,
	type ReleaseManifest,
} from './release-schema.ts';

export type PreviousRelease =
	| { readonly kind: 'bootstrap' }
	| { readonly kind: 'hosted'; readonly origin: string };
export interface StageReleaseOptions {
	readonly libraryDirectory: string;
	readonly guideDirectory: string;
	readonly outputDirectory: string;
	readonly version: string;
	readonly sourceCommit: string;
	readonly previous: PreviousRelease;
}
export interface StagedRelease {
	readonly catalog: ReleaseCatalog;
	readonly manifest: ReleaseManifest;
	readonly fileCount: number;
	readonly totalBytes: number;
}

async function readDirectory(directory: string, prefix: string): Promise<Map<string, Buffer>> {
	const entries = await readdir(directory, { recursive: true, withFileTypes: true });
	const files = await Promise.all(
		entries
			.filter((entry): boolean => entry.isFile())
			.map(async (entry): Promise<readonly [string, Buffer]> => {
				const sourcePath = join(entry.parentPath, entry.name);
				return [
					prefix + relative(directory, sourcePath).split(sep).join('/'),
					await readFile(sourcePath),
				];
			}),
	);
	return new Map(files);
}

export async function stageRelease(options: StageReleaseOptions): Promise<StagedRelease> {
	const [major] = versionNumbers(options.version);
	if (major < 1)
		throw new Error(
			'Pre-1.0 package versions do not publish a supported /vN path. Select the first release in a separate release PR.',
		);
	const previousFiles = new Map<string, Buffer>();
	let catalog: ReleaseCatalog = { schemaVersion: 1, majors: [] };
	const manifests = new Map<number, ReleaseManifest>();
	if (options.previous.kind === 'hosted') {
		const origin = options.previous.origin;
		catalog = releaseCatalogSchema.parse(
			JSON.parse((await fetchReleaseBytes(origin, 'release-catalog.json')).toString('utf8')),
		);
		await Promise.all(
			catalog.majors.map(async (release): Promise<void> => {
				const manifestPath = `v${release.major}/release.json`;
				const bytes = await fetchReleaseBytes(origin, manifestPath);
				if (hashBytes(bytes) !== release.manifestSha256)
					throw new Error(`History manifest hash mismatch: ${manifestPath}.`);
				const manifest = releaseManifestSchema.parse(JSON.parse(bytes.toString('utf8')));
				if (manifest.major !== release.major || manifest.version !== release.version)
					throw new Error('Catalog and retained manifest disagree.');
				await Promise.all(
					manifest.files.map(async (file): Promise<void> => {
						const path = `v${release.major}/${file.path}`;
						const body = await fetchReleaseBytes(origin, path);
						if (body.byteLength !== file.bytes || hashBytes(body) !== file.sha256)
							throw new Error(`History asset hash/size mismatch: ${path}.`);
						previousFiles.set(path, body);
					}),
				);
				previousFiles.set(manifestPath, bytes);
				manifests.set(release.major, manifest);
			}),
		);
	}
	const latestMajor = Math.max(0, ...catalog.majors.map((release): number => release.major));
	if (major < latestMajor)
		throw new Error('Deploying an older major from this main-line workflow is not permitted.');
	const existing = manifests.get(major);
	if (existing && compareVersions(options.version, existing.version) < 0)
		throw new Error('Refusing a package version downgrade.');
	const library = await readDirectory(options.libraryDirectory, '');
	const guide = await readDirectory(options.guideDirectory, 'authoring/');
	const incoming = new Map([...library, ...guide]);
	for (const required of ['runtime.js', 'theme.css', 'diagram-worker.js', 'authoring/SKILL.md'])
		if (!incoming.has(required)) throw new Error(`Incomplete release: ${required} is missing.`);
	for (const [path, bytes] of incoming) {
		const prior = previousFiles.get(`v${major}/${path}`);
		if (prior && immutableAssetPath(`v${major}/${path}`) && !prior.equals(bytes))
			throw new Error('Refusing different bytes at an immutable chunk URL.');
		if (existing?.version === options.version && (!prior || !prior.equals(bytes)))
			throw new Error('The same package version has different assets; declare a new release.');
		previousFiles.set(`v${major}/${path}`, bytes);
	}
	// Retain earlier hashed dependencies for modules still running in already-open pages.
	const prefix = `v${major}/`;
	const files: ReleaseFile[] = Array.from(previousFiles.entries())
		.filter(([path]): boolean => path.startsWith(prefix) && path !== `${prefix}release.json`)
		.map(([path, bytes]): ReleaseFile => ({
			path: path.slice(prefix.length),
			bytes: bytes.byteLength,
			sha256: hashBytes(bytes),
		}))
		.toSorted((left, right): number => left.path.localeCompare(right.path));
	const manifest =
		existing?.version === options.version
			? existing
			: releaseManifestSchema.parse({
					schemaVersion: 1,
					packageName: '@sunstack/demistify',
					major,
					version: options.version,
					sourceCommit: options.sourceCommit,
					files,
				});
	const manifestBytes =
		existing?.version === options.version
			? (previousFiles.get(`${prefix}release.json`) ?? jsonBytes(manifest))
			: jsonBytes(manifest);
	previousFiles.set(`${prefix}release.json`, manifestBytes);
	const nextCatalog = releaseCatalogSchema.parse({
		schemaVersion: 1,
		majors: [
			...catalog.majors.filter((release): boolean => release.major !== major),
			{ major, version: options.version, manifestSha256: hashBytes(manifestBytes) },
		].toSorted((left, right): number => left.major - right.major),
	});
	previousFiles.set('release-catalog.json', jsonBytes(nextCatalog));
	await mkdir(options.outputDirectory, { recursive: true });
	if ((await readdir(options.outputDirectory)).length !== 0)
		throw new Error(
			'Output directory must be empty; do not mix stale staged files with a release.',
		);
	await Promise.all(
		Array.from(previousFiles, async ([path, bytes]): Promise<void> => {
			const target = join(options.outputDirectory, path);
			await mkdir(dirname(target), { recursive: true });
			await writeFile(target, bytes, { flag: 'wx' });
		}),
	);
	return {
		catalog: nextCatalog,
		manifest,
		fileCount: previousFiles.size,
		totalBytes: Array.from(previousFiles.values()).reduce(
			(total, bytes): number => total + bytes.byteLength,
			0,
		),
	};
}
