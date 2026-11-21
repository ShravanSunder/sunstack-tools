/* oxlint-disable no-await-in-loop -- sequential navigation shares one page, viewport, and issue stream. */
import { readFile, realpath, stat } from 'node:fs/promises';
import { createServer, type Server } from 'node:http';
import { createRequire } from 'node:module';
import { extname, isAbsolute, join, relative, resolve, sep } from 'node:path';

const contentTypes: Readonly<Record<string, string>> = {
	'.html': 'text/html; charset=utf-8',
	'.htm': 'text/html; charset=utf-8',
	'.js': 'text/javascript',
	'.mjs': 'text/javascript',
	'.css': 'text/css',
	'.svg': 'image/svg+xml',
	'.png': 'image/png',
	'.jpg': 'image/jpeg',
	'.webp': 'image/webp',
	'.woff2': 'font/woff2',
	'.json': 'application/json',
};

interface Browser {
	newPage(options: { viewport: { width: number; height: number } }): Promise<Page>;
	close(): Promise<void>;
}

interface ConsoleMessage {
	type(): string;
	text(): string;
}

interface Page {
	// oxlint-disable-next-line id-length -- Playwright method.
	on(event: 'console', listener: (message: ConsoleMessage) => void): void;
	// oxlint-disable-next-line id-length -- Playwright method.
	on(event: 'pageerror', listener: (error: Error) => void): void;
	off(event: 'console', listener: (message: ConsoleMessage) => void): void;
	off(event: 'pageerror', listener: (error: Error) => void): void;
	goto(
		url: string,
		options: { waitUntil: 'domcontentloaded'; timeout: number },
	): Promise<{
		// oxlint-disable-next-line id-length -- Playwright response method.
		ok(): boolean;
		status(): number;
	} | null>;
	evaluate<Result>(script: () => Promise<Result> | Result): Promise<Result>;
	waitForFunction(
		script: () => boolean,
		argument: undefined,
		options: { timeout: number },
	): Promise<unknown>;
	close(): Promise<void>;
}

interface PageInspection {
	readonly messages: string[];
	readonly links: string[];
	readonly fragments: string[];
}

const widths = [1400, 390] as const;

function hasChromium(value: unknown): value is { chromium: { launch(): Promise<Browser> } } {
	return typeof value === 'object' && value !== null && 'chromium' in value;
}

async function launchBrowser(): Promise<Browser> {
	try {
		const require = createRequire(join(process.cwd(), 'package.json'));
		const module: unknown = require('playwright');
		if (!hasChromium(module)) throw new Error('The installed package does not expose Chromium.');
		return await module.chromium.launch();
	} catch (error) {
		throw new Error(
			`Cannot launch Chromium for --render. In this directory run pnpm add -D playwright and pnpm exec playwright install chromium. ${error instanceof Error ? error.message : String(error)}`,
			{ cause: error },
		);
	}
}

function containsPath(root: string, file: string): boolean {
	const child = relative(root, file);
	return !isAbsolute(child) && child !== '..' && !child.startsWith(`..${sep}`);
}

async function fileForUrl(base: string, url: URL): Promise<string> {
	const candidate = resolve(base, `.${decodeURIComponent(url.pathname)}`);
	if (!containsPath(base, candidate)) throw new Error('Path escapes the render root.');
	const physical = await realpath(candidate);
	if (!containsPath(base, physical)) throw new Error('Path resolves outside the render root.');
	if (!(await stat(physical)).isDirectory()) return physical;
	const index = await realpath(join(physical, 'index.html'));
	if (!containsPath(base, index))
		throw new Error('Directory index resolves outside the render root.');
	return index;
}

async function serve(base: string): Promise<{ server: Server; origin: string }> {
	const server = createServer((request, response): void => {
		const url = new URL(request.url ?? '/', 'http://local');
		void fileForUrl(base, url)
			.then(async (file) => {
				const body = await readFile(file);
				response.writeHead(200, {
					'content-type': contentTypes[extname(file)] ?? 'application/octet-stream',
				});
				response.end(body);
			})
			.catch((): void => {
				response.writeHead(404).end();
			});
	});
	await new Promise<void>((done, reject): void => {
		server.once('error', reject);
		server.listen(0, '127.0.0.1', done);
	});
	const address = server.address();
	if (typeof address !== 'object' || address === null) {
		server.close();
		throw new Error('Cannot start the local render-check server.');
	}
	return { server, origin: `http://127.0.0.1:${address.port}` };
}

