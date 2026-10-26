import { afterEach, expect, test } from 'vitest';
import { commands, page, userEvent } from 'vitest/browser';

import { fixtureLocator, openFixture, requiredElement } from './fixture-page.ts';

afterEach(async (): Promise<void> => {
	document.body.replaceChildren();
	await page.viewport(1600, 1000);
});

interface NativeInspection {
	readonly fixture: Document;
	readonly frame: ReturnType<typeof page.frameLocator>;
	readonly original: Element;
	readonly visualSelector: 'img' | 'ex-mermaid > svg';
}

async function verifyNativeGestures(target: NativeInspection): Promise<void> {
	const { fixture, frame, original, visualSelector } = target;
	const surface = visualSelector === 'img' ? '.ex-inspect-dialog' : '.ex-drawer';
	const inspector = frame.getByRole(visualSelector === 'img' ? 'dialog' : 'complementary', {
		includeHidden: true,
		name: visualSelector === 'img' ? 'Native inspection picture' : 'Wide comparison',
		exact: true,
	});
	await expect.element(inspector).toBeVisible();
	const viewport = requiredElement(fixture, `${surface} .ex-inspect-body`);
	const visual = requiredElement(viewport, visualSelector);
	const viewportLocator = fixtureLocator(frame, viewport);
	const magnification = (): string =>
		requiredElement(fixture, `${surface} .ex-inspect-header > span`).textContent ?? '';
	const initialBounds = visual.getBoundingClientRect();
	expect(magnification()).toBe('100%');
	expect(initialBounds.width).toBeGreaterThan(50);
	await userEvent.keyboard('{Meta>}');
	try {
		await viewportLocator.wheel({ delta: { y: -350 } });
	} finally {
		await userEvent.keyboard('{/Meta}');
	}
	await expect.poll((): number => Number.parseInt(magnification(), 10)).toBeGreaterThan(120);
	await expect
		.poll((): number => visual.getBoundingClientRect().width)
		.toBeGreaterThan(initialBounds.width + 20);
	expect(visual.getBoundingClientRect().height).toBeGreaterThan(initialBounds.height + 5);
	const zoomedWidth = visual.getBoundingClientRect().width;
	const zoomedLabel = magnification();
	const scrollSurfaces: Element[] = [];
	for (let ancestor = visual.parentElement; ancestor; ancestor = ancestor.parentElement) {
		scrollSurfaces.push(ancestor);
		if (ancestor === viewport) break;
	}
	const panSurface = scrollSurfaces.find((element): boolean => {
		const style = fixture.defaultView?.getComputedStyle(element);
		return (
			(/auto|scroll/.test(style?.overflowX ?? '') &&
				element.scrollWidth - element.clientWidth > 40) ||
			(/auto|scroll/.test(style?.overflowY ?? '') &&
				element.scrollHeight - element.clientHeight > 40)
		);
	});
	if (!panSurface) throw new Error('Zoomed native visual has no available overflow axis.');
	const horizontalOverflow = panSurface.scrollWidth - panSurface.clientWidth;
	const verticalOverflow = panSurface.scrollHeight - panSurface.clientHeight;
	expect(Math.max(horizontalOverflow, verticalOverflow)).toBeGreaterThan(40);
	const horizontalPan = horizontalOverflow > 40;
	panSurface.scrollLeft = 0;
	panSurface.scrollTop = 0;
	await fixtureLocator(frame, visual).wheel({ delta: horizontalPan ? { x: 100 } : { y: 100 } });
	await expect
		.poll((): number => (horizontalPan ? panSurface.scrollLeft : panSurface.scrollTop))
		.toBeGreaterThan(20);
	expect(magnification()).toBe(zoomedLabel);
	expect(Math.abs(visual.getBoundingClientRect().width - zoomedWidth)).toBeLessThan(2);
	panSurface.scrollLeft = 0;
	panSurface.scrollTop = 0;
	await commands.panInspection();
	await expect
		.poll((): number => (horizontalPan ? panSurface.scrollLeft : panSurface.scrollTop), {
			message: JSON.stringify({
				visual: visualSelector,
				initialWidth: initialBounds.width,
				zoomedWidth,
				zoomedLabel,
				scrollSurface: panSurface.localName,
				horizontalOverflow,
				verticalOverflow,
				viewportHorizontalOverflow: viewport.scrollWidth - viewport.clientWidth,
				viewportVerticalOverflow: viewport.scrollHeight - viewport.clientHeight,
			}),
		})
		.toBeGreaterThan(20);
	await inspector.getByRole('button', { name: 'Reset zoom', exact: true }).click();
	expect(magnification()).toBe('100%');
	expect(viewport.scrollLeft).toBe(0);
	expect(viewport.scrollTop).toBe(0);
	expect(panSurface.scrollLeft).toBe(0);
	expect(panSurface.scrollTop).toBe(0);
	await expect
		.poll((): number => Math.abs(visual.getBoundingClientRect().width - initialBounds.width))
		.toBeLessThan(2);
	await inspector.getByRole('button', { name: 'Close', exact: true }).click();
	if (visualSelector === 'img') await expect.element(inspector).not.toBeInTheDocument();
	else await expect.element(inspector).not.toBeVisible();
	expect(original.isConnected).toBe(true);
	expect(original.closest('ex-document')).not.toBeNull();
	expect(original.getBoundingClientRect().width).toBeGreaterThan(50);
	expect(original.getBoundingClientRect().height).toBeGreaterThan(5);
}

test('native picture gestures enlarge the fitted image, pan available overflow, reset and retain main picture', async (): Promise<void> => {
	await page.viewport(1600, 650);
	const { document: fixture, frame } = await openFixture('layout');
	const original = requiredElement(fixture, '#inspection-picture img');
	await expect
		.poll((): boolean => {
			const picture = fixture.querySelector<HTMLImageElement>('#inspection-picture img');
			return !!picture?.complete && picture.naturalWidth > 0;
		})
		.toBe(true);
	await frame.getByRole('img', { name: 'Native inspection picture', exact: true }).click();
	await verifyNativeGestures({ fixture, frame, original, visualSelector: 'img' });
});

test('native Mermaid gestures enlarge the fitted SVG, pan available overflow, reset and retain main diagram', async (): Promise<void> => {
	await page.viewport(1600, 650);
	const { document: fixture, frame } = await openFixture('layout');
	const owner = requiredElement(fixture, '#wide-diagram');
	owner.scrollIntoView();
	await expect
		.poll((): string => owner.querySelector('svg')?.textContent ?? '')
		.toContain('Report the observed outcome');
	const original = requiredElement(owner, ':scope > svg');
	await fixtureLocator(frame, owner).hover();
	await frame
		.getByRole('button', { name: 'Open Wide comparison in a sidebar', exact: true })
		.click();
	await verifyNativeGestures({ fixture, frame, original, visualSelector: 'ex-mermaid > svg' });
	expect(owner.querySelector(':scope > svg')).toBe(original);
	expect(original.textContent).toContain('Receive a complete request');
});
