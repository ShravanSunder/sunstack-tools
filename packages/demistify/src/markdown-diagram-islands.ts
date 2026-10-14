import { installIslandInspector } from './island-inspector.ts';
import { reportIssue } from './issues.ts';
import { normalizeIslandText } from './source-text.ts';
import { removeSvgFontImports } from './svg-fonts.ts';

/** Reads island source from a script or template child, or plain text when the author wrote no wrapper. */
function readIslandSource(owner: HTMLElement): string | undefined {
	const wrapper = owner.querySelector(':scope > script, :scope > template');
	if (wrapper instanceof HTMLTemplateElement) return wrapper.content.textContent ?? '';
	if (wrapper) return wrapper.textContent ?? '';
	return owner.children.length === 0 ? (owner.textContent ?? '') : undefined;
}

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
			const { renderMarkdownContent } = await import('./markdown-content.ts');
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

export class MermaidIsland extends HTMLElement {
	private renderPromise: Promise<void> | undefined;
	private visibilityObserver: IntersectionObserver | undefined;
	connectedCallback(): void {
		if (this.hasAttribute('prepared')) {
			installIslandInspector(this);
			return;
		}
		if (this.renderPromise) return;
		if (!this.hasAttribute('zoom')) this.setAttribute('zoom', 'modal');
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
		const sourceText = readIslandSource(this) ?? '';
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
			reportIssue(this, `Mermaid could not render: ${String(error)}`);
		}
	}
}
