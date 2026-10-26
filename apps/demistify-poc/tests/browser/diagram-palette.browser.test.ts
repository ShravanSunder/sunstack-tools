import { afterEach, expect, test } from 'vitest';
import { page } from 'vitest/browser';

import { openFixture, requiredElement } from './fixture-page.ts';

afterEach(async (): Promise<void> => {
	document.body.replaceChildren();
	await page.viewport(1600, 1000);
});

test.each([
	['light', 1600],
	['dark', 1600],
	['light', 390],
	['dark', 390],
])(
	'entity field columns do not overlap under the %s theme at %ipx',
	async (theme, width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture } = await openFixture('palette');
		fixture.documentElement.dataset['theme'] = theme;
		const diagram = requiredElement(fixture, '#er-model');
		diagram.scrollIntoView();
		await expect.poll((): boolean => diagram.hasAttribute('prepared')).toBe(true);
		await fixture.fonts.ready;
		for (const entity of diagram.querySelectorAll('g.entity')) {
			const columns = [...entity.querySelectorAll('text.mono')];
			for (let index = 0; index < columns.length; index += 2) {
				const type = columns[index];
				const name = columns[index + 1];
				if (!type || !name) throw new Error('Missing entity attribute columns.');
				expect(type.getBoundingClientRect().right).toBeLessThan(
					name.getBoundingClientRect().left - 2,
				);
				expect(Number.parseFloat(fixture.defaultView?.getComputedStyle(type).fontSize ?? '0')).toBe(
					Number(type.getAttribute('font-size')),
				);
			}
		}
		expect(diagram.querySelector('title')?.textContent).toBe('Stable machine identity');
	},
);

test.each([
	'er-model',
	'flow-model',
	'sequence-model',
	'state-model',
	'class-model',
	'chart-model',
])(
	'%s uses distinguishable shared colors with readable text in both themes',
	async (identity): Promise<void> => {
		const { document: fixture } = await openFixture('palette');
		const owner = requiredElement(fixture, `#${identity}`);
		owner.scrollIntoView();
		await expect.poll((): boolean => owner.hasAttribute('prepared')).toBe(true);
		await fixture.fonts.ready;
		const view = fixture.defaultView;
		const context = fixture.createElement('canvas').getContext('2d');
		if (!view || !context) throw new Error('Missing browser color measurement.');
		const luminance = (color: string): number => {
			context.clearRect(0, 0, 1, 1);
			context.fillStyle = color;
			context.fillRect(0, 0, 1, 1);
			const channels = context.getImageData(0, 0, 1, 1).data;
			return [0.2126, 0.7152, 0.0722].reduce((total, weight, index): number => {
				const value = (channels[index] ?? 0) / 255;
				return (
					total + weight * (value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4)
				);
			}, 0);
		};
		for (const theme of ['light', 'dark']) {
			fixture.documentElement.dataset['theme'] = theme;
			const shapes = [
				...owner.querySelectorAll<SVGElement>(
					'g[data-diagram-slot] > rect:first-of-type, .xychart-bar',
				),
			];
			const fills = new Set(shapes.map((shape): string => view.getComputedStyle(shape).fill));
			expect(fills.size).toBeGreaterThanOrEqual(2);
			for (const shape of shapes) {
				const foreground = luminance(
					view.getComputedStyle(fixture.documentElement).getPropertyValue('--ink'),
				);
				const background = luminance(view.getComputedStyle(shape).fill);
				expect(
					(Math.max(foreground, background) + 0.05) / (Math.min(foreground, background) + 0.05),
				).toBeGreaterThanOrEqual(4.5);
			}
		}
	},
);
