import { renderThemedDiagram } from './diagram-render.ts';
import { installIslandInspector, openIsland } from './island-inspector.ts';
import { reportIssue } from './issues.ts';
import { normalizeIslandText } from './source-text.ts';

/** Reads island source from a script or template child, or plain text when the author wrote no wrapper. */
function readIslandSource(owner: HTMLElement): string | undefined {
	const wrapper = owner.querySelector(':scope > script, :scope > template');
	if (wrapper instanceof HTMLTemplateElement) return wrapper.content.textContent ?? '';
	if (wrapper) return wrapper.textContent ?? '';
	return owner.children.length === 0 ? (owner.textContent ?? '') : undefined;
}

/** Renders diagram text closer to body size; the SVG still never exceeds its island. */
function enlargeDiagram(owner: HTMLElement): void {
	const svg = owner.querySelector(':scope > svg');
	const width = Number.parseFloat(svg?.getAttribute('width') ?? '');
	if (svg instanceof SVGSVGElement && width > 0 && width < 720)
		svg.style.width = `${Math.round(Math.min(width * 1.25, 720))}px`;
}

function openLinkedElement(target: HTMLElement): void {
	if (target.querySelector(':scope > .ex-inspect-controls')) {
		openIsland(target, 'sidebar');
		return;
	}
	target.scrollIntoView({ behavior: 'smooth', block: 'center' });
	target.focus({ preventScroll: true });
}

/** A diagram node whose id matches an element on the page takes that element's tone and opens it. */
function linkDiagramNodes(owner: HTMLElement): void {
	for (const node of owner.querySelectorAll<SVGGElement>('g.node[data-id]')) {
		const target = document.getElementById(node.dataset['id'] ?? '');
		if (!target || owner.contains(target)) continue;
		const tone = target.getAttribute('tone');
		if (tone) node.setAttribute('tone', tone);
		node.classList.add('ex-linked-node');
		node.setAttribute('tabindex', '0');
		node.setAttribute('role', 'button');
		node.setAttribute(
			'aria-label',
			`Open ${target.getAttribute('label') ?? node.dataset['label'] ?? ''}`,
		);
		node.addEventListener('click', (): void => openLinkedElement(target));
		node.addEventListener('keydown', (event: KeyboardEvent): void => {
			if (event.key !== 'Enter' && event.key !== ' ') return;
			event.preventDefault();
			openLinkedElement(target);
		});
	}
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
		this.visibilityObserver?.disconnect();
		this.visibilityObserver = new IntersectionObserver(
			(entries): void => {
				if (!entries.some((entry): boolean => entry.isIntersecting)) return;
				this.visibilityObserver?.disconnect();
				this.renderPromise ??= this.renderDiagram();
			},
			{ rootMargin: '300px 0px' },
		);
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
			this.innerHTML = renderThemedDiagram(renderMermaidSVG, normalizeIslandText(sourceText));
			this.setAttribute('prepared', '');
			enlargeDiagram(this);
			linkDiagramNodes(this);
			installIslandInspector(this);
			this.dispatchEvent(new Event('island-ready', { bubbles: true }));
		} catch (error) {
			this.textContent = `Mermaid could not render: ${String(error)}`;
			reportIssue(this, `Mermaid could not render: ${String(error)}`);
		}
	}
}
