import { readFile, realpath, stat } from 'node:fs/promises';
import { createServer, type Server } from 'node:http';
import { extname, isAbsolute, join, relative, resolve, sep } from 'node:path';

const contentTypes = new Map([
	['.html', 'text/html; charset=utf-8'],
	['.htm', 'text/html; charset=utf-8'],
	['.md', 'text/markdown; charset=utf-8'],
	['.txt', 'text/plain; charset=utf-8'],
	['.json', 'application/json'],
	['.js', 'text/javascript'],
	['.mjs', 'text/javascript'],
	['.css', 'text/css'],
	['.svg', 'image/svg+xml'],
	['.png', 'image/png'],
	['.jpeg', 'image/jpeg'],
	['.jpg', 'image/jpeg'],
	['.gif', 'image/gif'],
	['.webp', 'image/webp'],
	['.avif', 'image/avif'],
	['.ico', 'image/x-icon'],
	['.woff', 'font/woff'],
	['.woff2', 'font/woff2'],
	['.ttf', 'font/ttf'],
	['.wasm', 'application/wasm'],
	['.pdf', 'application/pdf'],
	['.mp4', 'video/mp4'],
	['.webm', 'video/webm'],
	['.mp3', 'audio/mpeg'],
	['.ogg', 'audio/ogg'],
	['.wav', 'audio/wav'],
]);

export function containsPath(root: string, file: string): boolean {
	const child = relative(root, file);
	return !isAbsolute(child) && child !== '..' && !child.startsWith(`..${sep}`);
}

function hiddenPath(root: string, file: string): boolean {
	return relative(root, file)
		.split(sep)
		.some((part): boolean => part.startsWith('.'));
}

export function localPageUrl(root: string, file: string, origin: string): URL {
	const path = relative(root, file).split(sep).map(encodeURIComponent).join('/');
	return new URL(path, `${origin}/`);
}

async function resolveLocalSiteFile(
	root: string,
	url: URL,
): Promise<{ readonly file: string; readonly directory: boolean }> {
	const candidate = resolve(root, `.${decodeURIComponent(url.pathname)}`);
	if (!containsPath(root, candidate) || hiddenPath(root, candidate))
		throw new Error('Path escapes the local site.');
	const physical = await realpath(candidate);
	if (!containsPath(root, physical) || hiddenPath(root, physical))
		throw new Error('Path resolves outside the local site.');
	const directory = (await stat(physical)).isDirectory();
	const file = directory ? await realpath(join(physical, 'index.html')) : physical;
	if (!containsPath(root, file) || hiddenPath(root, file) || !(await stat(file)).isFile())
		throw new Error('Invalid local site file.');
	return { file, directory };
}

export async function localSiteFile(root: string, url: URL): Promise<string> {
	return (await resolveLocalSiteFile(root, url)).file;
}

export async function serveLocalSite(
	root: string,
	options: {
		readonly browserAssetsOnly?: boolean;
		readonly prepared?: ReadonlyMap<string, string>;
	} = {},
): Promise<{ readonly server: Server; readonly origin: string }> {
	const server = createServer((request, response): void => {
		const address = server.address();
		if (
			typeof address !== 'object' ||
			address === null ||
			request.headers.host !== `127.0.0.1:${address.port}` ||
			(request.method !== 'GET' && request.method !== 'HEAD')
		) {
			response.writeHead(404).end();
			return;
		}

		void Promise.resolve()
			.then(async (): Promise<void> => {
				const url = new URL(request.url ?? '/', 'http://local');
				const candidate = resolve(root, `.${decodeURIComponent(url.pathname)}`);
				const prepared =
					containsPath(root, candidate) && !hiddenPath(root, candidate)
						? options.prepared?.get(candidate)
						: undefined;
				const { file, directory } =
					prepared === undefined
						? await resolveLocalSiteFile(root, url)
						: { file: candidate, directory: false };
				if (directory && !url.pathname.endsWith('/')) {
					url.pathname += '/';
					response
						.writeHead(308, {
							location: `http://127.0.0.1:${address.port}${url.pathname}${url.search}`,
						})
						.end();
					return;
				}
				const contentType = contentTypes.get(extname(file).toLowerCase());
				if (options.browserAssetsOnly && !contentType) {
					response.writeHead(404).end();
					return;
				}
				const body =
					request.method === 'HEAD'
						? undefined
						: (prepared ?? options.prepared?.get(file) ?? (await readFile(file)));
				response.writeHead(200, {
					'cache-control': 'no-store',
					'content-type': contentType ?? 'application/octet-stream',
					'x-content-type-options': 'nosniff',
				});
				response.end(body);
			})
			.catch((): void => {
				if (!response.headersSent) response.writeHead(404);
				response.end();
			});
	});
	await new Promise<void>((done, reject): void => {
		server.once('error', reject);
		server.listen(0, '127.0.0.1', done);
	});
	const address = server.address();
	if (typeof address !== 'object' || address === null) {
		server.close();
		throw new Error('Cannot start the local Demistify server.');
	}
	return { server, origin: `http://127.0.0.1:${address.port}` };
}
