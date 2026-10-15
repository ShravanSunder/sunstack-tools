import { readFile } from 'node:fs/promises';

import { createServer } from 'vite';
import type { ViteDevServer } from 'vite';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { planningViewsSchema } from '../../src/experiment-builders/planning-fixture.ts';

function renderedDocumentText(html: string): string {
	return html
		.replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, '')
		.replace(/<style\b[^>]*>[\s\S]*?<\/style>/gi, '')
		.replace(/<[^>]+>/g, ' ')
		.replace(/&#x([\da-f]+);/gi, (_match: string, digits: string): string =>
			String.fromCodePoint(Number.parseInt(digits, 16)),
		)
		.replace(/&#(\d+);/g, (_match: string, digits: string): string =>
			String.fromCodePoint(Number.parseInt(digits, 10)),
		)
		.replaceAll('&quot;', '"')
		.replaceAll('&apos;', "'")
		.replaceAll('&lt;', '<')
		.replaceAll('&gt;', '>')
		.replaceAll('&amp;', '&')
		.replace(/\s+/g, ' ');
}

let labServer: ViteDevServer | undefined;
let labOrigin: string;

beforeAll(async (): Promise<void> => {
	labServer = await createServer({
		server: { host: '127.0.0.1', port: 0, hmr: false, watch: null },
		logLevel: 'silent',
	});
	await labServer.listen();
	const serverAddress = labServer.httpServer?.address();
	if (!serverAddress || typeof serverAddress === 'string') {
		throw new Error('The lab server did not expose a local HTTP address.');
	}
	labOrigin = `http://127.0.0.1:${serverAddress.port}`;
});

afterAll(async (): Promise<void> => {
	await labServer?.close();
});

describe('local presentation lab', (): void => {
	for (const experimentId of [
		'visual-plan',
		'quick-json',
		'markdown-directives',
		'graph-diff',
		'html-islands',
	]) {
		it(`serves the ${experimentId} inspector and its independently rendered output`, async (): Promise<void> => {
			const inspectorResponse = await fetch(`${labOrigin}/experiments/${experimentId}.html`);
			expect(inspectorResponse.status).toBe(200);
			const inspectorHtml = await inspectorResponse.text();
			expect(inspectorHtml).toContain('Inspect authored source');
			expect(inspectorHtml).toContain(
				`src="${experimentId === 'html-islands' ? 'html-islands-live.html' : `${experimentId}-rendered.html`}"`,
			);
			expect(inspectorHtml).toContain('Costs and verified limitations');
			const outputResponse = await fetch(`${labOrigin}/experiments/${experimentId}-rendered.html`);
			expect(outputResponse.status).toBe(200);
			const outputHtml = await outputResponse.text();
			expect(outputHtml).toContain('planning');
			// vplan mounts React in the browser. This HTTP check covers its payload;
			// the real rendered MDX body is verified separately in Chrome.
			const visibleContent =
				experimentId === 'visual-plan' ? outputHtml : renderedDocumentText(outputHtml);
			const views = planningViewsSchema.parse(
				JSON.parse(await readFile('fixtures/planning-views.json', 'utf8')),
			);
			for (const view of Object.values(views)) {
				expect(visibleContent).toContain(view.title);
				for (const node of view.nodes) {
					expect(visibleContent, `${experimentId}: ${node.id}`).toContain(node.purpose);
					expect(visibleContent, `${experimentId}: ${node.id}`).toContain(node.boundary);
				}
			}
			expect(visibleContent).toContain('six planned nodes and six dependency relationships');
		});
	}

	it('serves the landing page and its real browser assets', async (): Promise<void> => {
		const pageResponse = await fetch(labOrigin, { signal: AbortSignal.timeout(5000) });
		expect(pageResponse.status).toBe(200);
		const pageHtml = await pageResponse.text();
		expect(pageHtml).toContain('Presentation lab');
		expect(pageHtml).toContain('/src/lab-entry.ts');
		expect(pageHtml).toContain('href="#source-report"');
		expect(pageHtml).toContain('<details id="source-report"');

		const entryResponse = await fetch(`${labOrigin}/src/lab-entry.ts`, {
			signal: AbortSignal.timeout(5000),
		});
		expect(entryResponse.status).toBe(200);
		expect(entryResponse.headers.get('content-type')).toContain('javascript');
		expect(await entryResponse.text()).toContain('source-report-text');

		const styleResponse = await fetch(`${labOrigin}/src/lab-styles.css`, {
			signal: AbortSignal.timeout(5000),
		});
		expect(styleResponse.status).toBe(200);
		expect(await styleResponse.text()).toContain('.experiment-list');
	});

	it('serves the supplied source report without rewriting it', async (): Promise<void> => {
		const sourceResponse = await fetch(`${labOrigin}/references/deep-research-report.md`, {
			signal: AbortSignal.timeout(5000),
		});
		expect(sourceResponse.status).toBe(200);
		const expectedSource = await readFile(
			new URL('../../references/deep-research-report.md', import.meta.url),
			'utf8',
		);
		expect(await sourceResponse.text()).toBe(expectedSource);
	});
});
