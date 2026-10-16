import { createIconButton } from '../core/icons.ts';

export interface DocumentToolbar {
	readonly element: HTMLElement;
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
	pagesButton.append('Pages');
	pagesButton.popoverTargetElement = pagePanel;
	const homeSource = pageLinks[0];
	const home = document.createElement('a');
	home.href = homeSource?.href ?? location.href.split('#')[0] ?? location.href;
	home.textContent = 'Home';
	pagePanel.append(home);
	for (const original of pageLinks.slice(1)) {
		const link = original.cloneNode(true);
		if (link instanceof HTMLAnchorElement) {
			if (new URL(link.href).pathname === location.pathname)
				link.setAttribute('aria-current', 'page');
			pagePanel.append(link);
		}
	}
	pagePanel.addEventListener('click', (event: MouseEvent): void => {
		if (event.target instanceof Element && event.target.closest('a')) pagePanel.hidePopover();
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
		pagePanel,
		updateTitle: (title: string): void => {
			current.textContent = title;
		},
	};
}
