import { bindTabKeyboard, labelControl } from '../core/controls.ts';
import { readIcon } from '../core/head.ts';
import { reportIssue } from '../core/issues.ts';
import { ComparisonChoice, type ComparisonOption } from './comparison-choice.ts';
import { copyInspectionContent, registerInspectionCopy } from './inspection-copy.ts';

/** One live copy of each option; CSS Grid owns its row and column placement. */
export class ComparisonViews {
	private readonly viewport = document.createElement('div');
	private readonly canvas = document.createElement('div');
	private readonly options: readonly ComparisonOption[];
	private readonly choices: readonly ComparisonChoice[];
	private readonly headers: readonly HTMLElement[];
	private readonly tabs: readonly HTMLButtonElement[];
	private readonly view: 'columns' | 'tabs';
	private readonly paired: boolean;
	private selected: number[];
	private events: AbortController | undefined;
	private readonly resize: ResizeObserver;

	constructor(
		private readonly owner: HTMLElement,
		sides: readonly HTMLElement[],
	) {
		this.view = owner.getAttribute('view') === 'tabs' ? 'tabs' : 'columns';
		this.paired = owner.hasAttribute('choose') && this.view === 'columns';
		this.options = sides.map((element, index): ComparisonOption => {
			element.id ||= `ex-comparison-option-${crypto.randomUUID()}`;
			element.classList.add('ex-comparison-option');
			delete element.dataset['side'];
			return {
				element,
				label:
					element.getAttribute('label') ??
					(index === 0 ? 'Before' : index === 1 ? 'After' : `View ${index + 1}`),
				icon: readIcon(element) ?? 'file',
			};
		});
		if (this.options.length < 2)
			reportIssue(owner, 'A comparison needs at least two alternatives.');
		const authored = (owner.getAttribute('selected') ?? '').split(/\s+/).filter(Boolean);
		const initial = authored.map((id): number =>
			this.options.findIndex((option): boolean => option.element.id === id),
		);
		if (initial.includes(-1))
			reportIssue(owner, 'A selected comparison ID does not name a child alternative.');
		const candidates = [
			...new Set([
				...initial.filter((index): boolean => index >= 0),
				...this.options.map((_option, index): number => index),
			]),
		];
		this.selected =
			this.view === 'tabs'
				? candidates.slice(0, 1)
				: this.paired
					? candidates.slice(0, 2)
					: this.options.map((_option, index): number => index);
		this.viewport.className = 'ex-comparison-viewport';
		this.canvas.className = 'ex-comparison-canvas';
		this.canvas.dataset['view'] = this.view;
		owner.classList.add('ex-comparison');
		owner.dataset['title'] =
			owner.getAttribute('label') ??
			this.options.map((option): string => option.label).join(' and ');
		const tabBar = document.createElement('div');
		tabBar.className = 'ex-tab-bar';
		tabBar.setAttribute('role', 'tablist');
		tabBar.setAttribute('aria-label', owner.dataset['title']);
		this.tabs =
			this.view === 'tabs'
				? this.options.map((option, index): HTMLButtonElement => {
						const button = document.createElement('button');
						button.type = 'button';
						button.id = `ex-comparison-tab-${crypto.randomUUID()}`;
						button.setAttribute('role', 'tab');
						button.setAttribute('aria-controls', option.element.id);
						labelControl(button, option.icon, option.label);
						option.element.setAttribute('role', 'tabpanel');
						option.element.setAttribute('aria-labelledby', button.id);
						button.addEventListener('click', (): void => this.select(0, index));
						return button;
					})
				: [];
		tabBar.append(...this.tabs);
		bindTabKeyboard(tabBar);
		this.choices = this.paired
			? this.selected.map(
					(_option, slot): ComparisonChoice =>
						new ComparisonChoice(
							slot === 0 ? 'Choose left comparison' : 'Choose right comparison',
							this.options,
							(index): void => this.select(slot, index),
						),
				)
			: [];
		this.headers =
			this.view === 'tabs'
				? []
				: this.selected.map((index, slot): HTMLElement => {
						const header = document.createElement('header');
						header.className = 'ex-comparison-header';
						const choice = this.choices[slot];
						if (choice) header.append(choice.button, choice.panel);
						else {
							const title = document.createElement('strong');
							title.textContent = this.options[index]?.label ?? '';
							header.append(title);
						}
						return header;
					});
		this.canvas.append(...this.headers, ...sides);
		this.viewport.append(this.canvas);
		if (this.view === 'tabs') owner.append(tabBar);
		owner.append(this.viewport);
		this.canvas.addEventListener('comparison-reveal', (event: Event): void => {
			if (!(event.target instanceof HTMLElement)) return;
			const index = this.options.findIndex((option): boolean => option.element === event.target);
			if (index < 0) return;
			event.stopPropagation();
			if (!this.selected.includes(index))
				this.select(this.view === 'tabs' ? 0 : this.selected.length - 1, index);
		});
		this.resize = new ResizeObserver((): void => this.updateStacking());
		registerInspectionCopy(owner, (): HTMLElement => this.createCopy());
		this.render();
	}

