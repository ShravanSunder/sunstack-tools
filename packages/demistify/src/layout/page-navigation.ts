import { createIconButton } from '../core/icons.ts';

export interface DocumentToolbar {
	readonly element: HTMLElement;
	readonly pagesButton: HTMLButtonElement;
	readonly updateTitle: (title: string) => void;
}

/** One native page-link list supplies Pages, Home, current title and breadcrumbs. No fetch needed. */
export function createDocumentToolbar(
	pageLinks: readonly HTMLAnchorElement[],
	openPages: () => void,
): DocumentToolbar {
	const element = document.createElement('header');
	element.className = 'ex-document-toolbar';
	const pagesButton = createIconButton('files', 'Browse pages', openPages);
	pagesButton.classList.add('ex-pages-trigger');
	pagesButton.append('Pages');
	const hint = document.createElement('kbd');
	hint.className = 'ex-shortcut-hint';
	hint.textContent = 'P';
	pagesButton.append(hint);
	const homeSource = pageLinks[0];
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
	document.body.append(element);
	return {
		element,
		pagesButton,
		updateTitle: (title: string): void => {
			current.textContent = title;
		},
	};
}
