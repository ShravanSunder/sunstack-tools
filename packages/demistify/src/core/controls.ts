import { createIconButton, createPhosphorIcon } from './icons.ts';

/** Control construction owns appearance and interaction; callers retain selected values. */
export function labelControl(control: HTMLElement, icon: string, text: string): void {
	control.replaceChildren(createPhosphorIcon(icon), document.createTextNode(text));
}

export function bindTabKeyboard(tablist: HTMLElement): void {
	tablist.addEventListener('keydown', (event: KeyboardEvent): void => {
		if (!(event.target instanceof Element) || event.altKey || event.ctrlKey || event.metaKey)
			return;
		const current = event.target.closest('[role="tab"]');
		const buttons = [...tablist.querySelectorAll<HTMLButtonElement>('[role="tab"]')];
		const index = buttons.findIndex((button): boolean => button === current);
		if (index < 0) return;
		const next =
			event.key === 'ArrowRight'
				? (index + 1) % buttons.length
				: event.key === 'ArrowLeft'
					? (index + buttons.length - 1) % buttons.length
					: event.key === 'Home'
						? 0
						: event.key === 'End'
							? buttons.length - 1
							: undefined;
		if (next === undefined) return;
		event.preventDefault();
		buttons[next]?.click();
		tablist.querySelector<HTMLButtonElement>('[aria-selected="true"]')?.focus();
	});
}

export interface ModeOption<TValue extends string> {
	readonly value: TValue;
	readonly label: string;
	readonly icon: string;
}

export function createModeChoices<TValue extends string>(
	label: string,
	options: readonly ModeOption<TValue>[],
	selected: TValue,
	onChange: (value: TValue) => void,
	iconsOnly = false,
): {
	readonly element: HTMLElement;
	readonly buttons: ReadonlyMap<TValue, HTMLButtonElement>;
	readonly select: (value: TValue) => void;
} {
	const element = document.createElement('div');
	element.className = 'ex-mode-choices';
	element.setAttribute('role', 'group');
	element.setAttribute('aria-label', label);
	const buttons = new Map<TValue, HTMLButtonElement>();
	const select = (value: TValue): void => {
		for (const [key, button] of buttons) {
			button.setAttribute('aria-pressed', String(key === value));
			button.tabIndex = key === value ? 0 : -1;
		}
	};
	for (const option of options) {
		const button = document.createElement('button');
		button.type = 'button';
		button.value = option.value;
		button.title = option.label;
		button.setAttribute('aria-label', option.label);
		button.append(createPhosphorIcon(option.icon));
		if (!iconsOnly) button.append(option.label);
		button.addEventListener('click', (): void => {
			select(option.value);
			onChange(option.value);
		});
		button.addEventListener('keydown', (event: KeyboardEvent): void => {
			if (event.altKey || event.ctrlKey || event.metaKey) return;
			const index = options.indexOf(option);
			const next =
				event.key === 'ArrowRight'
					? (index + 1) % options.length
					: event.key === 'ArrowLeft'
						? (index + options.length - 1) % options.length
						: event.key === 'Home'
							? 0
							: event.key === 'End'
								? options.length - 1
								: undefined;
			if (next === undefined) return;
			event.preventDefault();
			const value = options[next]?.value;
			if (value !== undefined) {
				buttons.get(value)?.click();
				buttons.get(value)?.focus();
			}
		});
		element.append(button);
		buttons.set(option.value, button);
	}
	select(selected);
	return { element, buttons, select };
}

export interface SearchField {
	readonly element: HTMLElement;
	readonly input: HTMLInputElement;
	readonly sync: () => void;
}

export function createSearchField(label: string, placeholder = label): SearchField {
	const element = document.createElement('div');
	element.className = 'ex-control-search';
	const input = document.createElement('input');
	input.type = 'search';
	input.placeholder = placeholder;
	input.setAttribute('aria-label', label);
	const clear = createIconButton('x', `Clear ${label.toLocaleLowerCase()}`, (): void => {
		input.value = '';
		input.dispatchEvent(new Event('input', { bubbles: true }));
		input.focus();
	});
	const sync = (): void => {
		clear.hidden = !input.value;
	};
	input.addEventListener('input', sync);
	element.append(createPhosphorIcon('magnifying-glass'), input, clear);
	sync();
	return { element, input, sync };
}

export function createSwitch(
	label: string,
	checked: boolean,
	onChange: (checked: boolean) => void,
): HTMLButtonElement {
	const button = document.createElement('button');
	button.type = 'button';
	button.className = 'ex-switch';
	button.setAttribute('role', 'switch');
	button.setAttribute('aria-checked', String(checked));
	const track = document.createElement('span');
	track.className = 'ex-switch-track';
	track.setAttribute('aria-hidden', 'true');
	button.append(track, label);
	button.addEventListener('click', (): void =>
		onChange(button.getAttribute('aria-checked') !== 'true'),
	);
	return button;
}
