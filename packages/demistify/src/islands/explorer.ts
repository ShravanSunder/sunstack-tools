import { assignHeadingFragments } from '../core/heading-fragments.ts';
import { reportIssue } from '../core/issues.ts';
import { registerReviewSource, type ReviewSourceResult } from '../core/review-document.ts';
import type { ExplorerController } from '../explorer/explorer-controller.ts';

/** Heavy graph/table UI loads only for documents that include the explorer island. */
export class ExplorerIsland extends HTMLElement {
	private controller: ExplorerController | undefined;
	private loading = false;
	private readonly loadError = document.createElement('p');
	connectedCallback(): void {
		if (this.controller) {
			this.controller.resume();
			return;
		}
		if (this.loading || this.hasAttribute('data-inspector-content')) return;
		this.loadError.remove();
		this.loading = true;
		this.setAttribute('aria-busy', 'true');
		const load = import('../explorer/explorer-controller.ts')
			.then(async ({ mountExplorer }): Promise<ReviewSourceResult> => {
				const mounted = await mountExplorer(this);
				this.controller = mounted.controller;
				assignHeadingFragments(this);
				performance.mark(`demistify:explorer:${this.id}:ready`);
				this.setAttribute('data-ready', '');
				this.removeAttribute('aria-busy');
				this.dispatchEvent(new CustomEvent('island-ready', { bubbles: true }));
				return { kind: 'loaded', source: mounted.source };
			})
			.catch((error: unknown): ReviewSourceResult => {
				this.removeAttribute('aria-busy');
				this.loadError.textContent = 'Explorer could not load.';
				this.loadError.setAttribute('role', 'status');
				this.append(this.loadError);
				reportIssue(this, error instanceof Error ? error.message : 'Explorer failed to load.');
				return { kind: 'failed' };
			})
			.finally((): void => {
				this.loading = false;
			});
		registerReviewSource(this, load);
	}
	/** Internal target-resolution delegate; no new authoring or navigation state. */
	revealTarget(target: Element): void {
		this.controller?.revealTarget(target);
	}
	disconnectedCallback(): void {
		this.controller?.dispose();
	}
}
