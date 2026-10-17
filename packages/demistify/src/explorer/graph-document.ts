import { resolveSubjectIcon } from '../core/subject-icons.ts';

/** Content contract for the explorer. Presentation and transient selection are library-owned. */
export interface ExplorerSource {
	readonly label: string;
	readonly href: string;
	readonly text: string;
}
export interface ExplorerGroup {
	readonly id: string;
	readonly label: string;
	readonly kind: 'phase' | 'role' | 'method' | 'principle';
}
export interface ExplorerNode {
	readonly id: string;
	readonly label: string;
	readonly description: string;
	readonly group: string;
	readonly issue: string;
	readonly status: 'current' | 'proposed';
	readonly sources: readonly ExplorerSource[];
}
export interface ExplorerEdge {
	readonly id: string;
	readonly source: string;
	readonly target: string;
	readonly expansion: 'composition' | 'related';
	readonly status: 'current' | 'proposed';
	readonly label: string;
	readonly why: string;
	readonly when: string;
	readonly use: string;
	readonly sources: readonly ExplorerSource[];
}
export interface ExplorerCell {
	readonly text: string;
	readonly nodes: readonly string[];
	readonly sources: readonly ExplorerSource[];
}
export interface ExplorerRow {
	readonly id: string;
	readonly nodes: readonly string[];
	readonly cells: readonly ExplorerCell[];
}
export interface ExplorerTable {
	readonly id: string;
	readonly title: string;
	readonly label?: string;
	readonly icon?: string;
	readonly description: string;
	readonly placement: 'overview' | 'details';
	readonly filter: 'all' | 'selection';
	readonly columns: readonly string[];
	readonly rows: readonly ExplorerRow[];
}
export interface GraphDocument {
	readonly version: 1;
	readonly sourceBase: string;
	readonly title: string;
	readonly description: string;
	readonly note: string;
	readonly groups: readonly ExplorerGroup[];
	readonly nodes: readonly ExplorerNode[];
	readonly edges: readonly ExplorerEdge[];
	readonly tables: readonly ExplorerTable[];
}
export type ExplorerMode = 'current' | 'proposed';
export interface CompositionView {
	readonly roots: ReadonlySet<string>;
	readonly nodes: readonly ExplorerNode[];
	readonly edges: readonly ExplorerEdge[];
	readonly related: readonly ExplorerEdge[];
}

function recordOf(value: unknown, field: string): Record<string, unknown> {
	if (typeof value !== 'object' || value === null || Array.isArray(value))
		throw new Error(`${field} must be an object.`);
	return { ...value };
}
function textOf(value: unknown, field: string): string {
	if (typeof value !== 'string') throw new Error(`${field} must be text.`);
	return value;
}
function arrayOf(value: unknown, field: string): readonly unknown[] {
	if (!Array.isArray(value)) throw new Error(`${field} must be an array.`);
	return value;
}
function stringsOf(value: unknown, field: string): readonly string[] {
	return arrayOf(value, field).map((item): string => textOf(item, field));
}
function choiceOf<TChoice extends string>(
	value: unknown,
	choices: readonly TChoice[],
	field: string,
): TChoice {
	for (const choice of choices) if (value === choice) return choice;
	throw new Error(`${field} must be one of: ${choices.join(', ')}.`);
}
function sourcesOf(value: unknown, sourceBase: string): readonly ExplorerSource[] {
	return arrayOf(value, 'sources').map((item): ExplorerSource => {
		const source = recordOf(item, 'source');
		const href = textOf(source['href'], 'source.href');
		const url = new URL(href, sourceBase);
		if (!['http:', 'https:'].includes(url.protocol))
			throw new Error('Source URL must use HTTP or HTTPS.');
		return {
			label: textOf(source['label'], 'source.label'),
			href: url.href,
			text: textOf(source['text'], 'source.text'),
		};
	});
}
function uniqueIds(items: readonly { readonly id: string }[], field: string): Set<string> {
	const ids = new Set<string>();
	for (const item of items) {
		if (!item.id || ids.has(item.id))
			throw new Error(`${field} contains an empty or duplicate id: ${item.id}.`);
		ids.add(item.id);
	}
	return ids;
}

