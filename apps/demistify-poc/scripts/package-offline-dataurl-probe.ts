/** Packaging experiment for the supplied single-page usages; no network or application server. */
import { readFile, writeFile } from 'node:fs/promises';
import { dirname, extname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { gzipSync } from 'node:zlib';

import { parseGraphDocument } from '../../../packages/demistify/src/explorer/graph-document.ts';

const [pageArgument, outputArgument] = process.argv.slice(2);
if (!pageArgument || !outputArgument)
	throw new Error(
		'Usage: node apps/demistify-poc/scripts/package-offline-dataurl-probe.ts PAGE_HTML OUTPUT_HTML',
	);
const pagePath = resolve(pageArgument);
const libraryRoot = fileURLToPath(new URL('../../../packages/demistify/dist/', import.meta.url));
const moduleUrls = new Map<string, string>();
const activeModules = new Set<string>();

/** Built ESM has explicit relative JS literals; unsupported/cyclic graphs fail instead of half-bundling. */
async function inlineModule(modulePath: string): Promise<string> {
	const cached = moduleUrls.get(modulePath);
	if (cached) return cached;
	if (activeModules.has(modulePath))
		throw new Error(
			'This offline probe does not support a cyclic built chunk graph. Use the complete asset-folder mode.',
		);
	activeModules.add(modulePath);
	let source = await readFile(modulePath, 'utf8');
	const references = [
		...source.matchAll(/\b(?:from|import)\s*(?:\(\s*)?(["'])(\.{1,2}\/[^"']+\.js)\1/g),
	];
	for (const match of references) {
		const literal = `${match[1]}${match[2]}${match[1]}`,
			relativePath = match[2];
		if (!relativePath) continue;
		const dependencyPath = resolve(dirname(modulePath), relativePath);
		// oxlint-disable-next-line no-await-in-loop -- recursively resolve the built dependency graph before its caller.
		const url = await inlineModule(dependencyPath);
		source = source.replaceAll(literal, JSON.stringify(url));
	}
	if (modulePath === resolve(libraryRoot, 'runtime.js') && source.includes('"diagram-worker.js"')) {
		const workerUrl = await inlineModule(resolve(libraryRoot, 'diagram-worker.js'));
		source = source.replaceAll('"diagram-worker.js"', JSON.stringify(workerUrl));
	}
	const url = `data:text/javascript;base64,${Buffer.from(source).toString('base64')}`;
	activeModules.delete(modulePath);
	moduleUrls.set(modulePath, url);
	return url;
}
async function inlineFontUrls(style: string): Promise<string> {
	let result = style;
	const matches = [...style.matchAll(/url\((['"]?)(\.\/[^)'"\s]+)\1\)/g)];
	for (const match of matches) {
		const relativePath = match[2];
		if (!relativePath) continue;
		if (extname(relativePath) !== '.woff2')
			throw new Error(`Unsupported CSS resource in offline probe: ${relativePath}`);
		// oxlint-disable-next-line no-await-in-loop -- each source literal is replaced after its resource is read.
		const bytes = await readFile(resolve(libraryRoot, relativePath));
		result = result.replaceAll(
			match[0],
			`url('data:font/woff2;base64,${bytes.toString('base64')}')`,
		);
	}
	return result;
}
let page = await readFile(pagePath, 'utf8');
if (/<img\b|<ex-mermaid\b[^>]*\bsrc=/i.test(page))
	throw new Error(
		'This bounded offline probe covers inline diagrams and explorer JSON, not image or diagram-file packaging.',
	);
const explorers = [...page.matchAll(/<ex-explorer\b([^>]*?)\bsrc=(['"])([^'"]+)\2([^>]*)>/g)];
for (const match of explorers) {
	const sourcePath = match[3];
	if (!sourcePath || /^(https?:|data:)/.test(sourcePath))
		throw new Error('Offline probe requires an explicit local explorer source.');
	// oxlint-disable-next-line no-await-in-loop -- resolve and validate each complete referenced document before replacing it.
	const source = await readFile(resolve(dirname(pagePath), sourcePath), 'utf8');
	parseGraphDocument(JSON.parse(source));
	page = page.replace(
		match[0],
		(): string =>
			`<ex-explorer${match[1]}${match[4]}><script type="application/json">${source.replaceAll('<', '\\u003c')}</script>`,
	);
}
const style = await inlineFontUrls(await readFile(resolve(libraryRoot, 'theme.css'), 'utf8'));
const runtimeUrl = await inlineModule(resolve(libraryRoot, 'runtime.js'));
const themeLink = /<link\b[^>]*href=(['"])[^'"]*theme\.css\1[^>]*>/;
const runtimeScript = /<script\b[^>]*src=(['"])[^'"]*runtime\.js\1[^>]*>\s*<\/script>/;
if (!themeLink.test(page) || !runtimeScript.test(page))
	throw new Error('Expected explicit shared runtime and theme links.');
page = page
	.replace(themeLink, (): string => `<style>${style}</style>`)
	.replace(runtimeScript, (): string => `<script type="module" src="${runtimeUrl}"></script>`);
await writeFile(resolve(outputArgument), page);
process.stdout.write(
	`${JSON.stringify({ htmlBytes: Buffer.byteLength(page), gzipBytes: gzipSync(page).length, embeddedModules: moduleUrls.size, referencedExplorerDocuments: explorers.length, limits: 'Fixed built ESM graph and supplied single-page usages. Full file-protocol/WebView and arbitrary page assets are not verified.' }, null, 2)}\n`,
);
