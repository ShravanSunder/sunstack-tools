/**
 * Orthogonal connector routing around boxes for hand-placed layouts (grid, flex or absolute).
 * Routes run on a sparse grid built from every box edge and the channel midlines between boxes, so
 * a route keeps clear of boxes, prefers gap centres and few bends. Overlapping routes are then
 * spread into lanes, and labels are placed where they cover no box and no other label.
 */
export interface Box {
	left: number;
	top: number;
	right: number;
	bottom: number;
}
export interface Point {
	x: number;
	y: number;
}
export type Side = 'left' | 'right' | 'top' | 'bottom';
export const sides: readonly Side[] = ['right', 'bottom', 'left', 'top'];

export interface RouteRequest {
	readonly source: Box;
	readonly target: Box;
	readonly sourceSides: readonly Side[];
	readonly targetSides: readonly Side[];
}

const clearance = 14;
const bendCost = 60;
const crossingCost = 120;
const awaySideCost = 80;
const laneGap = 7;
const normals: Record<Side, Point> = {
	left: { x: -1, y: 0 },
	right: { x: 1, y: 0 },
	top: { x: 0, y: -1 },
	bottom: { x: 0, y: 1 },
};

const centre = (box: Box): Point => ({
	x: (box.left + box.right) / 2,
	y: (box.top + box.bottom) / 2,
});
const inflate = (box: Box, margin: number): Box => ({
	left: box.left - margin,
	top: box.top - margin,
	right: box.right + margin,
	bottom: box.bottom + margin,
});
export const overlaps = (first: Box, second: Box): boolean =>
	first.left < second.right &&
	second.left < first.right &&
	first.top < second.bottom &&
	second.top < first.bottom;

/** Where a route leaves a side: the shared span when the boxes face each other, else the middle. */
function portOn(box: Box, side: Side, other: Box): Point {
	if (side === 'left' || side === 'right') {
		const low = Math.max(box.top, other.top);
		const high = Math.min(box.bottom, other.bottom);
		return {
			x: side === 'left' ? box.left : box.right,
			y: high - low > 24 ? (low + high) / 2 : (box.top + box.bottom) / 2,
		};
	}
	const low = Math.max(box.left, other.left);
	const high = Math.min(box.right, other.right);
	return {
		x: high - low > 24 ? (low + high) / 2 : (box.left + box.right) / 2,
		y: side === 'top' ? box.top : box.bottom,
	};
}

const step = (point: Point, side: Side, distance: number): Point => ({
	x: point.x + normals[side].x * distance,
	y: point.y + normals[side].y * distance,
});

/** Box edges, inflated edges and the midlines of the gaps between neighbouring edges. */
function axisLines(values: readonly number[], extra: readonly number[]): number[] {
	const edges = Array.from(new Set(values.map((value): number => Math.round(value)))).toSorted(
		(first, second): number => first - second,
	);
	const lines = new Set<number>(extra.map((value): number => Math.round(value * 2) / 2));
	for (const edge of edges) {
		lines.add(edge - clearance);
		lines.add(edge + clearance);
	}
	for (let index = 1; index < edges.length; index += 1) {
		const low = edges[index - 1] ?? 0;
		const high = edges[index] ?? 0;
		if (high - low > clearance * 2) lines.add((low + high) / 2);
	}
	return Array.from(lines).toSorted((first, second): number => first - second);
}

/** Minimal binary heap keyed by priority (path cost plus the distance still to go). */
class Queue {
	private readonly items: { priority: number; cost: number; state: number }[] = [];
	get size(): number {
		return this.items.length;
	}
	push(priority: number, cost: number, state: number): void {
		const items = this.items;
		items.push({ priority, cost, state });
		let index = items.length - 1;
		while (index > 0) {
			const parent = (index - 1) >> 1;
			const above = items[parent];
			const current = items[index];
			if (!above || !current || above.priority <= current.priority) break;
			items[parent] = current;
			items[index] = above;
			index = parent;
		}
	}
	pop(): { priority: number; cost: number; state: number } | undefined {
		const items = this.items;
		const top = items[0];
		const last = items.pop();
		if (items.length > 0 && last) {
			items[0] = last;
			let index = 0;
			for (;;) {
				const left = index * 2 + 1;
				const right = left + 1;
				let smallest = index;
				if ((items[left]?.priority ?? Infinity) < (items[smallest]?.priority ?? Infinity))
					smallest = left;
				if ((items[right]?.priority ?? Infinity) < (items[smallest]?.priority ?? Infinity))
					smallest = right;
				if (smallest === index) break;
				const swap = items[smallest];
				const current = items[index];
				if (!swap || !current) break;
				items[smallest] = current;
				items[index] = swap;
				index = smallest;
			}
		}
		return top;
	}
}

