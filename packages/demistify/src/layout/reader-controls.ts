type ReaderControlKind = 'walkthrough' | 'theme' | 'search' | 'pages' | 'comments' | 'map';
interface ReaderRegistration {
	readonly kind: ReaderControlKind;
	readonly identity: symbol;
}

const controlOrder: readonly ReaderControlKind[] = [
	'pages',
	'search',
	'theme',
	'comments',
	'walkthrough',
	'map',
];
const controls = new Map<HTMLButtonElement, ReaderRegistration>();
let host: HTMLElement | undefined;
let modalHost: HTMLElement | undefined;
let toolbarHost: { readonly element: HTMLElement; readonly identity: symbol } | undefined;

function controlHost(): HTMLElement {
	if (host) return host;
	host = document.createElement('div');
	host.className = 'ex-reader-controls';
	host.setAttribute('role', 'group');
	host.setAttribute('aria-label', 'Reader controls');
	document.body.append(host);
	return host;
}

/** Move the same controls into the document header, preserving their listeners and Tab order. */
export function registerReaderToolbar(element: HTMLElement): () => void {
	const identity = Symbol('reader toolbar');
	toolbarHost = { element, identity };
	const row = controlHost();
	element.insertBefore(row, element.querySelector('.ex-document-breadcrumbs'));
	return (): void => {
		if (toolbarHost?.identity !== identity) return;
		toolbarHost = undefined;
		document.body.append(row);
	};
}

function placeControls(): void {
	const row = controlHost();
	let previous: HTMLButtonElement | undefined;
	for (const kind of controlOrder)
		for (const [button, registration] of controls) {
			if (registration.kind !== kind) continue;
			if (kind === 'map' && modalHost) {
				const close = modalHost.querySelector(':scope > button:last-of-type');
				if (button.parentElement !== modalHost || button.nextElementSibling !== close)
					modalHost.insertBefore(button, close);
			} else {
				if (previous) {
					if (button.previousElementSibling !== previous) previous.after(button);
				} else if (row.firstElementChild !== button) row.prepend(button);
				previous = button;
			}
		}
	row.hidden = controls.size === 0;
}

/** Keep original live controls; a stale teardown cannot remove a newer registration. */
export function registerReaderControl(
	kind: ReaderControlKind,
	button: HTMLButtonElement,
): () => void {
	const identity = Symbol(kind);
	controls.set(button, { kind, identity });
	placeControls();
	return (): void => {
		if (controls.get(button)?.identity !== identity) return;
		controls.delete(button);
		button.remove();
		placeControls();
	};
}

/** Sidebar placement is detached before removal and attached after a standard phone sheet opens. */
export function setModalControlHost(header: HTMLElement | undefined): void {
	modalHost = header;
	placeControls();
}
