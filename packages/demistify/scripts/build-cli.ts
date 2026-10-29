import { execFile } from 'node:child_process';
import { chmod, mkdtemp, readFile, rename, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { build } from 'vite';
import { z } from 'zod';

/** Packages the existing authoring CLI beside the runtime, without a checkout or browser install. */
export async function buildAuthoringCli(outputDirectory: string): Promise<void> {
	const packageRoot = fileURLToPath(new URL('../', import.meta.url));
	const packageMetadata = z
		.object({ version: z.string() })
		.parse(JSON.parse(await readFile(resolve(packageRoot, 'package.json'), 'utf8')));
	const stagingDirectory = await mkdtemp(resolve(tmpdir(), 'demistify-cli-'));
	try {
		await build({
			configFile: false,
			publicDir: false,
			logLevel: 'silent',
			ssr: { noExternal: true },
			build: {
				ssr: resolve(packageRoot, 'bin/demistify.ts'),
				outDir: stagingDirectory,
				emptyOutDir: false,
				rolldownOptions: { output: { entryFileNames: 'demistify.js' } },
			},
		});
		await chmod(resolve(stagingDirectory, 'demistify.js'), 0o755);
		await writeFile(
			resolve(stagingDirectory, 'package.json'),
			JSON.stringify({
				name: '@sunstack/demistify-cli',
				version: packageMetadata.version,
				type: 'module',
				bin: { demistify: './demistify.js' },
				engines: { node: '>=26' },
				files: ['*.js', 'assets'],
			}),
		);
		const result = await promisify(execFile)(
			'npm',
			['pack', '--silent', '--ignore-scripts', '--pack-destination', outputDirectory],
			{ cwd: stagingDirectory },
		);
		await rename(
			resolve(outputDirectory, result.stdout.trim()),
			resolve(outputDirectory, 'demistify-cli.tgz'),
		);
	} finally {
		await rm(stagingDirectory, { recursive: true, force: true });
	}
}
