import { reportIssue } from '../core/issues.ts';

/**
 * Layers readers switch on and off: any element with layer="name" belongs to that layer (cards,
 * arrows, diagrams, the author's own markup). One toggle per layer; a hidden layer's boxes leave
 * the layout and arrows to them disappear. `off="name"` starts a layer hidden.
 */
export class LayersElement extends HTMLElement {
	connectedCallback(): void {
		if (this.hasAttribute('prepared')) return;
		this.setAttribute('prepared', '');
		const names = Array.from(
			new Set(
				Array.from(document.querySelectorAll('[layer]'), (element): string =>
					(element.getAttribute('layer') ?? '').trim(),
				).filter(Boolean),
			),
		);
		if (names.length === 0) {
			reportIssue(this, 'ex-layers found no element with layer="…" to switch.');
			return;
		}
		const off = new Set((this.getAttribute('off') ?? '').split(/\s+/).filter(Boolean));
		for (const name of off)
			if (!names.includes(name)) reportIssue(this, `off="${name}" names no layer on this page.`);
		const label = document.createElement('span');
		label.className = 'ex-layers-label';
		label.textContent = this.getAttribute('label') ?? 'Show';
		this.replaceChildren(label);
		for (const name of names) {
			const button = document.createElement('button');
			button.type = 'button';
			button.dataset['layer'] = name;
			button.textContent = name;
			button.addEventListener('click', (): void =>
				setLayer(name, button.getAttribute('aria-pressed') !== 'true'),
			);
			this.append(button);
			setLayer(name, !off.has(name));
		}
	}
}

/** Shows or hides one layer everywhere on the page and syncs its toggles. */
export function setLayer(name: string, shown: boolean): void {
	for (const element of document.querySelectorAll(`[layer="${CSS.escape(name)}"]`))
		element.classList.toggle('ex-layer-off', !shown);
	for (const button of document.querySelectorAll<HTMLButtonElement>(
		`ex-layers button[data-layer="${CSS.escape(name)}"]`,
	))
		button.setAttribute('aria-pressed', String(shown));
}

/** Turns on any layer that hides the element, so a route or link can show it. */
export function revealLayers(target: Element): void {
	for (let element: Element | null = target; element; element = element.parentElement)
		if (element.classList.contains('ex-layer-off'))
			setLayer(element.getAttribute('layer') ?? '', true);
}
