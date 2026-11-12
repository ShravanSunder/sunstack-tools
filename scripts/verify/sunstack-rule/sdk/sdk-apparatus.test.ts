import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
/** Harness sensitivity only: synthetic artifacts/HTTP never qualify an SDK candidate. */
import filesystem from 'node:fs';
import path from 'node:path';

import { describe, expect, it } from 'vitest';

import { SdkControlledBackend } from './sdk-controlled-backend.ts';
import { createSdkFileDefinitions, sdkInlineConfiguration } from './sdk-fixtures.ts';
import {
	requireCompletedSdkReport,
	requireSdkKind,
	requireSdkSources,
	observeSdkInline,
} from './sdk-journeys.ts';
import {
	sdkTargetSchema,
	confirmSdkFilePins,
	confirmSdkPayloadPins,
	inspectInstalledSdkContract,
	containedPath,
	type SdkExports,
} from './sdk-target.ts';

const evidenceRoot = '/private/tmp/2026-11-sunstack-sdk-verification';
function ownedDirectory(): string {
	filesystem.mkdirSync(evidenceRoot, { recursive: true });
	return filesystem.realpathSync(filesystem.mkdtempSync(path.join(evidenceRoot, 'apparatus-')));
}
const sourceContent = 'Independent source sentinel.\n';
const source = {
	origin: { kind: 'inline' },
	content: sourceContent,
	sha256: createHash('sha256').update(sourceContent).digest('hex'),
	representation: 'normalized_value',
	acquisition: 'inline',
};

