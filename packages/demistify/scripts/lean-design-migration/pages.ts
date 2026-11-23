import { dirname, relative, sep } from 'node:path';

import { parse, parseFragment, type DefaultTreeAdapterMap } from 'parse5';

import { escapeHtml } from '../../src/core/html-text.ts';
import { resolveDesignLink } from '../../src/node/design-authoring/design-location.ts';
import {
	normalizeExcerpt,
	passageText,
	readPageMarkup,
} from '../../src/node/design-authoring/design-page-markup.ts';
import type { LegacyRecord } from './legacy-records.ts';

type HtmlNode = DefaultTreeAdapterMap['node'];
type HtmlElement = DefaultTreeAdapterMap['element'];

export interface LocatedRecord {
	readonly namespace: string;
	readonly file: string;
	readonly record: LegacyRecord;
}
interface Edit {
	readonly start: number;
	readonly end: number;
	readonly text: string;
}
interface SourceSplit {
	readonly anchor: string;
	readonly retargeted: readonly { from: string; to: string }[];
}

export interface PageChange {
	readonly sourceSplit?: SourceSplit;
	readonly id: string;
	readonly oldAnchor: string;
	readonly anchor: string;
	readonly layout: string;
	readonly statement: string | undefined;
	readonly statementSource: 'paragraph' | 'table row' | undefined;
	readonly paraphraseWords: number | undefined;
	readonly sources: readonly { href: string; added: boolean }[];
}
export interface PagePlan {
	readonly html: string;
	readonly changes: readonly PageChange[];
	readonly retainedIds: number;
}

function isElement(node: HtmlNode): node is HtmlElement {
	return 'tagName' in node;
}

function attribute(node: HtmlElement, name: string): string | undefined {
	return node.attrs.find((item): boolean => item.name === name)?.value;
}

function nodeText(node: HtmlNode, omitLinks = false): string {
	if ('value' in node) return node.value;
	if (omitLinks && isElement(node) && node.tagName === 'a') return '';
	return 'childNodes' in node
		? node.childNodes.map((child): string => nodeText(child, omitLinks)).join('')
		: '';
}

function nextElement(node: HtmlElement): HtmlElement | undefined {
	const siblings = node.parentNode?.childNodes ?? [];
	return siblings.slice(siblings.indexOf(node) + 1).find(isElement);
}

function ruleWords(text: string): number {
	return text.match(/\S+/gu)?.length ?? 0;
}

const derivedSentence =
	/\s+(?:Governed by|Relies on|Checks)\s+[A-Z][A-Z0-9-]*\d+(?:(?:,\s*(?:and\s+)?|\s+and\s+)[A-Z][A-Z0-9-]*\d+)*(?:;\s*(?:relies on|governed by)\s+[A-Z][A-Z0-9-]*\d+(?:(?:,\s*(?:and\s+)?|\s+and\s+)[A-Z][A-Z0-9-]*\d+)*)?\./giu;

function matchesRelationships(sentence: string, record: LegacyRecord): boolean {
	if (!('relationships' in record)) return false;
	const relationships = record.relationships;
	return sentence.split(';').every((clause): boolean => {
		const verb = clause.trim();
		const relation = /^governed/iu.test(verb)
			? 'governed-by'
			: /^relies/iu.test(verb)
				? 'relies-on'
				: 'checks';
		const ids = clause.match(/\b[A-Z][A-Z0-9-]*\d+\b/gu);
		return (
			!!ids?.length &&
			ids.every((id): boolean =>
				relationships.some((edge): boolean => edge.relation === relation && edge.targetId === id),
			)
		);
	});
}

function withoutDerivedEnding(paragraph: string, record: LegacyRecord): string {
	const text = paragraph.trim();
	const ending = [...text.matchAll(derivedSentence)].at(-1);
	return ending &&
		ending.index + ending[0].length === text.length &&
		matchesRelationships(ending[0], record)
		? text.slice(0, ending.index).trim()
		: text;
}

function ruleParaphraseWords(
	record: LegacyRecord,
	node: HtmlElement,
	paragraph?: HtmlElement,
): number | undefined {
	if (record.kind !== 'rule' || record.destination.fragment !== record.id) return undefined;
	if (/^h[1-6]$/u.test(node.tagName) && paragraph)
		return ruleWords(withoutDerivedEnding(normalizeExcerpt(nodeText(paragraph, true)), record));
	if (node.tagName !== 'tr') return undefined;
	const parent = node.parentNode;
	const table = parent && 'parentNode' in parent ? parent.parentNode : undefined;
	if (!table || !isElement(table) || table.tagName !== 'table') return undefined;
	const head = table.childNodes.find(
		(child): child is HtmlElement => isElement(child) && child.tagName === 'thead',
	);
	const header = head?.childNodes.find(isElement);
	const index = header?.childNodes
		.filter(isElement)
		.findIndex((cell): boolean => normalizeExcerpt(nodeText(cell)) === 'What must be true');
	const cell =
		index === undefined || index < 0 ? undefined : node.childNodes.filter(isElement)[index];
	return cell ? ruleWords(normalizeExcerpt(nodeText(cell, true))) : undefined;
}