	connect(): void {
		this.events?.abort();
		this.events = new AbortController();
		for (const choice of this.choices) choice.connect(this.events.signal);
		this.resize.observe(this.viewport);
	}
	disconnect(): void {
		this.events?.abort();
		this.resize.disconnect();
	}

	private select(slot: number, index: number): void {
		if (this.selected[slot] === index) return;
		const previous = this.selected[slot];
		const opposite = this.selected.indexOf(index);
		if (opposite >= 0 && previous !== undefined) this.selected[opposite] = previous;
		this.selected[slot] = index;
		this.render();
	}

	private updateStacking(): void {
		const required = this.selected.length * 300 + (this.selected.length - 1) * 24;
		this.canvas.toggleAttribute(
			'data-stacked',
			this.view === 'columns' && this.viewport.clientWidth < required,
		);
	}

	private render(): void {
		for (const gap of this.canvas.querySelectorAll(
			':scope > .ex-comparison-option > .ex-comparison-gap',
		))
			gap.remove();
		const visible = this.selected.flatMap((index): ComparisonOption[] =>
			this.options[index] ? [this.options[index]] : [],
		);
		const rowLists = visible.map((option): string[] =>
			Array.from(option.element.children).map(
				(row): string => row.getAttribute('data-compare-row') ?? '',
			),
		);
		const aligned =
			this.view === 'columns' &&
			visible.every((option): boolean =>
				Array.from(option.element.childNodes).every(
					(node): boolean => node.nodeType !== Node.TEXT_NODE || !node.textContent?.trim(),
				),
			) &&
			rowLists.length > 0 &&
			rowLists.every(
				(keys): boolean =>
					keys.length > 0 && keys.every(Boolean) && new Set(keys).size === keys.length,
			);
		const keys = aligned ? [...new Set(rowLists.flat())] : [];
		const rowCount = Math.max(1, keys.length);
		this.canvas.style.setProperty('--comparison-columns', String(visible.length));
		this.canvas.style.setProperty('--comparison-rows', String(rowCount));
		this.canvas.toggleAttribute('data-aligned', aligned);
		for (const [index, option] of this.options.entries()) {
			const slot = this.selected.indexOf(index);
			option.element.hidden = slot < 0;
			if (slot < 0) continue;
			option.element.style.setProperty('--comparison-column', String(slot + 1));
			option.element.style.setProperty('--comparison-stack-row', String(slot * (rowCount + 1) + 2));
			if (this.view === 'columns') {
				option.element.setAttribute('role', 'region');
				option.element.setAttribute('aria-label', option.label);
			}
			const rows = Array.from(option.element.children).filter(
				(row): row is HTMLElement => row instanceof HTMLElement,
			);
			for (const row of rows)
				if (row.hasAttribute('data-compare-row'))
					row.style.removeProperty('--comparison-topic-row');
			for (const [rowIndex, key] of keys.entries()) {
				let row = rows.find(
					(candidate): boolean => candidate.getAttribute('data-compare-row') === key,
				);
				if (!row) {
					row = document.createElement('section');
					row.className = 'ex-comparison-gap';
					row.setAttribute('data-compare-row', key);
					row.setAttribute('aria-label', `${key}: not supplied`);
					row.textContent = 'Not supplied';
					option.element.append(row);
				}
				row.style.setProperty('--comparison-topic-row', String(rowIndex + 1));
			}
			const header = this.headers[slot];
			if (header) {
				header.style.setProperty('--comparison-column', String(slot + 1));
				header.style.setProperty('--comparison-stack-row', String(slot * (rowCount + 1) + 1));
			}
			this.choices[slot]?.select(index);
		}
		for (const [index, tab] of this.tabs.entries()) {
			tab.setAttribute('aria-selected', String(this.selected.includes(index)));
			tab.tabIndex = this.selected.includes(index) ? 0 : -1;
		}
		this.updateStacking();
		this.canvas.dispatchEvent(new Event('island-ready', { bubbles: true }));
	}

	private createCopy(): HTMLElement {
		const copy = this.owner.cloneNode(false);
		if (!(copy instanceof HTMLElement))
			throw new Error('A comparison copy must be an HTML element.');
		copy.removeAttribute('prepared');
		const sides = this.options.map((option): HTMLElement => {
			const side = copyInspectionContent(option.element);
			if (!side) throw new Error('A comparison alternative could not be copied.');
			for (const gap of side.querySelectorAll(':scope > .ex-comparison-gap')) gap.remove();
			return side;
		});
		copy.setAttribute(
			'selected',
			this.selected.map((index): string => sides[index]?.id ?? '').join(' '),
		);
		copy.append(...sides);
		return copy;
	}
}
