import { afterEach, expect, test } from 'vitest';
import { commands, page } from 'vitest/browser';

import { fixtureLocator, openFixture, requiredElement } from './fixture-page.ts';

afterEach(async (): Promise<void> => {
	document.body.replaceChildren();
	await commands.setReducedMotion('no-preference');
	await page.viewport(1600, 1000);
});

test('selected gallery title shows its authored tags and replaces stale tags', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('ui', '', 'journey.html');
	const gallery = requiredElement(fixture, '#journey-gallery');
	const local = fixtureLocator(frame, gallery);
	expect(requiredElement(gallery, '.ex-gallery-title').textContent).toBe(
		'One Mobile · #Journey · #One',
	);
	await local.getByRole('button', { name: 'Two Mobile', exact: true }).click();
	expect(requiredElement(gallery, '.ex-gallery-title').textContent).toBe(
		'Two Mobile · #Journey · #Two',
	);
});

test.each([1600, 390])(
	'thumbnail collapse at %ipx visibly interpolates and can reopen',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture('ui', '', 'journey.html');
		const gallery = requiredElement(fixture, '#journey-gallery');
		const local = fixtureLocator(frame, gallery);
		const body = requiredElement(gallery, '.ex-gallery-body');
		const viewer = requiredElement(gallery, '.ex-gallery-viewer');
		const strip = requiredElement(gallery, '.ex-gallery-strip');
		const sidebarSize = (): number =>
			width > 700 ? strip.getBoundingClientRect().width : strip.getBoundingClientRect().height;
		const initialSidebarSize = sidebarSize();
		const measure = (): number =>
			width > 700 ? viewer.getBoundingClientRect().width : viewer.getBoundingClientRect().height;
		const initial = measure();
		const full =
			width > 700 ? body.getBoundingClientRect().width : body.getBoundingClientRect().height;
		let movement: Animation[] = [];
		body.addEventListener(
			'transitionrun',
			(): void => {
				movement = body.getAnimations();
				for (const animation of movement) animation.pause();
			},
			{ once: true },
		);
		await local.getByRole('button', { name: 'Hide thumbnails', exact: true }).click();
		await expect.poll((): boolean => movement.length > 0).toBe(true);
		for (const animation of movement)
			animation.currentTime = Number(animation.effect?.getComputedTiming().duration ?? 0) / 2;
		await expect.poll(measure).toBeGreaterThan(initial + 1);
		expect(measure()).toBeLessThan(full - 1);
		expect(sidebarSize()).toBeGreaterThan(1);
		expect(sidebarSize()).toBeLessThan(initialSidebarSize - 1);
		for (const animation of body.getAnimations({ subtree: true })) animation.finish();
		await expect.poll(measure).toBeCloseTo(full, 0);
		expect(requiredElement(gallery, '.ex-gallery-strip').hasAttribute('inert')).toBe(true);
		await local.getByRole('button', { name: 'Show thumbnails', exact: true }).click();
		await expect.poll(measure).toBeCloseTo(initial, 0);
		await expect
			.element(fixtureLocator(frame, requiredElement(gallery, '.ex-gallery-strip')))
			.toBeVisible();
	},
);

test('reduced motion collapses thumbnails immediately without a transition', async (): Promise<void> => {
	await commands.setReducedMotion('reduce');
	const { document: fixture, frame } = await openFixture('ui', '', 'journey.html');
	const gallery = requiredElement(fixture, '#journey-gallery');
	expect(fixture.defaultView?.matchMedia('(prefers-reduced-motion: reduce)').matches).toBe(true);
	await fixtureLocator(frame, gallery)
		.getByRole('button', { name: 'Hide thumbnails', exact: true })
		.click();
	const body = requiredElement(gallery, '.ex-gallery-body');
	expect(body.getAnimations({ subtree: true })).toHaveLength(0);
	expect(requiredElement(gallery, '.ex-gallery-viewer').getBoundingClientRect().width).toBeCloseTo(
		body.getBoundingClientRect().width,
		0,
	);
});

test('gallery journey and walkthrough share player width, elevation and rich Markdown', async (): Promise<void> => {
	const normal = await openFixture('ui', '#narration', 'narration.html');
	const normalPlayer = requiredElement(normal.document, '.ex-route-player');
	const normalStyle = normal.document.defaultView?.getComputedStyle(normalPlayer);
	const expected = {
		width: normalPlayer.getBoundingClientRect().width,
		left: normalPlayer.getBoundingClientRect().left,
		shadow: normalStyle?.boxShadow,
		background: normalStyle?.backgroundColor,
		radius: normalStyle?.borderRadius,
	};
	expect(expected.shadow).not.toBe('none');
	document.body.replaceChildren();
	const journey = await openFixture('ui', '#journey', 'journey.html');
	const player = requiredElement(journey.document, '.ex-route-player');
	const style = journey.document.defaultView?.getComputedStyle(player);
	expect(player.getBoundingClientRect().width).toBeCloseTo(expected.width, 0);
	expect(player.getBoundingClientRect().left).toBeCloseTo(expected.left, 0);
	expect(style?.boxShadow).toBe(expected.shadow);
	expect(style?.backgroundColor).toBe(expected.background);
	expect(style?.borderRadius).toBe(expected.radius);
	await expect
		.element(journey.frame.getByText('Inspect the screen', { exact: true }))
		.toBeVisible();
	expect(player.querySelectorAll('.ex-route-narration li')).toHaveLength(2);
	expect(player.querySelector('.ex-route-narration code')?.textContent).toBe('SessionId');
});
