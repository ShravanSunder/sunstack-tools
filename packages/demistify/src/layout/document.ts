import { createIconButton } from '../core/icons.ts';
import { installIslandInspector } from '../islands/inspector.ts';

export class StructuralIsland extends HTMLElement {
	connectedCallback(): void {
		installIslandInspector(this);
	}
}
export class GridIsland extends HTMLElement {}
/** Bare text written between islands is Markdown; it becomes a Markdown island in place. */
function adoptBareMarkdown(container: Element): void {
	let run: Text[] = [];
	const flush = (): void => {
		const first = run[0];
		if (first && run.some((node): boolean => node.data.trim().length > 0)) {
			const script = document.createElement('script');
			script.type = 'text/markdown';
			script.textContent = run.map((node): string => node.data).join('');
			const island = document.createElement('ex-markdown');
			island.append(script);
			first.before(island);
			for (const node of run) node.remove();
		}
		run = [];
	};
	for (const node of Array.from(container.childNodes)) {
		if (node instanceof Text) run.push(node);
		else flush();
	}
	flush();
}

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
		if (!targetElement) return;
		for (let ancestor = targetElement.parentElement; ancestor; ancestor = ancestor.parentElement) {
			if (ancestor.getAttribute('role') === 'tabpanel') {
				const ownerTabs = ancestor.parentElement;
				ownerTabs
					?.querySelector<HTMLButtonElement>(`button[aria-controls="${CSS.escape(ancestor.id)}"]`)
					?.click();
			}
			if (ancestor instanceof HTMLDetailsElement) ancestor.open = true;
		}
	});
}
