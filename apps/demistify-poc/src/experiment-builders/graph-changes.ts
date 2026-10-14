import { z } from 'zod';

const graphIdentity = z.string().regex(/^[a-z][a-z0-9_]*$/);
const nodeSchema = z.strictObject({ id: graphIdentity, label: z.string().min(1) });
const edgeSchema = z.strictObject({
	id: graphIdentity,
	from: graphIdentity,
	to: graphIdentity,
	label: z.string().min(1),
});
export const graphSchema = z.strictObject({
	title: z.string().min(1),
	nodes: z.array(nodeSchema),
	edges: z.array(edgeSchema),
});
export type ArchitectureGraph = z.infer<typeof graphSchema>;

export const changeSetSchema = z.strictObject({
	base: z.string().min(1),
	changes: z.array(
		z.discriminatedUnion('kind', [
			z.strictObject({ kind: z.literal('add-node'), node: nodeSchema }),
			z.strictObject({
				kind: z.literal('change-node'),
				id: graphIdentity,
				label: z.string().min(1),
			}),
			z.strictObject({ kind: z.literal('remove-node'), id: graphIdentity }),
			z.strictObject({ kind: z.literal('add-edge'), edge: edgeSchema }),
			z.strictObject({ kind: z.literal('remove-edge'), id: graphIdentity }),
		]),
	),
});
export type GraphChangeSet = z.infer<typeof changeSetSchema>;

export function validateGraph(graph: ArchitectureGraph): void {
	const nodeIds = new Set<string>();
	const edgeIds = new Set<string>();
	for (const node of graph.nodes) {
		if (nodeIds.has(node.id)) throw new Error(`Duplicate node: ${node.id}`);
		nodeIds.add(node.id);
	}
	for (const edge of graph.edges) {
		if (edgeIds.has(edge.id)) throw new Error(`Duplicate edge: ${edge.id}`);
		edgeIds.add(edge.id);
		if (!nodeIds.has(edge.from) || !nodeIds.has(edge.to))
			throw new Error(`Unknown edge endpoint: ${edge.id}`);
	}
}

export function applyGraphChanges(
	baseGraph: ArchitectureGraph,
	changeSet: GraphChangeSet,
): ArchitectureGraph {
	validateGraph(baseGraph);
	const proposedGraph = structuredClone(baseGraph);
	for (const change of changeSet.changes) {
		switch (change.kind) {
			case 'add-node':
				if (proposedGraph.nodes.some((node): boolean => node.id === change.node.id))
					throw new Error(`Duplicate node: ${change.node.id}`);
				proposedGraph.nodes.push(structuredClone(change.node));
				break;
			case 'change-node': {
				const changedNode = proposedGraph.nodes.find((node): boolean => node.id === change.id);
				if (!changedNode) throw new Error(`Unknown node: ${change.id}`);
				changedNode.label = change.label;
				break;
			}
			case 'remove-node':
				if (!proposedGraph.nodes.some((node): boolean => node.id === change.id))
					throw new Error(`Unknown node: ${change.id}`);
				proposedGraph.nodes = proposedGraph.nodes.filter((node): boolean => node.id !== change.id);
				break;
			case 'add-edge':
				if (proposedGraph.edges.some((edge): boolean => edge.id === change.edge.id))
					throw new Error(`Duplicate edge: ${change.edge.id}`);
				proposedGraph.edges.push(structuredClone(change.edge));
				break;
			case 'remove-edge':
				if (!proposedGraph.edges.some((edge): boolean => edge.id === change.id))
					throw new Error(`Unknown edge: ${change.id}`);
				proposedGraph.edges = proposedGraph.edges.filter((edge): boolean => edge.id !== change.id);
				break;
		}
	}
	validateGraph(proposedGraph);
	return proposedGraph;
}

