import { readFile } from 'node:fs/promises';
/* oxlint-disable no-await-in-loop -- pages are checked one at a time in one browser to keep memory low. */
/**
 * Loads pages in headless Chromium at desktop and phone width and collects the issues the runtime
 * reports (arrow routes, overflow, diagrams, references), the checks that need real layout.
 */
import { createServer } from 'node:http';
import { extname, relative, resolve, sep } from 'node:path';

const contentTypes: Readonly<Record<string, string>> = {
	'.html': 'text/html',
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
interface Page {
	// oxlint-disable-next-line id-length -- Playwright's method name.
	on(event: 'console', listener: (message: { type(): string; text(): string }) => void): void;
	// oxlint-disable-next-line id-length -- Playwright's method name.
	on(event: 'pageerror', listener: (error: Error) => void): void;
	goto(url: string, options: { waitUntil: 'networkidle' }): Promise<unknown>;
	evaluate<Result>(script: () => Promise<Result> | Result): Promise<Result>;
	waitForTimeout(milliseconds: number): Promise<void>;
	close(): Promise<void>;
}

function hasChromium(value: unknown): value is { chromium: { launch(): Promise<Browser> } } {
	return typeof value === 'object' && value !== null && 'chromium' in value;
}

async function launchBrowser(): Promise<Browser> {
	for (const name of ['playwright', 'playwright-core']) {
		try {
			const module: unknown = await import(name);
			if (hasChromium(module)) return await module.chromium.launch();
		} catch {
			// Try the next package.
		}
	}
	throw new Error('--render needs Playwright: install "playwright" next to this package.');
}

export interface RenderFinding {
	readonly file: string;
	readonly width: string;
	readonly message: string;
}

/** Serves `root` and checks each file (paths under `root`) at desktop and phone width. */
export async function renderCheck(
	root: string,
	files: readonly string[],
): Promise<RenderFinding[]> {
	const base = resolve(root);
	const server = createServer((request, response): void => {
		const path = resolve(
			base,
			`.${decodeURIComponent(new URL(request.url ?? '/', 'http://local').pathname)}`,
		);
		if (path !== base && !path.startsWith(base + sep)) {
			response.writeHead(403).end();
			return;
		}
		readFile(path).then(
			(body): void => {
				response
					.writeHead(200, {
						'content-type': contentTypes[extname(path)] ?? 'application/octet-stream',
						'access-control-allow-origin': '*',
					})
					.end(body);
			},
			(): void => {
				response.writeHead(404).end();
			},
		);
	});
	await new Promise<void>((done): void => {
		server.listen(0, '127.0.0.1', done);
	});
	const address = server.address();
	const port = typeof address === 'object' && address ? address.port : 0;
	const browser = await launchBrowser();
	const findings: RenderFinding[] = [];
	try {
		for (const file of files) {
			const path = relative(base, resolve(file)).split(sep).join('/');
			for (const [width, size] of [
				['desktop', 1400],
				['phone', 390],
			] as const) {
				const page = await browser.newPage({ viewport: { width: size, height: 900 } });
				const messages = new Set<string>();
				page.on('console', (message): void => {
					const content = message.text();
					if (content.startsWith('[demistify]')) messages.add(content.replace('[demistify] ', ''));
					else if (message.type() === 'error') messages.add(`console error: ${content}`);
				});
				page.on('pageerror', (error): void => {
					messages.add(`page error: ${error.message}`);
				});
				await page.goto(`http://127.0.0.1:${port}/${path}`, { waitUntil: 'networkidle' });
				// Scroll so lazy islands render, and show every tab so hidden views are checked too.
				const overflow = await page.evaluate(async (): Promise<boolean> => {
					// oxlint-disable-next-line unicorn/consistent-function-scoping -- runs inside the page.
					const pause = (): Promise<void> =>
						new Promise((done): void => {
							setTimeout(done, 120);
						});
					for (let top = 0; top < document.body.scrollHeight; top += 600) {
						window.scrollTo(0, top);
						await pause();
					}
					for (const tab of document.querySelectorAll<HTMLElement>('[role="tab"]')) {
						tab.click();
						await pause();
					}
					window.scrollTo(0, 0);
					return document.documentElement.scrollWidth > window.innerWidth + 1;
				});
				await page.waitForTimeout(2600);
				if (overflow)
					messages.add('The page scrolls sideways; something is wider than the screen.');
				for (const message of messages) findings.push({ file, width, message });
				await page.close();
			}
		}
	} finally {
		await browser.close();
		server.close();
	}
	return findings;
}
