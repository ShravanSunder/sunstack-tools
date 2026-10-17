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
): { readonly element: HTMLElement; readonly inputs: ReadonlyMap<TValue, HTMLInputElement> } {
	const element = document.createElement('div');
	element.className = 'ex-mode-choices';
	element.setAttribute('role', 'radiogroup');
	element.setAttribute('aria-label', label);
	const name = `ex-mode-${crypto.randomUUID()}`;
	const inputs = new Map<TValue, HTMLInputElement>();
	for (const option of options) {
		const choice = document.createElement('label');
		choice.title = option.label;
		const input = document.createElement('input');
		input.type = 'radio';
		input.name = name;
		input.value = option.value;
		input.checked = option.value === selected;
		input.setAttribute('aria-label', option.label);
		input.addEventListener('change', (): void => {
			if (input.checked) onChange(option.value);
		});
		const face = document.createElement('span');
		face.append(createPhosphorIcon(option.icon));
		if (!iconsOnly) face.append(option.label);
		choice.append(input, face);
		element.append(choice);
		inputs.set(option.value, input);
	}
	return { element, inputs };
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
