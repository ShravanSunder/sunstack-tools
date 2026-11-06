import { icon, safeText } from './study-markup.js';

export interface SearchPickerOption {
	readonly value: string;
	readonly label: string;
	readonly detail?: string;
}
export interface SearchPickerConfig {
	readonly label: string;
	readonly placeholder: string;
	readonly icon: string;
	readonly options: readonly SearchPickerOption[];
	readonly selected: string;
}

/** One small picker owns focus and keyboard behavior; callers supply domain labels and choices. */
export class SearchPicker extends HTMLElement {
	private config: SearchPickerConfig = {
		label: 'Choose an item',
		placeholder: 'Search…',
		icon: 'list',
		options: [],
		selected: '',
	};
	private query = '';
	private events: AbortController | undefined;

	configure(config: SearchPickerConfig): void {
		this.config = config;
		if (this.isConnected) this.render();
	}

	connectedCallback(): void {
		this.events?.abort();
		this.events = new AbortController();
		const signal = this.events.signal;
		this.render();
		this.addEventListener(
			'toggle',
			(event): void => {
				if (!(event.target instanceof HTMLDetailsElement) || !event.target.open) return;
				if (document.activeElement === this.querySelector('summary'))
					this.querySelector<HTMLInputElement>('input')?.focus({ preventScroll: true });
			},
			{ signal, capture: true },
		);
		this.addEventListener(
			'input',
			(event): void => {
				if (!(event.target instanceof HTMLInputElement)) return;
				this.query = event.target.value;
				this.renderOptions();
			},
			{ signal },
		);
		this.addEventListener(
			'click',
			(event): void => {
				if (!(event.target instanceof Element)) return;
				const value = event.target.closest<HTMLElement>('button[data-picker-value]')?.dataset[
					'pickerValue'
				];
				if (!value || !this.config.options.some((option): boolean => option.value === value))
					return;
				this.close();
				this.dispatchEvent(
					new CustomEvent<{ value: string }>('picker-select', { detail: { value }, bubbles: true }),
				);
			},
			{ signal },
		);
		this.addEventListener(
			'keydown',
			(event): void => {
				if (event.key === 'Escape') {
					if (!this.querySelector('details')?.open) return;
					event.preventDefault();
					this.close();
					this.querySelector('summary')?.focus();
				} else if (event.key === 'Enter' && event.target instanceof HTMLInputElement) {
					event.preventDefault();
					this.querySelector<HTMLButtonElement>('button[data-picker-value]')?.click();
				} else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
					const details = this.querySelector('details');
					if (details) details.open = true;
					const buttons = [
						...this.querySelectorAll<HTMLButtonElement>('button[data-picker-value]'),
					];
					if (!buttons.length) return;
					event.preventDefault();
					const active = buttons.findIndex((button): boolean => button === document.activeElement);
					const next =
						active < 0
							? event.key === 'ArrowDown'
								? 0
								: buttons.length - 1
							: (active + (event.key === 'ArrowDown' ? 1 : -1) + buttons.length) % buttons.length;
					buttons[next]?.focus();
				}
			},
			{ signal },
		);
		document.addEventListener(
			'pointerdown',
			(event): void => {
				if (event.target instanceof Node && !this.contains(event.target)) this.close();
			},
			{ signal },
		);
	}

	disconnectedCallback(): void {
		this.events?.abort();
	}

	private close(): void {
		const details = this.querySelector('details');
		if (details) details.open = false;
	}

	private renderOptions(): void {
		const query = this.query.trim().toLocaleLowerCase();
		const visible = this.config.options.filter((option): boolean =>
			option.label.toLocaleLowerCase().includes(query),
		);
		const list = this.querySelector('.lab-picker-options');
		if (list)
			list.innerHTML = visible
				.map(
					(option): string =>
						`<button type="button" data-picker-value="${safeText(option.value)}" aria-pressed="${option.value === this.config.selected}"><span>${safeText(option.label)}</span><small>${safeText(option.detail ?? '')}</small>${option.value === this.config.selected ? icon('check-circle') : ''}</button>`,
				)
				.join('');
		const status = this.querySelector('[role="status"]');
		if (status) status.textContent = visible.length ? '' : 'No matches.';
	}

	private render(): void {
		const selected = this.config.options.find(
			(option): boolean => option.value === this.config.selected,
		);
		this.innerHTML = `<details class="lab-search-picker"><summary aria-label="${safeText(this.config.label)}">${icon(this.config.icon)}<span>${safeText(selected?.label ?? this.config.label)}</span><small>${safeText(selected?.detail ?? '')}</small>${icon('caret-down')}</summary><div class="lab-picker-panel"><label class="lab-picker-search">${icon('magnifying-glass')}<input type="search" aria-label="${safeText(this.config.placeholder.replace('…', ''))}" placeholder="${safeText(this.config.placeholder)}" value="${safeText(this.query)}" autocomplete="off" /></label><div class="lab-picker-options" aria-label="${safeText(this.config.label)} options"></div><small role="status"></small></div></details>`;
		this.renderOptions();
	}
}
