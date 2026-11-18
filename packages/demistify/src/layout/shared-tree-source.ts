import {
	navigationTargetHref,
	type NavigationTarget,
	type SharedMapNode,
} from '../core/shared-navigation.ts';
import { walkthroughAddressKeys } from './route.ts';
import type { LoadedNavigation } from './shared-navigation.ts';

type SharedTreeSource =
	| { readonly kind: 'ready'; readonly source: LoadedNavigation }
	| { readonly kind: 'unavailable'; readonly message: string };

type SourceListener = (source: SharedTreeSource) => void;

interface SourceBinding {
	readonly listeners: Set<SourceListener>;
	current?: SharedTreeSource;
}

const sourceBindings = new WeakMap<HTMLElement, SourceBinding>();

function bindingFor(owner: HTMLElement): SourceBinding {
	const binding = sourceBindings.get(owner) ?? { listeners: new Set<SourceListener>() };
	sourceBindings.set(owner, binding);
	return binding;
}

export function navigationContentRoot(owner: Element): HTMLElement {
	const reference = owner.getAttribute('for');
	return (
		(reference ? document.getElementById(reference) : null) ??
		document.querySelector<HTMLElement>('ex-document, main') ??
		document.body
	);
}

/** The matching navigation owner already loads and validates the shared source. */
export function subscribeToSharedTreeSource(
	tree: HTMLElement,
	listen: SourceListener,
): (() => void) | undefined {
	const scope = tree.closest('ex-document, main');
	const owners = Array.from(document.querySelectorAll<HTMLElement>('ex-nav')).filter(
		(owner): boolean => {
			const content = navigationContentRoot(owner);
			return !!scope && content.closest('ex-document, main') === scope && content.contains(tree);
		},
	);
	const owner = owners[0];
	if (!owner?.hasAttribute('src') || owners.length !== 1) {
		listen({
			kind: 'unavailable',
			message: 'This Tree needs one shared navigation source for its document.',
		});
		return undefined;
	}
	const binding = bindingFor(owner);
	binding.listeners.add(listen);
	if (binding.current) listen(binding.current);
	return (): void => {
		binding.listeners.delete(listen);
	};
}

export function publishSharedTreeSource(owner: HTMLElement, current: SharedTreeSource): void {
	const binding = bindingFor(owner);
	binding.current = current;
	for (const listen of binding.listeners) listen(current);
}

function pageAddress(address: URL): string {
	address.hash = '';
	for (const key of walkthroughAddressKeys) address.searchParams.delete(key);
	return address.href;
}

function selectPageEntries(nodes: readonly SharedMapNode[], pageId: string): SharedMapNode[] {
	return nodes.flatMap((node): SharedMapNode[] => {
		const children = selectPageEntries(node.children, pageId);
		return node.target?.page === pageId || children.length ? [{ ...node, children }] : [];
	});
}

export function sharedTreeEntries(
	source: LoadedNavigation,
	mode: 'page' | 'collection',
): readonly SharedMapNode[] | undefined {
	if (mode === 'collection') return source.navigation.map;
	const here = pageAddress(new URL(document.URL));
	const pages = source.navigation.pages.filter(
		(page): boolean => pageAddress(new URL(page.href, source.sourceUrl)) === here,
	);
	const page = pages[0];
	return page && pages.length === 1 ? selectPageEntries(source.navigation.map, page.id) : undefined;
}

/** Keep local fragments local when this document is open inside a Walkthrough. */
export function sharedTreeHref(source: LoadedNavigation, target: NavigationTarget): string {
	const destination = navigationTargetHref(source.navigation, target, source.sourceUrl);
	const address = new URL(destination);
	return address.hash && pageAddress(new URL(address)) === pageAddress(new URL(document.URL))
		? address.hash
		: destination;
}