type CheckPassage =
	| { statement: string; source: 'paragraph'; edit: Edit }
	| { statement: string; source: 'table row' };

function checkParagraph(
	html: string,
	paragraph: HtmlElement,
	record: LegacyRecord,
): CheckPassage | undefined {
	const location = paragraph.sourceCodeLocation;
	if (!location?.startTag || !location.endTag) return undefined;
	const start = location.startTag.endOffset;
	const end = location.endTag.startOffset;
	const markup = paragraph.childNodes.find((child): boolean => !('value' in child));
	const markupStart = markup?.sourceCodeLocation?.startOffset;
	if (markup && markupStart === undefined) return undefined;
	const leadingText = html.slice(start, markupStart ?? end);
	const clauses = [...leadingText.matchAll(derivedSentence)].filter(
		(match): boolean =>
			/^\s+Checks\b/iu.test(match[0]) && /[.!?]$/u.test(leadingText.slice(0, match.index)),
	);
	const [clause] = clauses;
	if (
		clauses.length > 1 ||
		(clause
			? !matchesRelationships(clause[0], record)
			: !!markup || /[.!?]\s+Checks\b/iu.test(leadingText))
	)
		return undefined;
	const statement = normalizeExcerpt(
		nodeText(parseFragment(clause ? leadingText.slice(0, clause.index) : leadingText)),
	);
	// These paragraphs introduce separate proof rows.
	if (
		!statement ||
		/^(?:Nothing below has been run\b|From the confirmation review\b)/iu.test(statement)
	)
		return undefined;
	const context = clause ? html.slice(start + clause.index + clause[0].length, end).trim() : '';
	const edit =
		context || paragraph.attrs.length
			? { start, end, text: context }
			: { start: location.startOffset, end: location.endOffset, text: '' };
	return { statement, source: 'paragraph', edit };
}

function checkStatement(
	html: string,
	record: LegacyRecord,
	node: HtmlElement,
	next: HtmlElement | undefined,
): CheckPassage | undefined {
	if (record.kind !== 'planned-check' || record.destination.fragment !== record.id)
		return undefined;
	if (/^h[1-6]$/u.test(node.tagName) && next?.tagName === 'p')
		return checkParagraph(html, next, record);
	if (node.tagName === 'tr') {
		const firstCell = node.childNodes.find(
			(child): child is HtmlElement =>
				isElement(child) && (child.tagName === 'td' || child.tagName === 'th'),
		);
		const label = firstCell && normalizeExcerpt(nodeText(firstCell));
		const prefix = `${record.id} · `;
		if (label?.startsWith(prefix) && label.length > prefix.length)
			return { statement: label.slice(prefix.length), source: 'table row' };
	}
	return undefined;
}

function passageLinks(anchor: HtmlElement, nextParagraph: HtmlElement | undefined): string[] {
	const links: string[] = [];
	function visit(node: HtmlNode): void {
		const href = isElement(node) && node.tagName === 'a' ? attribute(node, 'href') : undefined;
		if (href) links.push(href);
		if ('childNodes' in node) node.childNodes.forEach(visit);
	}
	visit(anchor);
	if (nextParagraph && /^h[1-6]$/u.test(anchor.tagName)) visit(nextParagraph);
	return links;
}

const evidenceContainers = new Set(['details', 'section', 'div', 'blockquote', 'ex-callout']);

export function recordEvidence(record: LegacyRecord): LegacyRecord['sources'] {
	if (record.kind === 'choice') return record.sources;
	if (record.kind !== 'assumption') return [];
	return record.resolution ? [...record.sources, record.resolution] : record.sources;
}

