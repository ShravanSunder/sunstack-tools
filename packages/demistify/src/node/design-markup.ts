import { parse, serialize, type DefaultTreeAdapterMap } from 'parse5';
import { z } from 'zod';

import {
	collectionSchema,
	recordSchema,
	type AuthoredRecord,
	type PreparedRecord,
} from '../design-documents/collection.ts';

type HtmlNode = DefaultTreeAdapterMap['node'];
type HtmlElement = DefaultTreeAdapterMap['element'];
export const designOwnerKinds: Readonly<Record<string, readonly AuthoredRecord['kind'][]>> = {
	'requirements.html': ['need'],
	'specification.html': ['specification'],
	'program-design.html': ['program-design'],
	'verification.html': ['scenario', 'observation'],
	'execution.html': ['frame', 'stack'],
};
const allowedParts: Readonly<Record<AuthoredRecord['kind'], readonly string[]>> = {
	need: [],
	specification: ['rule', 'cases'],
	'program-design': ['sketch', 'gaps'],
	scenario: ['procedure'],
	observation: [],
	frame: [],
	stack: [],
};
function children(node: HtmlNode): readonly HtmlNode[] {
	if ('tagName' in node && node.tagName === 'template' && 'content' in node)
		return node.content.childNodes;
	return 'childNodes' in node ? node.childNodes : [];
}
function elements(node: HtmlNode): HtmlElement[] {
	return [...('tagName' in node ? [node] : []), ...children(node).flatMap(elements)];
}
function attribute(element: HtmlElement, name: string): string | undefined {
	return element.attrs.find((item) => item.name === name)?.value;
}
function textContent(node: HtmlNode): string {
	return 'value' in node ? node.value : children(node).map(textContent).join('');
}

/** Local authored metadata, body ownership and parts share one preparation/checker contract. */
export function collectDesignRecords(html: string, page: string): readonly PreparedRecord[] {
	const nodes = elements(parse(html));
	const blocks = nodes.filter(
		(node) => node.tagName === 'script' && attribute(node, 'data-ex-records') !== undefined,
	);
	const ownerKinds = designOwnerKinds[page];
	if (blocks.length !== (ownerKinds ? 1 : 0))
		throw new Error(`${page}: expected ${ownerKinds ? 'one' : 'no'} inline data-ex-records block`);
	const block = blocks[0];
	const records = block ? z.array(recordSchema).parse(JSON.parse(textContent(block))) : [];
	const ids = new Map<string, HtmlElement>();
	for (const node of nodes) {
		const id = attribute(node, 'id');
		if (!id) continue;
		if (ids.has(id)) throw new Error(`${page}: duplicate body id ${id}`);
		ids.set(id, node);
	}
	const ownedBodies = new Set<HtmlElement>();
	const ownedParts = new Set<HtmlElement>();
	const prepared: PreparedRecord[] = [];
	for (const record of records) {
		if (!ownerKinds?.includes(record.kind))
			throw new Error(`${page}: wrong owner for ${record.id}`);
		let bodyHtml = '',
			summary = '';
		if ('bodyRef' in record) {
			const body = ids.get(record.bodyRef),
				summaryNode = ids.get(record.summaryRef);
			if (!body || !summaryNode || !elements(body).includes(summaryNode))
				throw new Error(
					`${page}/${record.id}: missing or unowned body/summary ${record.bodyRef}/${record.summaryRef}`,
				);
			if (ownedBodies.has(body))
				throw new Error(`${page}/${record.id}: body ${record.bodyRef} has multiple owners`);
			ownedBodies.add(body);
			const seenParts = new Set<string>();
			for (const node of elements(body)) {
				const part = attribute(node, 'data-ex-part');
				if (part === undefined) continue;
				if (!allowedParts[record.kind].includes(part))
					throw new Error(`${page}/${record.id}: unknown or forbidden part ${part}`);
				if (seenParts.has(part)) throw new Error(`${page}/${record.id}: duplicate part ${part}`);
				seenParts.add(part);
				ownedParts.add(node);
			}
			bodyHtml = serialize(body);
			summary = textContent(summaryNode).trim();
			if (!summary) throw new Error(`${page}/${record.id}: empty summary`);
		}
		prepared.push({ record, page, bodyHtml, summary });
	}
	for (const node of nodes) {
		const part = attribute(node, 'data-ex-part');
		if (part !== undefined && !ownedParts.has(node))
			throw new Error(`${page}: part ${part} outside an owned record body`);
	}
	return prepared;
}

