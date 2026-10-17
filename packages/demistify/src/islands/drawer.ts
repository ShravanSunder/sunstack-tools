interface DrawerContent {
	readonly header: HTMLElement;
	readonly body: HTMLElement;
	readonly restore: () => void;
}

const minimumWidth = 320;
const defaultWidth = 460;

let drawer: HTMLElement | undefined;
let current: { readonly restore: () => void; readonly returnFocus: HTMLElement | null } | undefined;

function setWidth(width: number): void {
	const clamped = Math.round(Math.max(minimumWidth, Math.min(width, window.innerWidth * 0.7)));
	document.documentElement.style.setProperty('--ex-drawer-width', `${clamped}px`);
}

/** Closes the drawer and returns its island's content to the page. */
export function closeDrawer(): void {
	if (!drawer || !current) return;
	const { restore, returnFocus } = current;
	current = undefined;
	drawer.hidden = true;
	drawer.replaceChildren(drawer.firstElementChild ?? '');
	delete document.documentElement.dataset['drawer'];
	restore();
	returnFocus?.focus({ preventScroll: true });
	window.dispatchEvent(new Event('resize'));
}

export function isDrawerOpen(): boolean {
	return current !== undefined;
}

function createDrawer(): HTMLElement {
	const panel = document.createElement('aside');
	panel.className = 'ex-drawer';
	panel.hidden = true;
	const handle = document.createElement('div');
	handle.className = 'ex-drawer-handle';
	handle.tabIndex = 0;
	handle.setAttribute('role', 'separator');
	handle.setAttribute('aria-orientation', 'vertical');
	handle.setAttribute('aria-label', 'Resize panel');
	handle.addEventListener('pointerdown', (event: PointerEvent): void => {
		handle.setPointerCapture(event.pointerId);
		document.documentElement.dataset['resizing'] = '';
	});
	handle.addEventListener('pointermove', (event: PointerEvent): void => {
		if (handle.hasPointerCapture(event.pointerId)) setWidth(window.innerWidth - event.clientX);
	});
	handle.addEventListener('pointerup', (event: PointerEvent): void => {
		handle.releasePointerCapture(event.pointerId);
		delete document.documentElement.dataset['resizing'];
		window.dispatchEvent(new Event('resize'));
	});
	handle.addEventListener('keydown', (event: KeyboardEvent): void => {
		const width = panel.getBoundingClientRect().width;
		if (event.key === 'ArrowLeft') setWidth(width + 32);
		else if (event.key === 'ArrowRight') setWidth(width - 32);
		else return;
		event.preventDefault();
		window.dispatchEvent(new Event('resize'));
	});
	panel.append(handle);
	// Escape closes the drawer first, before any route or page handler sees it.
	window.addEventListener(
		'keydown',
		(event: KeyboardEvent): void => {
			if (event.key !== 'Escape' || !current || document.querySelector('dialog[open]')) return;
			event.stopPropagation();
			closeDrawer();
		},
		{ capture: true },
	);
	document.body.append(panel);
	if (!document.documentElement.style.getPropertyValue('--ex-drawer-width')) setWidth(defaultWidth);
	return panel;
}

/**
 * Shows content in a docked panel that takes space: the page narrows beside it and keeps
 * scrolling, the panel's left edge resizes it, and opening another island replaces it.
 */
export function openDrawer(
	label: string,
	take: (close: () => void) => DrawerContent | undefined,
): void {
	const returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
	closeDrawer();
	drawer ??= createDrawer();
	const content = take(closeDrawer);
	if (!content) return;
	current = { restore: content.restore, returnFocus };
	drawer.setAttribute('aria-label', label);
	drawer.append(content.header, content.body);
	drawer.hidden = false;
	document.documentElement.dataset['drawer'] = '';
	window.dispatchEvent(new Event('resize'));
	content.header
		.querySelector<HTMLButtonElement>('button:last-of-type')
		?.focus({ preventScroll: true });
}