const sideDirection: Record<Side, number> = { right: 0, bottom: 1, left: 2, top: 3 };

/** Removes points that sit on a straight line between their neighbours. */
function simplify(points: readonly Point[]): Point[] {
	const result: Point[] = [];
	for (const point of points) {
		const previous = result.at(-1);
		if (previous && Math.abs(previous.x - point.x) < 0.5 && Math.abs(previous.y - point.y) < 0.5)
			continue;
		const before = result.at(-2);
		if (
			before &&
			previous &&
			((Math.abs(before.x - previous.x) < 0.5 && Math.abs(previous.x - point.x) < 0.5) ||
				(Math.abs(before.y - previous.y) < 0.5 && Math.abs(previous.y - point.y) < 0.5))
		)
			result.pop();
		result.push(point);
	}
	return result;
}

/** Index of the line equal to `value` (within half a pixel), else -1. */
function lineIndex(lines: readonly number[], value: number): number {
	let low = 0;
	let high = lines.length - 1;
	while (low <= high) {
		const middle = (low + high) >> 1;
		const line = lines[middle] ?? 0;
		if (Math.abs(line - value) < 0.5) return middle;
		if (line < value) low = middle + 1;
		else high = middle - 1;
	}
	return -1;
}

/** Index of the last line strictly below `value` (−1 if none). */
function gapBelow(lines: readonly number[], value: number): number {
	let low = 0;
	let high = lines.length - 1;
	let found = -1;
	while (low <= high) {
		const middle = (low + high) >> 1;
		if ((lines[middle] ?? 0) < value - 0.5) {
			found = middle;
			low = middle + 1;
		} else high = middle - 1;
	}
	return found;
}

const bump = (grid: Uint16Array, node: number): void => {
	grid[node] = (grid[node] ?? 0) + 1;
};

/**
 * Finds the cheapest orthogonal route from a side of the source to a side of the target that keeps
 * `clearance` away from every obstacle. Costs are length, bends and crossings of earlier routes; the
 * search is A* with the remaining Manhattan distance, and every obstacle and earlier route is
 * stamped onto the grid once, so each step is a lookup. Returns undefined when no gap is left.
 */
