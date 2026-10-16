/** Convert the supplied frozen explorer content; never copy its application JS or stylesheet. */
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';

import { z } from 'zod';

import {
	parseGraphDocument,
	type ExplorerCell,
	type ExplorerEdge,
	type ExplorerRow,
	type ExplorerSource,
	type ExplorerTable,
	type GraphDocument,
} from '../../../packages/demistify/src/explorer/graph-document.ts';

const sourceAnchor = z.object({
	path: z.string(),
	line: z.number().optional(),
	text: z.string().default(''),
	sourceUrl: z.string().optional(),
	via: z
		.object({
			path: z.string(),
			line: z.number().optional(),
			text: z.string().default(''),
			sourceUrl: z.string().optional(),
		})
		.optional(),
});
const sourceNode = z.object({
	id: z.string(),
	name: z.string(),
	description: z.string(),
	level: z.number(),
	path: z.string(),
	role: z.boolean().optional(),
	failure: z.boolean().optional(),
	proposed: z.boolean().optional(),
	kind: z.string().optional(),
});
const sourceEdge = z.object({
	source: z.string(),
	target: z.string(),
	kind: z.string(),
	relation: z.string().default(''),
	why: z.string().default(''),
	when: z.string().default(''),
	change: z.string().default(''),
	conditional: z.boolean().default(false),
	rationaleBasis: z.string().default(''),
	anchors: z.array(sourceAnchor),
});
const runChange = z.object({
	target: z.string().nullish(),
	label: z.string(),
	file: z.string(),
	anchor: sourceAnchor,
	when: z.string(),
	use: z.string(),
	why: z.string(),
	removed: z.string(),
	retained: z.string(),
	conditional: z.boolean(),
});
const sourceSchema = z.object({
	commit: z.string(),
	sourceUrl: z.string(),
	proposalRevision: z.string(),
	proposalDocument: z.string(),
	inspectedFiles: z.array(z.string()),
	nodes: z.array(sourceNode),
	edges: z.array(sourceEdge),
	proposals: z.array(sourceEdge),
	earlierProposals: z.array(sourceEdge),
	runs: z.array(
		z.object({
			run: z.string(),
			phase: z.string(),
			title: z.string(),
			movedTo: z.string(),
			stays: z.string(),
			pressure: z.string(),
			targets: z.array(z.string()),
			changes: z.array(runChange),
		}),
	),
	changes: z.array(
		z.object({
			id: z.string(),
			kind: z.string(),
			why: z.string(),
			what: z.string(),
			['pr']: z.number().optional(),
			prs: z.array(z.number()).optional(),
		}),
	),
	extractions: z.array(
		z.object({
			name: z.string(),
			action: z.string(),
			home: z.string(),
			homeState: z.string(),
			origins: z.array(z.string()),
			why: z.string(),
			when: z.string(),
			use: z.string(),
			keep: z.string(),
			failure: z.string(),
			control: z.string(),
			anchors: z.array(sourceAnchor),
			consumers: z.array(z.string()),
		}),
	),
	candidates: z.array(
		z.object({
			name: z.string(),
			category: z.string(),
			status: z.string(),
			why: z.string(),
			existing: z.string(),
			next: z.string(),
			consumers: z.array(z.string()),
		}),
	),
});
const [sourcePath, outputPath] = process.argv.slice(2);
if (!sourcePath || !outputPath)
	throw new Error(
		'Usage: node apps/demistify-poc/scripts/build-explorer-reference.ts SOURCE_HTML OUTPUT_DIRECTORY',
	);