function splitEvidence(
	group: readonly LocatedRecord[],
	options: Pick<Parameters<typeof planPage>[0], 'html' | 'file' | 'site'>,
	element: HtmlElement,
	fragment: string,
): { owner: LocatedRecord; split: SourceSplit } | undefined {
	if (!evidenceContainers.has(element.tagName)) return undefined;
	const { html, file, site } = options;
	for (const owner of group) {
		if (owner.record.id !== fragment) continue;
		const sources = recordEvidence(owner.record).filter(({ href }): boolean => {
			const target = resolveDesignLink(href, owner.file, site);
			return target?.file === file && target.fragment === fragment;
		});
		if (!sources.length) continue;
		const text = passageText(html, fragment);
		const matches = sources.filter(({ excerpt }) => text.includes(normalizeExcerpt(excerpt)));
		if (!matches.length) continue;
		if (sources.some((source) => source.verbatim && !matches.includes(source)))
			throw new Error(`${file}#${fragment}: the container does not hold every verbatim excerpt.`);
		const anchor = `${fragment}-source`;
		const retargeted = [...new Set(sources.map(({ href }) => href))].map((from) => ({
			from,
			to: from.slice(0, from.indexOf('#') + 1) + encodeURIComponent(anchor),
		}));
		return { owner, split: { anchor, retargeted } };
	}
	return undefined;
}

function insideCallout(element: HtmlElement): boolean {
	for (
		let parent = element.parentNode;
		parent;
		parent = 'parentNode' in parent ? parent.parentNode : null
	) {
		if (isElement(parent) && parent.tagName === 'ex-callout') return true;
	}
	return false;
}

function insertionOffset(element: HtmlElement): number | undefined {
	if (element.tagName === 'tr') {
		const cell = element.childNodes.find(
			(node): node is HtmlElement =>
				isElement(node) && (node.tagName === 'td' || node.tagName === 'th'),
		);
		return cell?.sourceCodeLocation?.endTag?.startOffset;
	}
	if (element.tagName === 'aside' || element.tagName === 'ex-card')
		return element.sourceCodeLocation?.endTag?.startOffset;
	if (/^h[1-6]$/u.test(element.tagName) || element.tagName === 'p')
		return element.sourceCodeLocation?.endOffset;
	return undefined;
}

function rebaseSource(designFile: string, pageFile: string, href: string, site: string): string {
	const target = resolveDesignLink(href, designFile, site);
	if (!target) return href;
	const path = relative(dirname(pageFile), target.file)
		.split(sep)
		.map(encodeURIComponent)
		.join('/');
	return `${path}${target.fragment ? `#${encodeURIComponent(target.fragment)}` : ''}`;
}

function removeDerivedProse(
	edits: Edit[],
	html: string,
	paragraph: HtmlElement,
	record: LegacyRecord,
): void {
	const location = paragraph.sourceCodeLocation;
	if (!location?.startTag || !location.endTag) return;
	const { endOffset: start } = location.startTag;
	const { startOffset: end } = location.endTag;
	const prose = html.slice(start, end);
	const lean = withoutDerivedEnding(prose, record);
	if (lean !== prose.trim()) edits.push({ start, end, text: lean });
}

