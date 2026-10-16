import { createHash } from 'node:crypto';

import { z } from 'zod';

const versionPattern = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
export const versionSchema = z.string().regex(versionPattern);
const hashSchema = z.string().regex(/^[a-f0-9]{64}$/);
const fileSchema = z.strictObject({
	path: z.string().regex(/^(?:[A-Za-z0-9_-][A-Za-z0-9._-]*\/)*[A-Za-z0-9_-][A-Za-z0-9._-]*$/),
	bytes: z.number().int().nonnegative(),
	sha256: hashSchema,
});
export const releaseManifestSchema = z
	.strictObject({
		schemaVersion: z.literal(1),
		packageName: z.literal('@sunstack/demistify'),
		major: z.number().int().positive(),
		version: versionSchema,
		sourceCommit: z.string().regex(/^[a-f0-9]{40}$/),
		files: z.array(fileSchema).min(1),
	})
	.superRefine((manifest, context): void => {
		if (Number(manifest.version.split('.')[0]) !== manifest.major)
			context.addIssue({ code: 'custom', message: 'Manifest major must match package version.' });
		if (new Set(manifest.files.map((file): string => file.path)).size !== manifest.files.length)
			context.addIssue({ code: 'custom', message: 'Duplicate manifest path.' });
	});
export const releaseCatalogSchema = z
	.strictObject({
		schemaVersion: z.literal(1),
		majors: z.array(
			z.strictObject({
				major: z.number().int().positive(),
				version: versionSchema,
				manifestSha256: hashSchema,
			}),
		),
	})
	.superRefine((catalog, context): void => {
		if (
			new Set(catalog.majors.map((release): number => release.major)).size !== catalog.majors.length
		)
			context.addIssue({ code: 'custom', message: 'Duplicate major in catalog.' });
		for (const release of catalog.majors)
			if (Number(release.version.split('.')[0]) !== release.major)
				context.addIssue({ code: 'custom', message: 'Catalog major must match version.' });
	});
export type ReleaseManifest = z.infer<typeof releaseManifestSchema>;
export type ReleaseCatalog = z.infer<typeof releaseCatalogSchema>;
export type ReleaseFile = z.infer<typeof fileSchema>;

export function hashBytes(bytes: Uint8Array): string {
	return createHash('sha256').update(bytes).digest('hex');
}
export function jsonBytes(value: unknown): Buffer {
	return Buffer.from(`${JSON.stringify(value, null, 2)}\n`);
}
export function versionNumbers(version: string): readonly [number, number, number] {
	return z
		.tuple([z.number().int(), z.number().int(), z.number().int()])
		.parse(versionSchema.parse(version).split('.').map(Number));
}
export function compareVersions(left: string, right: string): number {
	const first = versionNumbers(left);
	const second = versionNumbers(right);
	return first[0] - second[0] || first[1] - second[1] || first[2] - second[2];
}
export function publicOrigin(value: string): string {
	const origin = new URL(value);
	const local = ['127.0.0.1', 'localhost', '[::1]'].includes(origin.hostname);
	if (
		(origin.protocol !== 'https:' && !(local && origin.protocol === 'http:')) ||
		origin.username ||
		origin.password ||
		origin.pathname !== '/' ||
		origin.search ||
		origin.hash
	)
		throw new Error('Use an HTTPS asset origin, or a loopback HTTP origin for local proof.');
	return origin.origin;
}
export async function fetchReleaseBytes(origin: string, path: string): Promise<Buffer> {
	const response = await fetch(`${publicOrigin(origin)}/${path}`, {
		redirect: 'error',
		cache: 'no-store',
		signal: AbortSignal.timeout(20_000),
	});
	if (!response.ok)
		throw new Error(`Cannot retain ${path}: HTTP ${response.status}; no deploy permitted.`);
	return Buffer.from(await response.arrayBuffer());
}
