import { cp, mkdir } from 'node:fs/promises';
import { writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { build } from 'vite';

import { prepareUnderstandingPages } from './collect-understanding-records.ts';

const repositoryRoot = fileURLToPath(new URL('../../../', import.meta.url));
const output = resolve(repositoryRoot, 'apps/demistify-poc/public/experiments/understanding-lab');
const authored = resolve(repositoryRoot, 'apps/demistify-poc/experiments/understanding-lab');
// Validate before mkdir, bundling or copying: invalid refs leave the whole output tree intact.
const prepared = await prepareUnderstandingPages(authored);
await mkdir(output, { recursive: true });
await build({
	configFile: false,
	root: repositoryRoot,
	publicDir: false,
	build: {
		outDir: resolve(output, 'components'),
		emptyOutDir: true,
		lib: {
			entry: resolve(repositoryRoot, 'apps/demistify-poc/src/understanding-lab/register.ts'),
			formats: ['es'],
			fileName: (): string => 'register.js',
		},
	},
});
await cp(authored, output, { recursive: true });
await Promise.all(
	[...prepared.pages].map(([page, html]) => writeFile(resolve(output, page), html)),
);
await cp(resolve(repositoryRoot, 'packages/demistify/dist'), `${output}/library`, {
	recursive: true,
});
process.stdout.write(
	`Local lab prepared: ${prepared.collection.records.length} records, snapshot ${prepared.collection.identity}. No upload performed.\n`,
);
