import { idPrefixGlossary, recordKindGlossary } from '../core/glossary.ts';
import { createPhosphorIcon } from '../core/icons.ts';

type RecordKind = keyof typeof recordKindGlossary;
const prefixes: ReadonlyMap<string, string> = idPrefixGlossary;

interface GlossaryPopup {
	readonly element: HTMLSpanElement;
	trigger?: HTMLButtonElement | undefined;
	pinned: boolean;
}

const popups = new WeakMap<Document, GlossaryPopup>();

function popupFor(document: Document): GlossaryPopup {
	const existing = popups.get(document);
	if (existing) return existing;
	const element = document.createElement('span');
	element.id = `ex-glossary-${crypto.randomUUID()}`;
	element.className = 'ex-glossary-popup';
	element.popover = 'manual';
	element.setAttribute('role', 'tooltip');
	const popup: GlossaryPopup = { element, pinned: false };
	element.addEventListener('toggle', (): void => {
		if (!element.matches(':popover-open')) {
			popup.trigger?.setAttribute('aria-expanded', 'false');
			popup.trigger?.removeAttribute('aria-describedby');
			popup.trigger = undefined;
			popup.pinned = false;
		}
	});
	document.addEventListener('pointerdown', (event): void => {
		const target = event.target;
		if (
			element.matches(':popover-open') &&
			target instanceof Node &&
			!popup.trigger?.contains(target) &&
			!element.contains(target)
		)
			element.hidePopover();
	});
	document.addEventListener('keydown', (event): void => {
		if (event.key !== 'Escape' || !element.matches(':popover-open')) return;
		event.preventDefault();
		element.hidePopover();
	});
	element.addEventListener('pointerleave', (): void => {
		if (
			!popup.pinned &&
			!popup.trigger?.matches(':hover, :focus') &&
			element.matches(':popover-open')
		)
			element.hidePopover();
	});
	document.body.append(element);
	popups.set(document, popup);
	return popup;
}

function showExplanation(
	popup: GlossaryPopup,
	trigger: HTMLButtonElement,
	explanation: string,
): void {
	const { element } = popup;
	if (popup.trigger !== trigger) {
		popup.trigger?.setAttribute('aria-expanded', 'false');
		popup.trigger?.removeAttribute('aria-describedby');
		popup.pinned = false;
	}
	popup.trigger = trigger;
	element.textContent = explanation;
	trigger.setAttribute('aria-describedby', element.id);
	trigger.setAttribute('aria-expanded', 'true');
	if (!element.matches(':popover-open')) element.showPopover();
	const view = trigger.ownerDocument.defaultView;
	if (!view) return;
	const anchor = trigger.getBoundingClientRect();
	const panel = element.getBoundingClientRect();
	const left = Math.max(8, Math.min(anchor.left, view.innerWidth - panel.width - 8));
	const below = anchor.bottom + panel.height + 8 <= view.innerHeight;
	const top = below ? anchor.bottom + 4 : Math.max(8, anchor.top - panel.height - 4);
	element.style.left = `${left}px`;
	element.style.top = `${top}px`;
}

export function glossaryTerm(
	label: string,
	explanation: string,
	marker = false,
): HTMLButtonElement {
	const button = document.createElement('button');
	button.type = 'button';
	button.className = marker ? 'ex-glossary-term ex-glossary-marker' : 'ex-glossary-term';
	if (marker) {
		button.setAttribute('aria-label', `Explain ${label}`);
		button.append(createPhosphorIcon('info'));
	} else button.textContent = label;
	button.setAttribute('aria-expanded', 'false');

	const show = (): GlossaryPopup => {
		const popup = popupFor(button.ownerDocument);
		showExplanation(popup, button, explanation);
		return popup;
	};
	const hide = (force: boolean): void => {
		const popup = popups.get(button.ownerDocument);
		if (
			popup?.trigger === button &&
			(force || !popup.pinned) &&
			popup.element.matches(':popover-open')
		)
			popup.element.hidePopover();
	};

	button.addEventListener('pointerenter', (event): void => {
		if (event.pointerType === 'mouse') show();
	});
	button.addEventListener('pointerleave', (): void => {
		window.setTimeout((): void => {
			const popup = popups.get(button.ownerDocument);
			if (!button.matches(':hover, :focus') && !popup?.element.matches(':hover')) hide(false);
		}, 120);
	});
	button.addEventListener('focus', show);
	button.addEventListener('blur', (): void => queueMicrotask((): void => hide(true)));
	button.addEventListener('click', (): void => {
		const popup = popups.get(button.ownerDocument);
		if (popup?.trigger === button && popup.pinned) hide(true);
		else show().pinned = true;
	});
	return button;
}

export function explainRecord(id: string, kind?: RecordKind): HTMLButtonElement | '' {
	const localId = id.split('/').at(-1) ?? '';
	const prefix = /^[A-Z]+(?=[0-9-])/u.exec(localId)?.[0];
	const explanation = kind ? recordKindGlossary[kind] : prefixes.get(prefix ?? '');
	return explanation ? glossaryTerm(id, explanation, true) : '';
}
