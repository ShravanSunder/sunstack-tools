/** Local protocol experiment, not a Cloudflare cache or latency benchmark. */
import { readFile, readdir } from 'node:fs/promises';
import { createServer } from 'node:http';
import type { IncomingMessage, ServerResponse } from 'node:http';

// Both release roots contain the same frozen library bytes: payload size is a negative control.
type ReleaseVersion = '0.1.0' | '0.1.1';
let selectedRelease: ReleaseVersion = '0.1.0';
const libraryRoot = new URL('../../../packages/demistify/dist/', import.meta.url);
const assetPaths = (await readdir(libraryRoot, { recursive: true })).filter((path): boolean =>
	/\.(js|css|woff2|json)$/.test(path),
);
const assetEntries = await Promise.all(
	assetPaths.map(async (path): Promise<[string, Buffer]> => [
		path.replaceAll('\\', '/'),
		await readFile(new URL(path, libraryRoot)),
	]),
);
const assets = new Map(assetEntries);
if (!assets.has('runtime.js') || !assets.has('theme.css'))
	throw new Error('Build the library before running this probe.');

function serveRelease(request: IncomingMessage, response: ServerResponse): void {
	const path = new URL(request.url ?? '/', 'http://local').pathname;
	response.setHeader('Access-Control-Allow-Origin', '*');
	response.setHeader('Cache-Control', 'no-store');
	if (path === '/selected-release.json') {
		response.setHeader('Content-Type', 'application/json');
		response.end(JSON.stringify({ version: selectedRelease }));
		return;
	}
	if (path.startsWith('/demistify/0.1/')) {
		response.writeHead(302, {
			Location: path.replace('/demistify/0.1/', `/demistify/${selectedRelease}/`),
		});
		response.end();
		return;
	}
	const match = /^\/demistify\/(0\.1\.[01])\/(.+)$/.exec(path);
	const payload = match?.[2] ? assets.get(match[2]) : undefined;
	if (!payload) {
		response.writeHead(404).end('Missing asset');
		return;
	}
	response.setHeader('X-Demistify-Release', match?.[1] ?? '');
	response.setHeader('Cache-Control', 'public, max-age=31536000, immutable');
	response.setHeader(
		'Content-Type',
		path.endsWith('.js')
			? 'text/javascript'
			: path.endsWith('.css')
				? 'text/css'
				: path.endsWith('.woff2')
					? 'font/woff2'
					: 'application/json',
	);
	response.end(payload);
}

const server = createServer(serveRelease);
await new Promise<void>((resolve): void => {
	server.listen(0, '127.0.0.1', resolve);
});
const address = server.address();
if (!address || typeof address === 'string') throw new Error('No local address.');
const origin = `http://127.0.0.1:${address.port}`;

interface AssetObservation {
	readonly release: string | null;
	readonly path: string;
	readonly cache: string | null;
	readonly status: number;
}
async function observeAsset(path: string): Promise<AssetObservation> {
	const response = await fetch(origin + path);
	await response.arrayBuffer();
	return {
		release: response.headers.get('x-demistify-release'),
		path: new URL(response.url).pathname,
		cache: response.headers.get('cache-control'),
		status: response.status,
	};
}

try {
	const aliasCss = await observeAsset('/demistify/0.1/theme.css');
	selectedRelease = '0.1.1';
	const aliasRuntime = await observeAsset('/demistify/0.1/runtime.js');
	const pinnedCss = await observeAsset('/demistify/0.1.0/theme.css');
	const pinnedRuntime = await observeAsset('/demistify/0.1.0/runtime.js');

	selectedRelease = '0.1.0';
	const selectionResponse = await fetch(`${origin}/selected-release.json`);
	const selection: unknown = await selectionResponse.json();
	if (
		typeof selection !== 'object' ||
		selection === null ||
		!('version' in selection) ||
		(selection.version !== '0.1.0' && selection.version !== '0.1.1')
	)
		throw new Error('Invalid release selection.');
	const selectedBase = `/demistify/${selection.version}/`;
	selectedRelease = '0.1.1';
	const selectedCss = await observeAsset(`${selectedBase}theme.css`);
	const selectedRuntime = await observeAsset(`${selectedBase}runtime.js`);
	const worker = await observeAsset(`${selectedBase}diagram-worker.js`);
	const missing = await observeAsset(`${selectedBase}absent.js`);
	selectedRelease = '0.1.0';
	const rollback = await observeAsset('/demistify/0.1/runtime.js');
	const redirect = await fetch(`${origin}/demistify/0.1/runtime.js`, { redirect: 'manual' });

	const checks = {
		movingAliasesCanMix: aliasCss.release !== aliasRuntime.release,
		exactPinsStayTogether: pinnedCss.release === '0.1.0' && pinnedRuntime.release === '0.1.0',
		selectedReleaseStaysTogether:
			selectedCss.release === '0.1.0' && selectedRuntime.release === '0.1.0',
		workerSharesSelectedRoot: worker.release === '0.1.0',
		rollbackSelectsOldRoot: rollback.release === '0.1.0',
		missingAssetIsNotHtml: missing.status === 404 && missing.cache === 'no-store',
		redirectCarriesCors: redirect.headers.get('access-control-allow-origin') === '*',
	};
	process.stdout.write(
		`${JSON.stringify({ checks, observations: { aliasCss, aliasRuntime, pinnedCss, pinnedRuntime, selectedCss, selectedRuntime, worker, missing, rollback }, limits: 'Real loopback HTTP redirects and frozen library bytes. No browser module execution, Cloudflare backend, cache expiry, edge performance, upload immutability or deployment proof.' }, null, 2)}\n`,
	);
	if (Object.values(checks).some((passed): boolean => !passed)) process.exitCode = 1;
} finally {
	await new Promise<void>((resolve, reject): void => {
		server.close((error): void => {
			if (error) reject(error);
			else resolve();
		});
	});
}
