import { type Box, type Point, overlaps } from './canvas-router.ts';

export interface FlowCaptionLayout {
	readonly lines: readonly string[];
	readonly width: number;
	readonly height: number;
	readonly lineHeight: number;
	readonly columnGap: number;
	readonly rowGap: number;
}

interface CaptionRequest {
	readonly text: string;
	readonly measureText: (text: string) => number;
	readonly lineHeight: number;
}

/** Wraps complete caption text and reserves visible connector legs around its painted block. */
export function layoutFlowCaption(request: CaptionRequest): FlowCaptionLayout {
	const maximumLineWidth = 120;
	const lines: string[] = [];
	let line = '';
	for (const word of request.text.trim().split(/\s+/).filter(Boolean)) {
		const combined = line ? `${line} ${word}` : word;
		if (request.measureText(combined) <= maximumLineWidth) {
			line = combined;
			continue;
		}
		if (line) lines.push(line);
		line = '';
		// An identifier may contain no spaces. Wrap its characters without losing any of them.
		for (const character of word) {
			if (line && request.measureText(`${line}${character}`) > maximumLineWidth) {
				lines.push(line);
				line = '';
			}
			line += character;
		}
	}
	if (line) lines.push(line);
	const width = Math.max(0, ...lines.map(request.measureText));
	const height = lines.length * request.lineHeight;
	return {
		lines,
		width,
		height,
		lineHeight: request.lineHeight,
		columnGap: Math.max(160, width + 40),
		rowGap: Math.max(64, height + 40),
	};
}

/** Paint padding is included when keeping the caption off cards and other captions. */
export function flowCaptionBounds(position: Point, width: number, height: number): Box {
	return {
		left: position.x - width / 2 - 4,
		right: position.x + width / 2 + 4,
		top: position.y - height / 2 - 4,
		bottom: position.y + height / 2 + 4,
	};
}

interface PlacementRequest {
	readonly points: readonly Point[];
	readonly width: number;
	readonly height: number;
	readonly bounds: Box;
	readonly obstacles: readonly Box[];
	readonly placed: readonly Box[];
}

/** Keeps the caption on the actual connector; inadequate space is not repaired by detaching it. */
export function positionFlowCaption(request: PlacementRequest): Point | undefined {
	const { points, width, height, bounds, obstacles, placed } = request;
	const segments = points
		.slice(1)
		.map((after, index) => ({ before: points[index] ?? after, after }))
		.toSorted(
			(first, second): number =>
				Math.hypot(second.after.x - second.before.x, second.after.y - second.before.y) -
				Math.hypot(first.after.x - first.before.x, first.after.y - first.before.y),
		);
	for (const segment of segments) {
		for (const fraction of [0.5, 0.35, 0.65, 0.2, 0.8]) {
			const position = {
				x: segment.before.x + (segment.after.x - segment.before.x) * fraction,
				y: segment.before.y + (segment.after.y - segment.before.y) * fraction,
			};
			const caption = flowCaptionBounds(position, width, height);
			if (
				caption.left >= bounds.left &&
				caption.right <= bounds.right &&
				caption.top >= bounds.top &&
				caption.bottom <= bounds.bottom &&
				![...obstacles, ...placed].some((obstacle): boolean => overlaps(caption, obstacle))
			)
				return position;
		}
	}
	return undefined;
}

interface FacingPortsRequest {
	readonly source: Box;
	readonly target: Box;
	readonly start: Point;
	readonly end: Point;
	readonly orientation: 'horizontal' | 'vertical';
}

/** Facing cards share one coordinate when their sides overlap, avoiding needless elbow bends. */
export function alignFacingFlowPorts(
	request: FacingPortsRequest,
): readonly [Point, Point] | undefined {
	const horizontal = request.orientation === 'horizontal';
	const sourceLow = horizontal ? request.source.top : request.source.left;
	const sourceHigh = horizontal ? request.source.bottom : request.source.right;
	const targetLow = horizontal ? request.target.top : request.target.left;
	const targetHigh = horizontal ? request.target.bottom : request.target.right;
	const low = Math.max(sourceLow, targetLow);
	const high = Math.min(sourceHigh, targetHigh);
	if (high - low < 24) return undefined;
	const sourcePosition = horizontal ? request.start.y : request.start.x;
	const targetPosition = horizontal ? request.end.y : request.end.x;
	const fraction =
		((sourcePosition - sourceLow) / (sourceHigh - sourceLow) +
			(targetPosition - targetLow) / (targetHigh - targetLow)) /
		2;
	const shared = Math.max(low + 8, Math.min(high - 8, low + (high - low) * fraction));
	return horizontal
		? [
				{ x: request.start.x, y: shared },
				{ x: request.end.x, y: shared },
			]
		: [
				{ x: shared, y: request.start.y },
				{ x: shared, y: request.end.y },
			];
}
