import {
	searchDocumentDestinations,
	type SearchDestination,
	type SearchScope,
} from '../core/document-search.ts';
import { createIconButton, createPhosphorIcon } from '../core/icons.ts';
import { resolveTarget } from './target-resolution.ts';

/** One native dialog owns search interaction; existing document targets own navigation. */
export class DocumentSearch {
	readonly dialog = document.createElement('dialog');
	private readonly input = document.createElement('input');
	private readonly results = document.createElement('div');
	private readonly status = document.createElement('span');
	private events = new AbortController();
	private destinations: readonly SearchDestination[] = [];
	private matches: readonly SearchDestination[] = [];
	private scope: SearchScope = 'all';
	private selected = 0;
	private restoreFocus: HTMLElement | undefined;

	constructor(private readonly entries: () => readonly SearchDestination[]) {
		this.dialog.className = 'ex-command-dialog';
		this.dialog.setAttribute('aria-label', 'Search documents');
		this.input.type = 'search';
		this.input.autocomplete = 'off';
		this.input.setAttribute('role', 'combobox');
		this.input.setAttribute('aria-label', 'Search pages and headings');
		this.input.setAttribute('aria-autocomplete', 'list');
		this.input.setAttribute('aria-controls', 'ex-command-results');
		this.input.setAttribute('aria-expanded', 'true');
		this.results.id = 'ex-command-results';
		this.results.className = 'ex-command-results';
		this.results.setAttribute('role', 'listbox');
		this.results.setAttribute('aria-label', 'Destinations');
		const header = document.createElement('header');
		header.append(
			createPhosphorIcon('magnifying-glass'),
			this.input,
			createIconButton('x', 'Close search', (): void => this.dialog.close()),
		);
		const footer = document.createElement('footer');
		footer.append('↑ ↓ to move · ↵ to open', this.status);
		this.status.setAttribute('role', 'status');
		this.dialog.append(header, this.results, footer);
		this.input.addEventListener('input', (): void => this.render());
		this.dialog.addEventListener('click', (event): void => {
			if (event.target === this.dialog) this.dialog.close();
		});
		this.dialog.addEventListener('close', (): void =>
			this.restoreFocus?.focus({ preventScroll: true }),
		);
		this.dialog.addEventListener('keydown', (event): void => this.handleDialogKey(event));
	}

	connect(): void {
		this.events.abort();
		this.events = new AbortController();
		document.body.append(this.dialog);
		const { signal } = this.events;
		document.addEventListener('keydown', (event): void => this.handleShortcut(event), { signal });
		document.addEventListener(
			'keyup',
			(event): void => {
				if (event.key === 'Alt') this.showHints(false);
			},
			{ signal },
		);
		window.addEventListener('blur', (): void => this.showHints(false), { signal });
	}

	disconnect(): void {
		this.events.abort();
		this.dialog.remove();
	}

	open(scope: SearchScope, trigger?: HTMLElement): void {
		this.scope = scope;
		this.restoreFocus =
			trigger ??
			(document.activeElement instanceof HTMLElement ? document.activeElement : undefined);
		this.destinations = this.entries();
		this.input.value = '';
		this.input.placeholder =
			scope === 'walkthroughs'
				? 'Find a walkthrough'
				: scope === 'pages'
					? 'Find a page'
					: 'Search pages and headings';
		this.render();
		if (!this.dialog.open) this.dialog.showModal();
		this.input.focus();
	}

	private showHints(visible: boolean): void {
		document.documentElement.toggleAttribute('data-shortcut-hints', visible);
	}

	private handleShortcut(event: KeyboardEvent): void {
		if (event.key === 'Alt') this.showHints(true);
		if (event.defaultPrevented || event.isComposing || event.shiftKey || event.altKey) return;
		const target = event.target instanceof Element ? event.target : null;
		if (
			target?.closest(
				'input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="textbox"]',
			)
		)
			return;
		if (this.dialog.open || document.querySelector('dialog[open], :popover-open')) return;
		const command = (event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k';
		const key = !event.metaKey && !event.ctrlKey ? event.key.toLowerCase() : '';
		if (!command && key !== 'p' && key !== 'w') return;
		event.preventDefault();
		const scope = command ? 'all' : key === 'p' ? 'pages' : 'walkthroughs';
		const trigger = document.querySelector(
			scope === 'walkthroughs' ? '.ex-presentations-trigger' : '.ex-pages-trigger',
		);
		this.open(scope, trigger instanceof HTMLElement ? trigger : undefined);
	}

	private handleDialogKey(event: KeyboardEvent): void {
		if (event.isComposing || event.altKey || event.metaKey || event.ctrlKey) return;
		if (event.key === 'Escape') {
			event.preventDefault();
			this.dialog.close();
		} else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
			event.preventDefault();
			this.select(this.selected + (event.key === 'ArrowDown' ? 1 : -1));
		} else if (event.key === 'Enter') {
			event.preventDefault();
			this.activate(this.matches[this.selected]);
		} else if (this.scope !== 'all' && !this.input.value && /^[1-9]$/.test(event.key)) {
			const entry = this.matches[Number(event.key) - 1];
			if (entry) {
				event.preventDefault();
				this.activate(entry);
			}
		}
	}

