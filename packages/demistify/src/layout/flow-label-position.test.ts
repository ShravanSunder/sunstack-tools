import { describe, expect, it } from 'vitest';

import type { Box, Point } from './canvas-router.ts';
import { type FlowLabelRequest, positionFlowLabel } from './flow-label-position.ts';

function textBounds(point: Point, width: number, height: number): Box {
	return {
		left: point.x - width / 2,
		right: point.x + width / 2,
		top: point.y - height / 2,
		bottom: point.y + height / 2,
	};
}

function expectClear(request: FlowLabelRequest): Box {
	const position = positionFlowLabel(request);
	expect(position, 'The diagram has room for the complete caption').toBeDefined();
	if (!position) throw new Error('Missing caption placement');
	const text = textBounds(position, request.width, request.height);
	expect(text.left).toBeGreaterThanOrEqual(request.bounds.left);
	expect(text.right).toBeLessThanOrEqual(request.bounds.right);
	expect(text.top).toBeGreaterThanOrEqual(request.bounds.top);
	expect(text.bottom).toBeLessThanOrEqual(request.bounds.bottom);
	for (const obstacle of [...request.obstacles, ...request.placed]) {
		const separated =
			text.right <= obstacle.left ||
			text.left >= obstacle.right ||
			text.bottom <= obstacle.top ||
			text.top >= obstacle.bottom;
		expect(separated, 'The caption must not cover a card or another caption').toBe(true);
	}
	return text;
}

describe('flow caption containment', (): void => {
	const bounds = { left: 0, top: 0, right: 1600, bottom: 250 };
	const cards = [
		{ left: 24, top: 40, right: 450, bottom: 96 },
		{ left: 522, top: 40, right: 1030, bottom: 190 },
		{ left: 1102, top: 40, right: 1576, bottom: 96 },
	];

	it('keeps a long caption clear of unequal-height cards on a bent route', (): void => {
		// Arrange: a short final segment beside the taller destination, as in the owner screenshot.
		const request = {
			points: [
				{ x: 450, y: 68 },
				{ x: 486, y: 68 },
				{ x: 486, y: 115 },
				{ x: 522, y: 115 },
			],
			width: 244,
			height: 18,
			bounds,
			obstacles: cards,
			placed: [],
		};
		// Act / Assert: allow any clear position, rather than pinning the routing formula.
		expectClear(request);
	});

	it('separates a second long caption from cards and the first caption', (): void => {
		const first = expectClear({
			points: [
				{ x: 450, y: 68 },
				{ x: 486, y: 68 },
				{ x: 486, y: 115 },
				{ x: 522, y: 115 },
			],
			width: 244,
			height: 18,
			bounds,
			obstacles: cards,
			placed: [],
		});
		expectClear({
			points: [
				{ x: 1030, y: 115 },
				{ x: 1066, y: 115 },
				{ x: 1066, y: 68 },
				{ x: 1102, y: 68 },
			],
			width: 312,
			height: 18,
			bounds,
			obstacles: cards,
			placed: [first],
		});
	});

	it('uses an ordinary clear straight connection without requiring one exact position', (): void => {
		expectClear({
			points: [
				{ x: 180, y: 70 },
				{ x: 460, y: 70 },
			],
			width: 110,
			height: 18,
			bounds: { left: 0, top: 0, right: 650, bottom: 150 },
			obstacles: [
				{ left: 20, top: 40, right: 180, bottom: 100 },
				{ left: 460, top: 40, right: 630, bottom: 100 },
			],
			placed: [],
		});
	});

	it('reports no placement when the entire surface is occupied', (): void => {
		const occupied = { left: 0, top: 0, right: 120, bottom: 80 };
		const position = positionFlowLabel({
			points: [
				{ x: 10, y: 40 },
				{ x: 110, y: 40 },
			],
			width: 90,
			height: 18,
			bounds: occupied,
			obstacles: [occupied],
			placed: [],
		});
		expect(position).toBeUndefined();
	});
});