export function planPage(options: {
	readonly html: string;
	readonly file: string;
	readonly site: string;
	readonly namespace?: string | undefined;
	readonly records: readonly LocatedRecord[];
	readonly preservedAnchors: ReadonlyMap<string, ReadonlySet<string>>;
}): PagePlan {
	const { html, file, site, namespace, records, preservedAnchors } = options;
	const old = readPageMarkup(html, file);
	const elements = new Map<string, HtmlElement>();
	function visit(node: HtmlNode): void {
		if (isElement(node)) {
			const id = attribute(node, 'id');
			if (id) elements.set(id, node);
		}
		if ('childNodes' in node) node.childNodes.forEach(visit);
	}
	visit(parse(html, { sourceCodeLocationInfo: true }));
	const groups = new Map<string, LocatedRecord[]>();
	for (const location of records) {
		const fragment = location.record.destination.fragment;
		const group = groups.get(fragment) ?? [];
		group.push(location);
		groups.set(fragment, group);
	}
	const edits: Edit[] = [];
	const changes: PageChange[] = [];
	for (const [fragment, group] of groups) {
		const node = elements.get(fragment);
		if (!node) throw new Error(`${file}#${fragment}: old record destination is absent.`);
		const evidence = splitEvidence(group, options, node, fragment);
		if (evidence) {
			const id = `${evidence.owner.namespace}/${evidence.owner.record.id}`;
			const otherOwners = [...(preservedAnchors.get(fragment) ?? [])].filter(
				(owner) => owner !== id,
			);
			if (otherOwners.length)
				throw new Error(
					`${file}#${fragment}: source anchor is also required by ${otherOwners.join(', ')}.`,
				);
			if (old.ids.has(evidence.split.anchor))
				throw new Error(
					`${file}#${fragment}: source anchor #${evidence.split.anchor} already exists.`,
				);
		}
		if (evidence && insideCallout(node))
			throw new Error(`${file}#${fragment}: cannot place a record inside <ex-callout>.`);
		const offset = evidence ? node.sourceCodeLocation?.endTag?.endOffset : insertionOffset(node);
		if (offset === undefined) {
			const recordIds = group.map(({ namespace: owner, record }) => `${owner}/${record.id}`);
			throw new Error(
				`${file}#${fragment}: no safe placement for ${recordIds.join(', ')} at <${node.tagName}>.`,
			);
		}
		const next = nextElement(node);
		const nextParagraph = next?.tagName === 'p' ? next : undefined;
		const existingLinks = passageLinks(node, nextParagraph);
		const snippets: string[] = [];
		for (const location of group) {
			const { namespace: owner, record, file: designFile } = location;
			const sourceSplit = evidence?.owner === location ? evidence.split : undefined;
			const id = `${owner}/${record.id}`;
			const usesLocalId =
				owner === namespace &&
				(!old.ids.has(record.id) ||
					(fragment === record.id &&
						(sourceSplit !== undefined || !preservedAnchors.has(fragment))));
			const anchor = usesLocalId ? record.id : id;
			const ownCheck = checkStatement(html, record, node, next);
			const isExpectedPassage = node.tagName === 'h3' && !!nextParagraph && fragment === record.id;
			const sources: { href: string; added: boolean }[] = [];
			// A qualified binding and its legacy public fragment need separate elements.
			if (sourceSplit && anchor !== fragment)
				snippets.push(`<span id="${escapeHtml(fragment)}"></span>`);
			snippets.push(`<p data-design-record="${escapeHtml(anchor)}"></p>`);
			if (record.kind !== 'choice' && record.kind !== 'assumption') {
				for (const source of record.sources) {
					const target = resolveDesignLink(source.href, designFile, site);
					if (target?.file === file && target.fragment === fragment) continue;
					const alreadyLinked = existingLinks.some((href): boolean => {
						const existing = resolveDesignLink(href, file, site);
						return target
							? existing?.file === target.file && existing.fragment === target.fragment
							: href === source.href;
					});
					sources.push({ href: source.href, added: !alreadyLinked });
					if (!alreadyLinked)
						snippets.push(
							`<p><a href="${escapeHtml(rebaseSource(designFile, file, source.href, site))}">${escapeHtml(source.excerpt)}</a></p>`,
						);
				}
			}
			if (ownCheck?.source === 'paragraph') {
				edits.push(ownCheck.edit);
			} else if (
				isExpectedPassage &&
				nextParagraph &&
				(record.kind === 'need' || record.kind === 'rule')
			) {
				removeDerivedProse(edits, html, nextParagraph, record);
			}
			changes.push({
				id,
				...(sourceSplit && { sourceSplit }),
				oldAnchor: fragment,
				anchor,
				layout: isExpectedPassage
					? 'heading and paragraph'
					: `${node.tagName} ${group.length > 1 ? 'shared passage' : 'retained passage'}`,
				statement: ownCheck?.statement,
				statementSource: ownCheck?.source,
				paraphraseWords: ruleParaphraseWords(record, node, nextParagraph),
				sources,
			});
		}
		edits.push({ start: offset, end: offset, text: `\n${snippets.join('\n')}\n` });
		if (
			evidence ||
			(!preservedAnchors.has(fragment) &&
				changes.some((item): boolean => item.oldAnchor === fragment && item.anchor === fragment))
		) {
			const idLocation = node.sourceCodeLocation?.attrs?.['id'];
			if (!idLocation) throw new Error(`${file}#${fragment}: could not move the old id.`);
			const start =
				html[idLocation.startOffset - 1] === ' '
					? idLocation.startOffset - 1
					: idLocation.startOffset;
			edits.push({
				start,
				end: idLocation.endOffset,
				text: evidence ? ` id="${escapeHtml(evidence.split.anchor)}"` : '',
			});
		}
	}
	let revised = html;
	let previousStart = html.length;
	for (const { start, end, text } of edits.toSorted(
		(left, right): number => right.start - left.start || right.end - left.end,
	)) {
		if (end > previousStart)
			throw new Error(`${file}: overlapping migration edits at source offset ${start}.`);
		revised = `${revised.slice(0, start)}${text}${revised.slice(end)}`;
		previousStart = start;
	}
	const revisedMarkup = readPageMarkup(revised, file);
	for (const id of old.ids)
		if (!revisedMarkup.ids.has(id)) throw new Error(`${file}: migration would break #${id}.`);
	return { html: revised, changes, retainedIds: old.ids.size };
}
