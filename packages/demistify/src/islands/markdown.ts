import { reportIssue } from '../core/issues.ts';
import { readIslandSource } from '../core/source-text.ts';
import { installIslandInspector } from './inspector.ts';

/** Prose, headings, lists, tables and code; a mermaid fence becomes a diagram island. */
export class MarkdownIsland extends HTMLElement {
	private renderPromise: Promise<void> | undefined;
	connectedCallback(): void {
		if (this.hasAttribute('prepared')) {
			installIslandInspector(this);
			return;
		}
		this.renderPromise ??= this.renderContent();
	}
	private async renderContent(): Promise<void> {
		if (this.hasAttribute('prepared')) return;
		const sourceText = readIslandSource(this);
		if (sourceText === undefined) {
			this.setAttribute('prepared', '');
			installIslandInspector(this);
			return;
		}
		try {
			const { renderMarkdownContent } = await import('../render/markdown-content.ts');
			this.innerHTML = renderMarkdownContent(sourceText);
			this.setAttribute('prepared', '');
			installIslandInspector(this);
			this.dispatchEvent(new Event('island-ready', { bubbles: true }));
		} catch (error) {
			this.textContent = `Markdown could not render: ${String(error)}`;
			reportIssue(this, `Markdown could not render: ${String(error)}`);
		}
	}
}
