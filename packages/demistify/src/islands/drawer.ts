interface DrawerContent {
	readonly header: HTMLElement;
	readonly body: HTMLElement;
	readonly restore: () => void;
}
interface DrawerSession {
	readonly identity: symbol;
	readonly content: DrawerContent;
	readonly panel: HTMLElement;
	readonly returnFocus: HTMLElement | null;
}
const minimumWidth = 320;
const defaultWidth = 460;
let drawer: HTMLElement | undefined;
let current: DrawerSession | undefined;

function setWidth(width: number): void {
	const clamped = Math.round(Math.max(minimumWidth, Math.min(width, window.innerWidth * 0.7)));
	document.documentElement.style.setProperty('--ex-drawer-width', `${clamped}px`);
}

/** Restores the active sidebar session, including a narrow native sheet. */
export function closeDrawer(): void {
	const session = current;
	if (!session) return;
	current = undefined;
	delete document.documentElement.dataset['drawer'];
	if (session.panel instanceof HTMLDialogElement) {
		if (session.panel.open) session.panel.close();
		session.panel.remove();
	} else {
		session.panel.hidden = true;
		session.panel.replaceChildren(session.panel.firstElementChild ?? '');
	}
	session.content.restore();
	if (session.returnFocus?.isConnected) session.returnFocus.focus({ preventScroll: true });
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

/** One replacement owner for a docked sidebar and the native phone sheet. */
export function openDrawer(
	label: string,
	take: (close: () => void) => DrawerContent | undefined,
): () => void {
	const active = document.activeElement instanceof HTMLElement ? document.activeElement : null;
	const returnFocus = active && current?.panel.contains(active) ? current.returnFocus : active;
	closeDrawer();
	const panel =
		window.innerWidth < 900 ? document.createElement('dialog') : (drawer ??= createDrawer());
	const identity = Symbol('sidebar-session');
	const close = (): void => {
		if (current?.identity === identity) closeDrawer();
	};
	const content = take(close);
	if (!content) return close;
	const session: DrawerSession = { identity, content, panel, returnFocus };
	current = session;
	panel.setAttribute('aria-label', label);
	panel.append(content.header, content.body);
	panel.hidden = false;
	document.documentElement.dataset['drawer'] = '';
	if (panel instanceof HTMLDialogElement) {
		panel.className = 'ex-inspect-dialog';
		panel.dataset['mode'] = 'sidebar';
		panel.addEventListener('close', close, { once: true });
		document.body.append(panel);
		panel.showModal();
	}
	window.dispatchEvent(new Event('resize'));
	content.header
		.querySelector<HTMLButtonElement>('button:last-of-type')
		?.focus({ preventScroll: true });
	return close;
}
