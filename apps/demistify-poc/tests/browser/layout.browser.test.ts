import { afterEach, expect, test } from 'vitest';
import { page } from 'vitest/browser';

import {
	firstTextRect,
	fixtureLocator,
	insideViewport,
	openFixture,
	requiredElement,
} from './fixture-page.ts';

afterEach(async (): Promise<void> => {
	document.body.replaceChildren();
	await page.viewport(1600, 1000);
});

test('sequence messages remain readable in both themes and on a narrow screen', async (): Promise<void> => {
	const { document: fixture } = await openFixture('ui', '', 'sequence-readability.html');
	const diagram = requiredElement(fixture, '#sequence');
	await expect.poll((): boolean => diagram.hasAttribute('prepared')).toBe(true);
	await fixture.fonts.ready;
	const message = Array.from(diagram.querySelectorAll<SVGTextElement>('text')).find(
		(text): boolean => text.textContent === 'return the same identity',
	);
	const view = fixture.defaultView;
	if (!message || !view) throw new Error('Missing rendered sequence message.');
	const canvas = fixture.createElement('canvas');
	const context = canvas.getContext('2d');
	if (!context) throw new Error('Missing colour measurement context.');
	const luminance = (colour: string): number => {
		context.clearRect(0, 0, 1, 1);
		context.fillStyle = colour;
		context.fillRect(0, 0, 1, 1);
		const channels = context.getImageData(0, 0, 1, 1).data;
		return [0.2126, 0.7152, 0.0722].reduce((total, weight, index): number => {
			const channel = (channels[index] ?? 0) / 255;
			return (
				total + weight * (channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4)
			);
		}, 0);
	};
	for (const theme of ['dark', 'light']) {
		fixture.documentElement.dataset['theme'] = theme;
		const foreground = luminance(view.getComputedStyle(message).fill);
		const background = luminance(
			view.getComputedStyle(fixture.documentElement).getPropertyValue('--surface-page'),
		);
		expect
			.soft((Math.max(foreground, background) + 0.05) / (Math.min(foreground, background) + 0.05))
			.toBeGreaterThanOrEqual(4.5);
	}
	const verifyViewport = async (width: number): Promise<void> => {
		await page.viewport(width, 1000);
		const renderedSize =
			Number.parseFloat(view.getComputedStyle(message).fontSize) * (message.getScreenCTM()?.a ?? 0);
		expect.soft(renderedSize).toBeGreaterThanOrEqual(15.9);
		expect(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(width + 1);
	};
	await verifyViewport(1600);
	await verifyViewport(390);
});

async function verifyDiagramWidth(width: number): Promise<void> {
	await page.viewport(width, 1000);
	const { document: fixture, frame } = await openFixture('layout');
	const diagram = requiredElement(fixture, '#wide-diagram');
	diagram.scrollIntoView();
	await expect
		.poll((): string => diagram.querySelector('svg')?.textContent ?? '')
		.toContain('Report the observed outcome');
	const first = frame.getByText('Receive a complete request', { exact: false }).first();
	const last = frame.getByText('Report the observed outcome', { exact: false }).first();
	const firstNode = requiredElement(diagram, 'g.node[data-id="input"]');
	const lastNode = requiredElement(diagram, 'g.node[data-id="finish"]');
	expect(diagram.scrollWidth).toBeGreaterThan(diagram.clientWidth + 100);
	await expect.element(first).toBeVisible();
	diagram.scrollLeft = 0;
	expect(firstNode.getBoundingClientRect().left).toBeGreaterThanOrEqual(
		diagram.getBoundingClientRect().left - 2,
	);
	await fixtureLocator(frame, diagram).hover();
	const controls = requiredElement(diagram, '.ex-inspect-controls');
	expect(insideViewport(controls)).toBe(true);
	diagram.scrollLeft = diagram.scrollWidth;
	await expect
		.poll(
			(): boolean =>
				lastNode.getBoundingClientRect().right <= diagram.getBoundingClientRect().right + 2,
		)
		.toBe(true);
	await expect.element(last).toBeVisible();
	expect(insideViewport(controls)).toBe(true);
	expect(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(width + 2);
	await frame.getByRole('button', { name: 'Expand Wide comparison', exact: true }).click();
	await expect
		.element(frame.getByRole('dialog', { name: 'Wide comparison', exact: true }))
		.toBeVisible();
	await frame.getByRole('dialog').getByRole('button', { name: 'Close', exact: true }).click();
	document.body.replaceChildren();
}

test('wide diagrams scroll locally to both endpoints with reachable controls and no outer overflow', async (): Promise<void> => {
	await verifyDiagramWidth(1600);
	await verifyDiagramWidth(390);
});

test('tree multiline text stays beside its first-line icon and preserves inline code and loose paragraphs', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('layout');
	const tree = requiredElement(fixture, '#tight-labels');
	tree.scrollIntoView();
	await expect
		.poll((): string => tree.querySelector('.ex-tree-node')?.textContent ?? '')
		.toContain('Short and multiline labels');
	for (const text of ['Short label', 'A deliberately long label', 'Keep']) {
		const label = Array.from(tree.querySelectorAll('.ex-tree-label')).find((element): boolean =>
			(element.textContent ?? '').trim().startsWith(text),
		);
		if (!label) throw new Error(`Missing tree scenario: ${text}`);
		const row = label.parentElement;
		if (!row) throw new Error('Missing tree row.');
		const icon = requiredElement(row, '.ex-tone-icon');
		const textRect = firstTextRect(label);
		const iconRect = icon.getBoundingClientRect();
		expect(Math.abs(textRect.top - iconRect.top)).toBeLessThan(10);
		expect(textRect.left).toBeGreaterThanOrEqual(iconRect.right);
		if (text === 'A deliberately long label')
			expect(label.getBoundingClientRect().height).toBeGreaterThan(40);
	}
	const code = requiredElement(tree, 'code');
	const codeRect = code.getBoundingClientRect();
	const label = requiredElement(tree, 'code').closest('.ex-tree-label');
	if (!label) throw new Error('Inline code lost its label.');
	expect(Math.abs(codeRect.top - firstTextRect(label).top)).toBeLessThan(10);
	const loose = requiredElement(fixture, '#loose-labels');
	await expect
		.poll((): number => loose.querySelectorAll('.ex-tree-label p').length)
		.toBeGreaterThan(1);
	const paragraphs = loose.querySelectorAll('.ex-tree-label p');
	const firstParagraph = paragraphs[0];
	const secondParagraph = paragraphs[1];
	if (!firstParagraph || !secondParagraph) throw new Error('Loose paragraphs were flattened.');
	expect(secondParagraph.textContent).toContain('A second paragraph retains');
	expect(secondParagraph.getBoundingClientRect().top).toBeGreaterThan(
		firstParagraph.getBoundingClientRect().bottom + 3,
	);
	expect(
		Math.abs(
			secondParagraph.getBoundingClientRect().left - firstParagraph.getBoundingClientRect().left,
		),
	).toBeLessThan(2);
	await frame.getByText('A deliberately long label', { exact: false }).click();
	await expect
		.element(frame.getByText('A nested child has the same alignment', { exact: false }))
		.toBeVisible();
	const branchLabel = Array.from(tree.querySelectorAll('.ex-tree-label')).find(
		(element): boolean =>
			element.textContent?.trim().startsWith('A deliberately long label') ?? false,
	);
	const branch = branchLabel?.closest('.ex-tree-node');
	if (!branch) throw new Error('Missing tree branch.');
	const disclosure = fixtureLocator(frame, requiredElement(branch, '.ex-tree-disclosure'));
	await disclosure.click();
	await expect
		.element(frame.getByText('A nested child has the same alignment', { exact: false }))
		.not.toBeVisible();
	await disclosure.click();
	await expect
		.element(frame.getByText('A nested child has the same alignment', { exact: false }))
		.toBeVisible();
});