describe('SDK apparatus rejects false greens', () => {
	it('checks the native contract reply and refuses release/revision/target skew', async () => {
		const contract = {
			sdk_release: '0.12.0',
			contract_revision: 1,
			target: 'aarch64-apple-darwin',
		};
		const target = {
			release: '0.12.0',
			contractRevision: 1,
			arch: 'arm64',
			platform: 'darwin',
			libc: 'none',
		} as const;
		await expect(
			inspectInstalledSdkContract(
				{ contractInfo: async () => ({ kind: 'ready', contract }) },
				target,
			),
		).resolves.toEqual({ kind: 'ready', contract });
		await Promise.all(
			[
				{ ...contract, sdk_release: 'wrong' },
				{ ...contract, contract_revision: 2 },
				{ ...contract, target: 'x86_64-apple-darwin' },
			].map((changed) =>
				expect(
					inspectInstalledSdkContract(
						{ contractInfo: async () => ({ kind: 'ready', contract: changed }) },
						target,
					),
				).rejects.toThrow('ContractInfo differs'),
			),
		);
	});
	it('accepts the four Rust-tagged completed statuses without flattening them', () => {
		for (const kind of ['clean', 'nothing_selected', 'findings', 'uncertain']) {
			const report = { status: { kind } };
			expect(requireCompletedSdkReport({ kind: 'completed', report })['status']).toEqual({ kind });
		}
	});
	it('rejects inconsistent tagged endings, flat statuses and a missing report', () => {
		for (const kind of ['prepared', 'invalid', 'incomplete', 'cancelled', 'invented']) {
			expect(() =>
				requireCompletedSdkReport({ kind: 'completed', report: { status: { kind } } }),
			).toThrow('tagged completed status');
		}
		for (const status of ['clean', 'findings', {}, null]) {
			expect(() => requireCompletedSdkReport({ kind: 'completed', report: { status } })).toThrow(
				'tagged completed status',
			);
		}
		expect(() =>
			requireCompletedSdkReport({ kind: 'invalid', report: { status: { kind: 'clean' } } }),
		).toThrow('Expected completed');
		expect(() => requireCompletedSdkReport({ kind: 'completed' })).toThrow();
	});
	it('does not accept any failure as the expected unavailable case', () => {
		expect(requireSdkKind({ kind: 'unavailable' }, 'unavailable')['kind']).toBe('unavailable');
		expect(() => requireSdkKind({ kind: 'failed' }, 'unavailable')).toThrow('Expected unavailable');
	});
	it('rejects invented file origins and stale source hashes', () => {
		expect(() => requireSdkSources([source], 'inline')).not.toThrow();
		expect(() => requireSdkSources([source], 'file')).toThrow('No file');
		expect(() => requireSdkSources([{ ...source, content: 'mutated bytes' }], 'inline')).toThrow(
			'digest',
		);
		expect(() => requireSdkSources([], 'inline')).toThrow();
	});
	it('pins actual bytes and rejects a changed file, duplicate pins and symlinks', () => {
		const root = ownedDirectory();
		const file = path.join(root, 'artifact.node');
		filesystem.writeFileSync(file, 'candidate A');
		const pin = { path: file, sha256: createHash('sha256').update('candidate A').digest('hex') };
		expect(() => confirmSdkFilePins([pin])).not.toThrow();
		filesystem.writeFileSync(file, 'candidate B');
		expect(() => confirmSdkFilePins([pin])).toThrow('digest differs');
		expect(() => confirmSdkFilePins([pin, pin])).toThrow('Duplicate');
		const link = path.join(root, 'linked.node');
		filesystem.symlinkSync(file, link);
		expect(() => confirmSdkFilePins([{ ...pin, path: link }])).toThrow('link');
	});
	it('rejects relative target paths, omitted native artifact and abbreviated commits', () => {
		const input = {
			sourceRoot: '/source',
			commit: 'a'.repeat(40),
			consumerRoot: '/consumer',
			packageRoot: '/consumer/node_modules/@sunstack/sdk',
			entry: '/consumer/sdk.js',
			addon: '/consumer/sdk.node',
			tarball: '/candidate.tgz',
			tarballSha256: 'b'.repeat(64),
			files: Array.from({ length: 3 }, (_value, index) => ({
				path: `/consumer/${index}`,
				sha256: 'c'.repeat(64),
			})),
			release: '0.1.0',
			contractRevision: 1,
			platform: 'darwin',
			arch: 'arm64',
			libc: 'none',
		};
		expect(sdkTargetSchema.safeParse(input).success).toBe(true);
		expect(sdkTargetSchema.safeParse({ ...input, consumerRoot: './consumer' }).success).toBe(false);
		expect(sdkTargetSchema.safeParse({ ...input, addon: undefined }).success).toBe(false);
		expect(sdkTargetSchema.safeParse({ ...input, commit: 'abc123' }).success).toBe(false);
	});
	it('requires every wrapper/declaration and selected native-package file to be pinned', () => {
		const consumerRoot = ownedDirectory();
		const packageRoot = path.join(consumerRoot, 'wrapper');
		const nativeRoot = path.join(consumerRoot, 'native');
		filesystem.mkdirSync(packageRoot);
		filesystem.mkdirSync(nativeRoot);
		const payload = [
			path.join(packageRoot, 'package.json'),
			path.join(packageRoot, 'index.js'),
			path.join(packageRoot, 'types.d.ts'),
			path.join(nativeRoot, 'package.json'),
			path.join(nativeRoot, 'binding.node'),
		];
		for (const file of payload)
			filesystem.writeFileSync(file, file.endsWith('package.json') ? '{}' : 'pinned payload');
		const pins = payload.map((file) => ({
			path: file,
			sha256: createHash('sha256').update(filesystem.readFileSync(file)).digest('hex'),
		}));
		const options = {
			consumerRoot,
			packageRoot,
			addon: path.join(nativeRoot, 'binding.node'),
			files: pins,
		};
		expect(() => confirmSdkPayloadPins(options)).not.toThrow();
		expect(() =>
			confirmSdkPayloadPins({
				...options,
				files: pins.filter((file) => !file.path.endsWith('.d.ts')),
			}),
		).toThrow('not pinned');
		filesystem.writeFileSync(
			path.join(packageRoot, 'executed-helper.js'),
			'unlisted runtime helper',
		);
		expect(() => confirmSdkPayloadPins(options)).toThrow('not pinned');
	});
	it('does not confuse a sibling directory with a contained install', () => {
		expect(containedPath('/source', '/source/package')).toBe(true);
		expect(containedPath('/source', '/source-other/package')).toBe(false);
		expect(containedPath('/source', '/source/../other')).toBe(false);
	});
	it('refuses the provider route before target import without the explicit grant acknowledgement', () => {
		const result = spawnSync(
			process.execPath,
			[path.join(import.meta.dirname, 'sdk-verify.ts'), '--journey', 'provider'],
			{ encoding: 'utf8', timeout: 5000 },
		);
		expect(result.status).toBe(1);
		expect(result.stderr).toContain('Provider remains held');
	});
	it('full inline fixture references its authored utility and includes filter/rule definitions', () => {
		const text = JSON.stringify(
			sdkInlineConfiguration('/checked', 'http://127.0.0.1:1/systemone'),
			(_key, value: unknown) => (typeof value === 'bigint' ? String(value) : value),
		);
		expect(text).toContain('"matches":"sdk_paragraph"');
		expect(text).toContain('"type":"choice"');
		expect(text).toContain('"type":"noul"');
		const root = ownedDirectory();
		createSdkFileDefinitions(root, 'http://127.0.0.1:1/systemone');
		expect(
			filesystem.readFileSync(path.join(root, 'evidence/rules/paragraph.yml'), 'utf8'),
		).toContain('matches: sdk_paragraph');
	});
	it('an inert SDK cannot pass the inline journey', async () => {
		const backend = new SdkControlledBackend();
		await backend.start();
		const sdk: SdkExports = {
			contractInfo: async () => undefined,
			load: async () => undefined,
			check: async () => undefined,
			run: async () => ({
				kind: 'completed',
				report: { status: { kind: 'clean' }, sources: [source] },
			}),
		};
		try {
			// Target fields are not used by the pre-parity reached-request checks.
			const target = sdkTargetSchema.parse({
				sourceRoot: '/source',
				commit: 'a'.repeat(40),
				consumerRoot: '/consumer',
				packageRoot: '/consumer/package',
				entry: '/consumer/package/index.js',
				addon: '/consumer/addon.node',
				tarball: '/candidate.tgz',
				tarballSha256: 'b'.repeat(64),
				files: Array.from({ length: 3 }, (_value, index) => ({
					path: `/consumer/${index}`,
					sha256: 'c'.repeat(64),
				})),
				release: '0.1.0',
				contractRevision: 1,
				platform: 'darwin',
				arch: 'arm64',
				libc: 'none',
			});
			await expect(
				observeSdkInline({ sdk, target, backend, evidence: ownedDirectory() }),
			).rejects.toThrow('not reached');
		} finally {
			await backend.close();
		}
	});
	it('captures a real complete HTTP request before releasing its held answer', async () => {
		const backend = new SdkControlledBackend();
		await backend.start();
		backend.hold = true;
		const request = { questions: { actor: { type: 'noul' } } };
		const pending = fetch(backend.endpoint, { method: 'POST', body: JSON.stringify(request) });
		try {
			await backend.waitForRequests(1);
			expect(backend.captures[0]?.body).toEqual(request);
			backend.releaseHeld();
			const response = await pending;
			expect(response.status).toBe(200);
			expect(await response.json()).toMatchObject({
				answers: { actor: { type: 'noul', noul: 0.91 } },
			});
		} finally {
			await backend.close();
		}
	});
});
