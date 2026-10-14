import { adoptBareMarkdown } from '../core/bare-markdown.ts';
import { createIconButton } from '../core/icons.ts';
import { installIslandInspector } from '../islands/inspector.ts';
import { BoardIsland } from './board.ts';

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
		const targetLink = event.target.closest('a');
		const href = targetLink?.getAttribute('href');
		if (!href?.startsWith('#')) return;
		const targetElement = document.getElementById(href.slice(1));
		if (targetElement) revealElement(targetElement);
	});
}

/** Selects the tab and opens the details that hide an element, so it can be shown. */
export function revealElement(target: Element): void {
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
