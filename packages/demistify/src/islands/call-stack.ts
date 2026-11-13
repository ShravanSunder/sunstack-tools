import { reportIssue } from '../core/issues.ts';
import { registerReviewSource, type ReviewSourceResult } from '../core/review-document.ts';
import type { CallStackController } from '../design-documents/mount-reader.ts';

/** Register before asynchronous validation so failed input still has a Comments identity. */
export class CallStackIsland extends HTMLElement {
	private controller: CallStackController | undefined;
	private loading = false;
	connectedCallback(): void {
		if (this.controller) {
			this.controller.resume();
			return;
		}
		if (this.loading || this.hasAttribute('data-inspector-content')) return;
		this.loading = true;
		this.setAttribute('aria-busy', 'true');
		const load: Promise<ReviewSourceResult> = Promise.resolve()
			.then(async (): Promise<ReviewSourceResult> => {
				const { mountCallStack } = await import('../design-documents/mount-reader.ts');
				const mounted = mountCallStack(this);
				this.controller = mounted.controller;
				this.setAttribute('data-ready', '');
				this.dispatchEvent(new CustomEvent('island-ready', { bubbles: true }));
				return { kind: 'loaded', source: mounted.source };
			})
			.catch((error: unknown): ReviewSourceResult => {
				const message =
					error instanceof Error ? error.message : 'Design collection could not load.';
				const diagnostic = document.createElement('p');
				diagnostic.setAttribute('role', 'alert');
				diagnostic.textContent = message;
				this.replaceChildren(diagnostic);
				reportIssue(this, message);
				return { kind: 'failed' };
			})
			.finally((): void => {
				this.loading = false;
				this.removeAttribute('aria-busy');
			});
		registerReviewSource(this, load);
	}
	disconnectedCallback(): void {
		this.controller?.dispose();
	}
}