function graphThemeColor(themeColors: Readonly<Record<string, string>>, name: string): string {
	const color = themeColors[name];
	if (!color || !/^#[\da-f]{6}$/i.test(color))
		throw new Error(`Missing graph theme color: ${name}`);
	return color;
}

export function graphToD2(
	graph: ArchitectureGraph,
	themeColors: Readonly<Record<string, string>>,
	changes?: GraphChangeSet,
): string {
	const lines = ['direction: down', `style.fill: "${graphThemeColor(themeColors, 'sheet')}"`];
	for (const node of graph.nodes) {
		const change = changes?.changes.find((item): boolean =>
			item.kind === 'add-node'
				? item.node.id === node.id
				: (item.kind === 'change-node' || item.kind === 'remove-node') && item.id === node.id,
		);
		const status =
			change?.kind === 'add-node'
				? 'added'
				: change?.kind === 'remove-node'
					? 'removed'
					: change?.kind === 'change-node'
						? 'modified'
						: undefined;
		lines.push(`${node.id}: ${JSON.stringify(status ? `${node.label} · ${status}` : node.label)}`);
		const styleKind = status ?? 'unchanged';
		lines.push(`${node.id}.style.fill: "${graphThemeColor(themeColors, `diff-${styleKind}-bg`)}"`);
		lines.push(`${node.id}.style.stroke: "${graphThemeColor(themeColors, `diff-${styleKind}`)}"`);
		lines.push(`${node.id}.style.font-color: "${graphThemeColor(themeColors, 'ink')}"`);
	}
	for (const edge of graph.edges) {
		const change = changes?.changes.find((item): boolean =>
			item.kind === 'add-edge'
				? item.edge.id === edge.id
				: item.kind === 'remove-edge' && item.id === edge.id,
		);
		const status =
			change?.kind === 'add-edge'
				? 'added'
				: change?.kind === 'remove-edge'
					? 'removed'
					: undefined;
		lines.push(
			`${edge.from} -> ${edge.to}: {label: ${JSON.stringify(status ? `${edge.label} [${status}]` : edge.label)}; style.stroke: "${graphThemeColor(themeColors, `diff-${status ?? 'unchanged'}`)}"; style.font-color: "${graphThemeColor(themeColors, 'ink')}"}`,
		);
	}
	return `${lines.join('\n')}\n`;
}

export function graphToMermaid(graph: ArchitectureGraph, changes?: GraphChangeSet): string {
	const lines = ['flowchart TD'];
	for (const node of graph.nodes) {
		const change = changes?.changes.find((item): boolean =>
			item.kind === 'add-node'
				? item.node.id === node.id
				: (item.kind === 'remove-node' || item.kind === 'change-node') && item.id === node.id,
		);
		const status =
			change?.kind === 'add-node'
				? 'added'
				: change?.kind === 'remove-node'
					? 'removed'
					: change?.kind === 'change-node'
						? 'modified'
						: 'unchanged';
		lines.push(
			`  ${node.id}[${JSON.stringify(status === 'unchanged' ? node.label : `${node.label} · ${status}`)}]:::${status}`,
		);
	}
	graph.edges.forEach((edge, index): void => {
		const change = changes?.changes.find((item): boolean =>
			item.kind === 'add-edge'
				? item.edge.id === edge.id
				: item.kind === 'remove-edge' && item.id === edge.id,
		);
		const status =
			change?.kind === 'add-edge'
				? 'added'
				: change?.kind === 'remove-edge'
					? 'removed'
					: 'unchanged';
		lines.push(
			`  ${edge.from} -->|${JSON.stringify(status === 'unchanged' ? edge.label : `${edge.label} [${status}]`)}| ${edge.to}`,
		);
		lines.push(`  linkStyle ${index} stroke:var(--diff-${status})`);
	});
	for (const status of ['added', 'removed', 'modified', 'unchanged'])
		lines.push(
			`  classDef ${status} fill:var(--diff-${status}-bg),stroke:var(--diff-${status}),color:var(--ink)`,
		);
	return `${lines.join('\n')}\n`;
}

export function overlayGraph(
	baseGraph: ArchitectureGraph,
	changeSet: GraphChangeSet,
): ArchitectureGraph {
	const proposedGraph = applyGraphChanges(baseGraph, changeSet);
	return {
		title: baseGraph.title,
		nodes: [
			...proposedGraph.nodes,
			...baseGraph.nodes.filter(
				(node): boolean => !proposedGraph.nodes.some((item): boolean => item.id === node.id),
			),
		],
		edges: [
			...proposedGraph.edges,
			...baseGraph.edges.filter(
				(edge): boolean => !proposedGraph.edges.some((item): boolean => item.id === edge.id),
			),
		],
	};
}
