import { createPhosphorIcon } from '../core/icons.ts';
import {
	navigationTargetHref,
	type SharedMapNode,
	type SharedNavigation,
} from '../core/shared-navigation.ts';
import { MapIsland } from '../islands/map.ts';

/** Shared Maps show relationships in full; the page outline remains independent. */
export function createSharedMap(props: {
	readonly scope: HTMLElement;
	readonly navigation: SharedNavigation;
	readonly sourceUrl: string;
}): MapIsland {
	const { scope, navigation, sourceUrl } = props;
	const content = document.createElement('div');
	content.className = 'ex-navigation-map';
	const search = document.createElement('label');
	search.className = 'ex-navigation-map-search';
	const input = document.createElement('input');
	input.type = 'search';
	input.placeholder = 'Search groups and requirements';
	input.setAttribute('aria-label', input.placeholder);
	search.append(createPhosphorIcon('magnifying-glass'), input);
	const results = document.createElement('div');
	const empty = document.createElement('p');
	empty.className = 'ex-navigation-map-empty';
	empty.textContent = 'No matching groups or requirements.';
	empty.setAttribute('role', 'status');
	content.append(search, results, empty);
	const renderNode = (
		node: SharedMapNode,
		query: string,
		parentMatches: boolean,
	): HTMLLIElement | undefined => {
		const matches = parentMatches || !query || node.label.toLocaleLowerCase().includes(query);
		const children = node.children.flatMap((child): HTMLLIElement[] => {
			const item = renderNode(child, query, matches);
			return item ? [item] : [];
		});
		if (!matches && !children.length) return undefined;
		const item = document.createElement('li');
		const row = node.target ? document.createElement('a') : document.createElement('div');
		row.className = 'ex-navigation-map-row';
		if (row instanceof HTMLAnchorElement && node.target)
			row.href = navigationTargetHref(navigation, node.target, sourceUrl);
		const label = document.createElement('span');
		label.textContent = node.label;
		row.append(createPhosphorIcon(node.icon), label);
		item.append(row);
		if (children.length) {
			const list = document.createElement('ul');
			list.append(...children);
			item.append(list);
		}
		return item;
	};
	const render = (): void => {
		const query = input.value.trim().toLocaleLowerCase();
		const list = document.createElement('ul');
		for (const node of navigation.map) {
			const item = renderNode(node, query, false);
			if (item) list.append(item);
		}
		empty.hidden = list.children.length > 0;
		results.replaceChildren(list);
	};
	input.addEventListener('input', render);
	content.addEventListener('keydown', (event: KeyboardEvent): void => {
		if (event.ctrlKey || event.metaKey || event.altKey || event.isComposing) return;
		const links = Array.from(results.querySelectorAll<HTMLAnchorElement>('a'));
		const current = links.findIndex((link): boolean => link === document.activeElement);
		if (event.target === input) {
			if (event.key === 'ArrowDown' && links[0]) {
				event.preventDefault();
				links[0].focus();
			}
			return;
		}
		const next =
			event.key === 'ArrowDown'
				? Math.min(links.length - 1, current + 1)
				: event.key === 'ArrowUp'
					? Math.max(0, current - 1)
					: event.key === 'Home'
						? 0
						: event.key === 'End'
							? links.length - 1
							: undefined;
		if (next !== undefined && links[next]) {
			event.preventDefault();
			links[next].focus();
		}
	});
	render();
	const map = new MapIsland();
	map.setNavigationContent(scope, content);
	return map;
}
