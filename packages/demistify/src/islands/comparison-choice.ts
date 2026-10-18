import { labelControl } from '../core/controls.ts';
import { createPhosphorIcon } from '../core/icons.ts';
import { anchorPopover } from '../core/popover-position.ts';

export interface ComparisonOption {
	readonly element: HTMLElement;
	readonly label: string;
	readonly icon: string;
}

/** A content choice, rather than a reader setting for presentation or column count. */
export class ComparisonChoice {
	readonly button = document.createElement('button');
	readonly panel = document.createElement('div');
	private readonly items: readonly HTMLButtonElement[];
	constructor(
		label: string,
		private readonly options: readonly ComparisonOption[],
		select: (index: number) => void,
	) {
		this.button.type = 'button';
		this.button.className = 'ex-comparison-choice';
		this.button.setAttribute('aria-label', label);
		this.button.setAttribute('aria-expanded', 'false');
		this.panel.className = 'ex-comparison-choices';
		this.panel.popover = 'auto';
		this.panel.setAttribute('role', 'group');
		this.panel.setAttribute('aria-label', `${label} options`);
		this.button.popoverTargetElement = this.panel;
		this.items = options.map((option, index): HTMLButtonElement => {
			const item = document.createElement('button');
			item.type = 'button';
			labelControl(item, option.icon, option.label);
			item.addEventListener('click', (): void => {
				select(index);
				this.panel.hidePopover();
				this.button.focus();
			});
			return item;
		});
		this.panel.append(...this.items);
		this.panel.addEventListener('toggle', (event: ToggleEvent): void => {
			this.button.setAttribute('aria-expanded', String(event.newState === 'open'));
			if (event.newState === 'open')
				this.panel.querySelector<HTMLButtonElement>('[aria-pressed="true"]')?.focus();
		});
		this.panel.addEventListener('keydown', (event: KeyboardEvent): void => {
			if (event.altKey || event.ctrlKey || event.metaKey) return;
			const index = this.items.findIndex((item): boolean => item === event.target);
			const next =
				event.key === 'ArrowDown'
					? (index + 1) % this.items.length
					: event.key === 'ArrowUp'
						? (index + this.items.length - 1) % this.items.length
						: event.key === 'Home'
							? 0
							: event.key === 'End'
								? this.items.length - 1
								: undefined;
			if (next === undefined) return;
			event.preventDefault();
			this.items[next]?.focus();
		});
	}
	connect(signal: AbortSignal): void {
		anchorPopover(this.button, this.panel, signal);
	}
	select(index: number): void {
		const selected = this.options[index];
		if (!selected) return;
		labelControl(this.button, selected.icon, selected.label);
		this.button.append(createPhosphorIcon('caret-down'));
		for (const [itemIndex, item] of this.items.entries())
			item.setAttribute('aria-pressed', String(itemIndex === index));
	}
}