/** Parse once at the JSON boundary; graph and tables consume the same validated identities. */
export function parseGraphDocument(input: unknown): GraphDocument {
	const document = recordOf(input, 'graph document');
	if (document['version'] !== 1) throw new Error('Unsupported graph document version; expected 1.');
	const sourceBase = textOf(document['sourceBase'], 'sourceBase');
	if (!['http:', 'https:'].includes(new URL(sourceBase).protocol))
		throw new Error('sourceBase must use HTTP or HTTPS.');
	const groups = arrayOf(document['groups'], 'groups').map((item): ExplorerGroup => {
		const group = recordOf(item, 'group');
		return {
			id: textOf(group['id'], 'group.id'),
			label: textOf(group['label'], 'group.label'),
			kind: choiceOf(group['kind'], ['phase', 'role', 'method', 'principle'], 'group.kind'),
		};
	});
	const nodes = arrayOf(document['nodes'], 'nodes').map((item): ExplorerNode => {
		const node = recordOf(item, 'node');
		return {
			id: textOf(node['id'], 'node.id'),
			label: textOf(node['label'], 'node.label'),
			description: textOf(node['description'], 'node.description'),
			group: textOf(node['group'], 'node.group'),
			issue: textOf(node['issue'], 'node.issue'),
			status: choiceOf(node['status'], ['current', 'proposed'], 'node.status'),
			sources: sourcesOf(node['sources'], sourceBase),
		};
	});
	const edges = arrayOf(document['edges'], 'edges').map((item): ExplorerEdge => {
		const edge = recordOf(item, 'edge');
		return {
			id: textOf(edge['id'], 'edge.id'),
			source: textOf(edge['source'], 'edge.source'),
			target: textOf(edge['target'], 'edge.target'),
			expansion: choiceOf(edge['expansion'], ['composition', 'related'], 'edge.expansion'),
			status: choiceOf(edge['status'], ['current', 'proposed'], 'edge.status'),
			label: textOf(edge['label'], 'edge.label'),
			why: textOf(edge['why'], 'edge.why'),
			when: textOf(edge['when'], 'edge.when'),
			use: textOf(edge['use'], 'edge.use'),
			sources: sourcesOf(edge['sources'], sourceBase),
		};
	});
	const tables = arrayOf(document['tables'], 'tables').map((item): ExplorerTable => {
		const table = recordOf(item, 'table');
		const title = textOf(table['title'], 'table.title');
		const label = table['label'] === undefined ? title : textOf(table['label'], 'table.label');
		const requestedIcon =
			table['icon'] === undefined ? 'file' : textOf(table['icon'], 'table.icon');
		const icon = resolveSubjectIcon(requestedIcon);
		if (!icon)
			throw new Error(`Unknown table icon "${requestedIcon}". Use a curated subject icon.`);
		const rows = arrayOf(table['rows'], 'table.rows').map((entry): ExplorerRow => {
			const row = recordOf(entry, 'row');
			return {
				id: textOf(row['id'], 'row.id'),
				nodes: stringsOf(row['nodes'], 'row.nodes'),
				cells: arrayOf(row['cells'], 'row.cells').map((cellInput): ExplorerCell => {
					const cell = recordOf(cellInput, 'cell');
					return {
						text: textOf(cell['text'], 'cell.text'),
						nodes: stringsOf(cell['nodes'], 'cell.nodes'),
						sources: sourcesOf(cell['sources'], sourceBase),
					};
				}),
			};
		});
		return {
			id: textOf(table['id'], 'table.id'),
			title,
			label,
			icon,
			description: textOf(table['description'], 'table.description'),
			placement: choiceOf(table['placement'], ['overview', 'details'], 'table.placement'),
			filter: choiceOf(table['filter'], ['all', 'selection'], 'table.filter'),
			columns: stringsOf(table['columns'], 'table.columns'),
			rows,
		};
	});
	if (!nodes.length) throw new Error('Graph document needs at least one node.');
	const groupIds = uniqueIds(groups, 'groups');
	const nodeIds = uniqueIds(nodes, 'nodes');
	if (nodeIds.has('__issues')) throw new Error('Reserved node id: __issues.');
	uniqueIds(edges, 'edges');
	uniqueIds(tables, 'tables');
	const requireNode = (id: string): void => {
		if (!nodeIds.has(id)) throw new Error(`Unknown graph node: ${id}.`);
	};
	for (const node of nodes)
		if (!groupIds.has(node.group)) throw new Error(`Unknown node group: ${node.group}.`);
	for (const edge of edges) {
		requireNode(edge.source);
		requireNode(edge.target);
	}
	for (const table of tables) {
		if (table.placement === 'details' && ['connections', 'used-by'].includes(table.id))
			throw new Error(`Reserved detail table id: ${table.id}.`);
		uniqueIds(table.rows, `table ${table.id}`);
		for (const row of table.rows) {
			if (row.cells.length !== table.columns.length)
				throw new Error(`Wrong cell count in table ${table.id}, row ${row.id}.`);
			row.nodes.forEach(requireNode);
			for (const cell of row.cells) cell.nodes.forEach(requireNode);
		}
	}
	return {
		version: 1,
		sourceBase,
		title: textOf(document['title'], 'title'),
		description: textOf(document['description'], 'description'),
		note: textOf(document['note'], 'note'),
		groups,
		nodes,
		edges,
		tables,
	};
}

