import { dirname, relative, resolve, sep } from 'node:path';

import { parse, parseFragment, serialize, type DefaultTreeAdapterMap } from 'parse5';

import { documentSearchIndexSchema, type SearchDestination } from '../core/document-search.ts';
import { headingSlug } from '../core/heading-fragments.ts';
import { parseMarkdownSource } from '../core/markdown-source.ts';
import { renderMarkdownContent } from '../render/markdown-content.ts';

type HtmlNode = DefaultTreeAdapterMap['node'];
type HtmlElement = DefaultTreeAdapterMap['element'];

export interface SearchPageSource {
	readonly filePath: string;
	readonly html: string;
	readonly markdownFiles: Readonly<Record<string, string>>;
}
export interface IndexedSearchPage {
	readonly filePath: string;
	readonly html: string;
	readonly destinations: readonly SearchDestination[];
}
interface LocalDestination extends Pick<
	SearchDestination,
	'kind' | 'title' | 'ancestors' | 'description'
> {
	readonly fragment: string;
}

function elements(root: HtmlNode): HtmlElement[] {
	const found = 'tagName' in root ? [root] : [];
	const children =
		'content' in root ? root.content.childNodes : 'childNodes' in root ? root.childNodes : [];
	for (const child of children) found.push(...elements(child));
	return found;
}
function contentText(root: HtmlNode): string {
	if ('value' in root) return root.value;
	const children =
		'content' in root ? root.content.childNodes : 'childNodes' in root ? root.childNodes : [];
	return children.map(contentText).join('');
}
function attribute(element: HtmlElement, name: string): string {
	return element.attrs.find((item): boolean => item.name === name)?.value ?? '';
}
function setAttribute(element: HtmlElement, name: string, value: string): void {
	const existing = element.attrs.find((item): boolean => item.name === name);
	if (existing) existing.value = value;
	else element.attrs.push({ name, value });
}
function markdownText(element: HtmlElement, sources: Readonly<Record<string, string>>): string {
	const reference = attribute(element, 'src');
	if (reference) {
		const source = sources[reference];
		if (source === undefined) throw new Error(`Missing Markdown source for search: ${reference}`);
		return source;
	}
	const retained = element.childNodes.find(
		(child): child is HtmlElement =>
			'tagName' in child &&
			child.tagName === 'script' &&
			attribute(child, 'type') === 'application/json' &&
			child.attrs.some((item): boolean => item.name === 'data-ex-markdown-source'),
	);
	if (retained) return parseMarkdownSource(contentText(retained)).text;
	const carrier = element.childNodes.find(
		(child): child is HtmlElement =>
			'tagName' in child &&
			(child.tagName === 'template' ||
				(child.tagName === 'script' &&
					['text/markdown', 'text/plain'].includes(attribute(child, 'type')))),
	);
	return carrier ? contentText(carrier) : '';
}

