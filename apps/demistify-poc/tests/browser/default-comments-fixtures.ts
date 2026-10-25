import type { ViteDevServer } from 'vite';

import type { GraphDocument } from '../../../../packages/demistify/src/explorer/graph-document.ts';

type SourceKind = 'markdown' | 'graph';
interface SourceGate {
	readonly ready: Promise<void>;
	release(): void;
	requests: number;
	sent: boolean;
}
interface ReviewFixture {
	readonly markdown: SourceGate;
	readonly graph: SourceGate;
	readonly variant: string;
	readonly mode: string;
}
function sourceGate(): SourceGate {
	let release: (() => void) | undefined;
	const ready = new Promise<void>((resolve): void => {
		release = resolve;
	});
	return {
		ready,
		release: (): void => {
			if (!release) throw new Error('Source gate is not ready.');
			release();
		},
		requests: 0,
		sent: false,
	};
}
function graphSource(changed: boolean): GraphDocument {
	return {
		version: 1,
		sourceBase: 'https://example.test/',
		title: changed ? 'Revised pilot' : 'Pilot',
		description: 'One workflow and its originating team.',
		note: 'Illustrative source.',
		groups: [{ id: 'phases', label: 'Phases', kind: 'phase' }],
		nodes: [
			{
				id: 'pilot',
				label: 'Pilot',
				description: 'One customer.',
				group: 'phases',
				issue: '',
				status: 'current',
				sources: [],
			},
		],
		edges: [],
		tables: [],
	};
}
function fixtureHtml(mode: string): string {
	const attributes =
		mode === 'off'
			? ' comments="off"'
			: mode === 'explicit'
				? ' document-id="authored-page" revision="authored-r1"'
				: mode === 'identity'
					? ' document-id="authored-page"'
					: mode === 'revision'
						? ' revision="authored-r1"'
						: '';
	const surface =
		mode === 'double'
			? '<ex-comments for="content"></ex-comments><ex-comments for="content"></ex-comments>'
			: mode === 'manual-off'
				? '<ex-comments for="content"></ex-comments>'
				: '';
	return `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Source review</title><link rel="stylesheet" href="/__demistify/library/theme.css"><script type="module" src="/__demistify/library/runtime.js"></script></head><body><ex-document id="content"${mode === 'manual-off' ? ' comments="off"' : attributes}><h1>Source review</h1><p>One source, one review.</p><ex-markdown src="notes.md"></ex-markdown><ex-explorer src="graph.json"></ex-explorer></ex-document>${surface}</body></html>`;
}

/** Real HTTP loads pause at explicit gates; the production identity and fetch paths stay real. */
export function installDefaultCommentsFixtures(server: ViteDevServer): void {
	const fixtures = new Map<string, ReviewFixture>();
	server.middlewares.use('/__demistify/default-comments', (request, response, next): void => {
		const url = new URL(request.url ?? '/', 'http://fixture.test');
		response.setHeader('Cache-Control', 'no-store');
		if (url.pathname === '/control') {
			const caseId = url.searchParams.get('case') ?? '';
			const action = url.searchParams.get('action');
			if (action === 'prepare') {
				fixtures.set(caseId, {
					markdown: sourceGate(),
					graph: sourceGate(),
					variant: url.searchParams.get('variant') ?? '',
					mode: url.searchParams.get('mode') ?? '',
				});
			}
			const fixture = fixtures.get(caseId);
			if (!fixture) {
				response.writeHead(404).end();
				return;
			}
			if (action === 'markdown' || action === 'graph') fixture[action].release();
			if (action === 'release') {
				fixture.markdown.release();
				fixture.graph.release();
			}
			response.setHeader('Content-Type', 'application/json');
			response.end(
				JSON.stringify({
					markdown: fixture.markdown.requests,
					graph: fixture.graph.requests,
					markdownSent: fixture.markdown.sent,
					graphSent: fixture.graph.sent,
				}),
			);
			return;
		}
		const [, caseId, file] = url.pathname.split('/');
		const fixture = caseId ? fixtures.get(caseId) : undefined;
		if (!fixture) {
			next();
			return;
		}
		if (file === 'index.html') {
			response.setHeader('Content-Type', 'text/html');
			response.end(fixtureHtml(fixture.mode));
			return;
		}
		const kind: SourceKind | undefined =
			file === 'notes.md' ? 'markdown' : file === 'graph.json' ? 'graph' : undefined;
		if (!kind) {
			next();
			return;
		}
		const gate = fixture[kind];
		gate.requests++;
		void gate.ready.then((): void => {
			gate.sent = true;
			if (kind === 'markdown' && fixture.variant === 'failed') {
				response.writeHead(503).end('Unavailable');
				return;
			}
			response.setHeader(
				'Content-Type',
				kind === 'markdown' ? 'text/markdown' : 'application/json',
			);
			response.end(
				kind === 'markdown'
					? `## Notes\n\n${fixture.variant === 'markdown' ? 'Changed source.' : 'Original source.'}`
					: JSON.stringify(graphSource(fixture.variant === 'graph')),
			);
		});
	});
}
