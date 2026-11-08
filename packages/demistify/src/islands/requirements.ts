import { reportIssue } from '../core/issues.ts';
import { registerReviewSource, type ReviewSourceResult } from '../core/review-document.ts';
import type { DocumentReaderController } from '../design-documents/mount-reader.ts';

/** Register before asynchronous validation so failed input still has a Comments identity. */
export class RequirementsIsland extends HTMLElement {
	protected readonly readerKind: 'requirements' | 'call-stack' = 'requirements';
	private controller: DocumentReaderController | undefined;
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
				const { mountDocumentReader } = await import('../design-documents/mount-reader.ts');
				const mounted = mountDocumentReader(this, this.readerKind);
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
