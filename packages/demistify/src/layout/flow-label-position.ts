import { type Box, type Point, overlaps, placeLabel } from './canvas-router.ts';

export interface FlowLabelRequest {
	readonly points: readonly Point[];
	readonly width: number;
	readonly height: number;
	readonly bounds: Box;
	readonly obstacles: readonly Box[];
	readonly placed: readonly Box[];
}

/** Includes the caption's paint halo, so neighbouring labels retain a visible gap. */
export function flowLabelBounds(position: Point, width: number, height: number): Box {
	return {
		left: position.x - width / 2 - 3,
		right: position.x + width / 2 + 3,
		top: position.y - height / 2 - 2,
		bottom: position.y + height / 2 + 2,
	};
}

function distanceToRoute(position: Point, points: readonly Point[]): number {
	let closest = Infinity;
	for (let index = 1; index < points.length; index += 1) {
		const before = points[index - 1];
		const after = points[index];
		if (!before || !after) continue;
		const horizontal = after.x - before.x;
		const vertical = after.y - before.y;
		const squaredLength = horizontal * horizontal + vertical * vertical;
		const fraction =
			squaredLength === 0
				? 0
				: Math.max(
						0,
						Math.min(
							1,
							((position.x - before.x) * horizontal + (position.y - before.y) * vertical) /
								squaredLength,
						),
					);
		closest = Math.min(
			closest,
			Math.hypot(
				position.x - before.x - horizontal * fraction,
				position.y - before.y - vertical * fraction,
			),
		);
	}
	return closest;
}

/** Fits the measured caption on its route or in a nearby clear gutter within the Flow. */
export function positionFlowLabel(request: FlowLabelRequest): Point | undefined {
	const { points, width, height, bounds, obstacles, placed } = request;
	if (points.length < 2 || width <= 0 || height <= 0) return undefined;
	const clear = (position: Point): boolean => {
		const caption = flowLabelBounds(position, width, height);
		return (
			caption.left >= bounds.left &&
			caption.right <= bounds.right &&
			caption.top >= bounds.top &&
			caption.bottom <= bounds.bottom &&
			![...obstacles, ...placed].some((obstacle): boolean => overlaps(caption, obstacle))
		);
	};
	const onRoute = placeLabel(points, width, height, obstacles, [...placed]);
	if (onRoute && clear(onRoute)) return onRoute;

	// A short final bend cannot hold a long caption. The existing top/bottom padding and
	// inter-card gutters are alternatives; never accept the shared router's overlapping fallback.
	const centers = points.slice(1).map((after, index): Point => {
		const before = points[index] ?? after;
		return { x: (before.x + after.x) / 2, y: (before.y + after.y) / 2 };
	});
	const horizontal = [
		...new Set([
			...centers.map((point): number => point.x),
			...points.map((point): number => point.x),
			...obstacles.flatMap((box): number[] => [
				box.left - width / 2 - 6,
				box.right + width / 2 + 6,
			]),
		]),
	];
	const vertical = [
		...new Set([
			...centers.map((point): number => point.y),
			...points.map((point): number => point.y),
			...obstacles.flatMap((box): number[] => [
				box.top - height / 2 - 6,
				box.bottom + height / 2 + 6,
			]),
		]),
	];
	return horizontal
		.flatMap((x): Point[] => vertical.map((y): Point => ({ x, y })))
		.filter(clear)
		.toSorted(
			(first, second): number => distanceToRoute(first, points) - distanceToRoute(second, points),
		)[0];
}
