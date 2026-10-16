interface LibraryAssetsEnvironment {
	readonly ASSETS: {
		fetch(request: Request): Promise<Response>;
	};
}

/** Public library and frozen samples share headers; the landing page revalidates. */
export default {
	async fetch(request: Request, environment: LibraryAssetsEnvironment): Promise<Response> {
		const url = new URL(request.url);
		const response = await environment.ASSETS.fetch(request);
		const headers = new Headers(response.headers);
		headers.set('Access-Control-Allow-Origin', '*');
		headers.set('Timing-Allow-Origin', '*');
		const pinned =
			url.pathname.startsWith('/demistify/experimental/') ||
			/^\/examples\/2026-10-16-sol-luna-v[123]\//.test(url.pathname);
		headers.set(
			'Cache-Control',
			(response.status === 200 || response.status === 304) && pinned
				? 'public, max-age=31536000, immutable'
				: response.status === 200
					? 'no-cache'
					: 'no-store',
		);
		return new Response(response.body, { status: response.status, headers });
	},
};
