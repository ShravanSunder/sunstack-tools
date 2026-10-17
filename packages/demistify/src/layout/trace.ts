import { createDebouncedAction, type DebouncedAction } from '../core/debounce.ts';

/**
 * Hover and focus trace connections: pointing at a box lights up its arrows, pointing at a diagram
 * node lights up its edges and neighbours, and everything else in that picture dims.
 */
const traceable = '[data-from][data-to]';
const pendingDiagramHover = new WeakMap<Element, DebouncedAction>();

export function setTrace(container: Element, id: string | undefined): void {
	pendingDiagramHover.get(container)?.cancel();
	const items = Array.from(container.querySelectorAll<SVGElement | HTMLElement>(traceable));
	const touching = (item: SVGElement | HTMLElement): boolean =>
		id !== undefined && (item.dataset['from'] === id || item.dataset['to'] === id);
	const active = items.some(touching);
	container.classList.toggle('ex-tracing', active);
	const neighbours = new Set<string>();
	for (const item of items) {
		const lit = active && touching(item);
		item.classList.toggle('ex-traced', lit);
		if (lit) {
			neighbours.add(item.dataset['from'] ?? '');
			neighbours.add(item.dataset['to'] ?? '');
		}
	}
	for (const node of container.querySelectorAll<SVGElement>('[data-id]'))
		node.classList.toggle('ex-traced', active && neighbours.has(node.dataset['id'] ?? ''));
}

/** Diagram nodes trace their own edges inside one rendered diagram. */
export function traceDiagram(svg: SVGSVGElement): void {
	if (!svg.querySelector(traceable) || svg.hasAttribute('data-traced')) return;
	svg.setAttribute('data-traced', '');
	let hovered: SVGElement | undefined;
	const paintCurrent = (preferFocus = false): void => {
		if (!svg.isConnected || (hovered && !hovered.isConnected)) return;
		const focused = document.activeElement?.closest<SVGElement>('g[data-id]');
		const fallback =
			focused && svg.contains(focused)
				? focused
				: svg.querySelector<SVGElement>('g[data-id].ex-route-focus');
		const id = preferFocus
			? (fallback?.dataset['id'] ?? hovered?.dataset['id'])
			: (hovered?.dataset['id'] ?? fallback?.dataset['id']);
		setTrace(svg, id);
	};
	const pending = createDebouncedAction(paintCurrent, 100);
	pendingDiagramHover.set(svg, pending);
	const settleAction = (): void => {
		pending.cancel();
		paintCurrent(true);
	};
	svg.addEventListener('click', settleAction, true);
	svg.addEventListener('keydown', settleAction, true);
	for (const node of svg.querySelectorAll<SVGElement>('g[data-id]')) {
		const id = node.dataset['id'];
		const show = (): void => setTrace(svg, id);
		const clear = (): void => setTrace(svg, undefined);
		node.addEventListener('pointerenter', (): void => {
			hovered = node;
			pending.schedule();
		});
		node.addEventListener('pointerleave', (): void => {
			hovered = undefined;
			pending.schedule();
		});
		node.addEventListener('focus', show);
		node.addEventListener('blur', clear);
	}
}

/** The arrow endpoint the pointer or focus is in, if any. */
function endpointOf(target: EventTarget | null): string | undefined {
	if (!(target instanceof Element)) return undefined;
	const ends = new Set<string>();
	for (const arrow of document.querySelectorAll('ex-arrow'))
		for (const name of ['from', 'to']) ends.add(arrow.getAttribute(name) ?? '');
	for (let element: Element | null = target; element; element = element.parentElement)
		if (element.id && ends.has(element.id)) return element.id;
	return undefined;
}

function traceArrowsOf(id: string | undefined): void {
	for (const overlay of document.querySelectorAll('.ex-arrow-canvas, .ex-flow-overlay'))
		setTrace(overlay, id);
}

/** Boxes trace the arrows drawn between them, wherever those arrows are drawn. */
export function installArrowTracing(): void {
	let hoveredId: string | undefined;
	let hoveredTarget: Element | undefined;
	const paintCurrent = (preferFocus = false): void => {
		if (hoveredTarget && !hoveredTarget.isConnected) return;
		const focusedId = endpointOf(document.activeElement);
		traceArrowsOf(preferFocus ? (focusedId ?? hoveredId) : (hoveredId ?? focusedId));
	};
	const pending = createDebouncedAction(paintCurrent, 100);
	document.addEventListener('pointerover', (event): void => {
		const id = endpointOf(event.target);
		if (id === hoveredId) return;
		hoveredId = id;
		hoveredTarget = event.target instanceof Element ? event.target : undefined;
		pending.schedule();
	});
	document.documentElement.addEventListener('pointerleave', (): void => {
		hoveredId = undefined;
		hoveredTarget = undefined;
		pending.schedule();
	});
	document.addEventListener('focusin', (event): void => {
		pending.cancel();
		traceArrowsOf(endpointOf(event.target));
	});
	for (const event of ['click', 'keydown'])
		document.addEventListener(
			event,
			(): void => {
				pending.cancel();
				paintCurrent(true);
			},
			true,
		);
	for (const event of ['presentation-change', 'presentation-target'])
		document.addEventListener(
			event,
			(): void => {
				pending.cancel();
				traceArrowsOf(endpointOf(document.activeElement));
			},
			true,
		);
}
