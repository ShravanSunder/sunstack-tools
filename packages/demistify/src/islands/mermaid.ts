import { reportIssue } from '../core/issues.ts';
import { normalizeIslandText, readIslandSource } from '../core/source-text.ts';
import { renderThemedDiagram } from '../render/diagram.ts';
import { installIslandInspector, openIsland } from './inspector.ts';

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
