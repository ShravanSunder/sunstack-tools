import { execFile } from 'node:child_process';
import {
	chmod,
	cp,
	mkdir,
	mkdtemp,
	readFile,
	readdir,
	rename,
	rm,
	writeFile,
} from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { build } from 'vite';
import { z } from 'zod';

const packageRoot = fileURLToPath(new URL('../', import.meta.url));
const cdnArchiveName = 'demistify-cli.tgz';
const run = promisify(execFile);
const packageMetadataSchema = z.object({
	name: z.string(),
	version: z.string(),
	description: z.string(),
	devDependencies: z.object({ typescript: z.string() }),
});
const npmPackReceiptSchema = z.tuple([
	z.object({ filename: z.string(), integrity: z.string(), entryCount: z.number() }),
]);

async function packArchive(
	stagingDirectory: string,
	outputDirectory: string,
): Promise<z.infer<typeof npmPackReceiptSchema>[0]> {
	const { stdout } = await run(
		'npm',
		['pack', '--json', '--ignore-scripts', '--pack-destination', outputDirectory],
		{ cwd: stagingDirectory },
	);
	const [receipt] = npmPackReceiptSchema.parse(JSON.parse(stdout));
	return receipt;
}

function npmGuideMarkdown(markdown: string): string {
	return markdown
		.replaceAll(
			/^pnpm dlx (?:https:\/\/\S+\/demistify-cli\.tgz|"\$PWD\/[^"\r\n]+\/demistify-cli\.tgz") /gmu,
			'demistify ',
		)
		.replaceAll(/^# Or: npx --yes --package=\S+\/demistify-cli\.tgz demistify .+\n/gmu, '')
		.replaceAll(/^<!-- Release staging binds the logical archive filename[^\n]+\n\n/gmu, '')
		.replace(
			/^Use the supplied library base, normally [^\n]+/mu,
			'Use the runtime, theme and CLI from this package. Serve the complete `dist/` directory with the page. The example assumes you copied it beside the page as `demistify/`.',
		)
		.replaceAll('https://demistify.getagentstudio.dev/v1/', './demistify/')
		.replace(
			'(for v1, `./demistify/custom-elements.json` and `html-custom-data.json`)',
			'(`custom-elements.json` and `html-custom-data.json`)',
		)
		.replace(
			/^Run the CLI shipped at the page's library binding [^\n]+/mu,
			'Run the `demistify` command installed with this npm package, using Node 26+.',
		)
		.replace('Use the same base as the runtime and theme. ', '')
		.replace(
			/^From the directory containing `trail-guide\/`, using Node 26\+ and the example's matching local distribution\.[^\n]+/mu,
			'Run the installed `demistify` command from the directory containing `trail-guide/`.',
		)
		.replace(/^For a hosted binding, [^\n]+\. (The main guide owns[^\n]+)/mu, '$1')
		.replace('retaining the HTTPS library links', 'retaining the library links')
		.replace('Run with Node 26+; the CLI tarball includes its dependencies.', 'Run with Node 26+.');
}

async function stageNpmAssets(stagingDirectory: string, outputDirectory: string): Promise<void> {
	const guideDirectory = resolve(stagingDirectory, 'authoring');
	await Promise.all([
		cp(outputDirectory, resolve(stagingDirectory, 'dist'), {
			recursive: true,
			filter: (source): boolean => basename(source) !== cdnArchiveName,
		}),
		cp(resolve(packageRoot, 'skill'), guideDirectory, { recursive: true }),
		cp(resolve(packageRoot, 'release/npm-readme.md'), resolve(stagingDirectory, 'README.md')),
		cp(resolve(packageRoot, '../../LICENSE'), resolve(stagingDirectory, 'LICENSE')),
	]);
	const guideFiles = await readdir(guideDirectory, { recursive: true, withFileTypes: true });
	await Promise.all(
		guideFiles
			.filter((entry): boolean => entry.isFile() && entry.name.endsWith('.md'))
			.map(async (entry): Promise<void> => {
				const path = resolve(entry.parentPath, entry.name);
				const markdown = npmGuideMarkdown(await readFile(path, 'utf8'));
				if (markdown.includes(cdnArchiveName)) {
					throw new Error(`The npm guide still points to the CDN archive in ${entry.name}.`);
				}
				await writeFile(path, markdown);
			}),
	);
}

export async function buildAuthoringCli(
	outputDirectory: string,
	includeNpmPackage: boolean,
): Promise<void> {
	const packageMetadata = packageMetadataSchema.parse(
		JSON.parse(await readFile(resolve(packageRoot, 'package.json'), 'utf8')),
	);
	const stagingDirectory = await mkdtemp(resolve(tmpdir(), 'demistify-cli-'));
	try {
		await build({
			configFile: false,
			publicDir: false,
			logLevel: 'silent',
			// The compiler resolves its standard TypeScript libraries from its installed package.
			ssr: { noExternal: true, external: ['typescript'] },
			build: {
				ssr: resolve(packageRoot, 'bin/demistify.ts'),
				outDir: stagingDirectory,
				emptyOutDir: false,
				rolldownOptions: { output: { entryFileNames: 'demistify.js' } },
			},
		});
		await chmod(resolve(stagingDirectory, 'demistify.js'), 0o755);
		const manifestPath = resolve(stagingDirectory, 'package.json');
		const cliManifest = {
			version: packageMetadata.version,
			type: 'module',
			bin: { demistify: './demistify.js' },
			engines: { node: '>=26' },
			dependencies: { typescript: packageMetadata.devDependencies.typescript },
			files: ['*.js', 'assets'],
		};
		await writeFile(
			manifestPath,
			JSON.stringify({ name: '@sunstack/demistify-cli', ...cliManifest }),
		);
		const cdnPackage = await packArchive(stagingDirectory, outputDirectory);
		await rename(
			resolve(outputDirectory, cdnPackage.filename),
			resolve(outputDirectory, cdnArchiveName),
		);
		if (includeNpmPackage) {
			const npmDirectory = resolve(packageRoot, 'npm-dist');
			await stageNpmAssets(stagingDirectory, outputDirectory);
			await mkdir(npmDirectory, { recursive: true });
			await writeFile(
				manifestPath,
				JSON.stringify({
					...cliManifest,
					name: packageMetadata.name,
					description: packageMetadata.description,
					license: 'MIT',
					publishConfig: { access: 'public' },
					exports: {
						'./runtime': './dist/runtime.js',
						'./theme.css': './dist/theme.css',
					},
					files: [...cliManifest.files, 'dist', 'authoring'],
				}),
			);
			const npmPackage = await packArchive(stagingDirectory, npmDirectory);
			process.stdout.write(
				`Packed npm-dist/${npmPackage.filename} (${npmPackage.entryCount} files, ${npmPackage.integrity}).\n`,
			);
		}
	} finally {
		await rm(stagingDirectory, { recursive: true, force: true });
	}
}
