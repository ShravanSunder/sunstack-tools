import type { SearchDestination } from '../core/document-search.ts';
import {
	navigationTargetHref,
	sharedMapReference,
	sharedNavigationSchema,
	type SharedMapNode,
	type SharedNavigation,
} from '../core/shared-navigation.ts';
import { installRoutes, walkthroughAddressKeys, type Route } from './route.ts';
import { createSharedMap } from './shared-map.ts';

export interface LoadedNavigation {
	readonly navigation: SharedNavigation;
	readonly sourceUrl: string;
}

/** Search preserves the relationship path; selecting a group reveals its expanded Map row. */
export function sharedMapDestinations(source: LoadedNavigation): readonly SearchDestination[] {
	const entries: SearchDestination[] = [];
	const visit = (
		nodes: readonly SharedMapNode[],
		path: readonly number[],
		ancestors: readonly string[],
	): void => {
		for (const [index, node] of nodes.entries()) {
			const position = [...path, index];
			entries.push({
				kind: 'map',
				title: node.title,
				description: node.description,
				icon: node.icon,
				pageTitle: 'Map',
				ancestors,
				href:
					node.target && !node.children.length
						? navigationTargetHref(source.navigation, node.target, source.sourceUrl)
						: `#${sharedMapReference(position)}`,
			});
			visit(node.children, position, [...ancestors, node.title]);
		}
	};
	visit(source.navigation.map, [], []);
	return entries;
}

/** Load once at the source boundary; every control receives the same validated meaning. */
export async function loadSharedNavigation(
	reference: string,
	signal: AbortSignal,
): Promise<LoadedNavigation> {
	if (!reference.trim()) throw new Error('Shared navigation needs a JSON source URL.');
	const address = new URL(reference, document.baseURI);
	const response = await fetch(address, { signal, cache: 'no-cache' });
	if (!response.ok) throw new Error(`Navigation request failed: HTTP ${response.status}`);
	const raw: unknown = await response.json();
	const parsed = sharedNavigationSchema.safeParse(raw);
	if (!parsed.success)
		throw new Error(parsed.error.issues.map((issue): string => issue.message).join('; '));
	return { navigation: parsed.data, sourceUrl: response.url || address.href };
}

export function sharedPageLinks(source: LoadedNavigation): readonly HTMLAnchorElement[] {
	return source.navigation.pages.map((page): HTMLAnchorElement => {
		const link = document.createElement('a');
		link.href = new URL(page.href, source.sourceUrl).href;
		link.textContent = page.title;
		link.setAttribute('icon', page.icon);
		return link;
	});
}

export function sharedWalkthroughDestinations(
	source: LoadedNavigation,
): readonly SearchDestination[] {
	const home = source.navigation.pages[0];
	if (!home) return [];
	return source.navigation.walkthroughs.map((route): SearchDestination => {
		const first = route.steps[0];
		if (!first) throw new Error('A Walkthrough needs a first stop.');
		const address = new URL(
			navigationTargetHref(source.navigation, first.target, source.sourceUrl),
		);
		address.searchParams.set('ex-tour', new URL(home.href, source.sourceUrl).href);
		address.searchParams.set('ex-route', route.id);
		address.searchParams.set('ex-step', '0');
		return {
			kind: 'walkthrough',
			title: route.title,
			description: route.description,
			pageTitle: home.title,
			ancestors: [],
			href: address.href,
		};
	});
}

/** Reuse the existing player and drawer; JSON does not introduce another navigation engine. */
export function mountSharedNavigation(source: LoadedNavigation, scope: HTMLElement): () => void {
	const { navigation, sourceUrl } = source;
	const home = navigation.pages[0];
	if (!home) throw new Error('Shared navigation needs a home page.');
	const homeUrl = new URL(home.href, sourceUrl).href;
	const here = new URL(location.href);
	here.hash = '';
	for (const key of walkthroughAddressKeys) here.searchParams.delete(key);
	const routes: readonly Route[] = navigation.walkthroughs.map((route): Route => ({
		id: route.id,
		label: route.title,
		description: route.description,
		home: homeUrl,
		variations: false,
		steps: route.steps.map((step) => {
			const address = new URL(navigationTargetHref(navigation, step.target, sourceUrl));
			const fragment = decodeURIComponent(address.hash.slice(1));
			address.hash = '';
			const samePage = address.href === here.href;
			return {
				page: samePage ? undefined : address.href,
				targetId: fragment,
				narration: step.narration,
				open: undefined,
				icon: step.icon,
			};
		}),
	}));
	const map = navigation.map.length ? createSharedMap({ scope, navigation, sourceUrl }) : undefined;
	if (map) scope.append(map);
	const disposeRoutes = installRoutes(routes);
	return (): void => {
		map?.remove();
		disposeRoutes();
	};
}