export function designRecordReferences(
	record: AuthoredRecord,
): readonly (readonly [string, AuthoredRecord['kind']])[] {
	switch (record.kind) {
		case 'specification':
			return record.needs.map((id) => [id, 'need']);
		case 'program-design':
		case 'scenario':
			return record.specifications.map((id) => [id, 'specification']);
		case 'observation':
			return [[record.scenario, 'scenario']];
		case 'frame':
			return record.requirement ? [[record.requirement, 'need']] : [];
		case 'stack':
			return [
				[record.primaryNeed, 'need'],
				...record.frames.map((id): readonly [string, 'frame'] => [id, 'frame']),
			];
		case 'need':
			return [];
		default:
			return assertNever(record);
	}
}
function assertNever(record: never): never {
	throw new Error(`Unsupported record ${JSON.stringify(record)}`);
}

/** Single-page checks do not claim cross-page edge validation. */
export function checkDesignMarkup(html: string): readonly string[] {
	const nodes = elements(parse(html));
	const readers = nodes.filter((node) => node.tagName === 'ex-call-stack');
	const blocks = nodes.filter(
		(node) => node.tagName === 'script' && attribute(node, 'data-ex-records') !== undefined,
	);
	const problems: string[] = [];
	try {
		if (blocks.length > 1)
			throw new Error('Expected at most one data-ex-records block per owner page');
		const block = blocks[0];
		if (block) {
			const records = z.array(recordSchema).parse(JSON.parse(textContent(block)));
			const first = records[0];
			const owner = first
				? Object.entries(designOwnerKinds).find(([, kinds]) => kinds.includes(first.kind))?.[0]
				: 'requirements.html';
			if (!owner) throw new Error('Unknown record owner');
			collectDesignRecords(html, owner);
		} else if (nodes.some((node) => attribute(node, 'data-ex-part') !== undefined))
			throw new Error('data-ex-part requires an owned record body');
	} catch (error: unknown) {
		problems.push(error instanceof Error ? error.message : 'Invalid authored design records');
	}
	for (const tag of ['ex-call-stack'])
		if (readers.filter((reader) => reader.tagName === tag).length > 1)
			problems.push(`At most one ${tag} reader is supported per document`);
	if (readers.length) {
		const carriers = nodes.filter(
			(node) =>
				node.tagName === 'script' && attribute(node, 'data-ex-design-collection') !== undefined,
		);
		try {
			const carrier = carriers[0];
			if (carriers.length !== 1 || !carrier)
				throw new Error('Run demistify prepare: one prepared design collection is required');
			collectionSchema.parse(JSON.parse(textContent(carrier)));
		} catch (error: unknown) {
			problems.push(
				`Run demistify prepare: ${error instanceof Error ? error.message : 'invalid prepared design collection'}`,
			);
		}
	}
	return problems;
}

interface PreparedPageMarkup {
	readonly html: string;
	readonly page: string;
	readonly identity: string;
	readonly carrier: string;
}

/** Source offsets target the real tree boundaries while every authored byte stays intact. */
export function stampPreparedDesignPage({
	html,
	page,
	identity,
	carrier,
}: PreparedPageMarkup): string {
	const nodes = elements(parse(html, { sourceCodeLocationInfo: true }));
	const head = nodes.find((node) => node.tagName === 'head');
	const body = nodes.find((node) => node.tagName === 'body');
	const headEnd = head?.sourceCodeLocation?.endTag?.startOffset;
	const bodyEnd = body?.sourceCodeLocation?.endTag?.startOffset;
	if (headEnd === undefined || bodyEnd === undefined)
		throw new Error(`${page}: preparation requires explicit head and body closing tags`);
	const edits: { readonly start: number; readonly end: number; readonly text: string }[] = [
		{
			start: headEnd,
			end: headEnd,
			text: `<meta name="ex-design-collection" content="1:${identity}">`,
		},
		{ start: bodyEnd, end: bodyEnd, text: carrier },
	];
	for (const node of nodes) {
		const isCarrier =
			node.tagName === 'script' && attribute(node, 'data-ex-design-collection') !== undefined;
		const isMeta = node.tagName === 'meta' && attribute(node, 'name') === 'ex-design-collection';
		if (!isCarrier && !isMeta) continue;
		if (node.parentNode !== (isCarrier ? body : head))
			throw new Error(
				`${page}: derived collection marker is outside its ${isCarrier ? 'body' : 'head'} owner`,
			);
		const location = node.sourceCodeLocation;
		if (!location) throw new Error(`${page}: derived collection marker has no source location`);
		edits.push({ start: location.startOffset, end: location.endOffset, text: '' });
	}
	return edits
		.toSorted((left, right) => right.start - left.start)
		.reduce(
			(source, edit) => source.slice(0, edit.start) + edit.text + source.slice(edit.end),
			html,
		);
}
