import type { DiagramReply, DiagramRequest } from './diagram-worker.ts';
import { renderThemedDiagram } from './diagram.ts';

/** scripts/build-library.ts emits the worker beside runtime.js, which holds this module. */
const workerModuleName = 'diagram-worker.js';

interface PendingRender {
	readonly sourceText: string;
	readonly resolve: (svg: string) => void;
	readonly reject: (reason: unknown) => void;
}

const pendingRenders = new Map<number, PendingRender>();
let nextRequestId = 1;
let diagramWorker: Promise<Worker | undefined> | undefined;

async function renderOnMainThread(sourceText: string): Promise<string> {
	const { renderMermaidSVG } = await import('beautiful-mermaid');
	return renderThemedDiagram(renderMermaidSVG, sourceText);
}

/** Rebuilds a worker failure so that `String(failure)` reads as it would on the main thread. */
function rebuildFailure(reply: Extract<DiagramReply, { kind: 'failed' }>): unknown {
	if (reply.name === undefined) return reply.message;
	const failure = new Error(reply.message);
	failure.name = reply.name;
	return failure;
}

function startWorker(): Promise<Worker | undefined> {
	return new Promise((resolve): void => {
		let worker: Worker;
		let scriptUrl: string;
		try {
			// Browsers refuse a cross-origin worker script, and the library may come from a CDN, so a
			// same-origin module does nothing but import the library's worker.
			const moduleUrl = new URL(workerModuleName, import.meta.url).href;
			scriptUrl = URL.createObjectURL(
				new Blob([`import ${JSON.stringify(moduleUrl)};`], { type: 'text/javascript' }),
			);
			worker = new Worker(scriptUrl, { type: 'module', name: 'demistify-diagrams' });
		} catch {
			resolve(undefined);
			return;
		}
		worker.addEventListener('message', (event: MessageEvent<DiagramReply>): void => {
			const reply = event.data;
			if (reply.kind === 'ready') {
				URL.revokeObjectURL(scriptUrl);
				resolve(worker);
				return;
			}
			const pending = pendingRenders.get(reply.id);
			if (!pending) return;
			pendingRenders.delete(reply.id);
			if (reply.kind === 'rendered') pending.resolve(reply.svg);
			else pending.reject(rebuildFailure(reply));
		});
		// The worker could not start (CSP, a file page, a failed import) or stopped: the main thread
		// renders this and every later diagram.
		worker.addEventListener('error', (): void => {
			worker.terminate();
			URL.revokeObjectURL(scriptUrl);
			diagramWorker = Promise.resolve(undefined);
			resolve(undefined);
			for (const [id, pending] of pendingRenders) {
				pendingRenders.delete(id);
				renderOnMainThread(pending.sourceText).then(pending.resolve, pending.reject);
			}
		});
	});
}

/** Renders diagram text to themed SVG in a module worker when the page allows one, else here. */
export async function renderDiagramSvg(sourceText: string): Promise<string> {
	diagramWorker ??= typeof Worker === 'function' ? startWorker() : Promise.resolve(undefined);
	const worker = await diagramWorker;
	if (!worker) return renderOnMainThread(sourceText);
	return new Promise((resolve, reject): void => {
		const id = nextRequestId;
		nextRequestId += 1;
		pendingRenders.set(id, { sourceText, resolve, reject });
		// oxlint-disable-next-line unicorn/require-post-message-target-origin -- a worker has no target origin.
		worker.postMessage({ id, sourceText } satisfies DiagramRequest);
	});
}
