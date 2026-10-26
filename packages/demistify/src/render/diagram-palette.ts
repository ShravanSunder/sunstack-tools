const identityKinds: readonly string[] = ['node', 'actor', 'class-node', 'entity', 'subgraph'];
const identitySelector = identityKinds.map((kind): string => `g.${kind}[data-id]`).join(',');
const identityMarkup = new RegExp(
	`<g class="(?:${identityKinds.join('|')})" data-id="([^"]+)"[^>]*>`,
	'g',
);

function slotForIdentity(slots: Map<string, number>, identity: string): number {
	let slot = slots.get(identity);
	if (slot === undefined) {
		slot = (slots.size % 6) + 1;
		slots.set(identity, slot);
	}
	return slot;
}

/** Apply the same identity colors to server-rendered and newly rendered SVG markup. */
export function colorDiagramMarkup(svg: string): string {
	const slots = new Map<string, number>();
	return svg.replace(identityMarkup, (group: string, identity: string): string => {
		if (/data-shape="state-(?:start|end)"/.test(group)) return group;
		return group.replace(/>$/, ` data-diagram-slot="${slotForIdentity(slots, identity)}">`);
	});
}

/** Prepared SVGs keep their original nodes, links and state while adopting current theme styling. */
export function colorPreparedDiagram(svg: SVGSVGElement): void {
	const slots = new Map<string, number>();
	for (const group of svg.querySelectorAll(identitySelector)) {
		if (/^state-(?:start|end)$/.test(group.getAttribute('data-shape') ?? '')) continue;
		const identity = group.getAttribute('data-id');
		if (identity) group.setAttribute('data-diagram-slot', String(slotForIdentity(slots, identity)));
	}
}