export function routeConnector(
	request: RouteRequest,
	obstacles: readonly Box[],
	earlierRoutes: readonly (readonly Point[] | undefined)[] = [],
): Point[] | undefined {
	const { source, target } = request;
	const towards = centre(target);
	const from = centre(source);
	const blockers = [source, target, ...obstacles].map((box): Box => inflate(box, clearance));
	const starts = request.sourceSides.map((side) => {
		const port = portOn(source, side, target);
		return { side, port, stub: step(port, side, clearance) };
	});
	const ends = request.targetSides.map((side) => {
		const port = portOn(target, side, source);
		return { side, port, stub: step(port, side, clearance) };
	});
	const all = [source, target, ...obstacles];
	const columnLines = axisLines(
		all.flatMap((box): number[] => [box.left, box.right]),
		[...starts, ...ends].flatMap((end): number[] => [end.stub.x]),
	);
	const rowLines = axisLines(
		all.flatMap((box): number[] => [box.top, box.bottom]),
		[...starts, ...ends].flatMap((end): number[] => [end.stub.y]),
	);
	const columns = columnLines.length;
	const rows = rowLines.length;
	const nodeAt = (column: number, row: number): number => column * rows + row;
	const column = (value: number): number => columnLines[value] ?? 0;
	const row = (value: number): number => rowLines[value] ?? 0;

	// Stamp each obstacle: nodes strictly inside it, and steps whose middle is inside it.
	const blockedNode = new Uint8Array(columns * rows);
	const blockedRight = new Uint8Array(columns * rows);
	const blockedDown = new Uint8Array(columns * rows);
	for (const box of blockers) {
		for (let x = 0; x < columns; x += 1) {
			const atX = column(x);
			const nextX = column(x + 1);
			const nodeInsideX = atX > box.left + 0.5 && atX < box.right - 0.5;
			const stepInsideX =
				x + 1 < columns &&
				(atX + nextX) / 2 > box.left + 0.5 &&
				(atX + nextX) / 2 < box.right - 0.5;
			if (!nodeInsideX && !stepInsideX) continue;
			for (let y = 0; y < rows; y += 1) {
				const atY = row(y);
				const insideY = atY > box.top + 0.5 && atY < box.bottom - 0.5;
				const node = nodeAt(x, y);
				if (nodeInsideX && insideY) blockedNode[node] = 1;
				if (stepInsideX && insideY) blockedRight[node] = 1;
				const middleY = (atY + row(y + 1)) / 2;
				if (nodeInsideX && y + 1 < rows && middleY > box.top + 0.5 && middleY < box.bottom - 0.5)
					blockedDown[node] = 1;
			}
		}
	}

	// Stamp earlier routes: how many of their segments a step crosses. A step counts a crossing at
	// its start but not its end, so a crossing exactly on a grid line is counted once.
	const crossRight = new Uint16Array(columns * rows);
	const crossLeft = new Uint16Array(columns * rows);
	const crossDown = new Uint16Array(columns * rows);
	const crossUp = new Uint16Array(columns * rows);
	for (const route of earlierRoutes) {
		if (!route) continue;
		for (let index = 0; index < route.length - 1; index += 1) {
			const first = route[index];
			const second = route[index + 1];
			if (!first || !second) continue;
			if (Math.abs(first.y - second.y) < 0.5) {
				// A horizontal segment is crossed by vertical steps at columns strictly inside it.
				const low = Math.min(first.x, second.x);
				const high = Math.max(first.x, second.x);
				const onLine = lineIndex(rowLines, first.y);
				const below = gapBelow(rowLines, first.y);
				for (let x = 0; x < columns; x += 1) {
					if (column(x) <= low + 0.5 || column(x) >= high - 0.5) continue;
					if (onLine >= 0) {
						if (onLine + 1 < rows) bump(crossDown, nodeAt(x, onLine));
						if (onLine > 0) bump(crossUp, nodeAt(x, onLine));
					} else if (below >= 0 && below + 1 < rows) {
						bump(crossDown, nodeAt(x, below));
						bump(crossUp, nodeAt(x, below + 1));
					}
				}
			} else {
				const low = Math.min(first.y, second.y);
				const high = Math.max(first.y, second.y);
				const onLine = lineIndex(columnLines, first.x);
				const left = gapBelow(columnLines, first.x);
				for (let y = 0; y < rows; y += 1) {
					if (row(y) <= low + 0.5 || row(y) >= high - 0.5) continue;
					if (onLine >= 0) {
						if (onLine + 1 < columns) bump(crossRight, nodeAt(onLine, y));
						if (onLine > 0) bump(crossLeft, nodeAt(onLine, y));
					} else if (left >= 0 && left + 1 < columns) {
						bump(crossRight, nodeAt(left, y));
						bump(crossLeft, nodeAt(left + 1, y));
					}
				}
			}
		}
	}

	const indexOf = (point: Point): number => {
		const x = lineIndex(columnLines, point.x);
		const y = lineIndex(rowLines, point.y);
		return x < 0 || y < 0 ? -1 : nodeAt(x, y);
	};
	const pointOf = (node: number): Point => ({
		x: column(Math.floor(node / rows)),
		y: row(node % rows),
	});
	// States are node × arrival direction (0 right, 1 down, 2 left, 3 up); one extra state is the goal.
	const states = columns * rows * 4;
	const goal = states;
	const best = new Float64Array(states + 1).fill(Infinity);
	const previous = new Int32Array(states + 1).fill(-1);
	const endsByNode = new Map<number, { side: Side; port: Point }[]>();
	const endStubs: Point[] = [];
	for (const end of ends) {
		const node = indexOf(end.stub);
		if (node < 0 || blockedNode[node]) continue;
		endsByNode.set(node, [...(endsByNode.get(node) ?? []), end]);
		endStubs.push(pointOf(node));
	}
	if (endStubs.length === 0) return undefined;
	const remaining = (point: Point): number => {
		let shortest = Infinity;
		for (const stub of endStubs)
			shortest = Math.min(shortest, Math.abs(stub.x - point.x) + Math.abs(stub.y - point.y));
		return shortest;
	};
	const goalPort = new Map<number, Point>();
	const queue = new Queue();
	for (const start of starts) {
		const node = indexOf(start.stub);
		if (node < 0 || blockedNode[node]) continue;
		const facing =
			normals[start.side].x * (towards.x - from.x) + normals[start.side].y * (towards.y - from.y);
		const state = node * 4 + sideDirection[start.side];
		const cost = facing > 0 ? 0 : awaySideCost;
		if (cost < (best[state] ?? Infinity)) {
			best[state] = cost;
			previous[state] = -2 - starts.indexOf(start);
			queue.push(cost + remaining(pointOf(node)), cost, state);
		}
	}
	const moves: readonly (readonly [number, number])[] = [
		[1, 0],
		[0, 1],
		[-1, 0],
		[0, -1],
	];
	while (queue.size > 0) {
		const next = queue.pop();
		if (!next) break;
		const { cost, state } = next;
		if (cost > (best[state] ?? Infinity)) continue;
		if (state === goal) break;
		const node = Math.floor(state / 4);
		const direction = state % 4;
		for (const end of endsByNode.get(node) ?? []) {
			const arrive = (sideDirection[end.side] + 2) % 4;
			const facing =
				normals[end.side].x * (from.x - towards.x) + normals[end.side].y * (from.y - towards.y);
			const total = cost + (direction === arrive ? 0 : bendCost) + (facing > 0 ? 0 : awaySideCost);
			if (total < (best[goal] ?? Infinity)) {
				best[goal] = total;
				previous[goal] = state;
				goalPort.set(goal, end.port);
				queue.push(total, total, goal);
			}
		}
		const x = Math.floor(node / rows);
		const y = node % rows;
		moves.forEach(([moveX, moveY], turn): void => {
			if ((turn + 2) % 4 === direction) return;
			const nextX = x + moveX;
			const nextY = y + moveY;
			if (nextX < 0 || nextY < 0 || nextX >= columns || nextY >= rows) return;
			const neighbour = nodeAt(nextX, nextY);
			if (blockedNode[neighbour]) return;
			// Steps right and down are stamped on their start node; left and up on their end node.
			const blocked =
				turn === 0
					? blockedRight[node]
					: turn === 1
						? blockedDown[node]
						: turn === 2
							? blockedRight[neighbour]
							: blockedDown[neighbour];
			if (blocked) return;
			const crossings =
				turn === 0
					? crossRight[node]
					: turn === 1
						? crossDown[node]
						: turn === 2
							? crossLeft[node]
							: crossUp[node];
			const nextState = neighbour * 4 + turn;
			const there = pointOf(neighbour);
			const nextCost =
				cost +
				Math.abs(column(nextX) - column(x)) +
				Math.abs(row(nextY) - row(y)) +
				(turn === direction ? 0 : bendCost) +
				(crossings ?? 0) * crossingCost;
			if (nextCost < (best[nextState] ?? Infinity)) {
				best[nextState] = nextCost;
				previous[nextState] = state;
				queue.push(nextCost + remaining(there), nextCost, nextState);
			}
		});
	}
	const endPort = goalPort.get(goal);
	if (!endPort || previous[goal] === -1) return undefined;
	const path: Point[] = [endPort];
	let state = previous[goal] ?? -1;
	while (state >= 0) {
		path.push(pointOf(Math.floor(state / 4)));
		const before = previous[state] ?? -1;
		if (before <= -2) {
			const start = starts[-2 - before];
			if (start) path.push(start.port);
			break;
		}
		state = before;
	}
	return simplify(path.toReversed());
}