	private render(): void {
		this.matches = searchDocumentDestinations(this.destinations, this.input.value, this.scope);
		this.results.replaceChildren();
		const home = this.destinations.find((entry): boolean => entry.kind === 'page');
		for (const [index, entry] of this.matches.entries()) {
			const row = document.createElement('a');
			row.href = new URL(entry.href, document.baseURI).href;
			row.id = `ex-command-result-${index}`;
			row.setAttribute('role', 'option');
			row.setAttribute(
				'aria-label',
				entry.kind === 'heading'
					? [entry.pageTitle, ...entry.ancestors, entry.title].join(' › ')
					: [entry.title, entry.description].filter(Boolean).join(' '),
			);
			row.tabIndex = -1;
			if (entry.kind === 'page' && new URL(row.href).pathname === location.pathname)
				row.setAttribute('aria-current', 'page');
			row.append(
				createPhosphorIcon(
					entry.kind === 'page'
						? entry === home
							? 'house'
							: 'files'
						: entry.kind === 'journey'
							? 'images'
							: entry.kind === 'walkthrough'
								? 'path'
								: 'article',
				),
			);
			const copy = document.createElement('div');
			if (entry.kind === 'heading') {
				const context = document.createElement('span');
				context.className = 'ex-command-context';
				context.textContent = [entry.pageTitle, ...entry.ancestors].join(' › ');
				copy.append(context);
				if (!this.input.value)
					row.style.setProperty('--heading-depth', String(entry.ancestors.length));
			}
			const title = document.createElement('strong');
			title.textContent = entry.title;
			copy.append(title);
			if (entry.description) {
				const description = document.createElement('span');
				description.textContent = entry.description;
				copy.append(description);
			}
			row.append(copy);
			if (this.scope !== 'all' && !this.input.value && index < 9) {
				const hint = document.createElement('kbd');
				hint.textContent = String(index + 1);
				row.append(hint);
			}
			row.addEventListener('click', (event): void => {
				if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
				event.preventDefault();
				this.activate(entry);
			});
			this.results.append(row);
		}
		if (!this.matches.length) {
			const empty = document.createElement('p');
			empty.textContent = 'No matching destinations.';
			this.results.append(empty);
		}
		this.status.textContent = `${this.matches.length} results`;
		this.select(0);
	}

	private select(index: number): void {
		this.selected = Math.max(0, Math.min(this.matches.length - 1, index));
		for (const [rowIndex, row] of [...this.results.children].entries())
			row.setAttribute('aria-selected', String(rowIndex === this.selected));
		const selected = this.results.children[this.selected];
		if (selected) {
			this.input.setAttribute('aria-activedescendant', selected.id);
			selected.scrollIntoView({ block: 'nearest' });
		} else this.input.removeAttribute('aria-activedescendant');
	}

	private activate(entry: SearchDestination | undefined): void {
		if (!entry) return;
		const destination = new URL(entry.href, document.baseURI);
		this.dialog.close();
		const current = new URL(location.href);
		if (
			destination.origin !== current.origin ||
			destination.pathname !== current.pathname ||
			destination.search !== current.search
		) {
			location.assign(destination.href);
			return;
		}
		const fragment = decodeURIComponent(destination.hash.slice(1));
		const presentation = document.getElementById(fragment);
		if (presentation?.localName === 'ex-presentation') {
			presentation.dispatchEvent(new Event('presentation-start'));
			return;
		}
		if (!fragment) {
			history.pushState(null, '', destination.href);
			window.scrollTo({ top: 0, behavior: 'instant' });
			return;
		}
		void resolveTarget(fragment).then((target): void => {
			if (!target) return;
			history.pushState(null, '', destination.href);
			target.scrollIntoView({ block: 'start', behavior: 'instant' });
			if (target instanceof HTMLElement) {
				target.tabIndex = -1;
				target.focus({ preventScroll: true });
			}
		});
	}
}
