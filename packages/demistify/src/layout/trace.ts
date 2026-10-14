/**
 * Hover and focus trace connections: pointing at a box lights up its arrows, pointing at a diagram
 * node lights up its edges and neighbours, and everything else in that picture dims.
 */
const traceable = '[data-from][data-to]';

export function setTrace(container: Element, id: string | undefined): void {
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
	for (const node of svg.querySelectorAll<SVGElement>('g[data-id]')) {
		const id = node.dataset['id'];
		const show = (): void => setTrace(svg, id);
		const clear = (): void => setTrace(svg, undefined);
		node.addEventListener('pointerenter', show);
		node.addEventListener('pointerleave', clear);
		node.addEventListener('focus', show);
		node.addEventListener('blur', clear);
	}
}

let tracedId: string | undefined;

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
	if (id === tracedId) return;
	tracedId = id;
	for (const overlay of document.querySelectorAll('.ex-arrow-canvas, .ex-flow-overlay'))
		setTrace(overlay, id);
}

/** Boxes trace the arrows drawn between them, wherever those arrows are drawn. */
export function installArrowTracing(): void {
	document.addEventListener('pointerover', (event): void =>
		traceArrowsOf(endpointOf(event.target)),
	);
	document.addEventListener('focusin', (event): void => traceArrowsOf(endpointOf(event.target)));
}