/** Spreads collinear, overlapping segments of different routes into parallel lanes. */
export function separateLanes(routes: readonly (Point[] | undefined)[]): void {
	type Segment = { route: number; index: number; low: number; high: number };
	for (const horizontal of [true, false]) {
		const lines = new Map<number, Segment[]>();
		routes.forEach((points, route): void => {
			if (!points) return;
			for (let index = 0; index < points.length - 1; index += 1) {
				const first = points[index];
				const second = points[index + 1];
				if (!first || !second) continue;
				const flat = horizontal
					? Math.abs(first.y - second.y) < 0.5
					: Math.abs(first.x - second.x) < 0.5;
				if (!flat) continue;
				const key = Math.round(horizontal ? first.y : first.x);
				const [low, high] = horizontal
					? [Math.min(first.x, second.x), Math.max(first.x, second.x)]
					: [Math.min(first.y, second.y), Math.max(first.y, second.y)];
				lines.set(key, [...(lines.get(key) ?? []), { route, index, low, high }]);
			}
		});
		for (const segments of lines.values()) {
			const sorted = segments.toSorted((first, second): number => first.low - second.low);
			let cluster: Segment[] = [];
			let reach = -Infinity;
			const flush = (): void => {
				const members = Array.from(new Set(cluster.map((segment): number => segment.route)));
				if (members.length > 1)
					for (const segment of cluster) {
						const offset = (members.indexOf(segment.route) - (members.length - 1) / 2) * laneGap;
						const points = routes[segment.route];
						for (const point of [points?.[segment.index], points?.[segment.index + 1]]) {
							if (!point) continue;
							if (horizontal) point.y += offset;
							else point.x += offset;
						}
					}
				cluster = [];
			};
			for (const segment of sorted) {
				if (segment.low >= reach - 0.5) flush();
				cluster.push(segment);
				reach = Math.max(reach, segment.high);
			}
			flush();
		}
	}
}

