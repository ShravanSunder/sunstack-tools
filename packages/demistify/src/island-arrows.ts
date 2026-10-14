type IslandAnchor = 'left' | 'right' | 'top' | 'bottom';
interface ArrowPoint {
	readonly x: number;
	readonly y: number;
}
const activeArrows = new Set<ArrowIsland>();
let redrawFrame: number | undefined;

function scheduleArrows(): void {
	if (redrawFrame !== undefined) return;
	redrawFrame = requestAnimationFrame((): void => {
		redrawFrame = undefined;
		for (const arrow of activeArrows) arrow.drawConnection();
	});
}

function anchorPoint(bounds: DOMRect, anchor: IslandAnchor): ArrowPoint {
	return {
		x:
			anchor === 'left'
				? bounds.left
				: anchor === 'right'
					? bounds.right
					: bounds.left + bounds.width / 2,
		y:
			anchor === 'top'
				? bounds.top
				: anchor === 'bottom'
					? bounds.bottom
					: bounds.top + bounds.height / 2,
	};
}

function validAnchor(value: string): IslandAnchor {
	return value === 'left' || value === 'right' || value === 'top' || value === 'bottom'
		? value
		: 'right';
}

export class ArrowIsland extends HTMLElement {
	private overlay: SVGSVGElement | undefined;
	private path: SVGPathElement | undefined;
	private caption: SVGTextElement | undefined;
	private resizeObserver: ResizeObserver | undefined;
	private mutationObserver: MutationObserver | undefined;
	connectedCallback(): void {
		if (this.overlay) return;
		const markerId = `island-arrow-${crypto.randomUUID()}`;
		const overlay = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
		overlay.classList.add('ex-arrow-overlay');
		overlay.setAttribute('aria-hidden', 'true');
		overlay.innerHTML = `<defs><marker id="${markerId}" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M0,0 L8,4 L0,8 Z" fill="currentColor"/></marker></defs><path class="ex-arrow-line" marker-end="url(#${markerId})"/><text class="ex-arrow-label"/>`;
		this.overlay = overlay;
		this.path = overlay.querySelector<SVGPathElement>('.ex-arrow-line') ?? undefined;
		this.caption = overlay.querySelector<SVGTextElement>('.ex-arrow-label') ?? undefined;
		document.body.append(overlay);
		activeArrows.add(this);
		this.resizeObserver = new ResizeObserver(scheduleArrows);
		for (const name of ['from', 'to']) {
			const endpoint = document.getElementById(this.getAttribute(name) ?? '');
			if (endpoint) this.resizeObserver.observe(endpoint);
		}
		this.mutationObserver = new MutationObserver(scheduleArrows);
		if (this.parentElement)
			this.mutationObserver.observe(this.parentElement, {
				subtree: true,
				attributes: true,
				attributeFilter: ['style', 'class', 'hidden'],
				childList: true,
			});
		if (activeArrows.size === 1) {
			document.addEventListener('scroll', scheduleArrows, { capture: true, passive: true });
			window.addEventListener('resize', scheduleArrows, { passive: true });
			document.addEventListener('island-ready', scheduleArrows);
		}
		scheduleArrows();
	}
	disconnectedCallback(): void {
		activeArrows.delete(this);
		this.overlay?.remove();
		this.overlay = undefined;
		this.resizeObserver?.disconnect();
		this.mutationObserver?.disconnect();
		if (activeArrows.size === 0) {
			document.removeEventListener('scroll', scheduleArrows, true);
			window.removeEventListener('resize', scheduleArrows);
			document.removeEventListener('island-ready', scheduleArrows);
		}
	}
	drawConnection(): void {
		const source = document.getElementById(this.getAttribute('from') ?? '');
		const target = document.getElementById(this.getAttribute('to') ?? '');
		if (!source || !target || !this.overlay || !this.path || !this.caption) return;
		const sourceDialog = source.closest('dialog');
		const targetDialog = target.closest('dialog');
		const portal = sourceDialog && sourceDialog === targetDialog ? sourceDialog : document.body;
		if (this.overlay.parentElement !== portal) portal.append(this.overlay);
		const sourceBounds = source.getBoundingClientRect();
		const targetBounds = target.getBoundingClientRect();
		this.overlay.setAttribute(
			'visibility',
			sourceBounds.width && targetBounds.width ? 'visible' : 'hidden',
		);
		const horizontal =
			Math.abs(targetBounds.left - sourceBounds.left) >=
			Math.abs(targetBounds.top - sourceBounds.top);
		const forward = horizontal
			? targetBounds.left >= sourceBounds.left
			: targetBounds.top >= sourceBounds.top;
		const sourceAnchor =
			this.getAttribute('from-anchor') ??
			(horizontal ? (forward ? 'right' : 'left') : forward ? 'bottom' : 'top');
		const targetAnchor =
			this.getAttribute('to-anchor') ??
			(horizontal ? (forward ? 'left' : 'right') : forward ? 'top' : 'bottom');
		const start = anchorPoint(sourceBounds, validAnchor(sourceAnchor));
		const end = anchorPoint(targetBounds, validAnchor(targetAnchor));
		const bend = Math.min(100, Math.max(32, Math.hypot(end.x - start.x, end.y - start.y) / 3));
		const direction = forward ? 1 : -1;
		this.overlay.setAttribute('viewBox', `0 0 ${window.innerWidth} ${window.innerHeight}`);
		this.path.setAttribute(
			'd',
			horizontal
				? `M${start.x},${start.y} C${start.x + bend * direction},${start.y} ${end.x - bend * direction},${end.y} ${end.x},${end.y}`
				: `M${start.x},${start.y} C${start.x},${start.y + bend * direction} ${end.x},${end.y - bend * direction} ${end.x},${end.y}`,
		);
		this.caption.textContent = this.getAttribute('label') ?? '';
		this.caption.setAttribute('x', String((start.x + end.x) / 2));
		this.caption.setAttribute('y', String((start.y + end.y) / 2 - 8));
	}
}
