import { reportIssue } from '../core/issues.ts';
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
		void import('../explorer/explorer-controller.ts')
			.then(async ({ mountExplorer }): Promise<void> => {
				this.controller = await mountExplorer(this);
			})
			.catch((error: unknown): void => {
				this.removeAttribute('aria-busy');
				this.loadError.textContent = 'Explorer could not load.';
				this.loadError.setAttribute('role', 'status');
				this.append(this.loadError);
				reportIssue(this, error instanceof Error ? error.message : 'Explorer failed to load.');
			})
			.finally((): void => {
				this.loading = false;
			});
	}
	disconnectedCallback(): void {
		this.controller?.dispose();
	}
}
