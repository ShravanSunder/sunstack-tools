import { createIconButton, createPhosphorIcon } from './icons.ts';
import { storyDisclosures } from './story-content.ts';

export type DisclosureAction =
	| { readonly kind: 'empty' }
	| { readonly kind: 'expand'; readonly members: readonly HTMLDetailsElement[] }
	| { readonly kind: 'collapse'; readonly members: readonly HTMLDetailsElement[] };

export function readDisclosureAction(root: HTMLElement): DisclosureAction {
	const stories = [...root.querySelectorAll('ex-story')];
	if (root.localName === 'ex-story') stories.unshift(root);
	const members = new Set(stories.flatMap(storyDisclosures));
	for (const group of root.querySelectorAll('ex-section > details[slot="smaller"]'))
		if (group instanceof HTMLDetailsElement) members.add(group);
	const current = [...members];
	if (!current.length) return { kind: 'empty' };
	return {
		kind: current.every((member): boolean => member.open) ? 'collapse' : 'expand',
		members: current,
	};
}

export function applyDisclosureAction(root: HTMLElement, action: DisclosureAction): void {
	if (action.kind === 'empty') return;
	const current = readDisclosureAction(root);
	if (current.kind === 'empty') return;
	for (const member of current.members) member.open = action.kind === 'expand';
}

export interface ScopeControl {
	readonly button: HTMLButtonElement;
	readonly disconnect: () => void;
}

/** Native state is re-read on activation and on native changes shared with other scope controls. */
export function createDisclosureControl(root: HTMLElement, name: string): ScopeControl {
	let paintedAction: DisclosureAction['kind'] | undefined;
	const refresh = (): void => {
		const action = readDisclosureAction(root);
		if (action.kind === paintedAction) return;
		paintedAction = action.kind;
		const label = `${action.kind === 'collapse' ? 'Collapse' : 'Expand'} ${name}`;
		button.disabled = action.kind === 'empty';
		button.setAttribute('aria-label', label);
		button.title = label;
		button.replaceChildren(
			createPhosphorIcon(
				action.kind === 'collapse' ? 'caret-circle-double-up' : 'caret-circle-double-down',
			),
		);
	};
	const button = createIconButton('caret-circle-double-down', `Expand ${name}`, (): void => {
		applyDisclosureAction(root, readDisclosureAction(root));
		refresh();
	});
	button.classList.add('ex-fold-control');
	const observer = new MutationObserver(refresh);
	observer.observe(root, {
		subtree: true,
		childList: true,
		attributes: true,
		attributeFilter: ['open'],
	});
	root.addEventListener('toggle', refresh, true);
	refresh();
	return {
		button,
		disconnect: (): void => {
			observer.disconnect();
			root.removeEventListener('toggle', refresh, true);
		},
	};
}