/** Draws a route with rounded corners. */
export function roundedPath(points: readonly Point[], radius = 10): string {
	const [first, ...rest] = points;
	if (!first) return '';
	let path = `M${first.x},${first.y}`;
	for (let index = 0; index < rest.length; index += 1) {
		const corner = rest[index];
		const before = index === 0 ? first : rest[index - 1];
		const after = rest[index + 1];
		if (!corner || !before) continue;
		if (!after) {
			path += ` L${corner.x},${corner.y}`;
			continue;
		}
		const inLength = Math.hypot(corner.x - before.x, corner.y - before.y);
		const outLength = Math.hypot(after.x - corner.x, after.y - corner.y);
		const bend = Math.min(radius, inLength / 2, outLength / 2);
		const entry = {
			x: corner.x - ((corner.x - before.x) / (inLength || 1)) * bend,
			y: corner.y - ((corner.y - before.y) / (inLength || 1)) * bend,
		};
		const exit = {
			x: corner.x + ((after.x - corner.x) / (outLength || 1)) * bend,
			y: corner.y + ((after.y - corner.y) / (outLength || 1)) * bend,
		};
		path += ` L${entry.x},${entry.y} Q${corner.x},${corner.y} ${exit.x},${exit.y}`;
	}
	return path;
}

/** Picks a label centre on the route whose box covers no obstacle and no placed label. */
export function placeLabel(
	points: readonly Point[],
	width: number,
	height: number,
	obstacles: readonly Box[],
	placed: Box[],
): Point | undefined {
	const segments = points
		.slice(1)
		.map((point, index) => ({ from: points[index] ?? point, to: point }))
		.toSorted(
			(first, second): number =>
				Math.hypot(second.to.x - second.from.x, second.to.y - second.from.y) -
				Math.hypot(first.to.x - first.from.x, first.to.y - first.from.y),
		);
	const boxAt = (point: Point): Box => ({
		left: point.x - width / 2 - 3,
		right: point.x + width / 2 + 3,
		top: point.y - height / 2 - 2,
		bottom: point.y + height / 2 + 2,
	});
	for (const { from, to } of segments)
		for (const fraction of [0.5, 0.35, 0.65, 0.2, 0.8]) {
			const point = {
				x: from.x + (to.x - from.x) * fraction,
				y: from.y + (to.y - from.y) * fraction,
			};
			const box = boxAt(point);
			if (
				!obstacles.some((obstacle): boolean => overlaps(obstacle, box)) &&
				!placed.some((other): boolean => overlaps(other, box))
			) {
				placed.push(box);
				return point;
			}
		}
	const fallback = segments[0];
	if (!fallback) return undefined;
	const point = {
		x: (fallback.from.x + fallback.to.x) / 2,
		y: (fallback.from.y + fallback.to.y) / 2,
	};
	placed.push(boxAt(point));
	return point;
}
