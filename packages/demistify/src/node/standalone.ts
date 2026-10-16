/** Node-only packaging for the library's built ESM graph; HTML remains a document, not an app build. */
import { readFile } from 'node:fs/promises';
import { dirname, extname, relative, resolve } from 'node:path';

import { parseGraphDocument } from '../explorer/graph-document.ts';
import { embedMarkdownFiles, markdownFileReferences } from './markdown-files.ts';

export interface StandaloneHtmlOptions {
	readonly sourceHtml: string;
	readonly libraryDirectory: string;
	readonly resolveMarkdownSource?: (sourcePath: string) => Promise<string>;
	readonly resolveExplorerSource?: (sourcePath: string) => Promise<string>;
}
export interface StandaloneHtmlBundle {
	readonly html: string;
	readonly embeddedModules: number;
	readonly referencedExplorerDocuments: number;
}

/** Fixed built modules, fonts and supplied local graph data; unsupported asset kinds fail explicitly. */
export async function bundleStandaloneHtml(
	options: StandaloneHtmlOptions,
): Promise<StandaloneHtmlBundle> {
	const libraryRoot = resolve(options.libraryDirectory);
	interface EmbeddedReference {
		readonly literal: string;
		readonly moduleName: string;
	}
	interface EmbeddedModule {
		readonly name: string;
		readonly source: string;
		readonly references: readonly EmbeddedReference[];
	}
	const orderedModules: EmbeddedModule[] = [];
	const visitedModules = new Set<string>();
	const activeModules = new Set<string>();

	/** Keep each source once; the generated bootloader creates shared blob module URLs in dependency order. */
	async function collectModule(modulePath: string): Promise<string> {
		const name = relative(libraryRoot, modulePath).replaceAll('\\', '/');
		if (visitedModules.has(modulePath)) return name;
		if (activeModules.has(modulePath))
			throw new Error(
				'This offline bundle does not support a cyclic built chunk graph. Use the complete asset-folder mode.',
			);
		activeModules.add(modulePath);
		const source = await readFile(modulePath, 'utf8');
		const references: EmbeddedReference[] = [];
		for (const match of source.matchAll(
			/\b(?:from|import)\s*(?:\(\s*)?(["'])(\.{1,2}\/[^"']+\.js)\1/g,
		)) {
			const relativePath = match[2];
			if (!relativePath) continue;
			// oxlint-disable-next-line no-await-in-loop -- dependency URLs must exist before the importing module is created.
			const moduleName = await collectModule(resolve(dirname(modulePath), relativePath));
			references.push({ literal: `${match[1]}${relativePath}${match[1]}`, moduleName });
		}
		if (
			modulePath === resolve(libraryRoot, 'runtime.js') &&
			source.includes('"diagram-worker.js"')
		) {
			const moduleName = await collectModule(resolve(libraryRoot, 'diagram-worker.js'));
			references.push({ literal: '"diagram-worker.js"', moduleName });
		}
		activeModules.delete(modulePath);
		visitedModules.add(modulePath);
		orderedModules.push({ name, source, references });
		return name;
	}

	async function inlineFontUrls(style: string): Promise<string> {
		let result = style;
		const matches = [...style.matchAll(/url\((['"]?)(\.\/[^)'"\s]+)\1\)/g)];
		for (const match of matches) {
			const relativePath = match[2];
			if (!relativePath) continue;
			if (extname(relativePath) !== '.woff2')
				throw new Error(`Unsupported CSS resource in offline bundle: ${relativePath}`);
			// oxlint-disable-next-line no-await-in-loop -- each source literal is replaced after its resource is read.
			const bytes = await readFile(resolve(libraryRoot, relativePath));
			result = result.replaceAll(
				match[0],
				`url('data:font/woff2;base64,${bytes.toString('base64')}')`,
			);
		}
		return result;
	}
	const markdownSources: Record<string, string> = {};
	for (const reference of markdownFileReferences(options.sourceHtml)) {
		if (!options.resolveMarkdownSource)
			throw new Error('Provide an explicit resolver for referenced Markdown files.');
		// oxlint-disable-next-line no-await-in-loop -- embed each complete source before creating the offline bundle.
		markdownSources[reference] = await options.resolveMarkdownSource(reference);
	}
	let page = embedMarkdownFiles(options.sourceHtml, markdownSources);
	const unresolvedDiagram = [...page.matchAll(/<ex-mermaid\b([^>]*)>/gi)].some(
		(match): boolean =>
			/\bsrc=/.test(match[1] ?? '') && !/(?:^|\s)prepared(?:\s|$|=)/.test(match[1] ?? ''),
	);
	if (/<img\b/i.test(page) || unresolvedDiagram)
		throw new Error(
			'This bounded offline bundle covers inline diagrams and explorer JSON, not image or diagram-file packaging.',
		);
	const explorers = [...page.matchAll(/<ex-explorer\b([^>]*?)\bsrc=(['"])([^'"]+)\2([^>]*)>/g)];
	for (const match of explorers) {
		const sourcePath = match[3];
		if (!sourcePath || /^(https?:|data:)/.test(sourcePath))
			throw new Error('Offline bundle requires an explicit local explorer source.');
		if (!options.resolveExplorerSource)
			throw new Error('Provide an explicit resolver for referenced explorer JSON.');
		// oxlint-disable-next-line no-await-in-loop -- resolve and validate each complete document before replacing it.
		const source = await options.resolveExplorerSource(sourcePath);
		parseGraphDocument(JSON.parse(source));
		page = page.replace(
			match[0],
			(): string =>
				`<ex-explorer${match[1]}${match[4]}><script type="application/json">${source.replaceAll('<', '\\u003c')}</script>`,
		);
	}
	const style = await inlineFontUrls(await readFile(resolve(libraryRoot, 'theme.css'), 'utf8'));
	await collectModule(resolve(libraryRoot, 'runtime.js'));
	const bootstrapSource = `const modules=JSON.parse(document.getElementById('demistify-offline-modules').textContent);const urls=new Map();for(const module of modules){let source=module.source;for(const reference of module.references){const dependency=urls.get(reference.moduleName);if(!dependency)throw new Error('Missing offline dependency: '+reference.moduleName);source=source.replaceAll(reference.literal,JSON.stringify(dependency));}urls.set(module.name,URL.createObjectURL(new Blob([source],{type:'text/javascript'})));}await import(urls.get('runtime.js'));`;
	const runtimeMarkup = `<script id="demistify-offline-modules" type="application/json">${JSON.stringify(orderedModules).replaceAll('<', '\\u003c')}</script><script type="module">${bootstrapSource}</script>`;
	const themeLink = /<link\b[^>]*href=(['"])[^'"]*theme\.css\1[^>]*>/;
	const runtimeScript = /<script\b[^>]*src=(['"])[^'"]*runtime\.js\1[^>]*>\s*<\/script>/;
	if (!themeLink.test(page) || !runtimeScript.test(page))
		throw new Error('Expected explicit shared runtime and theme links.');
	page = page
		.replace(themeLink, (): string => `<style>${style}</style>`)
		.replace(runtimeScript, (): string => runtimeMarkup);
	return {
		html: page,
		embeddedModules: orderedModules.length,
		referencedExplorerDocuments: explorers.length,
	};
}
