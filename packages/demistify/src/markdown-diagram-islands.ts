import { installIslandInspector } from './island-inspector.ts';
import { normalizeIslandText } from './source-text.ts';
import { removeSvgFontImports } from './svg-fonts.ts';

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
		const sourceText = this.querySelector('script[type="text/markdown"]')?.textContent ?? '';
		try {
			const { renderMarkdownContent } = await import('./markdown-content.ts');
			this.innerHTML = renderMarkdownContent(sourceText);
			this.setAttribute('prepared', '');
			installIslandInspector(this);
			this.dispatchEvent(new Event('island-ready', { bubbles: true }));
		} catch (error) {
			this.textContent = `Markdown could not render: ${String(error)}`;
		}
	}
}

export class MermaidIsland extends HTMLElement {
	private renderPromise: Promise<void> | undefined;
	private visibilityObserver: IntersectionObserver | undefined;
	connectedCallback(): void {
		if (this.hasAttribute('prepared')) {
			installIslandInspector(this);
			return;
		}
		if (this.renderPromise) return;
		this.visibilityObserver?.disconnect();
		this.visibilityObserver = new IntersectionObserver((entries): void => {
			if (!entries.some((entry): boolean => entry.isIntersecting)) return;
			this.visibilityObserver?.disconnect();
			this.renderPromise ??= this.renderDiagram();
		});
		this.visibilityObserver.observe(this);
	}
	disconnectedCallback(): void {
		this.visibilityObserver?.disconnect();
	}
	private async renderDiagram(): Promise<void> {
		if (this.hasAttribute('prepared')) return;
		const sourceText =
			this.querySelector('script[type="text/plain"]')?.textContent ??
			this.querySelector<HTMLTemplateElement>('template')?.content.textContent ??
			'';
		try {
			const { renderMermaidSVG } = await import('beautiful-mermaid');
			this.innerHTML = removeSvgFontImports(
				renderMermaidSVG(normalizeIslandText(sourceText), {
					bg: 'var(--sheet)',
					fg: 'var(--ink)',
					accent: 'var(--accent)',
					surface: 'var(--accent-soft)',
					border: 'var(--rule)',
					font: 'IBM Plex Sans Condensed',
					transparent: true,
				}),
			);
			this.setAttribute('prepared', '');
			installIslandInspector(this);
			this.dispatchEvent(new Event('island-ready', { bubbles: true }));
		} catch (error) {
			this.textContent = `Mermaid could not render: ${String(error)}`;
		}
	}
}
