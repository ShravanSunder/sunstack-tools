import { createIconButton } from '../core/icons.ts';

/** One local Details panel per card choice, with its own disclosure state. */
export class ChoiceDetailPanel {
	readonly layout = document.createElement('div');
	readonly options = document.createElement('div');
	private readonly panels = document.createElement('div');
	private current: { readonly panel: HTMLElement; readonly trigger: HTMLButtonElement } | undefined;

	constructor() {
		this.layout.className = 'ex-choice-detail-layout';
		this.options.className = 'ex-choice-detail-options';
		this.panels.className = 'ex-choice-detail-panels';
		this.layout.append(this.options, this.panels);
		this.layout.addEventListener('keydown', (event): void => {
			if (event.key !== 'Escape' || event.defaultPrevented || !this.current) return;
			event.preventDefault();
			event.stopPropagation();
			this.close(true);
		});
	}

	add(contents: HTMLElement, label: string, id: string): HTMLButtonElement {
		const panel = document.createElement('aside');
		panel.id = id;
		panel.className = 'ex-choice-detail-panel';
		const panelLabel = `Details for ${label}`;
		panel.setAttribute('aria-label', panelLabel);
		panel.dataset['commentLabel'] = panelLabel;
		panel.hidden = true;
		contents.hidden = false;
		contents.removeAttribute('slot');

		const header = document.createElement('header');
		header.className = 'ex-choice-detail-header';
		const title = document.createElement('strong');
		title.textContent = label;
		const close = createIconButton('x', 'Close details', (): void => this.close(true));
		close.classList.add('ex-choice-detail-close');
		header.append(title, close);
		panel.append(header, contents);
		this.panels.append(panel);

		const trigger = createIconButton('article', `Open details for ${label}`, (): void => {
			if (panel.hidden) this.open(panel, trigger);
			else this.close(false);
		});
		panel.addEventListener('choice-detail-reveal', (): void => this.open(panel, trigger));
		trigger.setAttribute('aria-controls', id);
		trigger.setAttribute('aria-expanded', 'false');
		return trigger;
	}

	private open(panel: HTMLElement, trigger: HTMLButtonElement): void {
		if (this.current?.panel === panel) return;
		this.close(false);
		panel.hidden = false;
		trigger.setAttribute('aria-expanded', 'true');
		this.current = { panel, trigger };
	}

	private close(restoreFocus: boolean): void {
		if (!this.current) return;
		const { panel, trigger } = this.current;
		this.current = undefined;
		panel.hidden = true;
		trigger.setAttribute('aria-expanded', 'false');
		if (restoreFocus) trigger.focus();
	}
}
