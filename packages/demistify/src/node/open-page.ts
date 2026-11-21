import { spawn } from 'node:child_process';
import { realpath, stat } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';

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
	file: string,
	options: { readonly siteRoot?: string | undefined; readonly noBrowser: boolean },
): Promise<void> {
	const requestedPage = resolve(file);
	if (!/\.html?$/i.test(requestedPage)) throw new Error('Select an HTML page to open.');
	const page = await realpath(requestedPage);
	const root = await realpath(options.siteRoot ?? dirname(page));
	if (!containsPath(root, page)) throw new Error('The HTML page is outside the site root.');
	await localSiteFile(root, localPageUrl(root, page, 'http://local'));
	if (!/\.html?$/i.test(page) || !(await stat(page)).isFile())
		throw new Error('The page must resolve to an HTML file.');

	const { origin } = await serveLocalSite(root, { browserAssetsOnly: true });
	const url = localPageUrl(root, page, origin).href;
	process.stdout.write(`Open ${url}\nPress Ctrl+C to stop the preview.\n`);
	if (!options.noBrowser) openBrowser(url);
}
