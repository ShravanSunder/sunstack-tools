import { dirname, join } from 'node:path';

import type { DefaultTreeAdapterMap } from 'parse5';

import { classifyDocumentLink } from '../core/document-links.ts';
import {
	sharedNavigationSchema,
	type NavigationTarget,
	type SharedMapNode,
} from '../core/shared-navigation.ts';

interface NavigationProblem {
	readonly severity: 'error' | 'warning';
	readonly message: string;
}

/** Read the same shared contract as the browser, including its actual local destinations. */
export async function checkSharedNavigation(props: {
	readonly reference: string;
	readonly readSource: ((path: string) => Promise<string | undefined>) | undefined;
	readonly idsOnPage: (path: string) => Promise<ReadonlySet<string> | undefined>;
}): Promise<readonly NavigationProblem[]> {
	const { reference, readSource, idsOnPage } = props;
	if (/^[a-z][a-z0-9+.-]*:|^\/\//i.test(reference))
		return [
			{
				severity: 'warning',
				message:
					'Remote shared navigation is validated at runtime; this check reads local sources only.',
			},
		];
	const source = await readSource?.(reference);
	if (source === undefined)
		return [{ severity: 'error', message: `Missing shared navigation source: ${reference}` }];
	let raw: unknown;
	try {
		raw = JSON.parse(source);
	} catch {
		return [{ severity: 'error', message: `Malformed shared navigation JSON: ${reference}` }];
	}
	const parsed = sharedNavigationSchema.safeParse(raw);
	if (!parsed.success)
		return parsed.error.issues.map((issue): NavigationProblem => ({
			severity: 'error',
			message: issue.message,
		}));
	const navigation = parsed.data;
	const pages = await Promise.all(
		navigation.pages.map(async (page) => {
			if (/^[a-z][a-z0-9+.-]*:|^\/\//i.test(page.href))
				return { page, file: undefined, ids: undefined };
			const pathname = decodeURIComponent(page.href.split(/[?#]/)[0] ?? '');
			const file = join(dirname(reference), pathname);
			return { page, file, ids: await idsOnPage(file) };
		}),
	);
	const problems: NavigationProblem[] = [];
	for (const page of pages)
		if (page.file && !page.ids)
			problems.push({ severity: 'error', message: `Missing shared page: ${page.file}` });
	const targets: NavigationTarget[] = [];
	const visit = (node: SharedMapNode): void => {
		if (node.target) targets.push(node.target);
		for (const child of node.children) visit(child);
	};
	for (const node of navigation.map) visit(node);
	for (const route of navigation.walkthroughs)
		for (const step of route.steps) targets.push(step.target);
	const checked = new Set<string>();
	for (const target of targets) {
		const key = `${target.page}#${target.fragment ?? ''}`;
		if (checked.has(key)) continue;
		checked.add(key);
		const page = pages.find((entry): boolean => entry.page.id === target.page);
		if (page?.ids && target.fragment && !page.ids.has(target.fragment))
			problems.push({
				severity: 'error',
				message: `Missing shared navigation target: ${page.file}#${target.fragment}`,
			});
	}
	return problems;
}

type Element = DefaultTreeAdapterMap['element'];

interface TreeMarkupProblem extends NavigationProblem {
	readonly element: Element;
}

function isElement(node: DefaultTreeAdapterMap['node']): node is Element {
	return 'tagName' in node;
}

function attributeMap(element: Element): Map<string, string> {
	return new Map(
		element.attrs.map((attribute): [string, string] => [attribute.name, attribute.value]),
	);
}

export function checkSharedTreeMarkup(
	elements: readonly Element[],
	ids: ReadonlyMap<string, Element>,
	inertTemplateElements: ReadonlySet<Element>,
): TreeMarkupProblem[] {
	const problems: TreeMarkupProblem[] = [];
	const report = (
		element: Element,
		severity: NavigationProblem['severity'],
		message: string,
	): void => {
		problems.push({ element, severity, message });
	};
	const navigationOwners = elements.filter(
		(element): boolean => element.tagName === 'ex-nav' && !inertTemplateElements.has(element),
	);
	const commonScope = elements.find(
		(element): boolean =>
			['ex-document', 'main'].includes(element.tagName) && !inertTemplateElements.has(element),
	);
	const navigationScope = (owner: Element): Element | undefined =>
		ids.get(attributeMap(owner).get('for') ?? '') ?? commonScope;
	const checkTree = (tree: Element): void => {
		const inScope = (element: Element, scope: Element): boolean => {
			for (
				let parent = element.parentNode;
				parent;
				parent = 'parentNode' in parent ? parent.parentNode : null
			)
				if (parent === scope) return true;
			return false;
		};
		const owners = navigationOwners.filter((owner): boolean => {
			const scope = navigationScope(owner);
			return !!scope && inScope(tree, scope);
		});
		const owner = owners[0];
		if (!owner || !attributeMap(owner).has('src') || owners.length !== 1)
			report(tree, 'error', 'A shared Tree needs one shared navigation source for its document.');
		if (
			tree.childNodes.some((child): boolean => {
				if ('value' in child && Boolean(child.value.trim())) return true;
				if (!isElement(child)) return false;
				const attributes = attributeMap(child);
				return (
					child.tagName !== 'a' || attributes.get('slot') !== 'details' || !attributes.has('hidden')
				);
			})
		)
			report(tree, 'error', 'A shared Tree accepts only hidden, local Details links.');

		const fromIds = new Set<string>();
		const localDocument = new URL('https://demistify.invalid/');
		for (const link of tree.childNodes.filter(isElement)) {
			const attributes = attributeMap(link);
			if (link.tagName !== 'a' || attributes.get('slot') !== 'details') continue;
			const from = attributes.get('data-for') ?? '';
			const href = attributes.get('href') ?? '';
			const fromFragment = classifyDocumentLink(from, localDocument);
			const destination = classifyDocumentLink(href, localDocument);
			if (
				!from.startsWith('#') ||
				!href.startsWith('#') ||
				fromFragment.kind !== 'local' ||
				destination.kind !== 'local'
			) {
				report(link, 'error', 'Tree Details require local fragments in data-for and href.');
				continue;
			}
			const target = ids.get(destination.reference);
			if (
				!ids.has(fromFragment.reference) ||
				!target ||
				target === tree ||
				inScope(target, tree) ||
				inScope(tree, target)
			)
				report(link, 'error', 'Tree Details need existing, independent sections.');
			if (fromIds.has(fromFragment.reference))
				report(link, 'error', `Duplicate Tree Details for "#${fromFragment.reference}".`);
			fromIds.add(fromFragment.reference);
		}
	};

	for (const element of elements)
		if (
			element.tagName === 'ex-tree' &&
			attributeMap(element).has('navigation') &&
			!inertTemplateElements.has(element)
		)
			checkTree(element);
	return problems;
}
