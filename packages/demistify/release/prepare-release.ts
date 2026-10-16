import { execFileSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';

import { z } from 'zod';

import { stageRelease } from './stage-assets.ts';

const packageMetadata = z
	.strictObject({ version: z.string() })
	.passthrough()
	.parse(JSON.parse(await readFile('packages/demistify/package.json', 'utf8')));
const bootstrap = process.argv.includes('--bootstrap');
const origin = process.env['DEMISTIFY_ASSET_ORIGIN'];
if (!bootstrap && !origin)
	throw new Error(
		'Set DEMISTIFY_ASSET_ORIGIN to the existing public catalog origin. Only an explicit first-release operator setup may use --bootstrap.',
	);
const result = await stageRelease({
	libraryDirectory: resolve('packages/demistify/dist'),
	guideDirectory: resolve('packages/demistify/skill'),
	outputDirectory: resolve(
		process.env['DEMISTIFY_RELEASE_OUTPUT'] ?? 'tmp/demistify-release/public',
	),
	version: packageMetadata.version,
	sourceCommit: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(),
	previous: bootstrap ? { kind: 'bootstrap' } : { kind: 'hosted', origin: origin ?? '' },
});
process.stdout.write(
	`Staged v${result.manifest.major} (${result.manifest.version}); ${result.catalog.majors.length} retained majors, ${result.fileCount} files, ${result.totalBytes} bytes. No upload performed.\n`,
);
