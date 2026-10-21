import { describe, expect, it } from 'vitest';

import type { Box, Point } from './canvas-router.ts';
import {
	alignFacingFlowPorts,
	layoutFlowCaption,
	positionFlowCaption,
} from './flow-connector-layout.ts';

const measureText = (text: string): number => text.length * 8;
const bounds = { left: 0, top: 0, right: 1600, bottom: 260 };
const cards = [
	{ left: 24, top: 40, right: 440, bottom: 96 },
	{ left: 600, top: 40, right: 1000, bottom: 190 },
	{ left: 1160, top: 40, right: 1576, bottom: 96 },
];

function captionBox(position: Point, width: number, height: number): Box {
	return {
		left: position.x - width / 2,
		right: position.x + width / 2,
		top: position.y - height / 2,
		bottom: position.y + height / 2,
	};
}

function expectSeparated(caption: Box, obstacles: readonly Box[]): void {
	for (const box of obstacles)
		expect(
			caption.right <= box.left ||
				caption.left >= box.right ||
				caption.bottom <= box.top ||
				caption.top >= box.bottom,
			'A caption must not cover a card or another caption',
		).toBe(true);
}

describe('Flow connector layout', (): void => {
	it('wraps a caption at word boundaries and reserves minimum connector space', (): void => {
		const result = layoutFlowCaption({
			text: 'wake_send / schedule_create',
			measureText,
			lineHeight: 18,
		});
		expect(result.lines).toEqual(['wake_send /', 'schedule_create']);
		expect(result.columnGap).toBeGreaterThanOrEqual(160);
		expect(result.columnGap - result.width).toBeGreaterThanOrEqual(32);
		expect(result.rowGap - result.height).toBeGreaterThanOrEqual(32);
	});
	it('wraps a long identifier without dropping characters or overflowing its line', (): void => {
		const text = 'an_identifier_without_a_convenient_word_boundary';
		const result = layoutFlowCaption({ text, measureText, lineHeight: 18 });
		expect(result.lines.join('')).toBe(text);
		for (const line of result.lines) expect(measureText(line)).toBeLessThanOrEqual(120);
	});
	it('connects unequal-height neighbouring cards on one shared horizontal line', (): void => {
		const source = cards[0];
		const target = cards[1];
		if (!source || !target) throw new Error('Missing literal geometry');
		const result = alignFacingFlowPorts({
			source,
			target,
			start: { x: 440, y: 68 },
			end: { x: 600, y: 115 },
			orientation: 'horizontal',
		});
		expect(result).toBeDefined();
		if (!result) throw new Error('A shared facing span exists');
		expect(result[0].y).toBe(result[1].y);
		expect(result[0].y).toBeGreaterThan(source.top);
		expect(result[0].y).toBeLessThan(source.bottom);
		expect(result[0].x).toBe(440);
		expect(result[1].x).toBe(600);
	});
	it('connects stacked unequal-width cards on one shared vertical line', (): void => {
		const result = alignFacingFlowPorts({
			source: { left: 20, top: 20, right: 300, bottom: 90 },
			target: { left: 100, top: 200, right: 500, bottom: 280 },
			start: { x: 160, y: 90 },
			end: { x: 300, y: 200 },
			orientation: 'vertical',
		});
		expect(result).toBeDefined();
		if (!result) throw new Error('A shared facing span exists');
		expect(result[0].x).toBe(result[1].x);
		expect(result[0].x).toBeGreaterThan(100);
		expect(result[0].x).toBeLessThan(300);
	});
	it('keeps multiline captions on their connector and out of both cards', (): void => {
		const caption = layoutFlowCaption({
			text: 'wake_send / schedule_create',
			measureText,
			lineHeight: 18,
		});
		const position = positionFlowCaption({
			points: [
				{ x: 440, y: 68 },
				{ x: 600, y: 68 },
			],
			...caption,
			bounds,
			obstacles: cards,
			placed: [],
		});
		expect(position).toBeDefined();
		if (!position) throw new Error('The reserved gutter must fit the caption');
		expect(position.y).toBe(68);
		const box = captionBox(position, caption.width, caption.height);
		expectSeparated(box, cards);
		expect(box.top).toBeGreaterThanOrEqual(bounds.top);
		expect(box.bottom).toBeLessThanOrEqual(bounds.bottom);
	});
	it('keeps a second wrapped caption clear of the first one', (): void => {
		const caption = layoutFlowCaption({
			text: 'timer on the TARGET machine (PD4:297–303)',
			measureText,
			lineHeight: 18,
		});
		const previous = { left: 450, top: 40, right: 590, bottom: 100 };
		const position = positionFlowCaption({
			points: [
				{ x: 1000, y: 68 },
				{ x: 1160, y: 68 },
			],
			...caption,
			bounds,
			obstacles: cards,
			placed: [previous],
		});
		expect(position).toBeDefined();
		if (!position) throw new Error('The reserved gutter must fit the caption');
		expect(position.y).toBe(68);
		expectSeparated(captionBox(position, caption.width, caption.height), [...cards, previous]);
	});
	it('rejects the old crowded gutter instead of floating the label above the diagram', (): void => {
		const position = positionFlowCaption({
			points: [
				{ x: 450, y: 68 },
				{ x: 486, y: 68 },
				{ x: 486, y: 115 },
				{ x: 522, y: 115 },
			],
			width: 244,
			height: 18,
			bounds,
			obstacles: [
				{ left: 24, top: 40, right: 450, bottom: 96 },
				{ left: 522, top: 40, right: 1030, bottom: 190 },
			],
			placed: [],
		});
		expect(position).toBeUndefined();
	});
	it('reports no placement when the entire surface is occupied', (): void => {
		const occupied = { left: 0, top: 0, right: 120, bottom: 80 };
		expect(
			positionFlowCaption({
				points: [
					{ x: 10, y: 40 },
					{ x: 110, y: 40 },
				],
				width: 90,
				height: 18,
				bounds: occupied,
				obstacles: [occupied],
				placed: [],
			}),
		).toBeUndefined();
	});
});
