import { bindTabKeyboard, labelControl } from '../core/controls.ts';
import { readIcon } from '../core/head.ts';
import { installIslandInspector } from './inspector.ts';

export class TabsIsland extends HTMLElement {
	private bound = false;
	connectedCallback(): void {
		if (this.hasAttribute('data-inspector-content')) return;
		if (this.bound) return;
		this.bound = true;
		const existingBar = this.querySelector<HTMLElement>(':scope > .ex-tab-bar');
		const panels = Array.from(this.children).filter(
			(element): element is HTMLElement =>
				element instanceof HTMLElement &&
				element !== existingBar &&
				!element.classList.contains('ex-inspect-controls'),
		);
		const tabBar = existingBar ?? document.createElement('div');
		tabBar.className = 'ex-tab-bar';
		tabBar.setAttribute('role', 'tablist');
		const buttons = panels.map((panel, index): HTMLButtonElement => {
			const tabButton =
				existingBar?.querySelectorAll<HTMLButtonElement>('button')[index] ??
				document.createElement('button');
			tabButton.type = 'button';
			labelControl(
				tabButton,
				readIcon(panel) ?? 'file',
				panel.getAttribute('label') ?? `View ${index + 1}`,
			);
			tabButton.setAttribute('role', 'tab');
			panel.id ||= `ex-panel-${crypto.randomUUID()}`;
			tabButton.id = `${panel.id}-tab`;
			tabButton.setAttribute('aria-controls', panel.id);
			panel.setAttribute('role', 'tabpanel');
			panel.setAttribute('aria-labelledby', tabButton.id);
			tabButton.addEventListener('click', (): void => selectPanel(index));

			return tabButton;
		});
		function selectPanel(selectedIndex: number): void {
			panels.forEach((panel, index): void => {
				panel.hidden = index !== selectedIndex;
			});
			buttons.forEach((button, index): void => {
				button.setAttribute('aria-selected', String(index === selectedIndex));
				button.tabIndex = index === selectedIndex ? 0 : -1;
			});
		}
		bindTabKeyboard(tabBar);
		tabBar.append(...buttons);
		this.prepend(tabBar);
		selectPanel(
			Math.max(
				0,
				buttons.findIndex((button): boolean => button.getAttribute('aria-selected') === 'true'),
			),
		);
		if (this.hasAttribute('columns')) {
			let selected = 0;
			const fitsSideBySide = (): boolean =>
				this.clientWidth >= panels.length * 340 + (panels.length - 1) * 24;
			new ResizeObserver((): void => {
				const columns = fitsSideBySide();
				if (columns === (this.dataset['layout'] === 'columns')) return;
				if (columns)
					selected = Math.max(
						0,
						buttons.findIndex((button): boolean => button.getAttribute('aria-selected') === 'true'),
					);
				this.dataset['layout'] = columns ? 'columns' : 'tabs';
				panels.forEach((panel): void => {
					panel.setAttribute('role', columns ? 'region' : 'tabpanel');
					if (columns) panel.setAttribute('aria-label', panel.getAttribute('label') ?? '');
				});
				if (columns) for (const panel of panels) panel.hidden = false;
				else selectPanel(selected);
				this.dispatchEvent(new Event('island-ready', { bubbles: true }));
			}).observe(this);
		}
		installIslandInspector(this);
	}
}
