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
} from '../src/node/packaging.ts';
import { layoutAttributes, vocabulary } from '../src/vocabulary.ts';

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
			// The diagram worker sits beside runtime.js, which resolves it from its own URL, and shares
			// the engine chunk with the main-thread fallback.
			entry: {
				runtime: resolve(packageRoot, 'src/runtime.ts'),
				'diagram-worker': resolve(packageRoot, 'src/render/diagram-worker.ts'),
			},
			formats: ['es'],
			fileName: (_format: string, entryName: string): string => `${entryName}.js`,
		},
		rolldownOptions: { output: { chunkFileNames: 'chunks/[name]-[hash].js' } },
	},
});
const styles = await Promise.all(
	[
		'theme.css',
		'reference.css',
		'components.css',
		'navigation.css',
		'command-search.css',
		'presentations.css',
		'inspection-gestures.css',
		'vocabulary.css',
		'explorer.css',
		'gallery.css',
		'gallery-walkthrough.css',
		'evidence-trace.css',
		'comparison.css',
		'controls.css',
		'document-widths.css',
	].map((name): Promise<string> => readFile(resolve(packageRoot, 'src/styles', name), 'utf8')),
);
await mkdir(resolve(outputDirectory, 'fonts'), { recursive: true });
await Promise.all(
	localFontFiles().map(async (font): Promise<void> =>
		writeFile(resolve(outputDirectory, 'fonts', font.fileName), await readFile(font.sourceUrl)),
	),
);
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
// Editor and tooling data from the one vocabulary: a Custom Elements Manifest and VS Code HTML data.
const valuesOf = (type: (typeof vocabulary)[string]['attributes'][string]['type']): string =>
	type.kind === 'enum'
		? type.values.map((value): string => JSON.stringify(value)).join(' | ')
		: type.kind === 'number' || type.kind === 'flag'
			? type.kind === 'number'
				? 'number'
				: 'boolean'
			: 'string';
const declarations = Object.entries(vocabulary).map(([tagName, spec]) => {
	const declaration: Record<string, unknown> = {
		kind: 'class',
		name: tagName,
		tagName,
		customElement: true,
		description: spec.description,
		attributes: Object.entries({ ...layoutAttributes, ...spec.attributes }).map(
			([name, attribute]) => ({
				name,
				description: attribute.description,
				type: { text: valuesOf(attribute.type) },
			}),
		),
	};
	if (spec.legacy) declaration['deprecated'] = 'Earlier lab vocabulary; still rendered.';
	return declaration;
});
const customData = Object.entries(vocabulary).map(([name, spec]) => ({
	name,
	description: spec.description,
	attributes: Object.entries({ ...layoutAttributes, ...spec.attributes }).map(
		([attributeName, attribute]) => {
			const entry: Record<string, unknown> = {
				name: attributeName,
				description: attribute.description,
			};
			if (attribute.type.kind === 'enum')
				entry['values'] = attribute.type.values.map((value) => ({ name: value }));
			if (attribute.type.kind === 'flag') entry['valueSet'] = 'v';
			return entry;
		},
	),
}));
const json = (value: unknown): string => `${JSON.stringify(value, null, '\t')}\n`;
await writeFile(
	resolve(outputDirectory, 'custom-elements.json'),
	json({
		schemaVersion: '2.1.0',
		modules: [{ kind: 'javascript-module', path: 'runtime.js', declarations, exports: [] }],
	}),
);
await writeFile(
	resolve(outputDirectory, 'html-custom-data.json'),
	json({ version: 1.1, tags: customData }),
);
process.stdout.write(
	'Built drop-in Demistify runtime, lazy renderer chunks, theme, linked fonts, subset icons and editor data.\n',
);
