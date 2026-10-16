/** Independent content-preservation diagnostic; no browser mirror, network or source execution. */
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';

import { parseGraphDocument } from '../../../packages/demistify/src/explorer/graph-document.ts';

interface TextLeaf {
	readonly path: string;
	readonly text: string;
}
interface ObjectLeaf {
	readonly path: string;
	readonly value: Readonly<Record<string, unknown>>;
}
function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}
function textLeaves(value: unknown, path = ''): readonly TextLeaf[] {
	if (typeof value === 'string') return [{ path, text: value }];
	if (Array.isArray(value))
		return value.flatMap((item: unknown, index: number): readonly TextLeaf[] =>
			textLeaves(item, `${path}/${index}`),
		);
	if (isRecord(value))
		return Object.entries(value).flatMap(([key, item]): readonly TextLeaf[] =>
			textLeaves(item, `${path}/${key}`),
		);
	return [];
}
function objectLeaves(value: unknown, path = ''): readonly ObjectLeaf[] {
	if (Array.isArray(value))
		return value.flatMap((item: unknown, index: number): readonly ObjectLeaf[] =>
			objectLeaves(item, `${path}/${index}`),
		);
	if (!isRecord(value)) return [];
	return [
		{ path, value },
		...Object.entries(value).flatMap(([key, item]): readonly ObjectLeaf[] =>
			objectLeaves(item, `${path}/${key}`),
		),
	];
}
function normalizeWhitespace(value: string): string {
	return value.replaceAll(/\s+/g, ' ').trim();
}
function sourceIdentity(href: string, quotation: string): string {
	return JSON.stringify([href, normalizeWhitespace(quotation)]);
}
const [referenceArgument, graphArgument, outputArgument] = process.argv.slice(2);
if (!referenceArgument || !graphArgument)
	throw new Error(
		'Usage: node apps/demistify-poc/scripts/probe-explorer-preservation.ts SOURCE_HTML GRAPH_JSON [OUTPUT_JSON]',
	);
const [referenceHtml, graphJson] = await Promise.all([
	readFile(referenceArgument, 'utf8'),
	readFile(graphArgument, 'utf8'),
]);
const embedded = /<script id="composition-data" type="application\/json">([\s\S]*?)<\/script>/.exec(
	referenceHtml,
)?.[1];
if (!embedded) throw new Error('Reference needs its frozen composition-data JSON block.');
const reference: unknown = JSON.parse(embedded);
if (!isRecord(reference) || typeof reference['sourceUrl'] !== 'string')
	throw new Error('Reference needs its source URL base.');
const graph = parseGraphDocument(JSON.parse(graphJson));
const corpus = textLeaves(graph)
	.map((leaf): string => normalizeWhitespace(leaf.text))
	.join('\n');
const prose = textLeaves(reference).filter(
	(leaf): boolean => leaf.text.length >= 80 && !/^(?:https?:\/\/|plugins\/|docs\/)/.test(leaf.text),
);
const missingProse = prose
	.filter((leaf): boolean => !corpus.includes(normalizeWhitespace(leaf.text)))
	.map((leaf) => ({ path: leaf.path, characters: leaf.text.length }));
const actualSources = new Set(
	objectLeaves(graph).flatMap(({ value }): readonly string[] =>
		typeof value['href'] === 'string' && typeof value['text'] === 'string'
			? [sourceIdentity(value['href'], value['text'])]
			: [],
	),
);
let sourceAnchorOccurrences = 0;
const missingSources: string[] = [];
for (const { path, value } of objectLeaves(reference)) {
	const filePath = value['path'],
		quotation = value['text'];
	if (typeof filePath !== 'string' || typeof quotation !== 'string') continue;
	sourceAnchorOccurrences++;
	const sourceBase =
		value['sourceUrl'] === ''
			? reference['sourceUrl']
			: (value['sourceUrl'] ?? reference['sourceUrl']);
	const line = value['line'];
	if (typeof sourceBase !== 'string' || (line !== undefined && typeof line !== 'number'))
		throw new Error(`Malformed source anchor: ${path}.`);
	const href = `${sourceBase}${filePath}${line ? `#L${line}` : ''}`;
	if (!actualSources.has(sourceIdentity(href, quotation))) missingSources.push(path);
}
// Evidence must belong to the same relationship, not merely survive somewhere in the document.
const edgeById = new Map(graph.edges.map((edge) => [edge.id, edge]));
const missingEdgeAssociations: string[] = [];
let checkedEdges = 0;
for (const collection of ['edges', 'proposals']) {
	const originals = reference[collection];
	if (!Array.isArray(originals)) throw new Error(`Reference needs ${collection}.`);
	for (const [index, original] of originals.entries()) {
		if (!isRecord(original)) throw new Error('Malformed reference relationship.');
		const id = `${collection === 'edges' ? 'current' : 'proposed'}-${index}`;
		const actual = edgeById.get(id);
		checkedEdges++;
		if (
			!actual ||
			actual.source !== original['source'] ||
			actual.target !== original['target'] ||
			actual.status !== (collection === 'edges' ? 'current' : 'proposed')
		) {
			missingEdgeAssociations.push(`${collection}/${index}/identity`);
			continue;
		}
		for (const field of ['why', 'when', 'rationaleBasis', 'change']) {
			const expected = original[field];
			const actualText =
				field === 'when' ? actual.when : field === 'change' ? actual.use : actual.why;
			if (
				typeof expected === 'string' &&
				expected &&
				!normalizeWhitespace(actualText).includes(normalizeWhitespace(expected))
			)
				missingEdgeAssociations.push(`${collection}/${index}/${field}`);
		}
		const scopedSources = new Set(
			actual.sources.map((item): string => sourceIdentity(item.href, item.text)),
		);
		for (const { path, value } of objectLeaves(original['anchors'])) {
			const anchorPath = value['path'],
				quotation = value['text'];
			if (typeof anchorPath !== 'string' || typeof quotation !== 'string') continue;
			const base =
				value['sourceUrl'] === ''
					? reference['sourceUrl']
					: (value['sourceUrl'] ?? reference['sourceUrl']);
			const line = value['line'];
			if (typeof base !== 'string' || (line !== undefined && typeof line !== 'number'))
				throw new Error('Malformed relationship source.');
			if (
				!scopedSources.has(
					sourceIdentity(`${base}${anchorPath}${line ? `#L${line}` : ''}`, quotation),
				)
			)
				missingEdgeAssociations.push(`${collection}/${index}/anchors${path}`);
		}
	}
}
const result = {
	sourceSha256: createHash('sha256').update(referenceHtml).digest('hex'),
	graphSha256: createHash('sha256').update(graphJson).digest('hex'),
	proseStrings: prose.length,
	matchedProseStrings: prose.length - missingProse.length,
	missingProse,
	sourceAnchorOccurrences,
	matchedSourceOccurrences: sourceAnchorOccurrences - missingSources.length,
	missingSources,
	checkedEdges,
	missingEdgeAssociations,
	limits:
		'Long textual leaves use whitespace-normalized substring coverage. Source anchors compare the exact revision/line URL and normalized quotation, including via references, both globally and on each corresponding relationship. This proves preservation in validated graph content, not independent factual truth, rendered visibility or complete semantic equivalence.',
};
const output = `${JSON.stringify(result, null, 2)}\n`;
if (outputArgument) await writeFile(outputArgument, output);
process.stdout.write(output);
if (missingProse.length || missingSources.length || missingEdgeAssociations.length)
	process.exitCode = 1;
