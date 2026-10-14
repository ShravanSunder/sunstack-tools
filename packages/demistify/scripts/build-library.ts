import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import subsetFont from 'subset-font';
import { build } from 'vite';

import {
	linkedFontStyles,
	localFontFiles,
	localPhosphorStyles,
	shippedIconCodePoints,
} from '../src/packaging.ts';

const packageRoot = fileURLToPath(new URL('../', import.meta.url));
const outputDirectory = resolve(packageRoot, 'dist');
await build({
	configFile: false,
	publicDir: false,
	logLevel: 'silent',
	build: {
		outDir: outputDirectory,
		emptyOutDir: true,
		lib: {
			entry: resolve(packageRoot, 'src/runtime.ts'),
			formats: ['es'],
			fileName: (): string => 'runtime.js',
		},
		rolldownOptions: { output: { chunkFileNames: 'chunks/[name]-[hash].js' } },
	},
});
const styles = await Promise.all(
	[
		'theme.css',
		'reference-components.css',
		'component-styles.css',
		'navigation-styles.css',
		'vocabulary-styles.css',
	].map((name): Promise<string> => readFile(resolve(packageRoot, 'src', name), 'utf8')),
);
await mkdir(resolve(outputDirectory, 'fonts'), { recursive: true });
for (const font of localFontFiles())
	await writeFile(resolve(outputDirectory, 'fonts', font.fileName), await readFile(font.sourceUrl));
const iconFont = await subsetFont(
	await readFile(new URL(import.meta.resolve('@phosphor-icons/web/regular/Phosphor.woff2'))),
	await shippedIconCodePoints(),
	{ targetFormat: 'woff2' },
);
await writeFile(resolve(outputDirectory, 'Phosphor.woff2'), iconFont);
await writeFile(
	resolve(outputDirectory, 'theme.css'),
	[linkedFontStyles('./fonts'), await localPhosphorStyles('./Phosphor.woff2'), ...styles].join(
		'\n',
	),
);
process.stdout.write(
	'Built drop-in Demistify runtime, lazy renderer chunks, theme, linked fonts and subset icons.\n',
);
