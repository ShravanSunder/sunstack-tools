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
	readonly filtersButton: HTMLButtonElement;
	readonly filtersPanel: HTMLElement;
	readonly count: HTMLOutputElement;
	readonly reviewButton: HTMLButtonElement;
	readonly gridButton: HTMLButtonElement;
	readonly body: HTMLElement;
	readonly strip: HTMLElement;
	readonly viewer: HTMLElement;
	readonly toolbar: HTMLElement;
	readonly title: HTMLElement;
	readonly previous: HTMLButtonElement;
	readonly next: HTMLButtonElement;
	readonly previousViewport: HTMLButtonElement;
	readonly nextViewport: HTMLButtonElement;
	readonly viewport: HTMLSelectElement;
	readonly fit: HTMLButtonElement;
	readonly natural: HTMLButtonElement;
	readonly zoomOut: HTMLButtonElement;
	readonly zoomIn: HTMLButtonElement;
	readonly zoom: HTMLOutputElement;
	readonly sidebar: HTMLButtonElement;
	readonly fullscreen: HTMLButtonElement;
	readonly original: HTMLAnchorElement;
	readonly grid: HTMLElement;
	readonly message: HTMLElement;
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
	const search = galleryElement('input');
	search.type = 'search';
	search.placeholder = 'Search screenshots';
	search.setAttribute('aria-label', 'Search screenshots');
	const filtersButton = createIconButton('funnel-simple', 'Filter screenshots', (): void => {});
	filtersButton.append('Filters');
	const filtersPanel = galleryElement('div', 'ex-gallery-filters');
	filtersPanel.id = `${prefix}-filters`;
	filtersPanel.popover = 'auto';
	filtersPanel.setAttribute('aria-label', 'Screenshot filters');
	filtersButton.popoverTargetElement = filtersPanel;
	const count = galleryElement('output', 'ex-gallery-count');
	count.setAttribute('aria-live', 'polite');
	const reviewButton = createIconButton('sidebar-simple', 'Review view', (): void => {});
	const gridButton = createIconButton('squares-four', 'Grid view', (): void => {});
	header.append(
		search,
		filtersButton,
		count,
		controlGroup('Gallery layout', [reviewButton, gridButton]),
	);
	const body = galleryElement('div', 'ex-gallery-body');
	const strip = galleryElement('aside', 'ex-gallery-strip');
	strip.setAttribute('aria-label', 'Screenshots');
	const viewer = galleryElement('section', 'ex-gallery-viewer');
	const toolbar = galleryElement('div', 'ex-gallery-toolbar');
	const title = galleryElement('strong', 'ex-gallery-title');
	const previous = createIconButton('arrow-up', 'Previous screenshot · Up arrow', (): void => {});
	const next = createIconButton('arrow-down', 'Next screenshot · Down arrow', (): void => {});
	const previousViewport = createIconButton(
		'arrow-left',
		'Previous viewport · Left arrow',
		(): void => {},
	);
	const nextViewport = createIconButton(
		'arrow-right',
		'Next viewport · Right arrow',
		(): void => {},
	);
	const viewport = galleryElement('select');
	viewport.setAttribute('aria-label', 'Viewport category');
	const fit = createIconButton('corners-out', 'Fit screenshot', (): void => {});
	fit.append('Fit');
	const natural = createIconButton('resize', 'Natural size', (): void => {});
	natural.append('100%');
	const zoomOut = createIconButton('magnifying-glass-minus', 'Zoom out', (): void => {});
	const zoomIn = createIconButton('magnifying-glass-plus', 'Zoom in', (): void => {});
	const zoom = galleryElement('output', 'ex-gallery-zoom');
	const sidebar = createIconButton('sidebar-simple', 'Open screenshot in sidebar', (): void => {});
	const fullscreen = createIconButton('arrows-out', 'Expand screenshot', (): void => {});
	const original = galleryElement('a', 'ex-icon-button');
	original.target = '_blank';
	original.rel = 'noopener';
	original.title = 'Open original image';
	original.setAttribute('aria-label', 'Open original image');
	original.innerHTML = '<i class="ph ph-arrow-square-out" aria-hidden="true"></i>';
	toolbar.append(
		title,
		controlGroup('Screenshot navigation', [previous, next]),
		controlGroup('Viewport category', [previousViewport, viewport, nextViewport]),
		controlGroup('Image zoom', [fit, natural, zoomOut, zoom, zoomIn]),
		controlGroup('Viewing actions', [sidebar, fullscreen, original]),
	);
	viewer.append(toolbar);
	body.append(strip, viewer);
	const grid = galleryElement('div', 'ex-gallery-grid');
	const message = galleryElement('p', 'ex-gallery-message');
	message.setAttribute('role', 'status');
	return {
		header,
		search,
		filtersButton,
		filtersPanel,
		count,
		reviewButton,
		gridButton,
		body,
		strip,
		viewer,
		toolbar,
		title,
		previous,
		next,
		previousViewport,
		nextViewport,
		viewport,
		fit,
		natural,
		zoomOut,
		zoomIn,
		zoom,
		sidebar,
		fullscreen,
		original,
		grid,
		message,
	};
}
