import { createDebouncedAction } from '../core/debounce.ts';
import { escapeHtml } from '../core/html-text.ts';
import { roundedPath } from '../layout/canvas-router.ts';
import {
	tracedEdges,
	type CompositionView,
	type ExplorerEdge,
	type GraphDocument,
} from './graph-document.ts';

interface NodePosition {
	readonly x: number;
	readonly y: number;
	readonly width: number;
	readonly height: number;
	readonly column: number;
}
interface GraphRenderProps {
	readonly viewport: HTMLElement;
	readonly document: GraphDocument;
	readonly view: CompositionView;
	readonly onSelect: (nodeId: string) => void;
	readonly onConnection: (edgeId: string) => void;
}
const svgNamespace = 'http://www.w3.org/2000/svg';

/** Layout belongs to this visual island; it never positions the surrounding document. */
export function renderExplorerGraph(props: GraphRenderProps): void {
	const { viewport, document: graphDocument, view, onSelect, onConnection } = props;
	const positions = new Map<string, NodePosition>();
	const groups = graphDocument.groups.filter((group): boolean =>
		view.nodes.some((node): boolean => node.group === group.id),
	);
	const available = Math.max(760, viewport.clientWidth);
	const cardWidth = Math.max(
		215,
		Math.min(340, (available - 40 - (groups.length - 1) * 75) / Math.max(1, groups.length)),
	);
	const width = Math.max(available, groups.length * (cardWidth + 75) - 35);
	const columnStart = Math.max(
		20,
		(width - (groups.length * cardWidth + Math.max(0, groups.length - 1) * 75)) / 2,
	);
	const canvas = document.createElement('div');
	canvas.className = 'ex-explorer-canvas';
	const nodeLayer = document.createElement('div');
	nodeLayer.className = 'ex-explorer-nodes';
	let hoveredId: string | undefined;
	const pendingHover = createDebouncedAction((): void => {
		if (!canvas.isConnected) return;
		const focused = document.activeElement?.closest<HTMLElement>('.ex-explorer-node');
		paint(
			hoveredId ?? (focused && nodeLayer.contains(focused) ? focused.dataset['nodeId'] : undefined),
		);
	}, 100);
	const paintImmediately = (nodeId?: string): void => {
		pendingHover.cancel();
		paint(nodeId);
	};
	canvas.addEventListener('keydown', pendingHover.cancel, true);
	let rows = 1;
	for (const [column, group] of groups.entries()) {
		const nodes = view.nodes
			.filter((node): boolean => node.group === group.id)
			.toSorted((leftNode, rightNode): number => leftNode.label.localeCompare(rightNode.label));
		rows = Math.max(rows, nodes.length);
		const heading = document.createElement('strong');
		heading.className = 'ex-explorer-column';
		heading.textContent = group.label;
		heading.style.left = `${columnStart + column * (cardWidth + 75)}px`;
		heading.style.width = `${cardWidth}px`;
		nodeLayer.append(heading);
		for (const [row, node] of nodes.entries()) {
			const position = {
				x: columnStart + column * (cardWidth + 75),
				y: 80 + row * 172,
				width: cardWidth,
				height: 148,
				column,
			};
			positions.set(node.id, position);
			const button = document.createElement('button');
			button.type = 'button';
			button.className = `ex-explorer-node ex-explorer-${group.kind}`;
			button.dataset['nodeId'] = node.id;
			button.setAttribute('aria-pressed', String(view.roots.has(node.id)));
			button.style.cssText = `left:${position.x}px;top:${position.y}px;width:${cardWidth}px;height:148px`;
			button.innerHTML = `<strong>${escapeHtml(node.label)}</strong><span>${escapeHtml(node.description)}</span><small>${escapeHtml(group.label)}${node.issue ? ' · ISSUE' : ''}${node.status === 'proposed' ? ' · PROPOSED' : ''}</small>`;
			button.addEventListener('click', (): void => {
				pendingHover.cancel();
				onSelect(node.id);
			});
			button.addEventListener('pointerenter', (): void => {
				hoveredId = node.id;
				pendingHover.schedule();
			});
			button.addEventListener('pointerleave', (): void => {
				hoveredId = undefined;
				pendingHover.schedule();
			});
			button.addEventListener('focus', (): void => paintImmediately(node.id));
			button.addEventListener('blur', (): void => paintImmediately());
			nodeLayer.append(button);
		}
	}
	const height = 80 + rows * 172;
	canvas.style.width = `${width}px`;
	canvas.style.height = `${height}px`;
	const svg = document.createElementNS(svgNamespace, 'svg');
	svg.classList.add('ex-explorer-edges');
	svg.setAttribute('width', String(width));
	svg.setAttribute('height', String(height));
	svg.setAttribute('viewBox', `0 0 ${width} ${height}`);
	const markerPrefix = `explorer-${crypto.randomUUID()}`;
	const groupById = new Map(graphDocument.groups.map((group) => [group.id, group]));
	const nodeById = new Map(graphDocument.nodes.map((node) => [node.id, node]));
	const defs = document.createElementNS(svgNamespace, 'defs');
	for (const kind of ['phase', 'role', 'method', 'principle']) {
		const marker = document.createElementNS(svgNamespace, 'marker');
		marker.id = `${markerPrefix}-${kind}`;
		marker.setAttribute('viewBox', '0 0 8 8');
		marker.setAttribute('refX', '7');
		marker.setAttribute('refY', '4');
		marker.setAttribute('markerWidth', '8');
		marker.setAttribute('markerHeight', '8');
		marker.setAttribute('markerUnits', 'userSpaceOnUse');
		marker.setAttribute('orient', 'auto');
		const triangle = document.createElementNS(svgNamespace, 'path');
		triangle.setAttribute('d', 'M0,0 L8,4 L0,8 Z');
		triangle.setAttribute('fill', `var(--explorer-${kind})`);
		marker.append(triangle);
		defs.append(marker);
	}
	svg.append(defs);
	const edgeElements = new Map<string, SVGGElement>();
	for (const edge of view.edges) {
		const source = positions.get(edge.source),
			target = positions.get(edge.target);
		if (!source || !target) continue;
		const targetNode = nodeById.get(edge.target);
		const kind = targetNode ? (groupById.get(targetNode.group)?.kind ?? 'method') : 'method';
		const startX = source.x + source.width,
			startY = source.y + source.height / 2;
		const endX = target.x - 3,
			endY = target.y + target.height / 2;
		const adjacent = target.column === source.column + 1;
		const pathData = adjacent
			? `M${startX},${startY} C${startX + 32},${startY} ${endX - 32},${endY} ${endX},${endY}`
			: roundedPath(
					[
						{ x: startX, y: startY },
						{ x: startX + 15, y: startY },
						{ x: startX + 15, y: 58 },
						{ x: endX - 15, y: 58 },
						{ x: endX - 15, y: endY },
						{ x: endX, y: endY },
					],
					10,
				);
		const group = document.createElementNS(svgNamespace, 'g');
		group.dataset['edgeId'] = edge.id;
		group.classList.add('ex-explorer-edge');
		group.classList.toggle('ex-explorer-proposed', edge.status === 'proposed');
		const path = document.createElementNS(svgNamespace, 'path');
		path.setAttribute('d', pathData);
		path.setAttribute('stroke', `var(--explorer-${kind})`);
		path.setAttribute('marker-end', `url(#${markerPrefix}-${kind})`);
		path.classList.add('ex-explorer-line');
		const hit = document.createElementNS(svgNamespace, 'path');
		hit.setAttribute('d', pathData);
		hit.classList.add('ex-explorer-edge-hit');
		const title = document.createElementNS(svgNamespace, 'title');
		title.textContent = `${nodeById.get(edge.source)?.label} → ${targetNode?.label} · ${edge.label}${edge.status === 'proposed' ? ' · PROPOSED' : ''}`;
		group.append(path, hit, title);
		group.addEventListener('click', (): void => {
			pendingHover.cancel();
			onConnection(edge.id);
		});
		edgeElements.set(edge.id, group);
		svg.append(group);
	}
	function paint(nodeId?: string): void {
		const chosen = nodeId
			? tracedEdges(view, nodeId)
			: new Set(
					view.edges
						.filter((edge): boolean => view.roots.has(edge.source))
						.map((edge): string => edge.id),
				);
		for (const [id, element] of edgeElements) {
			element.classList.toggle('is-visible', chosen.has(id));
			element.classList.toggle('is-traced', Boolean(nodeId) && chosen.has(id));
		}
	}
	paint();
	canvas.append(svg, nodeLayer);
	viewport.replaceChildren(canvas);
}

export function edgeDescription(edge: ExplorerEdge): string {
	return `${edge.status === 'proposed' ? 'PROPOSED' : 'CURRENT'} · ${edge.label}`;
}
