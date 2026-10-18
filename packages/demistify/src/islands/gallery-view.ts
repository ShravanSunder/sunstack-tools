import { createModeChoices, createSearchField, type SearchField } from '../core/controls.ts';
import { createIconButton } from '../core/icons.ts';

export function galleryElement<TTag extends keyof HTMLElementTagNameMap>(
	tag: TTag,
	className = '',
	text = '',
): HTMLElementTagNameMap[TTag] {
	const element = document.createElement(tag);
	element.className = className;
	element.textContent = text;
	return element;
}

export interface GalleryControls {
	readonly header: HTMLElement;
	readonly search: HTMLInputElement;
	readonly searchField: SearchField;
	readonly filtersButton: HTMLButtonElement;
	readonly filtersPanel: HTMLElement;
	readonly help: HTMLButtonElement;
	readonly helpPanel: HTMLElement;
	readonly count: HTMLOutputElement;
	readonly reviewChoice: HTMLButtonElement;
	readonly gridChoice: HTMLButtonElement;
	readonly body: HTMLElement;
	readonly strip: HTMLElement;
	readonly viewer: HTMLElement;
	readonly toolbar: HTMLElement;
	readonly title: HTMLElement;
	readonly fit: HTMLButtonElement;
	readonly fill: HTMLButtonElement;
	readonly zoomOut: HTMLButtonElement;
	readonly zoomIn: HTMLButtonElement;
	readonly sidebar: HTMLButtonElement;
	readonly fullscreen: HTMLButtonElement;
	readonly original: HTMLAnchorElement;
	readonly grid: HTMLElement;
	readonly message: HTMLElement;
}

/** Keep native top-layer menus beside their invoker, within the available viewport. */
export function anchorGalleryPopover(
	button: HTMLButtonElement,
	panel: HTMLElement,
	signal: AbortSignal,
): void {
	const position = (): void => {
		const bounds = button.getBoundingClientRect();
		const viewportWidth = document.documentElement.clientWidth;
		const viewportHeight = document.documentElement.clientHeight;
		const edge = 8;
		const width = Math.min(320, viewportWidth - edge * 2);
		const below = viewportHeight - bounds.bottom - edge * 2;
		const above = bounds.top - edge * 2;
		const openAbove = below < 160 && above > below;
		panel.style.left = `${Math.max(edge, Math.min(bounds.left, viewportWidth - width - edge))}px`;
		panel.style.top = openAbove ? 'auto' : `${bounds.bottom + edge}px`;
		panel.style.bottom = openAbove ? `${viewportHeight - bounds.top + edge}px` : 'auto';
		panel.style.maxHeight = `min(75vh, ${Math.max(0, openAbove ? above : below)}px)`;
	};
	panel.addEventListener(
		'beforetoggle',
		(event: ToggleEvent): void => {
			if (event.newState === 'open') position();
		},
		{ signal },
	);
	const followInvoker = (): void => {
		if (panel.matches(':popover-open')) position();
	};
	window.addEventListener('resize', followInvoker, { signal });
	window.addEventListener('scroll', followInvoker, { signal, capture: true });
}

function controlGroup(label: string, controls: readonly HTMLElement[]): HTMLElement {
	const group = galleryElement('div', 'ex-gallery-control-group');
	group.setAttribute('role', 'group');
	group.setAttribute('aria-label', label);
	group.append(...controls);
	return group;
}

export function createGalleryControls(prefix: string): GalleryControls {
	const header = galleryElement('header', 'ex-gallery-header');
	const searchField = createSearchField('Search screenshots');
	const search = searchField.input;
	const filtersButton = createIconButton('funnel-simple', 'Filter screenshots', (): void => {});
	filtersButton.append('Filters');
	const filtersPanel = galleryElement('div', 'ex-gallery-filters');
	filtersPanel.id = `${prefix}-filters`;
	filtersPanel.popover = 'auto';
	filtersPanel.setAttribute('aria-label', 'Screenshot filters');
	filtersButton.popoverTargetElement = filtersPanel;
	const count = galleryElement('output', 'ex-gallery-count');
	count.setAttribute('aria-live', 'polite');
	const modes = createModeChoices(
		'Gallery layout',
		[
			{ value: 'review', label: 'Review view', icon: 'sidebar-simple' },
			{ value: 'grid', label: 'Grid view', icon: 'squares-four' },
		],
		'review',
		(): void => {},
		true,
	);
	const reviewChoice = modes.buttons.get('review');
	const gridChoice = modes.buttons.get('grid');
	if (!reviewChoice || !gridChoice) throw new Error('Gallery layout choices are missing.');
	header.append(searchField.element, filtersButton, count, modes.element);
	const body = galleryElement('div', 'ex-gallery-body');
	const strip = galleryElement('aside', 'ex-gallery-strip');
	strip.setAttribute('aria-label', 'Screenshots');
	const viewer = galleryElement('section', 'ex-gallery-viewer');
	const toolbar = galleryElement('div', 'ex-gallery-toolbar');
	const title = galleryElement('strong', 'ex-gallery-title');
	const fit = createIconButton('corners-out', 'Fit screenshot', (): void => {});
	fit.append('Fit');
	const fill = createIconButton('resize', 'Fill screenshot', (): void => {});
	fill.append('Fill');
	const zoomOut = createIconButton('magnifying-glass-minus', 'Zoom out', (): void => {});
	const zoomIn = createIconButton('magnifying-glass-plus', 'Zoom in', (): void => {});
	const sidebar = createIconButton('sidebar-simple', 'Open screenshot in sidebar', (): void => {});
	const fullscreen = createIconButton('arrows-out', 'Expand screenshot', (): void => {});
	const original = galleryElement('a', 'ex-icon-button');
	original.target = '_blank';
	original.rel = 'noopener';
	original.title = 'Open original image';
	original.setAttribute('aria-label', 'Open original image');
	original.innerHTML = '<i class="ph ph-arrow-square-out" aria-hidden="true"></i>';
	const help = createIconButton('keyboard', 'Keyboard shortcuts', (): void => {});
	const helpPanel = galleryElement('aside', 'ex-gallery-filters ex-gallery-shortcuts');
	helpPanel.id = `${prefix}-shortcuts`;
	helpPanel.popover = 'auto';
	helpPanel.setAttribute('aria-label', 'Keyboard shortcuts');
	helpPanel.innerHTML =
		'<strong>Keyboard shortcuts</strong><dl><dt>Left / Right</dt><dd>Previous / next screen</dd><dt>Up / Down</dt><dd>Previous / next variation of this screen</dd><dt>⌘ / Ctrl + wheel</dt><dd>Zoom</dd><dt>Drag or scroll</dt><dd>Pan</dd><dt>Esc</dt><dd>Close fullscreen or this help</dd></dl>';
	help.popoverTargetElement = helpPanel;
	header.append(helpPanel);
	toolbar.append(
		title,
		controlGroup('Image zoom', [fit, fill, zoomOut, zoomIn]),
		controlGroup('Viewing actions', [sidebar, fullscreen, original, help]),
	);
	viewer.append(toolbar);
	body.append(strip, viewer);
	const grid = galleryElement('div', 'ex-gallery-grid');
	const message = galleryElement('p', 'ex-gallery-message');
	message.setAttribute('role', 'status');
	return {
		header,
		search,
		searchField,
		filtersButton,
		filtersPanel,
		help,
		helpPanel,
		count,
		reviewChoice,
		gridChoice,
		body,
		strip,
		viewer,
		toolbar,
		title,
		fit,
		fill,
		zoomOut,
		zoomIn,
		sidebar,
		fullscreen,
		original,
		grid,
		message,
	};
}
