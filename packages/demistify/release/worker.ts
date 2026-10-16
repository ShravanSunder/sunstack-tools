import { immutableAssetPath } from './asset-policy.ts';

interface ReleaseAssetEnvironment {
	readonly ASSETS: { fetch(request: Request): Promise<Response> };
}
const contentTypes: Readonly<Record<string, string>> = {
	'.js': 'text/javascript; charset=utf-8',
	'.mjs': 'text/javascript; charset=utf-8',
	'.css': 'text/css; charset=utf-8',
	'.json': 'application/json; charset=utf-8',
	'.md': 'text/markdown; charset=utf-8',
	'.woff2': 'font/woff2',
};
export default {
	async fetch(request: Request, environment: ReleaseAssetEnvironment): Promise<Response> {
		const headers = new Headers({ 'Access-Control-Allow-Origin': '*', 'Timing-Allow-Origin': '*' });
		if (request.method === 'OPTIONS') {
			headers.set('Access-Control-Allow-Methods', 'GET, HEAD, OPTIONS');
			return new Response(null, { status: 204, headers });
		}
		if (!['GET', 'HEAD'].includes(request.method)) {
			headers.set('Allow', 'GET, HEAD, OPTIONS');
			return new Response('Method not allowed', { status: 405, headers });
		}
		const response = await environment.ASSETS.fetch(request);
		for (const [name, value] of response.headers) headers.set(name, value);
		headers.set('Access-Control-Allow-Origin', '*');
		headers.set('Timing-Allow-Origin', '*');
		const path = new URL(request.url).pathname;
		const immutable = immutableAssetPath(path);
		const success = response.status === 200 || response.status === 304;
		headers.set(
			'Cache-Control',
			success
				? immutable
					? 'public, max-age=31536000, immutable'
					: 'public, max-age=0, must-revalidate'
				: 'no-store',
		);
		const mime = contentTypes[path.slice(path.lastIndexOf('.'))];
		if (success && mime) headers.set('Content-Type', mime);
		return new Response(request.method === 'HEAD' ? null : response.body, {
			status: response.status,
			headers,
		});
	},
};
