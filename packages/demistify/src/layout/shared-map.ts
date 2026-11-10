import { createPhosphorIcon } from '../core/icons.ts';
import {
	navigationTargetHref,
	sharedMapReference,
	type SharedMapNode,
	type SharedNavigation,
} from '../core/shared-navigation.ts';
import { MapIsland } from '../islands/map.ts';

function focusMapRow(row: HTMLElement | undefined): void {
	row?.focus();
	row?.scrollIntoView({ block: 'nearest' });
}

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
	const renderNode = (nodeProps: {
		readonly node: SharedMapNode;
		readonly query: string;
		readonly parentMatches: boolean;
		readonly path: readonly number[];
	}): HTMLLIElement | undefined => {
		const { node, query, parentMatches, path } = nodeProps;
		const ownMatch = `${node.title} ${node.description}`.toLocaleLowerCase().includes(query);
		const matches = parentMatches || !query || ownMatch;
		const children = node.children.flatMap((child, index): HTMLLIElement[] => {
			const item = renderNode({
				node: child,
				query,
				parentMatches: matches,
				path: [...path, index],
			});
			return item ? [item] : [];
		});
		if (!matches && !children.length) return undefined;
		const item = document.createElement('li');
		const row = node.target ? document.createElement('a') : document.createElement('div');
		row.className = 'ex-navigation-map-row';
		row.id = sharedMapReference(path);
		row.tabIndex = node.target ? 0 : -1;
		row.setAttribute('aria-label', node.title);
		row.dataset['match'] = String(ownMatch);
		if (row instanceof HTMLAnchorElement && node.target)
			row.href = navigationTargetHref(navigation, node.target, sourceUrl);
		const copy = document.createElement('span');
		const title = document.createElement('span');
		title.className = 'ex-navigation-map-title';
		title.textContent = node.title;
		const description = document.createElement('span');
		description.className = 'ex-navigation-map-description';
		description.id = `${row.id}-description`;
		description.textContent = node.description;
		row.setAttribute('aria-describedby', description.id);
		copy.append(title, description);
		row.append(createPhosphorIcon(node.icon), copy);
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
		for (const [index, node] of navigation.map.entries()) {
			const item = renderNode({ node, query, parentMatches: false, path: [index] });
			if (item) list.append(item);
		}
		empty.hidden = list.children.length > 0;
		results.replaceChildren(list);
	};
	input.addEventListener('input', render);
	content.addEventListener('keydown', (event: KeyboardEvent): void => {
		if (event.ctrlKey || event.metaKey || event.altKey || event.isComposing) return;
		const rows = Array.from(results.querySelectorAll<HTMLElement>('.ex-navigation-map-row'));
		const current = rows.findIndex((row): boolean => row === document.activeElement);
		if (event.target === input) {
			if (['ArrowDown', 'ArrowUp', 'Enter'].includes(event.key)) {
				event.preventDefault();
				const firstMatch = rows.find((row): boolean => row.dataset['match'] === 'true');
				const row = event.key === 'ArrowUp' ? rows.at(-1) : (firstMatch ?? rows[0]);
				if (event.key === 'Enter' && row instanceof HTMLAnchorElement) row.click();
				else focusMapRow(row);
			}
			return;
		}
		const next =
			event.key === 'ArrowDown'
				? Math.min(rows.length - 1, current + 1)
				: event.key === 'ArrowUp'
					? Math.max(0, current - 1)
					: event.key === 'Home'
						? 0
						: event.key === 'End'
							? rows.length - 1
							: undefined;
		if (next !== undefined && rows[next]) {
			event.preventDefault();
			focusMapRow(rows[next]);
		} else if (event.key === 'ArrowLeft') {
			event.preventDefault();
			focusMapRow(
				rows[current]?.parentElement?.parentElement
					?.closest('li')
					?.querySelector<HTMLElement>(':scope > .ex-navigation-map-row') ?? input,
			);
		} else if (
			event.key === 'ArrowRight' ||
			(event.key === 'Enter' && !(event.target instanceof HTMLAnchorElement))
		) {
			event.preventDefault();
			focusMapRow(
				rows[current]?.parentElement?.querySelector<HTMLElement>(
					':scope > ul .ex-navigation-map-row',
				) ?? undefined,
			);
		}
	});
	render();
	const map = new MapIsland();
	map.setNavigationContent(scope, content, (reference): HTMLElement | undefined => {
		if (!/^ex-shared-map-\d+(?:-\d+)*$/.test(reference)) return undefined;
		input.value = '';
		render();
		return results.querySelector<HTMLElement>(`#${CSS.escape(reference)}`) ?? undefined;
	});
	return map;
}
