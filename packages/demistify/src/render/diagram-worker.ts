/**
 * Worker entry: renders diagram text off the page's main thread. The build emits it beside
 * runtime.js, and it shares the engine chunk that the main-thread fallback imports. The global
 * `addEventListener` and `postMessage` used here are the worker scope's.
 */
import { renderMermaidSVG } from 'beautiful-mermaid';

import { renderThemedDiagram } from './diagram.ts';

export interface DiagramRequest {
	readonly id: number;
	readonly sourceText: string;
}

export type DiagramReply =
	| { readonly kind: 'ready' }
	| { readonly kind: 'rendered'; readonly id: number; readonly svg: string }
	| {
			readonly kind: 'failed';
			readonly id: number;
			readonly name: string | undefined;
			readonly message: string;
	  };

// elkjs, the engine's layout library, turns into a worker of its own (and every render fails) when
// it sees `self` but no `document`. beautiful-mermaid deletes `self` and then reassigns it to
// prevent that, but a browser worker's `self` is an inherited getter: deleting it does nothing and
// reassigning it throws. A writable `self` and a placeholder `document` keep elkjs in-process, as
// on a page; the engine only tests `document` for presence.
Object.defineProperties(globalThis, {
	self: { value: globalThis, writable: true, configurable: true },
	document: { value: null, configurable: true },
});

function renderReply({ id, sourceText }: DiagramRequest): DiagramReply {
	try {
		return { kind: 'rendered', id, svg: renderThemedDiagram(renderMermaidSVG, sourceText) };
	} catch (error) {
		return {
			kind: 'failed',
			id,
			name: error instanceof Error ? error.name : undefined,
			message: error instanceof Error ? error.message : String(error),
		};
	}
}

addEventListener('message', (event: MessageEvent<DiagramRequest>): void => {
	postMessage(renderReply(event.data));
});
// A first render proves the engine works in this worker; if it throws, the worker never reports
// ready and the page renders its diagrams on the main thread instead.
renderThemedDiagram(renderMermaidSVG, 'flowchart LR\n  a --> b');
postMessage({ kind: 'ready' } satisfies DiagramReply);