function pageOutline(source: SearchPageSource): {
	readonly source: SearchPageSource;
	readonly document: DefaultTreeAdapterMap['document'];
	readonly pageTitle: string;
	readonly destinations: readonly LocalDestination[];
} {
	const document = parse(source.html);
	const nodes = elements(document);
	const usedIds = new Set(nodes.map((node): string => attribute(node, 'id')).filter(Boolean));
	const pageHeading = nodes.find((node): boolean => node.tagName === 'h1');
	const titleElement = nodes.find((node): boolean => node.tagName === 'title');
	const title = (
		pageHeading ? contentText(pageHeading) : titleElement ? contentText(titleElement) : ''
	).trim();
	if (!title) throw new Error(`Search page needs a title: ${source.filePath}`);
	const destinations: LocalDestination[] = [{ kind: 'page', title, ancestors: [], fragment: '' }];
	const parents: { level: number; title: string }[] = [];
	let firstTitle = true;
	const addHeading = (heading: HtmlElement, scope: string): void => {
		const headingTitle = contentText(heading).trim();
		if (!headingTitle) return;
		let fragment = attribute(heading, 'id');
		if (fragment && nodes.filter((node): boolean => attribute(node, 'id') === fragment).length > 1)
			throw new Error(`Ambiguous heading id ${fragment} in ${source.filePath}.`);
		if (!fragment) {
			const base = scope
				? `${scope}--${headingSlug(headingTitle)}`
				: `heading-${headingSlug(headingTitle)}`;
			fragment = base;
			let suffix = 1;
			while (usedIds.has(fragment)) fragment = `${base}-${++suffix}`;
			setAttribute(heading, 'id', fragment);
			usedIds.add(fragment);
		}
		const level = Number(heading.tagName.slice(1));
		while ((parents.at(-1)?.level ?? 0) >= level) parents.pop();
		if (level === 1 && firstTitle) {
			firstTitle = false;
			destinations[0] = { kind: 'page', title, ancestors: [], fragment };
			return;
		}
		destinations.push({
			kind: 'heading',
			title: headingTitle,
			ancestors: parents.map((parent): string => parent.title),
			fragment,
		});
		parents.push({ level, title: headingTitle });
	};
	const walk = (node: HtmlNode): void => {
		if (!('tagName' in node)) return;
		if (
			['figcaption', 'ex-gallery', 'ex-presentation', 'script', 'template', 'svg'].includes(
				node.tagName,
			)
		)
			return;
		if (node.tagName === 'ex-markdown') {
			if (node.attrs.some((item): boolean => item.name === 'prepared')) {
				for (const heading of elements(node).filter((child): boolean =>
					/^h[1-6]$/.test(child.tagName),
				))
					addHeading(heading, attribute(node, 'id'));
				return;
			}
			const rendered = parseFragment(
				renderMarkdownContent(markdownText(node, source.markdownFiles)),
			);
			const headings = elements(rendered).filter((child): boolean =>
				/^h[1-6]$/.test(child.tagName),
			);
			if (headings.length && !attribute(node, 'id'))
				throw new Error(
					`Give the Markdown heading island a stable id before indexing ${source.filePath}.`,
				);
			for (const heading of headings) addHeading(heading, attribute(node, 'id'));
			return;
		}
		if (/^h[1-6]$/.test(node.tagName)) addHeading(node, '');
		for (const child of node.childNodes) {
			if ('tagName' in child) walk(child);
			else if (
				'value' in child &&
				['ex-document', 'section', 'article', 'ex-card', 'ex-callout', 'details'].includes(
					node.tagName,
				)
			) {
				if (
					elements(parseFragment(renderMarkdownContent(child.value))).some((element): boolean =>
						/^h[1-6]$/.test(element.tagName),
					)
				)
					throw new Error(
						`Wrap searchable Markdown headings in an ex-markdown with a stable id in ${source.filePath}.`,
					);
			}
		}
	};
	const body = nodes.find((node): boolean => node.tagName === 'body');
	if (!body) throw new Error('Search page has no body.');
	walk(body);
	for (const presentation of nodes.filter((node): boolean => node.tagName === 'ex-presentation')) {
		const fragment = attribute(presentation, 'id');
		if (!fragment) throw new Error(`Give the walkthrough a stable id in ${source.filePath}.`);
		const stops = elements(presentation).filter((node): boolean => node.tagName === 'ex-step');
		const pictured =
			stops.length > 0 &&
			stops.every((step): boolean =>
				nodes.some(
					(target): boolean =>
						attribute(target, 'id') === attribute(step, 'to') &&
						['figure', 'img', 'ex-gallery'].includes(target.tagName),
				),
			);
		destinations.push({
			kind: pictured ? 'journey' : 'walkthrough',
			title: attribute(presentation, 'title') || fragment,
			description: attribute(presentation, 'description'),
			ancestors: [],
			fragment,
		});
	}
	return { source, document, pageTitle: title, destinations };
}

/** Generate every output before writing: a missing source never leaves half an indexed collection. */
export function indexCollectionSearch(
	pages: readonly SearchPageSource[],
): readonly IndexedSearchPage[] {
	const outlines = pages.map(pageOutline);
	return outlines.map((page): IndexedSearchPage => {
		const destinations = documentSearchIndexSchema.parse(
			outlines.flatMap((other): SearchDestination[] => {
				const file = relative(
					dirname(resolve(page.source.filePath)),
					resolve(other.source.filePath),
				)
					.split(sep)
					.map(encodeURIComponent)
					.join('/');
				return other.destinations.map((entry): SearchDestination => ({
					kind: entry.kind,
					title: entry.title,
					...(entry.description === undefined ? {} : { description: entry.description }),
					pageTitle: other.pageTitle,
					ancestors: entry.ancestors,
					href: `${file}${entry.fragment ? `#${encodeURIComponent(entry.fragment)}` : ''}`,
				}));
			}),
		);
		const body = elements(page.document).find((node): boolean => node.tagName === 'body');
		if (!body) throw new Error('Search page has no body.');
		for (const old of elements(page.document).filter(
			(node): boolean =>
				node.tagName === 'script' &&
				node.attrs.some((item): boolean => item.name === 'data-ex-search-index'),
		))
			if (old.parentNode)
				old.parentNode.childNodes = old.parentNode.childNodes.filter(
					(child): boolean => child !== old,
				);
		const payload = JSON.stringify(destinations).replaceAll('<', '\\u003c');
		const carrier = parseFragment(
			`<script type="application/json" data-ex-search-index>${payload}</script>`,
		);
		const script = carrier.childNodes.find(
			(child): child is HtmlElement => 'tagName' in child && child.tagName === 'script',
		);
		if (!script) throw new Error('Cannot embed search index.');
		script.parentNode = body;
		body.childNodes.push(script);
		return { filePath: page.source.filePath, html: serialize(page.document), destinations };
	});
}