export function compositionView(
	document: GraphDocument,
	selection: string,
	mode: ExplorerMode,
): CompositionView {
	const nodeById = new Map(document.nodes.map((node) => [node.id, node]));
	const roots = new Set(
		selection === '__issues'
			? document.nodes.filter((node): boolean => Boolean(node.issue)).map((node): string => node.id)
			: [selection],
	);
	for (const id of roots) if (!nodeById.has(id)) throw new Error(`Unknown graph selection: ${id}.`);
	const available = document.edges.filter(
		(edge): boolean => mode === 'proposed' || edge.status === 'current',
	);
	const outgoing = new Map<string, ExplorerEdge[]>();
	for (const edge of available) {
		if (edge.expansion !== 'composition') continue;
		const entries = outgoing.get(edge.source) ?? [];
		entries.push(edge);
		outgoing.set(edge.source, entries);
	}
	const visible = new Set<string>();
	const selectedEdges = new Map<string, ExplorerEdge>();
	const queue = [...roots];
	while (queue.length) {
		const id = queue.pop();
		if (id === undefined || visible.has(id)) continue;
		visible.add(id);
		for (const edge of outgoing.get(id) ?? []) {
			selectedEdges.set(edge.id, edge);
			queue.push(edge.target);
		}
	}
	return {
		roots,
		nodes: document.nodes.filter((node): boolean => visible.has(node.id)),
		edges: [...selectedEdges.values()],
		related: available.filter(
			(edge): boolean => roots.has(edge.source) && edge.expansion === 'related',
		),
	};
}

/** Trace ancestors and immediate outgoing uses, rather than flooding every descendant. */
export function tracedEdges(view: CompositionView, focusedId: string): ReadonlySet<string> {
	const chosen = new Set<string>();
	const visited = new Set<string>();
	const queue = [focusedId];
	while (queue.length) {
		const id = queue.pop();
		if (id === undefined || visited.has(id)) continue;
		visited.add(id);
		for (const edge of view.edges)
			if (edge.target === id) {
				chosen.add(edge.id);
				queue.push(edge.source);
			}
	}
	for (const edge of view.edges) if (edge.source === focusedId) chosen.add(edge.id);
	return chosen;
}