async function inspectPage(page: Page, address: URL): Promise<PageInspection> {
	const response = await page.goto(address.href, { waitUntil: 'domcontentloaded', timeout: 15000 });
	if (!response?.ok())
		throw new Error(`HTTP ${response?.status() ?? 'unavailable'} loading ${address.pathname}.`);

	// Scroll once to activate lazy islands and visit visible tabs using their normal handlers.
	await page.evaluate(async () => {
		// oxlint-disable-next-line unicorn/consistent-function-scoping -- runs in Chromium.
		const frame = (): Promise<void> =>
			new Promise((done): void => {
				requestAnimationFrame((): void => {
					requestAnimationFrame((): void => done());
				});
			});
		await document.fonts.ready;
		for (let top = 0; top < document.documentElement.scrollHeight; top += window.innerHeight) {
			window.scrollTo(0, top);
			await frame();
		}
		for (const tab of document.querySelectorAll<HTMLElement>('[role="tab"]')) {
			if (!tab.getClientRects().length) continue;
			tab.click();
			await frame();
		}
		window.scrollTo(0, 0);
	});
	await page
		.waitForFunction(
			() =>
				![...document.querySelectorAll('[aria-busy="true"]')].some(
					(element): boolean =>
						element.localName.startsWith('ex-') && element.getClientRects().length > 0,
				),
			undefined,
			{ timeout: 4000 },
		)
		.catch((): void => {
			// Collect the names of still-busy islands in the same DOM report below.
		});

	return page.evaluate((): PageInspection => {
		const ignore = 'code, pre, kbd, samp, [data-inspector-copy], [data-inspector-content]';
		const messages: string[] = [];
		if (document.documentElement.scrollWidth > window.innerWidth + 1)
			messages.push(
				`Page overflows horizontally (${document.documentElement.scrollWidth}px at ${window.innerWidth}px).`,
			);
		for (const element of document.querySelectorAll('*')) {
			const name = element.localName;
			if (
				!name.startsWith('ex-') ||
				element.closest(ignore) ||
				!element.getClientRects().length ||
				getComputedStyle(element).visibility === 'hidden'
			)
				continue;
			const label = element.id ? `<${name} id="${element.id}">` : `<${name}>`;
			if (!customElements.get(name)) messages.push(`Visible ${label} is unregistered.`);
			else if (element.getAttribute('aria-busy') === 'true')
				messages.push(`Visible ${label} did not finish rendering.`);
		}
		for (const cell of document.querySelectorAll('th, td')) {
			if (!cell.getClientRects().length || cell.closest(ignore)) continue;
			const text = document.createTreeWalker(cell, NodeFilter.SHOW_TEXT);
			while (text.nextNode()) {
				if (text.currentNode.parentElement?.closest(ignore)) continue;
				const raw = /<\/?[a-z][a-z0-9-]*(?:\s[^<>]*)?\s*\/?>/i.exec(
					text.currentNode.textContent ?? '',
				);
				if (!raw) continue;
				messages.push(
					`Raw markup ${raw[0]} in a table cell. Render it as markup or mark it as code.`,
				);
				break;
			}
		}
		return {
			messages,
			links: [...document.querySelectorAll<HTMLAnchorElement>('a[href]')]
				.filter((anchor): boolean => !anchor.closest(ignore))
				.map((anchor): string => anchor.href),
			fragments: [
				...[...document.querySelectorAll('[id]')].map((element): string => element.id),
				...[...document.querySelectorAll('a[name]')].map(
					(element): string => element.getAttribute('name') ?? '',
				),
			],
		};
	});
}

interface RenderFinding {
	readonly file: string;
	readonly width: number;
	readonly message: string;
}

async function inspectLinks(
	inspection: PageInspection,
	address: URL,
	page: Page,
	base: string,
): Promise<string[]> {
	const messages: string[] = [];
	const fragments = new Map([[address.href, new Set(inspection.fragments)]]);
	for (const href of new Set(inspection.links)) {
		const target = new URL(href, address);
		if (target.origin !== address.origin) continue;
		const fragment = target.hash.slice(1);
		target.hash = '';
		try {
			const file = await fileForUrl(base, target);
			if (!fragment || !/\.html?$/i.test(file)) continue;
			let ids = fragments.get(target.href);
			if (!ids) {
				ids = new Set((await inspectPage(page, target)).fragments);
				fragments.set(target.href, ids);
			}
			if (!ids.has(decodeURIComponent(fragment)))
				messages.push(`Missing local fragment: ${target.pathname}#${fragment}`);
		} catch {
			messages.push(
				`Missing or unreadable local destination: ${target.pathname}${fragment ? `#${fragment}` : ''}`,
			);
		}
	}
	return messages;
}

/** Check local documents at the desktop and phone widths used by the authoring CLI. */
export async function renderCheck(
	root: string,
	files: readonly string[],
): Promise<RenderFinding[]> {
	const base = await realpath(root);
	const browser = await launchBrowser();
	try {
		const { server, origin } = await serve(base);
		try {
			const findings: RenderFinding[] = [];
			for (const file of files) findings.push(...(await inspectFile(browser, base, file, origin)));
			return findings;
		} finally {
			server.close();
		}
	} finally {
		await browser.close();
	}
}

async function inspectFile(
	browser: Browser,
	base: string,
	file: string,
	origin: string,
): Promise<RenderFinding[]> {
	const findings: RenderFinding[] = [];
	const physical = await realpath(resolve(file));
	if (!containsPath(base, physical))
		throw new Error(`Render input is outside the current directory: ${file}`);
	const path = relative(base, physical).split(sep).map(encodeURIComponent).join('/');
	const address = new URL(path, `${origin}/`);
	for (const width of widths) {
		const page = await browser.newPage({ viewport: { width, height: 900 } });
		const messages = new Set<string>();
		const recordConsole = (message: ConsoleMessage): void => {
			const content = message.text();
			if (content.startsWith('[demistify]')) messages.add(content.replace('[demistify] ', ''));
			else if (message.type() === 'error') messages.add(`console error: ${content}`);
		};
		const recordPageError = (error: Error): void => {
			messages.add(`page error: ${error.message}`);
		};
		page.on('console', recordConsole);
		page.on('pageerror', recordPageError);
		try {
			const inspection = await inspectPage(page, address);
			for (const message of inspection.messages) messages.add(message);
			// Linked pages are loaded to resolve rendered fragments. Only requested pages own diagnostics.
			page.off('console', recordConsole);
			page.off('pageerror', recordPageError);
			for (const message of await inspectLinks(inspection, address, page, base))
				messages.add(message);
		} catch (error) {
			messages.add(error instanceof Error ? error.message : String(error));
		} finally {
			await page.close();
		}
		for (const message of messages) findings.push({ file, width, message });
	}
	return findings;
}
