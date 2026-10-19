import { createIconButton, createPhosphorIcon } from '../core/icons.ts';

export interface DocumentToolbar {
	readonly element: HTMLElement;
	readonly pagesButton: HTMLButtonElement;
	readonly pagePanel: HTMLElement;
	readonly updateTitle: (title: string) => void;
}

/** One native page-link list supplies Pages, Home, current title and breadcrumbs. No fetch needed. */
export function createDocumentToolbar(pageLinks: readonly HTMLAnchorElement[]): DocumentToolbar {
	const element = document.createElement('header');
	element.className = 'ex-document-toolbar';
	const pagePanel = document.createElement('nav');
	pagePanel.className = 'ex-pages-panel';
	pagePanel.popover = 'auto';
	pagePanel.setAttribute('aria-label', 'Pages');
	const pagesButton = createIconButton('files', 'Browse pages', (): void => {});
	pagesButton.popoverTargetElement = pagePanel;
	pagesButton.classList.add('ex-pages-trigger');
	pagesButton.append('Pages');
	const hint = document.createElement('kbd');
	hint.className = 'ex-shortcut-hint';
	hint.textContent = 'P';
	pagesButton.append(hint);
	const homeSource = pageLinks[0];
	const homeLink = document.createElement('a');
	homeLink.href = homeSource?.getAttribute('href') ?? location.href;
	homeLink.append(createPhosphorIcon('house'), 'Home');
	homeLink.autofocus = true;
	const links = [
		homeLink,
		...pageLinks.slice(1).map((source): HTMLAnchorElement => {
			const link = document.createElement('a');
			link.href = source.getAttribute('href') ?? source.href;
			link.append(createPhosphorIcon('files'), source.textContent?.trim() ?? '');
			return link;
		}),
	];
	for (const [index, link] of links.entries()) {
		if (new URL(link.href).pathname === location.pathname)
			link.setAttribute('aria-current', 'page');
		if (index < 9) {
			const number = document.createElement('kbd');
			number.textContent = String(index + 1);
			number.setAttribute('aria-hidden', 'true');
			link.append(number);
		}
	}
	pagePanel.append(...links);
	pagePanel.addEventListener('click', (event): void => {
		if (event.target instanceof Element && event.target.closest('a')) pagePanel.hidePopover();
	});
	pagePanel.addEventListener('keydown', (event): void => {
		if (event.altKey || event.ctrlKey || event.metaKey || event.isComposing) return;
		const index = links.findIndex((link): boolean => link === document.activeElement);
		const next =
			event.key === 'Home'
				? 0
				: event.key === 'End'
					? links.length - 1
					: event.key === 'ArrowDown'
						? Math.min(links.length - 1, index + 1)
						: event.key === 'ArrowUp'
							? Math.max(0, index - 1)
							: undefined;
		if (next !== undefined) {
			event.preventDefault();
			links[next]?.focus();
		} else if (/^[1-9]$/.test(event.key) && links[Number(event.key) - 1]) {
			event.preventDefault();
			links[Number(event.key) - 1]?.click();
		}
	});
	const breadcrumbs = document.createElement('nav');
	breadcrumbs.className = 'ex-document-breadcrumbs';
	breadcrumbs.setAttribute('aria-label', 'Breadcrumb');
	if (homeSource && new URL(homeSource.href).pathname !== location.pathname) {
		const ancestor = document.createElement('a');
		ancestor.href = homeSource.href;
		ancestor.textContent = homeSource.textContent;
		const separator = document.createElement('span');
		separator.textContent = '›';
		separator.setAttribute('aria-hidden', 'true');
		breadcrumbs.append(ancestor, separator);
	}
	const current = document.createElement('strong');
	current.setAttribute('aria-current', 'page');
	breadcrumbs.append(current);
	element.append(pagesButton, breadcrumbs);
	document.body.append(element, pagePanel);
	return {
		element,
		pagesButton,
		pagePanel,
		updateTitle: (title: string): void => {
			current.textContent = title;
		},
	};
}
