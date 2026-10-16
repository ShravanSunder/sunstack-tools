/** CLI consumer of the shared Node packaging boundary; no network or application server. */
import { readFile, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { gzipSync } from 'node:zlib';

import { bundleStandaloneHtml } from '@sunstack/demistify/packaging';

const [pageArgument, outputArgument] = process.argv.slice(2);
if (!pageArgument || !outputArgument)
	throw new Error(
		'Usage: node apps/demistify-poc/scripts/package-offline-probe.ts PAGE_HTML OUTPUT_HTML',
	);
const pagePath = resolve(pageArgument);
const bundle = await bundleStandaloneHtml({
	sourceHtml: await readFile(pagePath, 'utf8'),
	libraryDirectory: fileURLToPath(new URL('../../../packages/demistify/dist/', import.meta.url)),
	resolveExplorerSource: async (sourcePath: string): Promise<string> =>
		readFile(resolve(dirname(pagePath), sourcePath), 'utf8'),
});
await writeFile(resolve(outputArgument), bundle.html);
process.stdout.write(
	`${JSON.stringify({ htmlBytes: Buffer.byteLength(bundle.html), gzipBytes: gzipSync(bundle.html).length, embeddedModules: bundle.embeddedModules, referencedExplorerDocuments: bundle.referencedExplorerDocuments, limits: 'Fixed built ESM graph and supplied single-page usages. Full file-protocol/WebView and arbitrary page assets are not verified.' }, null, 2)}\n`,
);
