import { mkdir, mkdtemp, readFile, readdir, unlink, writeFile } from 'node:fs/promises';
import { createServer, type Server } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import { stageRelease, type StageReleaseOptions } from '../../release/stage-assets.ts';
import worker from '../../release/worker.ts';

const servers: Server[] = [];
afterEach(async (): Promise<void> => {
	await Promise.all(
		servers.splice(0).map(
			(server): Promise<void> =>
				new Promise((done, reject): void => {
					server.close((error): void => (error ? reject(error) : done()));
					server.closeAllConnections();
				}),
		),
	);
});
async function fixture(version: string): Promise<StageReleaseOptions> {
	const directory = await mkdtemp(join(tmpdir(), '2026-10-demistify-release-'));
	const libraryDirectory = join(directory, 'library');
	const guideDirectory = join(directory, 'guide');
	await mkdir(join(libraryDirectory, 'chunks'), { recursive: true });
	await mkdir(guideDirectory);
	await Promise.all(
		Object.entries({
			'runtime.js': 'import("./chunks/engine-abcdefgh.js")',
			'theme.css': 'body { color: white; }',
			'diagram-worker.js': 'self.onmessage = () => {}',
			'chunks/engine-abcdefgh.js': 'export const value = 1;',
		}).map(([path, content]): Promise<void> => writeFile(join(libraryDirectory, path), content)),
	);
	await writeFile(join(guideDirectory, 'SKILL.md'), '# Version-matched guide\n');
	return {
		libraryDirectory,
		guideDirectory,
		outputDirectory: join(directory, 'staged'),
		version,
		sourceCommit: 'a'.repeat(40),
		previous: { kind: 'bootstrap' },
	};
}
async function serve(directory: string): Promise<string> {
	const server = createServer((request, response): void => {
		const path = new URL(request.url ?? '/', 'http://localhost').pathname;
		readFile(join(directory, path)).then(
			(body): void => {
				response.writeHead(200).end(body);
			},
			(): void => {
				response.writeHead(404).end();
			},
		);
	});
	servers.push(server);
	await new Promise<void>((done): void => {
		server.listen(0, '127.0.0.1', done);
	});
	const address = server.address();
	if (!address || typeof address === 'string') throw new Error('Expected loopback address.');
	return `http://127.0.0.1:${address.port}`;
}

