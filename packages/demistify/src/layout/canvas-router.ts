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
const inside = (box: Box, point: Point): boolean =>
	point.x > box.left + 0.5 &&
	point.x < box.right - 0.5 &&
	point.y > box.top + 0.5 &&
	point.y < box.bottom - 0.5;
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

/** Minimal binary heap keyed by cost. */
class Queue {
	private readonly items: { cost: number; state: number }[] = [];
	get size(): number {
		return this.items.length;
	}
	push(cost: number, state: number): void {
		const items = this.items;
		items.push({ cost, state });
		let index = items.length - 1;
		while (index > 0) {
			const parent = (index - 1) >> 1;
			const above = items[parent];
			const current = items[index];
			if (!above || !current || above.cost <= current.cost) break;
			items[parent] = current;
			items[index] = above;
			index = parent;
		}
	}
	pop(): { cost: number; state: number } | undefined {
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
				if ((items[left]?.cost ?? Infinity) < (items[smallest]?.cost ?? Infinity)) smallest = left;
				if ((items[right]?.cost ?? Infinity) < (items[smallest]?.cost ?? Infinity))
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

const directionIndex = (from: Point, to: Point): number =>
	to.x > from.x ? 0 : to.y > from.y ? 1 : to.x < from.x ? 2 : 3;
const sideDirection: Record<Side, number> = { right: 0, bottom: 1, left: 2, top: 3 };

/**
 * True when the move from `first` to `second` crosses an earlier segment inside that segment. The
 * move counts its starting point but not its end, so a crossing at a grid point is counted once.
 */
function segmentsCross(first: Point, second: Point, third: Point, fourth: Point): boolean {
	const moveFlat = Math.abs(first.y - second.y) < 0.5;
	const otherFlat = Math.abs(third.y - fourth.y) < 0.5;
	if (moveFlat === otherFlat) return false;
	const crossX = moveFlat ? third.x : first.x;
	const crossY = moveFlat ? first.y : third.y;
	const insideOther = otherFlat
		? crossX > Math.min(third.x, fourth.x) + 0.5 && crossX < Math.max(third.x, fourth.x) - 0.5
		: crossY > Math.min(third.y, fourth.y) + 0.5 && crossY < Math.max(third.y, fourth.y) - 0.5;
	const along = moveFlat
		? (crossX - first.x) / (second.x - first.x || 1)
		: (crossY - first.y) / (second.y - first.y || 1);
	return insideOther && along > -0.001 && along < 0.999;
}

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

/**
 * Finds the cheapest orthogonal route from a side of the source to a side of the target that keeps
 * `clearance` away from every obstacle. Returns undefined when the boxes leave no gap to pass.
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
	const free = new Uint8Array(columns * rows);
	for (let column = 0; column < columns; column += 1)
		for (let row = 0; row < rows; row += 1) {
			const point = { x: columnLines[column] ?? 0, y: rowLines[row] ?? 0 };
			free[nodeAt(column, row)] = blockers.some((box): boolean => inside(box, point)) ? 0 : 1;
		}
	const indexOf = (point: Point): number => {
		const column = columnLines.findIndex((value): boolean => Math.abs(value - point.x) < 0.75);
		const row = rowLines.findIndex((value): boolean => Math.abs(value - point.y) < 0.75);
		return column < 0 || row < 0 ? -1 : nodeAt(column, row);
	};
	const pointOf = (node: number): Point => ({
		x: columnLines[Math.floor(node / rows)] ?? 0,
		y: rowLines[node % rows] ?? 0,
	});
	// States are node × arrival direction (0 right, 1 down, 2 left, 3 up); one extra state is the goal.
	const states = columns * rows * 4;
	const goal = states;
	const best = new Float64Array(states + 1).fill(Infinity);
	const previous = new Int32Array(states + 1).fill(-1);
	const endsByNode = new Map<number, { side: Side; port: Point }[]>();
	for (const end of ends) {
		const node = indexOf(end.stub);
		if (node < 0 || !free[node]) continue;
		endsByNode.set(node, [...(endsByNode.get(node) ?? []), end]);
	}
	const goalPort = new Map<number, Point>();
	const queue = new Queue();
	for (const start of starts) {
		const node = indexOf(start.stub);
		if (node < 0 || !free[node]) continue;
		const facing =
			normals[start.side].x * (towards.x - from.x) + normals[start.side].y * (towards.y - from.y);
		const state = node * 4 + sideDirection[start.side];
		const cost = facing > 0 ? 0 : awaySideCost;
		if (cost < (best[state] ?? Infinity)) {
			best[state] = cost;
			previous[state] = -2 - starts.indexOf(start);
			queue.push(cost, state);
		}
	}
	const crossings = (first: Point, second: Point): number => {
		let count = 0;
		for (const route of earlierRoutes) {
			if (!route) continue;
			for (let index = 0; index < route.length - 1; index += 1) {
				const start = route[index];
				const end = route[index + 1];
				if (start && end && segmentsCross(first, second, start, end)) count += 1;
			}
		}
		return count;
	};
	const midpointFree = (first: Point, second: Point): boolean => {
		const middle = { x: (first.x + second.x) / 2, y: (first.y + second.y) / 2 };
		return !blockers.some((box): boolean => inside(box, middle));
	};
	while (queue.size > 0) {
		const next = queue.pop();
		if (!next) break;
		const { cost, state } = next;
		if (cost > (best[state] ?? Infinity)) continue;
		if (state === goal) break;
		const node = Math.floor(state / 4);
		const direction = state % 4;
		const here = pointOf(node);
		for (const end of endsByNode.get(node) ?? []) {
			const arrive = (sideDirection[end.side] + 2) % 4;
			const facing =
				normals[end.side].x * (from.x - towards.x) + normals[end.side].y * (from.y - towards.y);
			const total = cost + (direction === arrive ? 0 : bendCost) + (facing > 0 ? 0 : awaySideCost);
			if (total < (best[goal] ?? Infinity)) {
				best[goal] = total;
				previous[goal] = state;
				goalPort.set(goal, end.port);
				queue.push(total, goal);
			}
		}
		const column = Math.floor(node / rows);
		const row = node % rows;
		const neighbours: [number, number][] = [
			[column + 1, row],
			[column, row + 1],
			[column - 1, row],
			[column, row - 1],
		];
		for (const [nextColumn, nextRow] of neighbours) {
			if (nextColumn < 0 || nextRow < 0 || nextColumn >= columns || nextRow >= rows) continue;
			const neighbour = nodeAt(nextColumn, nextRow);
			if (!free[neighbour]) continue;
			const there = pointOf(neighbour);
			if (!midpointFree(here, there)) continue;
			const turn = directionIndex(here, there);
			if ((turn + 2) % 4 === direction) continue;
			const nextState = neighbour * 4 + turn;
			const nextCost =
				cost +
				Math.abs(there.x - here.x) +
				Math.abs(there.y - here.y) +
				(turn === direction ? 0 : bendCost) +
				crossings(here, there) * crossingCost;
			if (nextCost < (best[nextState] ?? Infinity)) {
				best[nextState] = nextCost;
				previous[nextState] = state;
				queue.push(nextCost, nextState);
			}
		}
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
