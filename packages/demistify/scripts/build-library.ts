import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { build } from 'vite';

import { localFontStyles, localPhosphorStyles } from '../src/packaging.ts';

const packageRoot = fileURLToPath(new URL('../', import.meta.url));
const outputDirectory = resolve(packageRoot, 'dist');
await mkdir(outputDirectory, { recursive: true });
await build({
	configFile: false,
	publicDir: false,
	logLevel: 'silent',
	build: {
		outDir: outputDirectory,
		emptyOutDir: false,
		lib: {
			entry: resolve(packageRoot, 'src/runtime.ts'),
			formats: ['es'],
			fileName: (): string => 'runtime.js',
		},
		rolldownOptions: { output: { chunkFileNames: 'chunks/[name]-[hash].js' } },
	},
});
const styles = await Promise.all(
	['theme.css', 'reference-components.css', 'component-styles.css', 'navigation-styles.css'].map(
		(name): Promise<string> => readFile(resolve(packageRoot, 'src', name), 'utf8'),
	),
);
const iconFont = await readFile(
	new URL(import.meta.resolve('@phosphor-icons/web/regular/Phosphor.woff2')),
);
await writeFile(resolve(outputDirectory, 'Phosphor.woff2'), iconFont);
await writeFile(
	resolve(outputDirectory, 'theme.css'),
	[await localFontStyles(), await localPhosphorStyles('./Phosphor.woff2'), ...styles].join('\n'),
);
process.stdout.write(
	'Built drop-in Demistify runtime, lazy renderer chunks, theme and local icons.\n',
);
