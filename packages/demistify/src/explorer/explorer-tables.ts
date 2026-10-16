import type {
	ExplorerCell,
	ExplorerEdge,
	ExplorerNode,
	ExplorerSource,
	ExplorerTable,
} from './graph-document.ts';
import { edgeDescription } from './graph-renderer.ts';

export function appendSources(container: HTMLElement, sources: readonly ExplorerSource[]): void {
	for (const source of sources) {
		const detail = document.createElement('details');
		const summary = document.createElement('summary');
		const link = document.createElement('a');
		link.href = source.href;
		link.textContent = source.label;
		link.target = '_blank';
		link.rel = 'noopener';
		summary.append(link);
		detail.append(summary);
		if (source.text) {
			const quote = document.createElement('blockquote');
			quote.textContent = source.text;
			detail.append(quote);
		}
		container.append(detail);
	}
}
function nodeButton(node: ExplorerNode, select: (id: string) => void): HTMLButtonElement {
	const button = document.createElement('button');
	button.type = 'button';
	button.textContent = node.label;
	button.className = 'ex-explorer-link';
	button.addEventListener('click', (): void => select(node.id));
	return button;
}
function tableShell(columns: readonly string[]): {
	readonly container: HTMLElement;
	readonly body: HTMLTableSectionElement;
} {
	const container = document.createElement('div');
	container.className = 'ex-explorer-table-scroll';
	const table = document.createElement('table');
	const head = document.createElement('thead');
	const row = document.createElement('tr');
	for (const column of columns) {
		const cell = document.createElement('th');
		cell.scope = 'col';
		cell.textContent = column;
		row.append(cell);
	}
	head.append(row);
	const body = document.createElement('tbody');
	table.append(head, body);
	container.append(table);
	return { container, body };
}
interface ConnectionTableProps {
	readonly edges: readonly ExplorerEdge[];
	readonly nodeById: ReadonlyMap<string, ExplorerNode>;
	readonly selectedEdge: string;
	readonly incoming: boolean;
	readonly select: (id: string) => void;
}
export function connectionTable(props: ConnectionTableProps): HTMLElement {
	const { container, body } = tableShell([
		props.incoming ? 'Used by' : 'Connection · status',
		'Why needed',
		'When / how used',
		'Source evidence',
	]);
	for (const edge of props.edges) {
		const row = document.createElement('tr');
		row.dataset['edgeId'] = edge.id;
		row.classList.toggle('is-selected', edge.id === props.selectedEdge);
		const relation = document.createElement('td');
		const source = props.nodeById.get(edge.source),
			target = props.nodeById.get(edge.target);
		if (source) relation.append(nodeButton(source, props.select));
		if (!props.incoming && target) {
			relation.append(document.createTextNode(' → '), nodeButton(target, props.select));
		}
		const status = document.createElement('small');
		status.textContent = edgeDescription(edge);
		status.className = `ex-explorer-status ${edge.status}`;
		relation.append(status);
		const why = document.createElement('td');
		why.textContent = edge.why;
		const use = document.createElement('td');
		use.textContent = [edge.when, edge.use].filter(Boolean).join('\n\n');
		const sources = document.createElement('td');
		appendSources(sources, edge.sources);
		row.append(relation, why, use, sources);
		body.append(row);
	}
	if (!props.edges.length) {
		const row = document.createElement('tr');
		const cell = document.createElement('td');
		cell.colSpan = 4;
		cell.textContent = 'No declared connections in this view.';
		row.append(cell);
		body.append(row);
	}
	return container;
}
function appendCell(
	container: HTMLElement,
	cell: ExplorerCell,
	nodes: ReadonlyMap<string, ExplorerNode>,
	select: (id: string) => void,
): void {
	if (cell.text) {
		const text = document.createElement('p');
		text.textContent = cell.text;
		container.append(text);
	}
	for (const id of cell.nodes) {
		const node = nodes.get(id);
		if (node) container.append(nodeButton(node, select));
	}
	appendSources(container, cell.sources);
}
interface SupplementaryTableProps {
	readonly table: ExplorerTable;
	readonly selection: string;
	readonly nodes: ReadonlyMap<string, ExplorerNode>;
	readonly select: (id: string) => void;
}
export function supplementaryTable({
	table,
	selection,
	nodes,
	select,
}: SupplementaryTableProps): HTMLElement {
	const section = document.createElement('section');
	section.className = 'ex-explorer-supplement';
	const heading = document.createElement(table.placement === 'overview' ? 'h2' : 'h3');
	heading.textContent = table.title;
	section.append(heading);
	const description = document.createElement('p');
	description.className = 'ex-explorer-caption';
	description.textContent = table.description;
	section.append(description);
	const matching = table.rows.filter((row): boolean => row.nodes.includes(selection));
	const rows = table.filter === 'selection' && matching.length ? matching : table.rows;
	const { container, body } = tableShell(table.columns);
	for (const item of rows) {
		const row = document.createElement('tr');
		row.dataset['rowId'] = item.id;
		row.classList.toggle('is-selected', item.nodes.includes(selection));
		for (const cell of item.cells) {
			const element = document.createElement('td');
			appendCell(element, cell, nodes, select);
			row.append(element);
		}
		body.append(row);
	}
	section.append(container);
	return section;
}
