import { createHash } from 'node:crypto';
import { readFile, readdir, mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

import { parse, serialize, type DefaultTreeAdapterMap } from 'parse5';
import { z } from 'zod';

import {
	recordSchema,
	type AuthoredRecord,
	type PreparedCollection,
	type PreparedRecord,
} from '../src/understanding-lab/collection-data.ts';

type Node = DefaultTreeAdapterMap['node'];
type Element = DefaultTreeAdapterMap['element'];
const ownerPages = [
	'requirements.html',
	'specification.html',
	'program-design.html',
	'verification.html',
	'execution.html',
];
const allowedKinds: Readonly<Record<string, readonly AuthoredRecord['kind'][]>> = {
	'requirements.html': ['need'],
	'specification.html': ['specification'],
	'program-design.html': ['program-design'],
	'verification.html': ['scenario', 'observation'],
	'execution.html': ['frame', 'stack'],
};
function children(node: Node): readonly Node[] {
	if ('tagName' in node && node.tagName === 'template' && 'content' in node)
		return node.content.childNodes;
	return 'childNodes' in node ? node.childNodes : [];
}
function elements(node: Node): Element[] {
	return [...('tagName' in node ? [node] : []), ...children(node).flatMap(elements)];
}
function attribute(element: Element, name: string): string | undefined {
	return element.attrs.find((item) => item.name === name)?.value;
}
function textContent(node: Node): string {
	return 'value' in node ? node.value : children(node).map(textContent).join('');
}
function references(record: AuthoredRecord): readonly [string, AuthoredRecord['kind']][] {
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
				...record.frames.map((id): [string, 'frame'] => [id, 'frame']),
			];
		case 'need':
			return [];
		default:
			throw new Error('Unsupported record kind');
	}
}
/** Validate every forward edge and rich-body reference before any output is mutated. */
export async function prepareUnderstandingPages(
	inputDirectory: string,
): Promise<{ collection: PreparedCollection; pages: ReadonlyMap<string, string> }> {
	const authoredPages = new Map(
		await Promise.all(
			(await readdir(inputDirectory))
				.filter((file) => file.endsWith('.html'))
				.map(
					async (file) =>
						[file, await readFile(join(inputDirectory, file), 'utf8')] satisfies [string, string],
				),
		),
	);
	const records: PreparedRecord[] = [];
	for (const page of ownerPages) {
		const html = authoredPages.get(page);
		if (!html) throw new Error(`Missing record owner page ${page}`);
		const nodes = elements(parse(html));
		const blocks = nodes.filter(
			(node) => node.tagName === 'script' && attribute(node, 'data-lab-records') !== undefined,
		);
		if (blocks.length !== 1) throw new Error(`${page}: expected one inline record block`);
		const block = blocks[0];
		if (!block) throw new Error(`Missing records ${page}`);
		const raw: unknown = JSON.parse(textContent(block));
		const parsed = z.array(recordSchema).parse(raw);
		const ids = new Map<string, Element>();
		for (const node of nodes) {
			const id = attribute(node, 'id');
			if (!id) continue;
			if (ids.has(id)) throw new Error(`${page}: duplicate body id ${id}`);
			ids.set(id, node);
		}
		for (const record of parsed) {
			if (!allowedKinds[page]?.includes(record.kind))
				throw new Error(`${page}: wrong owner for ${record.id}`);
			let bodyHtml = '',
				summary = '';
			if ('bodyRef' in record) {
				const body = ids.get(record.bodyRef),
					summaryNode = ids.get(record.summaryRef);
				if (!body || !summaryNode)
					throw new Error(
						`${record.id}: missing body/summary ${record.bodyRef}/${record.summaryRef}`,
					);
				bodyHtml = serialize(body);
				summary = textContent(summaryNode).trim();
				if (!summary) throw new Error(`${record.id}: empty summary`);
			}
			records.push({ record, page, bodyHtml, summary });
		}
	}
	const byId = new Map<string, PreparedRecord>();
	for (const prepared of records) {
		if (byId.has(prepared.record.id)) throw new Error(`Duplicate record ${prepared.record.id}`);
		byId.set(prepared.record.id, prepared);
	}
	for (const { record } of records)
		for (const [target, kind] of references(record)) {
			if (byId.get(target)?.record.kind !== kind)
				throw new Error(`${record.id}: missing or wrong-kind ${kind} reference ${target}`);
		}
	const identity = createHash('sha256').update(JSON.stringify(records)).digest('hex');
	const collection = { identity, records };
	const carrier = `<script type="application/json" data-lab-collection>\n${JSON.stringify(collection).replaceAll('<', '\\u003c')}\n</script>`;
	const pages = new Map(
		[...authoredPages].map(([page, html]) => [
			page,
			html
				.replace('</head>', () => `<meta name="lab-collection" content="${identity}" /></head>`)
				.replace('</body>', () => `${carrier}\n</body>`),
		]),
	);
	return { collection, pages };
}
/** Output writing follows complete collection validation, including disposable proof inputs. */
export async function writeUnderstandingPages(
	inputDirectory: string,
	outputDirectory: string,
): Promise<PreparedCollection> {
	const prepared = await prepareUnderstandingPages(inputDirectory);
	await mkdir(outputDirectory, { recursive: true });
	await Promise.all(
		[...prepared.pages].map(([page, html]) => writeFile(join(outputDirectory, page), html)),
	);
	return prepared.collection;
}
