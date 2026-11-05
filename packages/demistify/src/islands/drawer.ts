interface DrawerContent {
	readonly header: HTMLElement;
	readonly body: HTMLElement;
	readonly restore: () => void;
}
interface DrawerSession {
	readonly identity: symbol;
	readonly label: string;
	readonly content: DrawerContent;
	readonly panel: HTMLElement;
	readonly returnFocus: HTMLElement | null;
}
const minimumWidth = 320;
const defaultWidth = 460;
let drawer: HTMLElement | undefined;
let current: DrawerSession | undefined;
const bottomDrawers: DrawerIsland[] = [];

function hidePresentation(panel: HTMLElement): void {
	if (panel instanceof HTMLDialogElement) {
		if (panel.open) panel.close();
		panel.remove();
	} else {
		panel.hidden = true;
		panel.replaceChildren(panel.firstElementChild ?? '');
	}
}

function showPresentation(session: DrawerSession): void {
	const { panel, content, identity, label } = session;
	panel.setAttribute('aria-label', label);
	panel.append(content.header, content.body);
	panel.hidden = false;
	if (panel instanceof HTMLDialogElement) {
		panel.className = 'ex-inspect-dialog';
		panel.dataset['mode'] = 'sidebar';
		panel.addEventListener(
			'close',
			(): void => {
				if (current?.identity === identity && current.panel === panel) closeDrawer();
			},
			{ once: true },
		);
		document.body.append(panel);
		panel.showModal();
	}
}

/** Rehome the same live session when the viewport crosses the native sheet boundary. */
function adaptPresentation(): void {
	const session = current;
	if (!session || session.panel instanceof HTMLDialogElement === window.innerWidth < 900) return;
	const focused = document.activeElement instanceof HTMLElement ? document.activeElement : null;
	const panel =
		window.innerWidth < 900 ? document.createElement('dialog') : (drawer ??= createDrawer());
	const next = { ...session, panel };
	current = next;
	showPresentation(next);
	hidePresentation(session.panel);
	if (focused?.isConnected) focused.focus({ preventScroll: true });
}
window.addEventListener('resize', adaptPresentation);

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
	hidePresentation(session.panel);
	session.content.restore();
	if (session.returnFocus?.isConnected) {
		const trigger = session.returnFocus;
		const visibleTarget =
			trigger.getClientRects().length > 0
				? trigger
				: trigger
						.closest('.ex-inspect-controls')
						?.querySelector<HTMLElement>('.ex-inspect-trigger:not(.ex-inspect-sidebar)');
		visibleTarget?.focus({ preventScroll: true });
	}
	window.dispatchEvent(new Event('resize'));
}

export function isDrawerOpen(): boolean {
	return current !== undefined || bottomDrawers.length > 0;
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
	const session: DrawerSession = { identity, label, content, panel, returnFocus };
	current = session;
	document.documentElement.dataset['drawer'] = '';
	showPresentation(session);
	window.dispatchEvent(new Event('resize'));
	content.header
		.querySelector<HTMLButtonElement>('button:last-of-type')
		?.focus({ preventScroll: true });
	return close;
}

window.addEventListener(
	'keydown',
	(event: KeyboardEvent): void => {
		if (event.key !== 'Escape' || event.defaultPrevented || document.querySelector('dialog:modal'))
			return;
		const bottom = bottomDrawers.at(-1);
		if (current && (!bottom || current.panel.contains(document.activeElement))) closeDrawer();
		else return;
		event.preventDefault();
		event.stopPropagation();
	},
	{ capture: true },
);

// Bottom workspaces let nested controls consume Escape before closing their container.
window.addEventListener('keydown', (event: KeyboardEvent): void => {
	if (event.key !== 'Escape' || event.defaultPrevented || document.querySelector('dialog:modal'))
		return;
	const bottom = bottomDrawers.at(-1);
	if (!bottom) return;
	bottom.close();
	event.preventDefault();
	event.stopPropagation();
});

/** A live, non-modal workspace using the same drawer presentation as Comments and inspection. */
export class DrawerIsland extends HTMLElement {
	private returnFocus: HTMLElement | null = null;
	private reflectingOpen = false;
	private initialized = false;

	static get observedAttributes(): string[] {
		return ['open'];
	}

	get open(): boolean {
		return this.hasAttribute('open');
	}

	connectedCallback(): void {
		this.classList.add('ex-drawer');
		this.popover = 'manual';
		this.setAttribute('role', 'dialog');
		if (!this.hasAttribute('aria-label'))
			this.setAttribute('aria-label', this.getAttribute('label') ?? 'Details');
		this.initialized = true;
		if (this.open) this.show();
		else this.hidden = true;
	}

	attributeChangedCallback(): void {
		if (!this.isConnected || !this.initialized || this.reflectingOpen) return;
		if (this.open) this.show();
		else this.close();
	}

	disconnectedCallback(): void {
		this.close();
		this.initialized = false;
	}

	show(): void {
		if (!this.isConnected || !this.initialized || bottomDrawers.includes(this)) return;
		// Leave an ordinary modal enlargement before opening an interactive workspace.
		const modal = this.closest<HTMLDialogElement>('dialog:modal');
		if (modal) {
			const trigger = document.activeElement instanceof HTMLElement ? document.activeElement : null;
			modal.addEventListener(
				'close',
				(): void => {
					this.show();
					this.returnFocus = trigger;
				},
				{ once: true },
			);
			modal.close();
			return;
		}
		if (this.getAttribute('size') !== 'content')
			for (const existing of bottomDrawers.toReversed()) existing.close();
		this.returnFocus =
			document.activeElement instanceof HTMLElement ? document.activeElement : null;
		bottomDrawers.push(this);
		this.hidden = false;
		this.reflectingOpen = true;
		this.setAttribute('open', '');
		this.reflectingOpen = false;
		this.showPopover();
		this.querySelector<HTMLElement>('[autofocus],button')?.focus({ preventScroll: true });
		window.dispatchEvent(new Event('resize'));
	}

	close(): void {
		const index = bottomDrawers.indexOf(this);
		if (index < 0) return;
		for (const nested of bottomDrawers.slice(index + 1).toReversed()) nested.close();
		bottomDrawers.splice(index, 1);
		if (this.matches(':popover-open')) this.hidePopover();
		this.hidden = true;
		this.reflectingOpen = true;
		this.removeAttribute('open');
		this.reflectingOpen = false;
		if (this.returnFocus?.isConnected) this.returnFocus.focus({ preventScroll: true });
		this.dispatchEvent(new Event('close'));
		window.dispatchEvent(new Event('resize'));
	}
}
