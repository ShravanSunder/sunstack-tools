import { spawn } from 'node:child_process';
import { readFile, realpath, stat } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';

import { findDesignInput } from './design-authoring/design-location.ts';
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
	const page = await realpath(requestedPage);
	const limit = options.siteRoot === undefined ? undefined : await realpath(options.siteRoot);
	if (limit && !containsPath(limit, page))
		throw new Error('The HTML page is outside the site root.');
	const directory = dirname(page);
	const html = await readFile(page, 'utf8');
	const usesDesign =
		/\bdata-design-(?:record|need)\s*=|(?:navigation|deliverables|project)\.generated\.json/i.test(
			html,
		);
	const [project, design] = usesDesign
		? await Promise.all([
				findDesignInput(directory, 'project.ts', limit),
				findDesignInput(directory, 'design.ts', limit),
			])
		: [undefined, undefined];
	const root = limit ?? project ?? design ?? directory;
	if (!containsPath(root, page)) throw new Error('The HTML page is outside the site root.');
	await localSiteFile(root, localPageUrl(root, page, 'http://local'));
	if (!/\.html?$/i.test(page) || !(await stat(page)).isFile())
		throw new Error('The page must resolve to an HTML file.');

	let prepared: Map<string, string> | undefined;
	if (project || design) {
		const build = project
			? await (
					await import('./design-authoring/check-project.ts')
				).buildProject(project, { siteRoot: root })
			: design
				? await (
						await import('./design-authoring/check-design.ts')
					).buildDesignSet(design, { siteRoot: root })
				: undefined;
		if (build)
			prepared = new Map([
				...build.preparedPages,
				...[...build.generated].map(([file, value]): [string, string] => [
					file,
					JSON.stringify(value),
				]),
			]);
	}
	const { origin } = await serveLocalSite(root, {
		browserAssetsOnly: true,
		...(prepared && { prepared }),
	});
	const url = localPageUrl(root, page, origin).href;
	process.stdout.write(`Open ${url}\nPress Ctrl+C to stop the preview.\n`);
	if (!options.noBrowser) openBrowser(url);
}
