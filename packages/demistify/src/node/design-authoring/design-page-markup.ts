import { parse, type DefaultTreeAdapterMap } from 'parse5';

interface MarkupRange {
	readonly start: number;
	readonly end: number;
}

export interface RecordBinding {
	readonly reference: string;
	readonly tag: string;
	readonly startTag: MarkupRange;
	readonly endTag: MarkupRange;
}

export interface PageMarkup {
	readonly ids: Set<string>;
	readonly links: string[];
	readonly navigationSources: string[];
	readonly deliverySources: string[];
	readonly placements: RecordBinding[];
	readonly trees: RecordBinding[];
	hasDocumentBase: boolean;
}

const recordTextTags = new Set([
	'p',
	'span',
	'div',
	'h1',
	'h2',
	'h3',
	'h4',
	'h5',
	'h6',
	'li',
	'dt',
	'dd',
	'td',
	'th',
	'blockquote',
	'ex-choice',
]);

export function hasDesignBindings(html: string): boolean {
	const generatedSources = new Map([
		['ex-nav', 'navigation.generated.json'],
		['ex-deliverables', 'deliverables.generated.json'],
		['ex-project-overview', 'project.generated.json'],
	]);
	function visit(node: DefaultTreeAdapterMap['node']): boolean {
		if ('attrs' in node) {
			const source = generatedSources.get(node.tagName);
			for (const { name, value } of node.attrs) {
				if (
					name === 'data-design-record' ||
					(name === 'data-design-need' && node.tagName === 'ex-tree')
				)
					return true;
				if (name !== 'src' || !source) continue;
				const href = value.trim();
				if (!/^(?:[a-z][a-z0-9+.-]*:|\/\/)/i.test(href) && href.split('/').at(-1) === source)
					return true;
			}
		}
		return 'childNodes' in node && node.childNodes.some(visit);
	}
	return visit(parse(html));
}

export function readPageMarkup(html: string, file: string): PageMarkup {
	const result: PageMarkup = {
		ids: new Set(),
		links: [],
		navigationSources: [],
		deliverySources: [],
		placements: [],
		trees: [],
		hasDocumentBase: false,
	};
	function visit(node: DefaultTreeAdapterMap['node']): void {
		if ('attrs' in node) {
			const get = (name: string): string | undefined =>
				node.attrs.find(({ name: attribute }): boolean => attribute === name)?.value;
			const id = get('id');
			const reference = get('data-design-record');
			const need = get('data-design-need');
			const anchor = reference ?? id;
			if (anchor) {
				if (result.ids.has(anchor))
					throw new Error(`Duplicate HTML id or record placement "${anchor}" in ${file}.`);
				result.ids.add(anchor);
			}
			if (reference !== undefined || need !== undefined) {
				const location = node.sourceCodeLocation;
				const tag = node.tagName;
				if (!location?.startTag || !location.endTag)
					throw new Error(`${file}: <${tag}> bindings need explicit closing tags.`);
				const { startTag, endTag } = location;
				const range = (part: typeof startTag): MarkupRange => ({
					start: part.startOffset,
					end: part.endOffset,
				});
				if (reference !== undefined) {
					if (!recordTextTags.has(tag))
						throw new Error(`${file}: <${tag}> cannot hold a record placement.`);
					if (
						id !== undefined ||
						(tag === 'ex-choice' &&
							(get('title') !== undefined || get('data-decision') !== undefined))
					)
						throw new Error(`${file}: ${reference} supplies its own anchor and Choice question.`);
					if (tag !== 'ex-choice' && html.slice(startTag.endOffset, endTag.startOffset).trim())
						throw new Error(`${file}: record placement ${reference} must be empty.`);
					result.placements.push({
						reference,
						tag,
						startTag: range(startTag),
						endTag: range(endTag),
					});
				}
				if (need !== undefined) {
					if (
						tag !== 'ex-tree' ||
						reference !== undefined ||
						get('navigation') !== undefined ||
						get('title') !== undefined ||
						html.slice(startTag.endOffset, endTag.startOffset).trim()
					)
						throw new Error(`${file}: add data-design-need to an empty ex-tree.`);
					result.trees.push({
						reference: need,
						tag,
						startTag: range(startTag),
						endTag: range(endTag),
					});
				}
			}
			const href = get('href');
			if (href) result.links.push(href);
			if (node.tagName === 'base' && href !== undefined) result.hasDocumentBase = true;
			if (node.tagName === 'ex-nav') result.navigationSources.push(get('src') ?? '');
			if (node.tagName === 'ex-deliverables') result.deliverySources.push(get('src') ?? '');
		}
		if ('childNodes' in node) node.childNodes.forEach(visit);
	}
	visit(parse(html, { sourceCodeLocationInfo: true }));
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

type PageNode = DefaultTreeAdapterMap['node'];
type PageElement = DefaultTreeAdapterMap['element'];

function elementId(element: PageElement): string | undefined {
	return element.attrs.find(({ name }) => name === 'id' || name === 'data-design-record')?.value;
}

function sectionOwner(element: PageElement): PageElement | undefined {
	const sections = new Set(['article', 'aside', 'body', 'ex-document', 'main', 'nav', 'section']);
	let parent = element.parentNode;
	while (parent) {
		if ('tagName' in parent && sections.has(parent.tagName)) return parent;
		parent = 'parentNode' in parent ? parent.parentNode : null;
	}
	return undefined;
}

function fragmentMarkup(html: string, fragment: string): string {
	const elements: PageElement[] = [];
	function collect(node: PageNode): void {
		if ('attrs' in node) elements.push(node);
		if ('childNodes' in node) node.childNodes.forEach(collect);
	}
	collect(parse(html, { sourceCodeLocationInfo: true }));
	const target = elements.find((element) => elementId(element) === fragment);
	const location = target?.sourceCodeLocation;
	if (!target || !location) return '';
	if (!/^h[1-6]$/.test(target.tagName)) return html.slice(location.startOffset, location.endOffset);
	const owner = sectionOwner(target);
	const containingEnd = owner?.sourceCodeLocation?.endTag?.startOffset ?? html.length;
	const next = elements.find(
		(element) =>
			/^h[1-6]$/.test(element.tagName) &&
			element.tagName <= target.tagName &&
			(element.sourceCodeLocation?.startOffset ?? -1) > location.startOffset &&
			sectionOwner(element) === owner,
	);
	return html.slice(location.startOffset, next?.sourceCodeLocation?.startOffset ?? containingEnd);
}

export function passageText(html: string, fragment?: string): string {
	const parts: string[] = [];
	function visit(node: PageNode): void {
		if ('tagName' in node && excludedTextTags.has(node.tagName)) return;
		if ('value' in node) parts.push(node.value);
		const boundary = 'tagName' in node && textBoundaryTags.has(node.tagName);
		if (boundary) parts.push(' ');
		if ('childNodes' in node) node.childNodes.forEach(visit);
		if (boundary) parts.push(' ');
	}
	visit(parse(fragment === undefined ? html : fragmentMarkup(html, fragment)));
	return normalizeExcerpt(parts.join(''));
}
