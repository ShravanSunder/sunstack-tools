import { createIconButton } from '../core/icons.ts';
import { openDrawer } from './drawer.ts';

type InspectionMode = 'modal' | 'sidebar';

const installedInspectors = new WeakSet<HTMLElement>();

/** Content islands get inspection controls by default; layout containers and the document do not. */
const contentIslands = new Set([
	'ex-markdown',
	'ex-mermaid',
	'ex-card',
	'ex-callout',
	'ex-code',
	'ex-file-tree',
	'ex-tabs',
	'ex-zoom',
	'ex-decision',
	'ex-compare',
	'ex-diagram-diff',
	'ex-timeline',
	'ex-layer',
]);

/** Surfaces whose own inspector already shows the Markdown placed directly inside them. */
const textContainers = new Set(['ex-zoom', 'ex-card', 'ex-callout', 'ex-decision', 'details']);

/**
 * A Markdown island gains nothing from its own controls when it holds only headings, or when it
 * sits directly in a surface that is inspected as a whole (its controls would cover the surface's).
 */
function hasInspectableDepth(owner: HTMLElement): boolean {
	if (owner.localName !== 'ex-markdown') return true;
	if (textContainers.has(owner.parentElement?.localName ?? '')) return false;
	return Array.from(owner.children).some((block): boolean => !/^h[1-6]$/.test(block.localName));
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

/** Opens any island in a right sidebar or a large centred view; the real content moves and returns. */
export function openIsland(owner: HTMLElement, mode: InspectionMode): void {
	const controls = owner.querySelector<HTMLElement>(':scope > .ex-inspect-controls');
	if (controls) openInspector(owner, controls, mode);
}

/** Adds sidebar and expand controls to a content island unless it opts out with zoom="none". */
export function installIslandInspector(owner: HTMLElement, always: boolean = false): void {
	if (
		owner.getAttribute('zoom') === 'none' ||
		owner.hasAttribute('data-inspector-content') ||
		installedInspectors.has(owner) ||
		(!always && (!contentIslands.has(owner.localName) || !hasInspectableDepth(owner)))
	)
		return;
	installedInspectors.add(owner);
	owner.classList.add('ex-inspectable');
	const label = inspectionLabel(owner);
	const controls = document.createElement('span');
	controls.className = 'ex-inspect-controls';
	controls.setAttribute('role', 'group');
	controls.setAttribute('aria-label', `Inspect ${label}`);
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
		installIslandInspector(this, true);
		for (const detail of this.querySelectorAll<HTMLElement>(':scope > [slot="details"]'))
			detail.hidden = true;
	}
}
