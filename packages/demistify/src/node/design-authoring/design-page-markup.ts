import { parse, type DefaultTreeAdapterMap } from 'parse5';

import type { DesignRecord } from './schema.ts';

interface AuthoredFragment {
	readonly source: string;
	readonly heading?: true;
	readonly designItem: string | undefined;
	readonly authority: string | undefined;
}

export interface PageMarkup {
	ids: Map<string, AuthoredFragment>;
	links: string[];
	navigationSources: string[];
	deliverySources: string[];
	decisionChoices: { id: string; reference: string; title: string }[];
	hasDocumentBase: boolean;
}

export function readPageMarkup(html: string, file: string): PageMarkup {
	const result: PageMarkup = {
		ids: new Map(),
		links: [],
		navigationSources: [],
		deliverySources: [],
		decisionChoices: [],
		hasDocumentBase: false,
	};
	const sections = new Set(['article', 'aside', 'body', 'ex-document', 'main', 'nav', 'section']);
	interface Heading {
		readonly rank: number;
		readonly id: string | undefined;
		readonly start: number;
	}
	const groups: { headings: Heading[]; end: number }[] = [];
	const preformattedText: { start: number; end: number }[] = [];
	function visit(
		node: DefaultTreeAdapterMap['node'],
		headings: Heading[],
		inPreformattedText = false,
	): void {
		let currentHeadings = headings;
		const preservesWhitespace =
			inPreformattedText ||
			('tagName' in node && ['pre', 'listing', 'textarea'].includes(node.tagName));
		if (preservesWhitespace && 'value' in node && node.sourceCodeLocation) {
			const { startOffset, endOffset } = node.sourceCodeLocation;
			preformattedText.push({ start: startOffset, end: endOffset });
		}
		if ('attrs' in node) {
			const get = (name: string): string | undefined =>
				node.attrs.find((attribute): boolean => attribute.name === name)?.value;
			const id = get('id');
			const href = get('href');
			const location = node.sourceCodeLocation;
			if (sections.has(node.tagName)) {
				currentHeadings = [];
				groups.push({
					headings: currentHeadings,
					end: location?.endTag?.startOffset ?? location?.endOffset ?? html.length,
				});
			}
			if (/^h[1-6]$/.test(node.tagName) && location)
				currentHeadings.push({
					rank: Number(node.tagName.slice(1)),
					id,
					start: location.startOffset,
				});
			if (id) {
				if (result.ids.has(id)) throw new Error(`Duplicate HTML id "${id}" in ${file}.`);
				const source = node.sourceCodeLocation;
				result.ids.set(id, {
					source: source ? html.slice(source.startOffset, source.endOffset) : '',
					designItem: get('data-design-item'),
					authority: get('data-authority'),
				});
			}
			if (href) result.links.push(href);
			if (node.tagName === 'base' && href !== undefined) result.hasDocumentBase = true;
			if (node.tagName === 'ex-nav') result.navigationSources.push(get('src') ?? '');
			if (node.tagName === 'ex-deliverables') result.deliverySources.push(get('src') ?? '');
			if (node.tagName === 'ex-choice') {
				const reference = get('data-decision');
				if (reference !== undefined)
					result.decisionChoices.push({
						id: id ?? '',
						reference: reference.trim(),
						title: get('title') ?? '',
					});
			}
		}
		if ('childNodes' in node) {
			for (const child of node.childNodes) visit(child, currentHeadings, preservesWhitespace);
		}
	}
	const documentHeadings: Heading[] = [];
	groups.push({ headings: documentHeadings, end: html.length });
	visit(parse(html, { sourceCodeLocationInfo: true }), documentHeadings);

	for (const { headings, end } of groups) {
		const nextByRank = Array.from({ length: 6 }, (): number => end);
		for (const { rank, id, start } of headings.toReversed()) {
			const fragment = id ? result.ids.get(id) : undefined;
			if (id && fragment) {
				const passageEnd = Math.min(...nextByRank.slice(0, rank));
				const preservesWhitespace = preformattedText.some(
					(text): boolean => text.start < passageEnd && text.end >= passageEnd,
				);
				result.ids.set(id, {
					...fragment,
					source: html.slice(start, passageEnd),
					...(preservesWhitespace ? {} : { heading: true }),
				});
			}
			nextByRank[rank - 1] = start;
		}
	}
	return result;
}

const textBoundaryTags = new Set([
	'address',
	'article',
	'aside',
	'blockquote',
	'br',
	'div',
	'h1',
	'h2',
	'h3',
	'h4',
	'h5',
	'h6',
	'li',
	'p',
	'section',
	'td',
	'th',
	'tr',
]);
const excludedTextTags = new Set(['head', 'script', 'style', 'template']);

export function normalizeExcerpt(text: string): string {
	return text.replace(/\s+/gu, ' ').trim();
}

export function passageText(html: string): string {
	const parts: string[] = [];
	function visit(node: DefaultTreeAdapterMap['node']): void {
		if ('value' in node) parts.push(node.value);
		if ('tagName' in node && excludedTextTags.has(node.tagName)) return;
		const boundary = 'tagName' in node && textBoundaryTags.has(node.tagName);
		if (boundary) parts.push(' ');
		if ('childNodes' in node) node.childNodes.forEach(visit);
		if (boundary) parts.push(' ');
	}
	visit(parse(html));
	return normalizeExcerpt(parts.join(''));
}

export function checkDesignItemMarkup(
	record: DesignRecord,
	fragment: AuthoredFragment | undefined,
): void {
	if (!fragment || (record.kind !== 'choice' && record.kind !== 'assumption')) return;
	const itemKind = record.kind === 'choice' ? 'decision' : 'assumption';
	if (fragment.designItem !== undefined && fragment.designItem !== itemKind)
		throw new Error(
			`${record.id}: expected data-design-item="${itemKind}" on its HTML destination.`,
		);

	if (record.kind !== 'choice' || fragment.authority === undefined) return;
	const { authority } = record;
	const expected = authority.state === 'owner-open' ? 'unknown' : authority.sourceKind;
	if (fragment.authority !== expected)
		throw new Error(`${record.id}: expected data-authority="${expected}" on its HTML destination.`);
}
