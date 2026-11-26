import { parse, type DefaultTreeAdapterMap } from 'parse5';

import {
	decisionDetailsId,
	migratedOptionAnchor,
	migratedOptionDetailsId,
} from '../../core/design-record-view.ts';
import { readSectionLayout, sectionMarkupProblem, type SectionLayout } from './section-layout.ts';

interface MarkupRange {
	readonly start: number;
	readonly end: number;
}

export interface RecordBinding {
	readonly reference: string;
	readonly tag: string;
	readonly startTag: MarkupRange;
	readonly endTag: MarkupRange;
	readonly status?: true;
	readonly options?: readonly ChoiceOptionBinding[];
}

interface ChoiceOptionBinding extends MarkupRange {
	readonly id: string;
	readonly anchor?: string;
	readonly detailsAnchor: string;
}

export interface PageMarkup {
	readonly ids: Set<string>;
	readonly sectionLayout: SectionLayout;
	readonly links: string[];
	readonly navigationSources: string[];
	readonly deliverySources: string[];
	readonly projectOverviewSources: string[];
	readonly projectDetailsSources: string[];
	readonly recordSourceMarkers: string[];
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

export function designPreviewSources(html: string): readonly string[] | undefined {
	const generatedSources = new Map([
		['ex-nav', 'navigation.generated.json'],
		['ex-deliverables', 'deliverables.generated.json'],
		['ex-initiative-overview', 'initiative.generated.json'],
		['ex-project-overview', 'project-overview.generated.json'],
	]);
	let bound = false;
	const sources = new Set<string>();
	function visit(node: DefaultTreeAdapterMap['node']): void {
		if ('attrs' in node) {
			const source = generatedSources.get(node.tagName);
			if (
				node.tagName === 'meta' &&
				node.attrs.some(({ name, value }) => name === 'name' && value === 'demistify-records')
			) {
				sources.add(node.attrs.find(({ name }) => name === 'content')?.value ?? '');
			}
			for (const { name, value } of node.attrs) {
				if (
					(node.tagName === 'ex-section-item' && name === 'record') ||
					name === 'data-design-record' ||
					name === 'data-design-status' ||
					(name === 'data-design-need' && node.tagName === 'ex-tree')
				)
					bound = true;
				if (name !== 'src' || !source) continue;
				const href = value.trim();
				if (!/^(?:[a-z][a-z0-9+.-]*:|\/\/)/i.test(href) && href.split('/').at(-1) === source) {
					sources.add(href);
					bound = true;
				}
			}
		}
		if ('childNodes' in node) node.childNodes.forEach(visit);
	}
	visit(parse(html));
	return bound ? [...sources] : undefined;
}

export function readPageMarkup(html: string, file: string): PageMarkup {
	const elements: DefaultTreeAdapterMap['element'][] = [];
	const result: Omit<PageMarkup, 'sectionLayout'> = {
		ids: new Set(),
		links: [],
		navigationSources: [],
		deliverySources: [],
		projectOverviewSources: [],
		projectDetailsSources: [],
		recordSourceMarkers: [],
		placements: [],
		trees: [],
		hasDocumentBase: false,
	};
	function addAnchor(anchor: string): void {
		if (result.ids.has(anchor))
			throw new Error(`Duplicate HTML id or record placement "${anchor}" in ${file}.`);
		result.ids.add(anchor);
	}
	function visit(node: DefaultTreeAdapterMap['node']): void {
		if ('attrs' in node) {
			elements.push(node);
			const sectionProblem = sectionMarkupProblem(node);
			if (sectionProblem)
				throw new Error(`${file}:${node.sourceCodeLocation?.startLine ?? 1}: ${sectionProblem}`);
			const get = (name: string): string | undefined =>
				node.attrs.find(({ name: attribute }): boolean => attribute === name)?.value;
			const id = get('id');
			const sectionReference =
				node.tagName === 'ex-section-item' ? get('record')?.trim() : undefined;
			const boundReference = get('data-design-record');
			if (sectionReference && boundReference !== undefined)
				throw new Error(
					`${file}: Write the record ID once, in the section item's record attribute.`,
				);
			const reference = sectionReference ?? boundReference;
			const choiceChildren = sectionReference
				? node.childNodes.filter(
						(child): child is PageElement => 'tagName' in child && child.tagName === 'ex-choice',
					)
				: [];
			if (choiceChildren.length > 1)
				throw new Error(`${file}: ${sectionReference} needs exactly one native ex-choice.`);
			const choiceChild = choiceChildren[0];
			if (
				choiceChild &&
				node.childNodes.some(
					(child) =>
						child !== choiceChild &&
						('tagName' in child || ('value' in child && child.value.trim())),
				)
			)
				throw new Error(
					`${file}: Write the Choice body and options inside its single ex-choice in ${sectionReference}.`,
				);
			const need = get('data-design-need');
			const status = get('data-design-status');
			const anchor = reference ?? id;
			if (anchor) addAnchor(anchor);
			if (reference !== undefined || need !== undefined || status !== undefined) {
				const binding = choiceChild ?? node;
				const location = binding.sourceCodeLocation;
				const tag = binding.tagName;
				if (!location?.startTag || !location.endTag)
					throw new Error(`${file}: <${tag}> bindings need explicit closing tags.`);
				const { startTag, endTag } = location;
				const range = (part: typeof startTag): MarkupRange => ({
					start: part.startOffset,
					end: part.endOffset,
				});
				if (status !== undefined) {
					if (
						tag !== 'span' ||
						reference !== undefined ||
						need !== undefined ||
						html.slice(startTag.endOffset, endTag.startOffset).trim()
					)
						throw new Error(`${file}: use an empty span for a decision status.`);
					result.placements.push({
						reference: status,
						tag,
						status: true,
						startTag: range(startTag),
						endTag: range(endTag),
					});
				}
				if (reference !== undefined) {
					if (tag === 'ex-card')
						throw new Error(
							`${file}: Write ${reference} as an ex-section-item with a bare authored body.`,
						);
					if (!sectionReference && !recordTextTags.has(tag))
						throw new Error(`${file}: <${tag}> cannot hold a record placement.`);
					if (
						id !== undefined ||
						(tag === 'ex-choice' &&
							binding.attrs.some(({ name }) => ['id', 'title', 'data-decision'].includes(name)))
					)
						throw new Error(`${file}: ${reference} supplies its own anchor and Choice question.`);
					if (
						tag === 'ex-choice' &&
						binding.attrs.some(
							({ name, value }) =>
								name === 'multiple' || name === 'details' || (name === 'view' && value !== 'cards'),
						)
					)
						throw new Error(
							`${file}: ${reference} uses one answer and the standard Choice cards and Details. Remove multiple, details or a non-card view.`,
						);
					if (
						!sectionReference &&
						tag !== 'ex-choice' &&
						html.slice(startTag.endOffset, endTag.startOffset).trim()
					)
						throw new Error(`${file}: record placement ${reference} must be empty.`);
					const options: ChoiceOptionBinding[] = [];
					let hasExplanation = false;
					if (sectionReference || tag === 'ex-choice') {
						addAnchor(decisionDetailsId(reference));
					}
					if (tag === 'ex-choice') {
						for (const child of binding.childNodes) {
							if (!('tagName' in child)) continue;
							if (
								child.tagName === 'template' &&
								child.attrs.some(({ name }) => name === 'data-record-body')
							) {
								if (hasExplanation || !child.sourceCodeLocation)
									throw new Error(`${file}: ${reference} has more than one explanation template.`);
								hasExplanation = true;
								continue;
							}
							if (child.tagName !== 'ex-option') continue;
							const details = child.childNodes.filter(
								(element) =>
									'tagName' in element &&
									element.attrs.some(
										(attribute) => attribute.name === 'slot' && attribute.value === 'details',
									),
							);
							if (
								details.length > 1 ||
								details.some((element) => !('tagName' in element) || element.tagName !== 'template')
							)
								throw new Error(
									`${file}: ${reference} option Details use one <template slot="details"> containing the authored HTML.`,
								);
							const option = child.attrs.find(({ name }) => name === 'data-option')?.value;
							if (!option || child.attrs.some(({ name }) => name === 'id' || name === 'title'))
								throw new Error(
									`${file}: ${reference} options use data-option and take their title from the record.`,
								);
							const opening = child.sourceCodeLocation?.startTag;
							if (!opening)
								throw new Error(`${file}: missing the opening tag for option ${option}.`);
							const oldAnchor = child.attrs.find(
								({ name }) => name === migratedOptionAnchor,
							)?.value;
							if (oldAnchor !== undefined && !/^[^\s]+$/u.test(oldAnchor))
								throw new Error(
									`${file}: ${reference} option ${option} needs a nonempty fragment ID without spaces.`,
								);
							const detailsAnchor =
								oldAnchor && details.length
									? migratedOptionDetailsId(oldAnchor)
									: decisionDetailsId(reference, option);
							options.push({
								id: option,
								...(oldAnchor && { anchor: oldAnchor }),
								detailsAnchor,
								start: opening.startOffset,
								end: opening.endOffset,
							});
							addAnchor(oldAnchor ?? `${reference}--${option}`);
							addAnchor(detailsAnchor);
						}
					}
					result.placements.push({
						reference,
						tag,
						startTag: range(startTag),
						endTag: range(endTag),
						...(tag === 'ex-choice' && { options }),
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
			if (node.tagName === 'ex-project-overview')
				result.projectDetailsSources.push(get('src') ?? '');
			if (node.tagName === 'ex-initiative-overview')
				result.projectOverviewSources.push(get('src') ?? '');
			if (node.tagName === 'meta' && get('name') === 'demistify-records')
				result.recordSourceMarkers.push(get('content') ?? '');
			if (
				node.tagName === 'template' &&
				(get('data-record-body') !== undefined || get('slot') === 'details') &&
				'content' in node
			)
				visit(node.content);
		}
		if ('childNodes' in node) node.childNodes.forEach(visit);
	}
	visit(parse(html, { sourceCodeLocationInfo: true }));
	return { ...result, sectionLayout: readSectionLayout(elements, file) };
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
	return element.attrs.find(
		({ name }) =>
			name === 'id' ||
			name === 'data-design-record' ||
			(element.tagName === 'ex-section-item' && name === 'record'),
	)?.value;
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
