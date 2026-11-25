import { parse, type DefaultTreeAdapterMap } from 'parse5';

/**
 * What the move reads from an HTML page: record bindings, links, ids, and the units
 * a passage can move as: a heading section, or one ex-section-item.
 */
export type PageElement = DefaultTreeAdapterMap['element'];
type PageNode = DefaultTreeAdapterMap['node'];

/** One HTML file of the initiative, with the project whose bare references it uses. */
export interface PageSource {
	readonly file: string;
	readonly text: string;
	/** Project namespace of the page; none for a page outside every project. */
	readonly owner: string | undefined;
}

export interface Binding {
	readonly kind: 'placement' | 'status' | 'tree';
	readonly reference: string;
	readonly element: PageElement;
	readonly attribute: string;
	readonly start: number;
	readonly end: number;
}

interface UrlAttribute {
	readonly name: string;
	readonly value: string;
	readonly start: number;
	readonly end: number;
}

export interface ParsedPage extends PageSource {
	readonly bindings: readonly Binding[];
	readonly urls: readonly UrlAttribute[];
	readonly ids: ReadonlyMap<string, PageElement>;
	readonly anchors: Set<string>;
	readonly insertAt: number | undefined;
	readonly sectionHolds: ReadonlySet<string>;
}

export interface PageUnit {
	readonly kind: 'heading-section' | 'section-item';
	/** The passage without its section wrapper, tried when the wrapper's id clashes. */
	readonly inner?: { readonly start: number; readonly end: number };
	readonly page: string;
	readonly start: number;
	readonly end: number;
	readonly heading: string;
}

const terminators = new Set([
	'ex-deliverables',
	'ex-initiative-overview',
	'ex-nav',
	'ex-section',
	'ex-section-item',
	'h1',
]);
const wrappers = new Set(['section', 'article']);
const containers = new Set(['body', 'ex-document', 'main', 'article', 'html']);

function headingLevel(element: PageElement): number | undefined {
	const match = /^h([1-6])$/u.exec(element.tagName);
	return match ? Number(match[1]) : undefined;
}

function isElement(node: PageNode): node is PageElement {
	return 'tagName' in node;
}

export function attributeValue(element: PageElement, name: string): string | undefined {
	return element.attrs.find((attribute) => attribute.name === name)?.value;
}

function textOf(node: PageNode): string {
	if ('value' in node && typeof node.value === 'string' && node.nodeName === '#text')
		return node.value;
	return 'childNodes' in node ? node.childNodes.map(textOf).join('') : '';
}

export function parsePage(page: PageSource): ParsedPage {
	const document = parse(page.text, { sourceCodeLocationInfo: true });
	const bindings: Binding[] = [];
	const urls: UrlAttribute[] = [];
	const ids = new Map<string, PageElement>();
	const sectionHolds = new Set<string>();
	let documentEnd: number | undefined;
	let deliverablesStart: number | undefined;
	let bodyEnd: number | undefined;
	const visit = (node: PageNode): void => {
		if (isElement(node)) {
			const location = node.sourceCodeLocation;
			const span = (name: string): { start: number; end: number } | undefined => {
				const attribute = location?.attrs?.[name];
				return attribute ? { start: attribute.startOffset, end: attribute.endOffset } : undefined;
			};
			const bind = (kind: Binding['kind'], attribute: string): void => {
				const value = attributeValue(node, attribute);
				const range = span(attribute);
				if (value !== undefined && range)
					bindings.push({ kind, reference: value.trim(), element: node, attribute, ...range });
			};
			if (node.tagName === 'ex-section-item') bind('placement', 'record');
			bind('placement', 'data-design-record');
			bind('status', 'data-design-status');
			if (node.tagName === 'ex-tree') bind('tree', 'data-design-need');
			for (const name of ['href', 'src']) {
				const value = attributeValue(node, name);
				const range = span(name);
				if (value !== undefined && range) urls.push({ name, value, ...range });
			}
			const id = attributeValue(node, 'id');
			if (id) ids.set(id, node);
			if (node.tagName === 'ex-section') {
				const holds = attributeValue(node, 'holds');
				if (holds) sectionHolds.add(holds.trim());
			}
			if (node.tagName === 'ex-document' && documentEnd === undefined)
				documentEnd = location?.endTag?.startOffset;
			if (node.tagName === 'ex-deliverables' && deliverablesStart === undefined)
				deliverablesStart = location?.startOffset;
			if (node.tagName === 'body') bodyEnd = location?.endTag?.startOffset;
			if (node.tagName === 'template' && 'content' in node) visit(node.content);
		}
		if ('childNodes' in node) node.childNodes.forEach(visit);
	};
	visit(document);
	const anchors = new Set(ids.keys());
	for (const binding of bindings) if (binding.kind === 'placement') anchors.add(binding.reference);
	return {
		...page,
		bindings,
		urls,
		ids,
		anchors,
		insertAt: deliverablesStart ?? documentEnd ?? bodyEnd,
		sectionHolds,
	};
}

/** The heading section around an element: its nearest heading and what follows to the next. */
export function headingSection(page: ParsedPage, element: PageElement): PageUnit | undefined {
	let node: PageElement = element;
	for (;;) {
		const parent = node.parentNode;
		if (!parent || !('childNodes' in parent)) return undefined;
		const siblings = parent.childNodes.filter(isElement);
		const index = siblings.indexOf(node);
		for (let position = index; position >= 0; position -= 1) {
			const sibling = siblings[position];
			if (!sibling) break;
			if (position !== index && terminators.has(sibling.tagName) && sibling.tagName !== 'h1')
				return undefined;
			const level = headingLevel(sibling);
			if (level === undefined) continue;
			if (level === 1) return undefined;
			const next = siblings.slice(position + 1).find((candidate) => {
				const candidateLevel = headingLevel(candidate);
				return (
					(candidateLevel !== undefined && candidateLevel <= level) ||
					terminators.has(candidate.tagName)
				);
			});
			const heading = textOf(sibling).replace(/\s+/gu, ' ').trim();
			const start = sibling.sourceCodeLocation?.startOffset;
			const end =
				next?.sourceCodeLocation?.startOffset ??
				(isElement(parent) ? parent.sourceCodeLocation?.endTag?.startOffset : undefined);
			if (start === undefined || end === undefined) return undefined;
			// A section element that holds only this heading's passage moves whole, with its id.
			const wrapper =
				isElement(parent) && wrappers.has(parent.tagName) ? parent.sourceCodeLocation : undefined;
			if (wrapper && position === 0 && !next) {
				const trailing = /^\s*/u.exec(page.text.slice(wrapper.endOffset))?.[0].length ?? 0;
				return {
					kind: 'heading-section',
					page: page.file,
					start: wrapper.startOffset,
					end: wrapper.endOffset + trailing,
					heading,
					inner: { start, end },
				};
			}
			return { kind: 'heading-section', page: page.file, start, end, heading };
		}
		if (!isElement(parent) || containers.has(parent.tagName)) return undefined;
		node = parent;
	}
}

export function itemUnit(page: ParsedPage, element: PageElement): PageUnit | undefined {
	const location = element.sourceCodeLocation;
	if (!location) return undefined;
	return {
		kind: 'section-item',
		page: page.file,
		start: location.startOffset,
		// Take the whitespace after it too, so the page keeps no empty line.
		end: location.endOffset + (/^\s*/u.exec(page.text.slice(location.endOffset))?.[0].length ?? 0),
		heading: `ex-section-item ${attributeValue(element, 'record') ?? ''}`.trim(),
	};
}
