import { GalleryIsland } from '../islands/gallery.ts';
import { BoardIsland } from './board.ts';
import { revealLayers } from './layers.ts';

/** All links and presentation stops reveal the same containing UI before selecting a target. */
export function revealElement(target: Element): void {
	revealLayers(target);
	for (
		let ancestor = target instanceof HTMLElement ? target : target.parentElement;
		ancestor;
		ancestor = ancestor.parentElement
	) {
		if (ancestor.classList.contains('ex-comparison-option'))
			ancestor.dispatchEvent(new Event('comparison-reveal', { bubbles: true }));
		// An alternative may itself be the destination; the other actions reveal its containers.
		if (ancestor === target) continue;
		if (ancestor.getAttribute('role') === 'tabpanel' && ancestor.hidden)
			ancestor.parentElement
				?.querySelector<HTMLButtonElement>(`button[aria-controls="${CSS.escape(ancestor.id)}"]`)
				?.click();
		if (ancestor instanceof HTMLDetailsElement) ancestor.open = true;
		if (ancestor instanceof BoardIsland) ancestor.focusOn(target);
	}
}

function targetElement(reference: string): Element | undefined {
	const exact = document.getElementById(reference);
	if (exact) return exact;
	for (const gallery of document.querySelectorAll('ex-gallery')) {
		if (gallery instanceof GalleryIsland) {
			const capture = gallery.revealCapture(reference);
			if (capture) return capture;
		}
	}
	const divider = reference.lastIndexOf('--');
	if (divider < 0) return undefined;
	const owner = document.getElementById(reference.slice(0, divider));
	if (owner?.localName !== 'ex-mermaid') return undefined;
	revealElement(owner);
	owner.scrollIntoView({ behavior: 'instant', block: 'center' });
	return (
		owner.querySelector(`g.node[data-id="${CSS.escape(reference.slice(divider + 2))}"]`) ??
		undefined
	);
}

export function resolveTarget(
	reference: string,
	signal?: AbortSignal,
): Promise<Element | undefined> {
	return new Promise((resolve): void => {
		const finish = (target: Element | undefined): void => {
			clearTimeout(deadline);
			document.removeEventListener('island-ready', attempt);
			signal?.removeEventListener('abort', cancel);
			resolve(target);
		};
		const cancel = (): void => finish(undefined);
		const attempt = (): void => {
			if (signal?.aborted) {
				finish(undefined);
				return;
			}
			const target = targetElement(reference);
			if (!target) return;
			revealElement(target);
			if (
				target.matches(
					'ex-mermaid:not([prepared]),ex-markdown:not([prepared]),ex-tree:not([prepared])',
				)
			) {
				target.scrollIntoView({ behavior: 'instant', block: 'center' });
				return;
			}
			finish(target);
		};
		document.addEventListener('island-ready', attempt);
		signal?.addEventListener('abort', cancel, { once: true });
		const deadline = setTimeout((): void => finish(undefined), 5000);
		attempt();
	});
}

/** Scoped node fragments work for ordinary links as well as presentation stops. */
export function assignDiagramFragments(owner: HTMLElement): void {
	if (!owner.id) {
		const markdown = owner.closest('ex-markdown[id]');
		if (markdown)
			owner.id = `${markdown.id}--diagram-${Array.from(markdown.querySelectorAll('ex-mermaid')).indexOf(owner) + 1}`;
	}
	if (!owner.id) return;
	for (const node of owner.querySelectorAll<SVGGElement>('g.node[data-id]'))
		node.id = `${owner.id}--${node.dataset['id'] ?? ''}`;
}
