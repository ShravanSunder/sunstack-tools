import { createIconButton } from '../core/icons.ts';
import { openDrawer } from './drawer.ts';

type InspectionMode = 'modal' | 'sidebar';

const installedInspectors = new WeakSet<HTMLElement>();

/**
 * Which islands get inspection controls. Containers (tabs, grids, flows, boards, regions) never do:
 * they arrange; their content carries its own affordances. Visuals worth enlarging get sidebar and
 * expand buttons; an island whose depth is hidden gets a "More" button; inline <details> are their
 * own affordance; prose, plain cards and stats get nothing. A bare `zoom` attribute forces the
 * buttons and `zoom="none"` removes them.
 */
const visualIslands = new Set(['ex-mermaid', 'ex-code', 'ex-file-tree']);
type Affordance = 'enlarge' | 'more';

function affordanceFor(owner: HTMLElement): Affordance | undefined {
	const zoom = owner.getAttribute('zoom');
	if (zoom === 'none') return undefined;
	if (visualIslands.has(owner.localName)) return 'enlarge';
	if (owner.querySelector(':scope > [slot="details"]')) return 'more';
	return zoom === null ? undefined : 'enlarge';
}

function inspectionLabel(owner: HTMLElement): string {
	return (
		owner.getAttribute('label') ??
		owner.dataset['title'] ??
		(owner.localName === 'ex-tabs'
			? owner.querySelector('[role="tab"][aria-selected="true"]')?.textContent
			: undefined) ??
		owner.querySelector('h1,h2,h3,h4,h5,h6')?.textContent ??
		owner.localName.replace('ex-', '')
	);
}

interface InspectionContent {
	readonly header: HTMLElement;
	readonly body: HTMLElement;
	readonly restore: () => void;
}

/**
 * Moves an island's real content into an inspection shell with zoom controls, revealing hidden
 * details; restore() puts every node and state back where it was.
 */
function takeContent(
	owner: HTMLElement,
	controls: HTMLElement,
	label: string,
	close: () => void,
	reserveWidth: boolean,
): InspectionContent | undefined {
	const originalStyle = owner.getAttribute('style');
	const bounds = owner.getBoundingClientRect();
	const contentNodes = Array.from(owner.childNodes).filter((node): boolean => node !== controls);
	const detailStates = Array.from(
		owner.querySelectorAll<HTMLElement>(':scope > [slot="details"]'),
	).map((element) => ({ element, hidden: element.hidden }));
	const disclosureStates = Array.from(owner.querySelectorAll('details')).map((element) => ({
		element,
		open: element.open,
	}));
	const content = owner.cloneNode(false);
	if (!(content instanceof HTMLElement)) return undefined;
	content.removeAttribute('id');
	content.removeAttribute('style');
	content.removeAttribute('zoom');
	content.removeAttribute('mode');
	content.removeAttribute('span');
	content.removeAttribute('sticky');
	content.classList.remove('ex-inspectable', 'ex-route-focus');
	content.setAttribute('data-inspector-content', '');
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
		magnification.textContent = `${Math.round(scale * 100)}%`;
	};
	header.append(title);
	addButton('magnifying-glass-minus', 'Zoom out', (): void => updateScale(scale - 0.25));
	header.append(magnification);
	addButton('magnifying-glass-plus', 'Zoom in', (): void => updateScale(scale + 0.25));
	addButton('arrow-counter-clockwise', 'Reset zoom', (): void => updateScale(1));
	const closeButton = addButton('x', 'Close', close);
	closeButton.autofocus = true;
	// The island keeps its place on the page while its content is shown elsewhere.
	if (reserveWidth) owner.style.width = `${bounds.width}px`;
	owner.style.height = `${bounds.height}px`;
	owner.classList.add('ex-inspected');
	const placeholder = document.createElement('p');
	placeholder.className = 'ex-inspect-placeholder';
	placeholder.textContent = `${label} is open in the panel.`;
	content.append(...contentNodes);
	owner.insertBefore(placeholder, controls);
	body.append(content);
	for (const detail of detailStates) detail.element.hidden = false;
	for (const disclosure of disclosureStates) disclosure.element.open = true;
	return {
		header,
		body,
		restore: (): void => {
			placeholder.remove();
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
	const inspection = takeContent(owner, controls, label, (): void => dialog.close(), true);
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
		takeContent(owner, controls, label, close, false),
	);
}

function openInspector(owner: HTMLElement, controls: HTMLElement, mode: InspectionMode): void {
	if (mode === 'sidebar') openSidebar(owner, controls);
	else openModal(owner, controls, mode);
}

/**
 * Opens any island in the side panel or the large view; the real content moves and returns. An
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
	if (owner.hasAttribute('data-inspector-content') || installedInspectors.has(owner)) return;
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
		if (this.hasAttribute('data-inspector-content')) return;
		for (const detail of this.querySelectorAll<HTMLElement>(':scope > [slot="details"]'))
			detail.hidden = true;
		installIslandInspector(this);
	}
}
