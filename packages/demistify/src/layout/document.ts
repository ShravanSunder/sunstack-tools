import { adoptBareMarkdown } from '../core/bare-markdown.ts';
import { createIconButton } from '../core/icons.ts';
import { installIslandInspector, openIsland } from '../islands/inspector.ts';
import { BoardIsland } from './board.ts';
import { revealLayers } from './layers.ts';

export class StructuralIsland extends HTMLElement {
	connectedCallback(): void {
		installIslandInspector(this);
	}
}
export class GridIsland extends HTMLElement {}
export class DocumentIsland extends StructuralIsland {
	override connectedCallback(): void {
		adoptBareMarkdown(this);
		for (const panel of this.querySelectorAll(':scope ex-tabs > section')) adoptBareMarkdown(panel);
		super.connectedCallback();
		if (!document.querySelector('ex-nav')) {
			const navigation = document.createElement('ex-nav');
			this.id ||= 'ex-content';
			navigation.setAttribute('for', this.id);
			document.body.prepend(navigation);
		}
		// Layers get their toggles at the top of the page unless the author placed them.
		if (document.querySelector('[layer]') && !document.querySelector('ex-layers')) {
			const layers = document.createElement('ex-layers');
			const first = this.firstElementChild;
			if (first) first.after(layers);
			else this.prepend(layers);
		}
		if (!document.querySelector('[data-ex-theme]')) {
			const toggle = createIconButton('circle-half', 'Switch light or dark theme', (): void => {});
			toggle.dataset['exTheme'] = '';
			toggle.classList.add('ex-theme-toggle');
			document.body.append(toggle);
		}
	}
}

/** Theme toggling and in-page links that reveal their target inside tabs or details. */
export function installDocumentBehavior(): void {
	document.addEventListener('click', (event: MouseEvent): void => {
		if (!(event.target instanceof Element)) return;
		if (event.target.closest('[data-ex-theme]')) {
			const current =
				document.documentElement.dataset['theme'] ??
				(matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');
			document.documentElement.dataset['theme'] = current === 'dark' ? 'light' : 'dark';
			return;
		}
		// A picture opens in the side panel, beside the text that explains it; a figure travels with
		// its caption. The panel's expand button gives the large view.
		const image = event.target.closest('img');
		if (
			image?.closest('ex-document') &&
			!image.closest('a, .ex-drawer, dialog, ex-compare[slider], [data-inspector-content]')
		) {
			const owner = image.closest('figure') ?? image.parentElement;
			if (owner instanceof HTMLElement) openIsland(owner, 'sidebar');
			return;
		}
		const targetLink = event.target.closest('a');
		const href = targetLink?.getAttribute('href');
		if (!href?.startsWith('#')) return;
		const targetElement = document.getElementById(href.slice(1));
		if (targetElement) revealElement(targetElement);
	});
}

/** Selects the tab and opens the details that hide an element, so it can be shown. */
export function revealElement(target: Element): void {
	revealLayers(target);
	for (let ancestor = target.parentElement; ancestor; ancestor = ancestor.parentElement) {
		if (ancestor.getAttribute('role') === 'tabpanel' && ancestor.hidden) {
			ancestor.parentElement
				?.querySelector<HTMLButtonElement>(`button[aria-controls="${CSS.escape(ancestor.id)}"]`)
				?.click();
		}
		if (ancestor instanceof HTMLDetailsElement) ancestor.open = true;
		if (ancestor instanceof BoardIsland) ancestor.focusOn(target);
	}
}
