import { createIconButton } from '../core/icons.ts';
import { openDrawer } from './drawer.ts';
import { copyInspectionContent } from './inspection-copy.ts';
import { installInspectionGestures } from './inspection-gestures.ts';

type InspectionMode = 'modal' | 'sidebar';

const installedInspectors = new WeakSet<HTMLElement>();

/**
 * Which islands get inspection controls. Containers (tabs, grids, flows, boards, regions) never do:
 * they arrange; their content carries its own affordances. Visuals worth enlarging get sidebar and
 * expand buttons; an island whose depth is hidden gets a "More" button; inline <details> are their
 * own affordance; prose, plain cards and stats get nothing. A bare `zoom` attribute forces the
 * buttons and `zoom="none"` removes them.
 */
const visualIslands = new Set(['ex-mermaid', 'ex-code', 'ex-file-tree', 'ex-compare']);
type Affordance = 'enlarge' | 'more';

function affordanceFor(owner: HTMLElement): Affordance | undefined {
	const zoom = owner.getAttribute('zoom');
	if (zoom === 'none') return undefined;
	if (visualIslands.has(owner.localName)) return 'enlarge';
	if (owner.querySelector(':scope > [slot="details"]')) return 'more';
	return zoom === null ? undefined : 'enlarge';
}

/** What an untitled island is called in the panel's header. */
const plainNames: Readonly<Record<string, string>> = {
	'ex-mermaid': 'Diagram',
	'ex-code': 'Code',
	'ex-file-tree': 'Files',
	'ex-tree': 'Tree',
	figure: 'Picture',
};

function inspectionLabel(owner: HTMLElement): string {
	return (
		owner.getAttribute('label') ??
		owner.dataset['title'] ??
		(owner.localName === 'ex-tabs'
			? owner.querySelector('[role="tab"][aria-selected="true"]')?.textContent
			: undefined) ??
		owner.querySelector('h1,h2,h3,h4,h5,h6')?.textContent ??
		(owner.localName === 'ex-compare'
			? Array.from(owner.querySelectorAll<HTMLElement>(':scope > [data-side]'), (side): string =>
					String(side.dataset['side']),
				).join(' and ')
			: undefined) ??
		owner.querySelector(':scope > figcaption')?.textContent?.trim() ??
		owner.querySelector(':scope > img')?.getAttribute('alt') ??
		plainNames[owner.localName] ??
		owner.localName.replace('ex-', '')
	);
}

/**
 * Gives a picture or diagram its shape for the fit rule in the stylesheet: the proportions, so it
 * can fit the view's height without stretching, and, for a photo or a drawn diagram, its natural
 * width, so it grows to at most twice that size and a small one stays sharp. Vector pictures may
 * grow freely.
 */
