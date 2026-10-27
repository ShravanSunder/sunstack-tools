import { reportIssue } from '../core/issues.ts';
import { normalizeIslandText, readIslandSource } from '../core/source-text.ts';
import { assignDiagramFragments } from '../layout/target-resolution.ts';
import { traceDiagram } from '../layout/trace.ts';
import { renderDiagramSvg } from '../render/diagram-client.ts';
import { colorPreparedDiagram } from '../render/diagram-palette.ts';
import { hasGraphEdges } from '../render/graph-source.ts';
import { installIslandInspector, openIsland } from './inspector.ts';

/**
 * Renders diagram text closer to body size, and never shrinks it below a readable size: a diagram
 * that would have to shrink further (a long chain in a narrow column) scrolls sideways instead.
 */
function sizeDiagram(owner: HTMLElement): void {
	const svg = owner.querySelector(':scope > svg');
	const width = Number.parseFloat(svg?.getAttribute('width') ?? '');
	if (!(svg instanceof SVGSVGElement) || !(width > 0)) return;
	const preferredWidth = width < 720 ? Math.min(width * 1.25, 720) : width;
	let minimumTextSize = 16;
	for (const label of svg.querySelectorAll('text')) {
		const fontSize = Number.parseFloat(label.getAttribute('font-size') ?? '');
		if (fontSize > 0) minimumTextSize = Math.min(minimumTextSize, fontSize);
	}
	const readableWidth = Math.ceil(width * (16 / minimumTextSize));
	// CSS applies the floor against the current column width, including after a tab or drawer resize.
	svg.style.setProperty(
		'--diagram-width',
		`${Math.max(Math.round(preferredWidth), readableWidth)}px`,
	);
	svg.style.setProperty('--diagram-readable-width', `${readableWidth}px`);
}

function openLinkedElement(target: HTMLElement): void {
	// An island opens beside the diagram, so the reader keeps the picture in view.
	if (target.localName.startsWith('ex-')) {
		openIsland(target, 'sidebar');
		return;
	}
	target.scrollIntoView({ behavior: 'smooth', block: 'center' });
	target.focus({ preventScroll: true });
}

/** Reports diagram text the engine accepted but drew wrong. */
function checkDiagram(owner: HTMLElement, sourceText: string): void {
	for (const group of owner.querySelectorAll<SVGGElement>('g.subgraph[data-id]')) {
		if (Number.parseFloat(group.querySelector('rect')?.getAttribute('width') ?? '1') > 0) continue;
		reportIssue(
			owner,
			`Subgraph "${group.dataset['id'] ?? ''}" is empty: declare its nodes inside the subgraph block.`,
		);
	}
	if (hasGraphEdges(sourceText) && !owner.querySelector('.edge'))
		reportIssue(
			owner,
			'No arrows rendered: check the arrow syntax, e.g. a --> b or a -->|label| b.',
		);
}

/** A diagram node whose id matches an element on the page takes that element's tone and opens it. */
function linkDiagramNodes(owner: HTMLElement): void {
	for (const node of owner.querySelectorAll<SVGGElement>('g.node[data-id]')) {
		const target = document.getElementById(node.dataset['id'] ?? '');
		if (!target || owner.contains(target) || node.classList.contains('ex-linked-node')) continue;
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
		// A cold scoped fragment must find and reveal this lazy island before its SVG exists.
		assignDiagramFragments(this);
		if (this.hasAttribute('prepared')) {
			// Pre-rendered by the packager: link and trace the shipped SVG.
			assignDiagramFragments(this);
			sizeDiagram(this);
			linkDiagramNodes(this);
			const svg = this.querySelector(':scope > svg');
			if (svg instanceof SVGSVGElement) {
				colorPreparedDiagram(svg);
				traceDiagram(svg);
			}
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
			this.innerHTML = await renderDiagramSvg(normalizeIslandText(sourceText));
			this.setAttribute('prepared', '');
			assignDiagramFragments(this);
			sizeDiagram(this);
			checkDiagram(this, sourceText);
			linkDiagramNodes(this);
			const svg = this.querySelector(':scope > svg');
			if (svg instanceof SVGSVGElement) traceDiagram(svg);
			installIslandInspector(this);
			this.dispatchEvent(new Event('island-ready', { bubbles: true }));
		} catch (error) {
			this.textContent = `Mermaid could not render: ${String(error)}`;
			reportIssue(this, `Mermaid could not render: ${String(error)}`);
		}
	}
}
