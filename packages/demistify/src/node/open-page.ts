import { spawn } from 'node:child_process';
import { readFile, realpath, stat } from 'node:fs/promises';
import { basename, dirname, join, relative, resolve } from 'node:path';

import { designPreviewSources } from './design-authoring/design-page-markup.ts';
import { containsPath, localPageUrl, localSiteFile, serveLocalSite } from './local-site.ts';

function openBrowser(url: string): void {
	const platform = process.platform;
	const command =
		platform === 'darwin' ? 'open' : platform === 'win32' ? 'rundll32.exe' : 'xdg-open';
	const args = platform === 'win32' ? ['url.dll,FileProtocolHandler', url] : [url];
	const browser = spawn(command, args, { detached: true, stdio: 'ignore' });
	const reportFailure = (): void => {
		process.stderr.write(`Could not start the default browser. Open ${url} manually.\n`);
	};
	browser.once('error', reportFailure);
	browser.once('exit', (code): void => {
		if (code !== null && code !== 0) reportFailure();
	});
	browser.unref();
}

export async function openPage(
	pagePath: string,
	options: { readonly siteRoot?: string | undefined; readonly noBrowser: boolean },
): Promise<void> {
	const requestedPage = resolve(pagePath);
	if (!/\.html?$/i.test(requestedPage)) throw new Error('Select an HTML page to open.');
	const physicalPage = await realpath(requestedPage);
	const selectedRoot = options.siteRoot === undefined ? undefined : resolve(options.siteRoot);
	const limit = selectedRoot === undefined ? undefined : await realpath(selectedRoot);
	if (limit && !containsPath(limit, physicalPage))
		throw new Error('The HTML page is outside the site root.');
	if (!/\.html?$/i.test(physicalPage) || !(await stat(physicalPage)).isFile())
		throw new Error('The page must resolve to an HTML file.');

	const declaredRequest =
		limit && selectedRoot && containsPath(selectedRoot, requestedPage)
			? resolve(limit, relative(selectedRoot, requestedPage))
			: join(await realpath(dirname(requestedPage)), basename(requestedPage));
	if (limit && !containsPath(limit, declaredRequest))
		throw new Error('The HTML page is outside the site root.');
	const html = await readFile(physicalPage, 'utf8');
	const sources = designPreviewSources(html);
	const owner =
		sources &&
		(await (
			await import('./design-authoring/preview-owner.ts')
		).findDesignPreviewOwner({
			requestedPage: declaredRequest,
			physicalPage,
			siteRoot: limit,
			sources,
		}));
	const page = owner?.page ?? physicalPage;
	const root = limit ?? owner?.directory ?? dirname(physicalPage);
	if ((await localSiteFile(root, localPageUrl(root, page, 'http://local'))) !== physicalPage)
		throw new Error(`Design page changed while opening ${page}.`);

	if (owner) {
		const build =
			owner.kind === 'project'
				? await (
						await import('./design-authoring/check-project.ts')
					).buildProject(owner.directory, { siteRoot: root, write: true })
				: await (
						await import('./design-authoring/check-design.ts')
					).buildDesignSet(owner.directory, { siteRoot: root, write: true });
		if (!build.preparedPages.has(page))
			throw new Error(`Design ownership changed while opening ${page}.`);
	}
	const { origin } = await serveLocalSite(root, { browserAssetsOnly: true });
	const url = localPageUrl(root, page, origin).href;
	process.stdout.write(`Open ${url}\nPress Ctrl+C to stop the preview.\n`);
	if (!options.noBrowser) openBrowser(url);
}