function describeShape(visual: HTMLImageElement | SVGSVGElement): void {
	if (visual instanceof HTMLImageElement) {
		if (!visual.complete || visual.naturalHeight === 0) {
			visual.addEventListener('load', (): void => describeShape(visual), { once: true });
			return;
		}
		visual.style.setProperty('--aspect', String(visual.naturalWidth / visual.naturalHeight));
		const vector = /\.svg($|[?#])|^data:image\/svg/i.test(visual.currentSrc || visual.src);
		if (!vector) visual.style.setProperty('--natural-width', `${visual.naturalWidth}px`);
		return;
	}
	const width = Number.parseFloat(visual.getAttribute('width') ?? '');
	const height = Number.parseFloat(visual.getAttribute('height') ?? '');
	if (!(width > 0 && height > 0)) return;
	visual.style.setProperty('--aspect', String(width / height));
	visual.style.setProperty('--natural-width', `${width}px`);
}

interface InspectionContent {
	readonly header: HTMLElement;
	readonly body: HTMLElement;
	readonly restore: () => void;
}

/**
 * A modal temporarily uses the live content; a sidebar gets an independent, interactive copy.
 * Both reveal details without changing the document’s saved disclosure state.
 */
function takeContent(
	owner: HTMLElement,
	controls: HTMLElement,
	label: string,
	close: () => void,
	reserveWidth: boolean,
	expand?: () => void,
): InspectionContent | undefined {
	const contentOwner = reserveWidth ? owner : copyInspectionContent(owner);
	if (!contentOwner) return undefined;
	const originalStyle = owner.getAttribute('style');
	const bounds = owner.getBoundingClientRect();
	const contentNodes = Array.from(contentOwner.childNodes).filter(
		(node): boolean => node !== controls,
	);
	const detailStates = Array.from(
		contentOwner.querySelectorAll<HTMLElement>(':scope > [slot="details"]'),
	).map((element) => ({ element, hidden: element.hidden }));
	const disclosureStates = Array.from(contentOwner.querySelectorAll('details')).map((element) => ({
		element,
		open: element.open,
	}));
	const content = reserveWidth ? owner.cloneNode(false) : contentOwner;
	if (!(content instanceof HTMLElement)) return undefined;
	content.removeAttribute('id');
	content.removeAttribute('style');
	content.removeAttribute('zoom');
	content.removeAttribute('mode');
	content.removeAttribute('span');
	content.removeAttribute('sticky');
	content.classList.remove('ex-inspectable', 'ex-route-focus');
	if (reserveWidth) content.setAttribute('data-inspector-content', '');
	const header = document.createElement('header');
	header.className = 'ex-inspect-header';
	const title = document.createElement('strong');
	title.textContent = label;
	const body = document.createElement('div');
	body.className = 'ex-inspect-body';
	let scale = 1;
	const magnification = document.createElement('span');
	magnification.textContent = '100%';
	const addButton = (
		iconName: string,
		buttonLabel: string,
		action: () => void,
	): HTMLButtonElement => {
		const button = createIconButton(iconName, buttonLabel, action);
		header.append(button);
		return button;
	};
	const updateScale = (nextScale: number): void => {
		scale = Math.max(0.5, Math.min(2.5, nextScale));
		content.style.zoom = String(scale);
		// Pictures fit the view at 100%; zooming lets them grow past it.
		content.style.setProperty('--inspect-scale', String(scale));
		magnification.textContent = `${Math.round(scale * 100)}%`;
	};
	header.append(title);
	addButton('magnifying-glass-minus', 'Zoom out', (): void => updateScale(scale - 0.25));
	header.append(magnification);
	addButton('magnifying-glass-plus', 'Zoom in', (): void => updateScale(scale + 0.25));
	addButton('arrow-counter-clockwise', 'Reset zoom', (): void => {
		updateScale(1);
		body.scrollTo({ left: 0, top: 0 });
	});
	if (expand) addButton('arrows-out', 'Expand', expand);
	const closeButton = addButton('x', 'Close', close);
	closeButton.autofocus = true;
	// The island keeps its place on the page while its content is shown elsewhere.
	if (reserveWidth) {
		owner.style.width = `${bounds.width}px`;
		owner.style.height = `${bounds.height}px`;
	}
	owner.classList.add('ex-inspected');
	const placeholder = document.createElement('p');
	placeholder.className = 'ex-inspect-placeholder';
	placeholder.textContent = `${label} is open in the panel.`;
	if (reserveWidth) content.append(...contentNodes);
	for (const visual of content.querySelectorAll<HTMLImageElement | SVGSVGElement>(
		'img, ex-mermaid > svg[width]',
	))
		describeShape(visual);
	if (reserveWidth) owner.insertBefore(placeholder, controls);
	body.append(content);
	if (content.querySelector('img, svg'))
		installInspectionGestures(body, { readScale: (): number => scale, setScale: updateScale });
	for (const detail of detailStates) detail.element.hidden = false;
	for (const disclosure of disclosureStates) disclosure.element.open = true;
	return {
		header,
		body,
		restore: (): void => {
			placeholder.remove();
			if (!reserveWidth) {
				owner.classList.remove('ex-inspected');
				return;
			}
			for (const node of contentNodes) owner.insertBefore(node, controls);
			for (const detail of detailStates) detail.element.hidden = detail.hidden;
			for (const disclosure of disclosureStates) disclosure.element.open = disclosure.open;
			if (originalStyle === null) owner.removeAttribute('style');
			else owner.setAttribute('style', originalStyle);
			owner.classList.remove('ex-inspected');
			document.dispatchEvent(new Event('island-ready'));
		},
	};
}

/** The large centred view: a modal dialog over the page. */
function openModal(owner: HTMLElement, controls: HTMLElement, mode: InspectionMode): void {
	const label = inspectionLabel(owner);
	const returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
	const dialog = document.createElement('dialog');
	dialog.className = 'ex-inspect-dialog';
	dialog.dataset['mode'] = mode;
	dialog.setAttribute('aria-label', label);
	const inspection = takeContent(
		owner,
		controls,
		label,
		(): void => dialog.close(),
		mode !== 'sidebar',
	);
	if (!inspection) return;
	dialog.append(inspection.header, inspection.body);
	dialog.addEventListener(
		'close',
		(): void => {
			inspection.restore();
			dialog.remove();
			returnFocus?.focus();
		},
		{ once: true },
	);
	document.body.append(dialog);
	dialog.showModal();
}

/** The right drawer on wide screens; a full-width sheet over the page on narrow ones. */
function openSidebar(owner: HTMLElement, controls: HTMLElement): void {
	if (window.innerWidth < 900) {
		openModal(owner, controls, 'sidebar');
		return;
	}
	const label = inspectionLabel(owner);
	openDrawer(label, (close): InspectionContent | undefined =>
		takeContent(owner, controls, label, close, false, (): void => {
			close();
			openModal(owner, controls, 'modal');
		}),
	);
}

function openInspector(owner: HTMLElement, controls: HTMLElement, mode: InspectionMode): void {
	if (mode === 'sidebar') openSidebar(owner, controls);
	else openModal(owner, controls, mode);
}

/**
 * Opens an independent sidebar view or temporarily enlarges the live content in a modal. An
 * island without visible controls (routes and diagram links open plain cards too) gets hidden ones.
 */
export function openIsland(owner: HTMLElement, mode: InspectionMode): void {
	let controls = owner.querySelector<HTMLElement>(':scope > .ex-inspect-controls');
	if (!controls) {
		controls = document.createElement('span');
		controls.className = 'ex-inspect-controls';
		controls.hidden = true;
		owner.append(controls);
	}
	openInspector(owner, controls, mode);
}

/** Adds the island's inspection affordance, if it has one (see affordanceFor). */
export function installIslandInspector(owner: HTMLElement): void {
	if (
		owner.closest('[data-inspector-content], [data-inspector-copy]') ||
		installedInspectors.has(owner)
	)
		return;
	const affordance = affordanceFor(owner);
	if (!affordance) return;
	installedInspectors.add(owner);
	const label = inspectionLabel(owner);
	const controls = document.createElement('span');
	controls.className = 'ex-inspect-controls';
	controls.setAttribute('role', 'group');
	controls.setAttribute('aria-label', `Inspect ${label}`);
	if (affordance === 'more') {
		controls.classList.add('ex-inspect-more');
		const more = createIconButton('caret-right', `More about ${label}`, (): void =>
			openInspector(owner, controls, 'sidebar'),
		);
		more.prepend('More');
		controls.append(more);
		owner.append(controls);
		return;
	}
	owner.classList.add('ex-inspectable');
	const sidebar = createIconButton('sidebar-simple', `Open ${label} in a sidebar`, (): void =>
		openInspector(owner, controls, 'sidebar'),
	);
	const expand = createIconButton('arrows-out', `Expand ${label}`, (): void =>
		openInspector(owner, controls, 'modal'),
	);
	sidebar.classList.add('ex-inspect-trigger');
	expand.classList.add('ex-inspect-trigger');
	controls.append(sidebar, expand);
	owner.append(controls);
}

export class ZoomIsland extends HTMLElement {
	connectedCallback(): void {
		// The inspector's copy shows everything, details included.
		if (this.hasAttribute('data-inspector-content') || this.hasAttribute('data-inspector-copy'))
			return;
		for (const detail of this.querySelectorAll<HTMLElement>(':scope > [slot="details"]'))
			detail.hidden = true;
		installIslandInspector(this);
	}
}