const html = await readFile(resolve(sourcePath), 'utf8');
const embedded = /<script id="composition-data" type="application\/json">([\s\S]*?)<\/script>/.exec(
	html,
)?.[1];
if (!embedded) throw new Error('Reference has no composition-data JSON.');
const data = sourceSchema.parse(JSON.parse(embedded));
const nodeById = new Map(data.nodes.map((node) => [node.id, node]));
const sourceLink = (anchor: z.infer<typeof sourceAnchor>): ExplorerSource => ({
	label: `${anchor.path.split('/').slice(-2).join('/')}${anchor.line ? `:${anchor.line}` : ''}`,
	href: `${anchor.sourceUrl && anchor.sourceUrl !== data.sourceUrl ? anchor.sourceUrl : ''}${anchor.path}${anchor.line ? `#L${anchor.line}` : ''}`,
	text: anchor.text,
});
const evidenceOf = (anchors: readonly z.infer<typeof sourceAnchor>[]): readonly ExplorerSource[] =>
	anchors.flatMap((anchor): ExplorerSource[] => [
		sourceLink(anchor),
		...(anchor.via ? [sourceLink(anchor.via)] : []),
	]);
const cell = (
	text: string,
	nodes: readonly string[] = [],
	sources: readonly ExplorerSource[] = [],
): ExplorerCell => ({ text, nodes, sources });
const tables: ExplorerTable[] = [];
tables.push({
	id: 'overview',
	title: 'Proposed phase decomposition · nine phases',
	description: `${data.proposalRevision} · Spec reviewed · not implemented. Shared procedure is a reference file, not a skill.`,
	placement: 'overview',
	filter: 'all',
	columns: ['Phase', 'Judgment reused / teaching extracted', 'What stays in the phase'],
	rows: data.runs.map((run): ExplorerRow => ({
		id: run.run,
		nodes: [run.phase],
		cells: [
			cell(`${run.run} · ${run.title}`, [run.phase]),
			cell(run.movedTo, run.targets),
			cell(run.stays),
		],
	})),
});
tables.push({
	id: 'proposed-changes',
	title: 'Proposed changes',
	description:
		'What leaves the selected phase and what it keeps. These are proposals, not implemented behavior.',
	placement: 'details',
	filter: 'selection',
	columns: [
		'Run · current home',
		'Proposed owner · when / use',
		'Removed from the phase',
		'Stays in the phase',
	],
	rows: data.runs.flatMap((run): ExplorerRow[] =>
		run.changes.map((change, index): ExplorerRow => ({
			id: `${run.run}-${index}`,
			nodes: [run.phase, ...(change.target ? [change.target] : [])],
			cells: [
				cell(`${run.run} · ${run.title}`, [run.phase], [sourceLink(change.anchor)]),
				cell(
					`${change.label}\n${change.conditional ? 'Conditional · ' : ''}When: ${change.when}\nUse: ${change.use}\nWhy: ${change.why}`,
					change.target ? [change.target] : [],
				),
				cell(change.removed),
				cell(`${change.retained}\n\nPressure cases: ${run.pressure}`),
			],
		})),
	),
});
tables.push({
	id: 'authored-changes',
	title: 'Authored changes',
	description: 'Authored in PRs #131 and #132. Testing and behavior evaluation remain deferred.',
	placement: 'details',
	filter: 'all',
	columns: ['Skill · change', 'Why needed', 'What it teaches / how used', 'Source · PR'],
	rows: data.changes.map((change, index): ExplorerRow => {
		const node = nodeById.get(change.id);
		if (!node) throw new Error(`Change references unknown node ${change.id}`);
		const prs = change.prs ?? (change.pr === undefined ? [] : [change.pr]);
		return {
			id: `change-${index}`,
			nodes: [change.id],
			cells: [
				cell(`${change.kind} · AUTHORED`, [change.id]),
				cell(change.why),
				cell(change.what),
				cell(
					'',
					[],
					[
						sourceLink({ path: node.path, text: '' }),
						...prs.map((pullRequestNumber): ExplorerSource => ({
							label: `PR #${pullRequestNumber}`,
							href: `https://github.com/ShravanSunder/sunstack/pull/${pullRequestNumber}`,
							text: '',
						})),
					],
				),
			],
		};
	}),
});
tables.push({
	id: 'extractions',
	title: 'Teaching to extract or reuse',
	description:
		'Remaining proposals from earlier analysis. Historical phase design is not adopted. PROPOSED does not mean implemented or loaded.',
	placement: 'details',
	filter: 'all',
	columns: [
		'Teaching → proposed home',
		'Currently in',
		'Why · when / use',
		'What stays local · controls / sources',
	],
	rows: data.extractions.map((item, index): ExplorerRow => ({
		id: `extraction-${index}`,
		nodes: item.consumers,
		cells: [
			cell(`${item.name}\nPROPOSED · ${item.action}\n→ ${item.home}\n${item.homeState}`),
			cell(item.origins.join('\n'), item.consumers),
			cell(`${item.why}\nWhen: ${item.when}\nUse: ${item.use}`),
			cell(
				`${item.keep}\nFailure: ${item.failure}\nValid control: ${item.control}`,
				[],
				evidenceOf(item.anchors),
			),
		],
	})),
});
tables.push({
	id: 'candidates',
	title: 'Other building blocks',
	description: 'Candidates under consideration; these are not accepted implementation claims.',
	placement: 'details',
	filter: 'all',
	columns: ['Building block', 'Why consider it', 'Existing coverage', 'Who uses it · next change'],
	rows: data.candidates.map((item, index): ExplorerRow => ({
		id: `candidate-${index}`,
		nodes: item.consumers,
		cells: [
			cell(`${item.name}\n${item.category} · ${item.status}`),
			cell(item.why),
			cell(item.existing),
			cell(item.next, item.consumers),
		],
	})),
});
tables.push({
	id: 'earlier-proposals',
	title: 'Earlier proposals',
	description:
		'Earlier practice-level proposals not yet in source; not drawn as current or latest proposed composition.',
	placement: 'details',
	filter: 'all',
	columns: ['Connection', 'Why', 'When / use', 'Sources'],
	rows: data.earlierProposals.map((edge, index): ExplorerRow => ({
		id: `earlier-${index}`,
		nodes: [edge.source, edge.target],
		cells: [
			cell('Earlier proposal · not drawn', [edge.source, edge.target]),
			cell(`${edge.why}\n${edge.rationaleBasis}`),
			cell(`When: ${edge.when}\nUse: ${edge.change}`),
			cell('', [], evidenceOf(edge.anchors)),
		],
	})),
});
tables.push({
	id: 'source-inventory',
	title: 'Source inventory',
	description: `Snapshot ${data.commit}. Declared references do not establish runtime loading, installation or behavioral evaluation.`,
	placement: 'details',
	filter: 'all',
	columns: ['Inspected source'],
	rows: data.inspectedFiles.map((path, index): ExplorerRow => ({
		id: `source-${index}`,
		nodes: [],
		cells: [cell('', [], [sourceLink({ path, text: '' })])],
	})),
});
const reverseTeaching: Readonly<Record<string, string>> = {
	'test-design|principle-write-tests-that-can-fail':
		'Before choosing scenarios, name the concrete wrong implementations each obligation forbids.',
	'role:reviewer|principle-write-tests-that-can-fail':
		'For each test reviewed, judge whether its actual assertion path can catch the specified wrong behavior and allow valid outcomes.',
	'role:verifier|principle-write-tests-that-can-fail':
		'For each test used as proof, check whether its recorded red failed for the intended wrong behavior, rather than setup or discovery.',
	'principle-prove-it-works|principle-write-tests-that-can-fail':
		'Connect real-path proof to a test that can reject wrong behavior. One automated real-path test may serve both judgments.',
};
const edges: ExplorerEdge[] = [data.edges, data.proposals].flatMap(
	(collection, collectionIndex): ExplorerEdge[] =>
		collection.map((edge, index): ExplorerEdge => {
			const source = nodeById.get(edge.source),
				target = nodeById.get(edge.target);
			if (!source || !target)
				throw new Error(`Unknown edge endpoints: ${edge.source} → ${edge.target}`);
			const expands =
				source.level !== 0 &&
				(source.level === 1
					? target.level === 0
					: !target.role && (target.level === 1 || target.level === 0));
			const label =
				edge.relation === 'responsibility'
					? 'names role responsibility'
					: target.role
						? 'uses role teaching'
						: target.kind === 'shared-reference'
							? 'loads shared procedure'
							: target.level === 0
								? 'applies judgment'
								: 'uses method';
			return {
				id: `${collectionIndex === 0 ? 'current' : 'proposed'}-${index}`,
				source: edge.source,
				target: edge.target,
				expansion: expands ? 'composition' : 'related',
				status: collectionIndex === 0 ? 'current' : 'proposed',
				label,
				why: [edge.why, edge.rationaleBasis, reverseTeaching[`${edge.source}|${edge.target}`]]
					.filter(Boolean)
					.join('\n\n'),
				when: `${edge.conditional ? 'Conditional · ' : ''}${edge.when}`,
				use: edge.change,
				sources: evidenceOf(edge.anchors),
			};
		}),
);
const graph = {
	version: 1,
	sourceBase: data.sourceUrl,
	title: 'How the job is composed',
	description:
		'Explore the same frozen skill composition, its proposed changes and source evidence using reusable HTML islands.',
	note: `Current source ${data.commit}. Proposed: ${data.proposalRevision} — ${data.proposalDocument}. Skill descriptions are frontmatter claims; the proposed shared procedure's description comes from the proposal. A shared procedure is a reference file, not a skill. Roles describe responsibility context, not execution order. Orchestrators are deferred. Relationship colors describe target kinds; PROPOSED is a separate status. This is authored branch source, not an installed or behavior-tested release. A declared relationship does not prove a runtime load, assignment or usefulness.`,
	groups: [
		{ id: 'phases', label: 'L2 · Phases', kind: 'phase' },
		{ id: 'roles', label: 'Role skills', kind: 'role' },
		{ id: 'methods', label: 'L1 · Practices / procedure', kind: 'method' },
		{ id: 'principles', label: 'L0 · Principles', kind: 'principle' },
	],
	nodes: data.nodes.map((node) => ({
		id: node.id,
		label: node.name,
		description: node.description,
		group: node.role
			? 'roles'
			: node.level === 0
				? 'principles'
				: node.level === 1
					? 'methods'
					: 'phases',
		issue: node.failure ? 'No direct principle links in current source' : '',
		status: node.proposed ? 'proposed' : 'current',
		sources: [sourceLink({ path: node.path, text: '' })],
	})),
	edges,
	tables,
} satisfies GraphDocument;
parseGraphDocument(graph);
const directory = resolve(outputPath);
await mkdir(directory, { recursive: true });
await writeFile(resolve(directory, 'planning.json'), JSON.stringify(graph));
await writeFile(
	resolve(directory, 'index.html'),
	'<!doctype html>\n<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Skill composition · Demistify</title><link rel="stylesheet" href="demistify/theme.css"><script type="module" src="demistify/runtime.js"></script></head><body><ex-explorer id="skills" src="planning.json" selected="spec-design" view="proposed"></ex-explorer></body></html>\n',
);
await writeFile(
	resolve(directory, 'source-manifest.json'),
	JSON.stringify(
		{
			sourceSha256: createHash('sha256').update(html).digest('hex'),
			sourceBytes: Buffer.byteLength(html),
			sourceCommit: data.commit,
			nodes: graph.nodes.length,
			currentEdges: data.edges.length,
			proposedEdges: data.proposals.length,
			runs: data.runs.length,
			tables: graph.tables.map((table) => ({ id: table.id, rows: table.rows.length })),
			limitations: [
				'Original reference visual is not browser-verified here.',
				'This output is a new component rendering, not a copy of the original application.',
				'Normalization and full referenced content belong in the token ledger.',
			],
		},
		null,
		2,
	),
);
process.stdout.write(
	`Built explorer content: ${graph.nodes.length} nodes, ${graph.edges.length} relationships, ${graph.tables.length} supplementary tables.\n`,
);
