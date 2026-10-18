/** Copy rendered islands without moving their live document nodes or duplicating their IDs. */
const copyFactories = new WeakMap<HTMLElement, () => HTMLElement>();

/** Stateful islands retain their own reconstruction boundary. */
export function registerInspectionCopy(owner: HTMLElement, createCopy: () => HTMLElement): void {
	copyFactories.set(owner, createCopy);
}

function copyNode(source: Node): Node {
	if (source instanceof HTMLElement) {
		const factory = copyFactories.get(source);
		if (factory) return factory();
	}
	const copy = source.cloneNode(false);
	if (source instanceof HTMLTemplateElement && copy instanceof HTMLTemplateElement) {
		copy.content.append(source.content.cloneNode(true));
		return copy;
	}
	for (const child of source.childNodes) {
		if (child instanceof Element && child.matches('.ex-inspect-controls')) continue;
		copy.appendChild(copyNode(child));
	}
	if (source instanceof HTMLInputElement && copy instanceof HTMLInputElement) {
		copy.value = source.value;
		copy.checked = source.checked;
	}
	if (source instanceof HTMLTextAreaElement && copy instanceof HTMLTextAreaElement)
		copy.value = source.value;
	if (source instanceof HTMLSelectElement && copy instanceof HTMLSelectElement)
		copy.value = source.value;
	return copy;
}

export function copyInspectionContent(owner: HTMLElement): HTMLElement | undefined {
	const copy = copyNode(owner);
	if (!(copy instanceof HTMLElement)) return undefined;
	copy.setAttribute('data-inspector-copy', '');
	const elements = [copy, ...copy.querySelectorAll('*')];
	const prefix = `ex-inspection-${crypto.randomUUID()}-`;
	const ids = new Map<string, string>();
	for (const element of elements) {
		if (!element.id) continue;
		const nextId = `${prefix}${element.id}`;
		ids.set(element.id, nextId);
		element.id = nextId;
	}
	for (const element of elements) {
		// Cloning markup does not clone listeners: each island reconnects its rendered controls.
		element.classList.remove('ex-linked-node', 'ex-traced', 'ex-route-focus');
		element.removeAttribute('data-traced');
		for (const attribute of Array.from(element.attributes)) {
			let value = attribute.value;
			if (
				(element.localName === 'ex-compare' && attribute.name === 'selected') ||
				['for', 'aria-controls', 'aria-labelledby', 'aria-describedby', 'from', 'to'].includes(
					attribute.name,
				)
			)
				value = value
					.split(/\s+/)
					.map((id): string => ids.get(id) ?? id)
					.join(' ');
			else if (['href', 'xlink:href'].includes(attribute.name) && value.startsWith('#'))
				value = `#${ids.get(value.slice(1)) ?? value.slice(1)}`;
			value = value.replace(
				/url\(#([^)]*)\)/g,
				(_match: string, id: string): string => `url(#${ids.get(id) ?? id})`,
			);
			if (value !== attribute.value) element.setAttribute(attribute.name, value);
		}
	}
	return copy;
}