describe('major-only release retention across real filesystem and HTTP', (): void => {
	it('updates v1, retains an earlier lazy chunk, and creates no minor-version path', async (): Promise<void> => {
		const initial = await fixture('1.0.0');
		await stageRelease(initial);
		const next = await fixture('1.1.0');
		await unlink(join(next.libraryDirectory, 'chunks/engine-abcdefgh.js'));
		await writeFile(
			join(next.libraryDirectory, 'runtime.js'),
			'import("./chunks/engine-ijklmnop.js")',
		);
		await writeFile(
			join(next.libraryDirectory, 'chunks/engine-ijklmnop.js'),
			'export const value = 2;',
		);
		const result = await stageRelease({
			...next,
			previous: { kind: 'hosted', origin: await serve(initial.outputDirectory) },
		});
		expect(await readFile(join(next.outputDirectory, 'v1/runtime.js'), 'utf8')).toBe(
			'import("./chunks/engine-ijklmnop.js")',
		);
		expect(await readFile(join(next.outputDirectory, 'v1/chunks/engine-abcdefgh.js'), 'utf8')).toBe(
			'export const value = 1;',
		);
		expect(await readdir(next.outputDirectory)).toEqual(['release-catalog.json', 'v1']);
		expect(result.catalog.majors.map((release): string => release.version)).toEqual(['1.1.0']);
	});
	it('adds v2 without changing retained v1 runtime or its manifest bytes', async (): Promise<void> => {
		const initial = await fixture('1.3.2');
		await stageRelease(initial);
		const next = await fixture('2.0.0');
		await writeFile(join(next.libraryDirectory, 'runtime.js'), 'export const breaking = true;');
		await stageRelease({
			...next,
			previous: { kind: 'hosted', origin: await serve(initial.outputDirectory) },
		});
		await Promise.all(
			['v1/runtime.js', 'v1/release.json'].map(async (path): Promise<void> => {
				const [actual, expected] = await Promise.all([
					readFile(join(next.outputDirectory, path)),
					readFile(join(initial.outputDirectory, path)),
				]);
				expect(actual).toEqual(expected);
			}),
		);
		expect(await readFile(join(next.outputDirectory, 'v2/runtime.js'), 'utf8')).toBe(
			'export const breaking = true;',
		);
	});
	it('rejects corrupt retained bytes before staging anything', async (): Promise<void> => {
		const initial = await fixture('1.0.0');
		await stageRelease(initial);
		const original = await readFile(join(initial.outputDirectory, 'v1/runtime.js'));
		await writeFile(
			join(initial.outputDirectory, 'v1/runtime.js'),
			'x'.repeat(original.byteLength),
		);
		const next = await fixture('2.0.0');
		await expect(
			stageRelease({
				...next,
				previous: { kind: 'hosted', origin: await serve(initial.outputDirectory) },
			}),
		).rejects.toThrow('hash/size mismatch');
		await expect(readdir(next.outputDirectory)).rejects.toMatchObject({ code: 'ENOENT' });
	});
	it('does not treat missing history as a first deployment', async (): Promise<void> => {
		const initial = await fixture('1.0.0');
		await expect(
			stageRelease({
				...initial,
				previous: { kind: 'hosted', origin: await serve(initial.libraryDirectory) },
			}),
		).rejects.toThrow('HTTP 404');
	});
	it('rejects changed bytes under the same version and accepts an identical retry', async (): Promise<void> => {
		const initial = await fixture('1.0.0');
		await stageRelease(initial);
		const origin = await serve(initial.outputDirectory);
		const retry = await fixture('1.0.0');
		await expect(
			stageRelease({ ...retry, previous: { kind: 'hosted', origin } }),
		).resolves.toMatchObject({ manifest: { version: '1.0.0' } });
		const changed = await fixture('1.0.0');
		await writeFile(join(changed.libraryDirectory, 'runtime.js'), 'changed');
		await expect(
			stageRelease({ ...changed, previous: { kind: 'hosted', origin } }),
		).rejects.toThrow('same package version');
	});
	it('rejects changed content at an already cached immutable chunk URL', async (): Promise<void> => {
		const initial = await fixture('1.0.0');
		await stageRelease(initial);
		const next = await fixture('1.1.0');
		await writeFile(
			join(next.libraryDirectory, 'chunks/engine-abcdefgh.js'),
			'export const value = 2;',
		);
		await expect(
			stageRelease({
				...next,
				previous: { kind: 'hosted', origin: await serve(initial.outputDirectory) },
			}),
		).rejects.toThrow('immutable chunk URL');
	});
	it('rejects an accidental downgrade and does not publish pre-1.0 as v1', async (): Promise<void> => {
		const initial = await fixture('1.2.0');
		await stageRelease(initial);
		const next = await fixture('1.1.0');
		await expect(
			stageRelease({
				...next,
				previous: { kind: 'hosted', origin: await serve(initial.outputDirectory) },
			}),
		).rejects.toThrow('downgrade');
		await expect(stageRelease(await fixture('0.1.0'))).rejects.toThrow('Pre-1.0');
	});
});

it('revalidates mutable entry files, caches hashed chunks, and preserves CORS on real responses', async (): Promise<void> => {
	await Promise.all(
		(
			[
				['/v1/runtime.js', false],
				['/v1/theme.css', false],
				['/v1/chunks/engine-abcdefgh.js', true],
			] as const
		).map(async ([path, immutable]): Promise<void> => {
			const response = await worker.fetch(new Request(`https://example.test${path}`), {
				ASSETS: {
					fetch: async (): Promise<Response> =>
						new Response('literal asset body', { headers: { ETag: '"fixture"' } }),
				},
			});
			expect(await response.text()).toBe('literal asset body');
			expect(response.headers.get('cache-control')).toContain(
				immutable ? 'immutable' : 'must-revalidate',
			);
			expect(response.headers.get('access-control-allow-origin')).toBe('*');
		}),
	);
	const missing = await worker.fetch(new Request('https://example.test/v1/missing.js'), {
		ASSETS: {
			fetch: async (): Promise<Response> => new Response('missing', { status: 404 }),
		},
	});
	expect(missing.status).toBe(404);
	expect(missing.headers.get('cache-control')).toBe('no-store');
});
