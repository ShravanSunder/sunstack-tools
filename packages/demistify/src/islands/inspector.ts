import { createIconButton } from '../core/icons.ts';

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
		owner.querySelector('h1,h2,h3,h4,h5,h6')?.textContent ??
		owner.localName.replace('ex-', '')
	);
}

function openInspector(owner: HTMLElement, controls: HTMLElement, mode: InspectionMode): void {
	const label = inspectionLabel(owner);
	const returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
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
	const dialog = document.createElement('dialog');
	dialog.className = 'ex-inspect-dialog';
	dialog.dataset['mode'] = mode;
	dialog.setAttribute('aria-label', label);
	const header = document.createElement('header');
	header.className = 'ex-inspect-header';
	const title = document.createElement('strong');
	title.textContent = label;
	const body = document.createElement('div');
	body.className = 'ex-inspect-body';
	const content = owner.cloneNode(false);
	if (!(content instanceof HTMLElement)) return;
	content.removeAttribute('id');
	content.removeAttribute('style');
	content.removeAttribute('zoom');
	content.removeAttribute('mode');
	content.classList.remove('ex-inspectable');
	content.setAttribute('data-inspector-content', '');
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
	const closeButton = addButton('x', 'Close', (): void => dialog.close());
	closeButton.autofocus = true;
	owner.style.width = `${bounds.width}px`;
	owner.style.height = `${bounds.height}px`;
	content.append(...contentNodes);
	body.append(content);
	dialog.append(header, body);
	dialog.addEventListener(
		'close',
		(): void => {
			for (const node of contentNodes) owner.insertBefore(node, controls);
			for (const detail of detailStates) detail.element.hidden = detail.hidden;
			for (const disclosure of disclosureStates) disclosure.element.open = disclosure.open;
			if (originalStyle === null) owner.removeAttribute('style');
			else owner.setAttribute('style', originalStyle);
			dialog.remove();
			returnFocus?.focus();
			document.dispatchEvent(new Event('island-ready'));
		},
		{ once: true },
	);
	document.body.append(dialog);
	for (const detail of detailStates) detail.element.hidden = false;
	for (const disclosure of disclosureStates) disclosure.element.open = true;
	dialog.showModal();
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
		installIslandInspector(this, true);
		for (const detail of this.querySelectorAll<HTMLElement>(':scope > [slot="details"]'))
			detail.hidden = true;
	}
}
