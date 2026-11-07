import { type DefaultTreeAdapterMap, parseFragment } from 'parse5';

import { normalizeIslandText } from '../core/source-text.ts';
import { renderMarkdownContent } from '../render/markdown-content.ts';

type AuthoredElement = DefaultTreeAdapterMap['element'];

export interface MapMarkupProblem {
	readonly element: AuthoredElement;
	readonly rule: 'map' | 'fragment';
	readonly message: string;
	readonly severity?: 'error' | 'warning';
}

function childrenOf(element: AuthoredElement): AuthoredElement[] {
	return element.childNodes.filter((child): child is AuthoredElement => 'tagName' in child);
}

function descendants(element: AuthoredElement): AuthoredElement[] {
	return childrenOf(element).flatMap((child): AuthoredElement[] =>
		[child].concat(descendants(child)),
	);
}

function attribute(element: AuthoredElement, name: string): string | undefined {
	return element.attrs.find((candidate): boolean => candidate.name === name)?.value;
}

function isTemplate(element: AuthoredElement): element is DefaultTreeAdapterMap['template'] {
	return element.tagName === 'template';
}

function textOf(element: AuthoredElement): string {
	const nodes = isTemplate(element) ? element.content.childNodes : element.childNodes;
	return nodes
		.map((node): string => ('value' in node ? node.value : 'tagName' in node ? textOf(node) : ''))
		.join('');
}

/** A static document has no deployment URL: only fragment-only hrefs prove same-page identity. */
function fragment(
	href: string,
):
	| { readonly kind: 'local'; readonly id: string }
	| { readonly kind: 'invalid' }
	| { readonly kind: 'other' } {
	if (!href.startsWith('#') || href === '#') return { kind: 'other' };
	try {
		return { kind: 'local', id: decodeURIComponent(href.slice(1)) };
	} catch {
		return { kind: 'invalid' };
	}
}

/** Check authored Map structure and decidable local links before runtime changes the tree. */
export function checkMapMarkup(
	elements: readonly AuthoredElement[],
	targets: ReadonlySet<string>,
): MapMarkupProblem[] {
	const problems: MapMarkupProblem[] = [];
	const maps = elements.filter((element): boolean => element.tagName === 'ex-map');
	if (!maps.length && !elements.some((element): boolean => element.tagName === 'ex-story'))
		return problems;
	const checkLink = (element: AuthoredElement, owner = element): void => {
		const href = attribute(element, 'href');
		if (href === undefined) return;
		const destination = fragment(href);
		if (
			destination.kind === 'invalid' ||
			(destination.kind === 'local' && !targets.has(destination.id))
		)
			problems.push({
				element: owner,
				rule: 'fragment',
				message:
					destination.kind === 'invalid'
						? `Malformed local fragment "${href}".`
						: `Missing local target "${destination.id}".`,
			});
	};
	for (const anchor of elements.filter((element): boolean => element.tagName === 'a'))
		checkLink(anchor);
	for (const map of maps) {
		const report = (message: string): void => {
			problems.push({ element: map, rule: 'map', message });
		};
		if (maps.length > 1) report('A page has at most one Map.');
		const rootId = attribute(map, 'for');
		const root = rootId
			? elements.find((element): boolean => attribute(element, 'id') === rootId)
			: undefined;
		if (!root || root === map) report('Map needs an existing page scope id.');
		const children = childrenOf(map);
		const tree = children[0];
		if (children.length !== 1 || tree?.tagName !== 'ex-tree') {
			report('Map needs exactly one authored ex-tree.');
			continue;
		}
		const wrapper = childrenOf(tree).find(
			(child): boolean => child.tagName === 'script' || child.tagName === 'template',
		);
		const source = wrapper ? textOf(wrapper) : childrenOf(tree).length === 0 ? textOf(tree) : '';
		const rendered = parseFragment(renderMarkdownContent(normalizeIslandText(source)));
		const list = rendered.childNodes.find(
			(node): node is AuthoredElement =>
				'tagName' in node && (node.tagName === 'ul' || node.tagName === 'ol'),
		);
		if (!list) {
			report('Map Tree needs an authored Markdown list.');
			continue;
		}
		const links = descendants(list).filter((element): boolean => element.tagName === 'a');
		const hasDeploymentDependentLinks = links.some(
			(link): boolean => fragment(attribute(link, 'href') ?? '').kind === 'other',
		);
		for (const link of links) checkLink(link, tree);
		if (!root) continue;
		for (const section of descendants(root).filter(
			(element): boolean => element.tagName === 'ex-section',
		)) {
			const id = attribute(section, 'id');
			const branches = childrenOf(list).filter((item): boolean =>
				childrenOf(item)
					.filter((child): boolean => child.tagName !== 'ul' && child.tagName !== 'ol')
					.flatMap((child): AuthoredElement[] => [child].concat(descendants(child)))
					.some((link): boolean => {
						if (link.tagName !== 'a') return false;
						const destination = fragment(attribute(link, 'href') ?? '');
						return destination.kind === 'local' && destination.id === id;
					}),
			);
			if (branches.length === 0 && hasDeploymentDependentLinks)
				problems.push({
					element: map,
					rule: 'map',
					severity: 'warning',
					message: `Cannot establish the branch for section "${id ?? ''}" without the deployed document URL; use its fragment-only href for a static check.`,
				});
			else if (branches.length !== 1)
				report(`Map needs one top-level linked branch for section "${id ?? ''}".`);
			for (const branch of branches) {
				const label = branch.childNodes
					.filter(
						(node): boolean =>
							!('tagName' in node) || (node.tagName !== 'ul' && node.tagName !== 'ol'),
					)
					.map((node): string =>
						'value' in node ? node.value : 'tagName' in node ? textOf(node) : '',
					)
					.join('')
					.trim();
				if (/(?:\s[·•]\s*\d+|\s\(\d+\))$/.test(label))
					report(
						`Omit the authored total on section "${id ?? ''}"; Map derives it from membership.`,
					);
			}
		}
	}
	return problems;
}
