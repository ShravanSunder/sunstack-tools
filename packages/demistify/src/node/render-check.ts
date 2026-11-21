/* oxlint-disable no-await-in-loop -- sequential navigation shares one page, viewport, and issue stream. */
import { realpath } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { join, resolve } from 'node:path';

import { containsPath, localPageUrl, localSiteFile, serveLocalSite } from './local-site.ts';

interface Browser {
	newPage(options: { viewport: { width: number; height: number } }): Promise<Page>;
	close(): Promise<void>;
}

interface ConsoleMessage {
	type(): string;
	text(): string;
}

interface TabControl {
	evaluate<Result>(script: (element: Element) => Result): Promise<Result>;
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
	locator(selector: string): { elementHandles(): Promise<TabControl[]> };
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

async function inspectPage(page: Page, address: URL): Promise<PageInspection> {
	const response = await page.goto(address.href, { waitUntil: 'domcontentloaded', timeout: 15000 });
	if (!response?.ok())
		throw new Error(`HTTP ${response?.status() ?? 'unavailable'} loading ${address.pathname}.`);

	const messages = new Set<string>();
	const links = new Set<string>();
	const fragments = new Set<string>();

	async function collectVisibleState(): Promise<void> {
		await settleVisibleState(page);
		const visible = await inspectVisibleState(page);
		for (const message of visible.messages) messages.add(message);
		for (const link of visible.links) links.add(link);
		for (const fragment of visible.fragments) fragments.add(fragment);
	}

	await collectVisibleState();
	// Use a fixed snapshot so activation cannot add more tabs to this inspection.
	const tabs = await page.locator('[role="tab"]').elementHandles();
	for (const tab of tabs) {
		const activated = await tab.evaluate((element): boolean => {
			if (
				!(element instanceof HTMLElement) ||
				!element.getClientRects().length ||
				getComputedStyle(element).visibility === 'hidden' ||
				element.getAttribute('aria-selected') === 'true'
			)
				return false;
			element.click();
			return true;
		});
		if (activated) await collectVisibleState();
	}

	return { messages: [...messages], links: [...links], fragments: [...fragments] };
}

async function settleVisibleState(page: Page): Promise<void> {
	// Scroll to activate lazy islands in the current panel.
	await page.evaluate(async (): Promise<void> => {
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
		window.scrollTo(0, 0);
		await frame();
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
}

async function inspectVisibleState(page: Page): Promise<PageInspection> {
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
			const file = await localSiteFile(base, target);
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
		const { server, origin } = await serveLocalSite(base);
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
	const address = localPageUrl(base, physical, origin);
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
