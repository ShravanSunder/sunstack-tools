import { afterEach, expect, test } from 'vitest';
import { page } from 'vitest/browser';

import { browserScreenshotPath } from './browser-evidence.ts';
import { openFixture, requiredElement } from './fixture-page.ts';

afterEach(async (): Promise<void> => {
	document.body.replaceChildren();
	await page.viewport(1600, 1000);
});

function currentFixture(): Document {
	const iframe = document.querySelector('iframe');
	if (!(iframe instanceof HTMLIFrameElement) || !iframe.contentDocument)
		throw new Error('The real fixture document is unavailable.');
	return iframe.contentDocument;
}

test('a JSON Walkthrough starts from a peer page and continues across pages through the same source', async (): Promise<void> => {
	const { frame } = await openFixture('shared-navigation', '', 'state.html');
	await frame.getByRole('button', { name: 'Browse walkthroughs', exact: true }).click();
	await frame
		.getByRole('option', { name: /How can central sleep without losing delivery/ })
		.click();
	await expect
		.element(frame.getByRole('region', { name: 'Walkthrough player', exact: true }))
		.toBeVisible();
	await expect
		.poll(
			(): string | undefined =>
				currentFixture().querySelector('.ex-route-count')?.textContent ?? undefined,
		)
		.toBe('1 / 5');
	await expect
		.poll((): string | undefined => currentFixture().querySelector('.ex-route-focus')?.id)
		.toBe('seq-mail');
	await frame.getByRole('button', { name: 'Next step', exact: true }).click();
	await expect
		.element(frame.getByRole('heading', { name: 'Where delivery state lives', exact: true }))
		.toBeVisible();
	await expect
		.poll(
			(): string | undefined =>
				currentFixture().querySelector('.ex-route-count')?.textContent ?? undefined,
		)
		.toBe('2 / 5');
	await expect
		.poll((): string | undefined => currentFixture().querySelector('.ex-route-focus')?.id)
		.toBe('stored-custody');
	await frame.getByRole('button', { name: 'Next step', exact: true }).click();
	await expect
		.poll(
			(): string | undefined =>
				currentFixture().querySelector('.ex-route-count')?.textContent ?? undefined,
		)
		.toBe('3 / 5');
	await expect
		.poll((): string | undefined => currentFixture().querySelector('.ex-route-focus')?.id)
		.toBe('seq-hibernate');
	await frame.getByRole('button', { name: 'Close walkthrough', exact: true }).click();
	await frame.getByRole('button', { name: 'Map', exact: true }).click();
	await frame
		.getByRole('complementary', { name: 'Map', exact: true })
		.getByRole('link', { name: 'Stored custody survives the nap', exact: true })
		.click();
	await expect
		.element(frame.getByRole('heading', { name: 'Where delivery state lives', exact: true }))
		.toBeVisible();
	await expect.poll((): string => currentFixture().location.hash).toBe('#stored-custody');
	await expect
		.element(frame.getByRole('button', { name: 'Browse walkthroughs', exact: true }))
		.toBeVisible();
});

test.each([
	{ width: 1600, theme: 'light' },
	{ width: 1600, theme: 'dark' },
	{ width: 390, theme: 'light' },
	{ width: 390, theme: 'dark' },
])(
	'shared Map retains its full search field, icons and expanded rows at $width in $theme',
	async ({ width, theme }): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture('shared-navigation', '', 'state.html');
		fixture.documentElement.dataset['theme'] = theme;
		await frame.getByRole('button', { name: 'Map', exact: true }).click();
		const map = frame.getByRole('complementary', { name: 'Map', exact: true });
		await expect
			.element(map.getByRole('link', { name: 'Send, sleep, then handle the answer', exact: true }))
			.toBeVisible();
		const input = requiredElement(fixture, '.ex-navigation-map-search input');
		const field = requiredElement(fixture, '.ex-navigation-map-search');
		expect(input.getBoundingClientRect().width).toBeGreaterThan(
			field.getBoundingClientRect().width * 0.7,
		);
		expect(input.getBoundingClientRect().right).toBeLessThanOrEqual(
			field.getBoundingClientRect().right,
		);
		expect(fixture.defaultView?.getComputedStyle(input).borderLeftWidth).toBe('0px');
		const rows = Array.from(fixture.querySelectorAll('.ex-navigation-map-row'));
		expect(rows.length).toBe(8);
		for (const row of rows) {
			expect(row.querySelector('.ph')).not.toBeNull();
		}
		await expect
			.poll((): boolean =>
				rows.every((row): boolean => row.getBoundingClientRect().right <= width + 1),
			)
			.toBe(true);
		expect(fixture.querySelector('.ex-navigation-map button')).toBeNull();
		await expect
			.poll(
				(): string | undefined =>
					fixture.defaultView?.getComputedStyle(
						requiredElement(fixture, '.ex-drawer[data-presentation="map"]'),
					).opacity,
			)
			.toBe('1');
		await page.screenshot({ path: browserScreenshotPath(`shared-map-${width}-${theme}.png`) });
	},
);

test.each([
	['unavailable.html', 'HTTP 404'],
	['invalid.html', 'Unknown page id'],
])(
	'a shared source failure in %s is visible while ordinary content and outline remain usable',
	async (file, message): Promise<void> => {
		const { document: fixture, frame } = await openFixture('shared-navigation', '', file);
		await expect.element(frame.getByRole('alert')).toBeVisible();
		await expect
			.poll((): string => fixture.querySelector('[role="alert"]')?.textContent ?? '')
			.toContain(message);
		await expect
			.element(frame.getByRole('heading', { name: 'Where delivery state lives', exact: true }))
			.toBeVisible();
		expect(fixture.querySelector('[aria-label="Page navigation"]')).not.toBeNull();
		expect(fixture.querySelector('.ex-map-trigger')).toBeNull();
	},
);

test('a peer page reads Pages, expanded Map and Walkthroughs from the shared JSON', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('shared-navigation', '', 'state.html');
	await frame.getByRole('button', { name: 'Browse pages', exact: true }).click();
	await expect
		.element(frame.getByRole('link', { name: /Where delivery state lives/ }))
		.toBeVisible();
	await frame.getByRole('button', { name: 'Map', exact: true }).click();
	const map = frame.getByRole('complementary', { name: 'Map', exact: true });
	await expect
		.element(map.getByRole('link', { name: 'Stored custody survives the nap', exact: true }))
		.toBeVisible();
	await expect
		.element(map.getByRole('link', { name: 'Send, sleep, then handle the answer', exact: true }))
		.toBeVisible();
	expect(fixture.querySelector('ex-map button[aria-expanded]')).toBeNull();
	const search = map.getByRole('searchbox', { name: 'Search groups and requirements' });
	await search.fill('backlog');
	await expect
		.element(map.getByRole('link', { name: 'Drain a backlog without flooding', exact: true }))
		.toBeVisible();
	await expect
		.poll((): readonly string[] =>
			Array.from(fixture.querySelectorAll('.ex-navigation-map-row'))
				.filter((row): boolean => row.getClientRects().length > 0)
				.map((row): string => row.textContent ?? ''),
		)
		.not.toContain('Stored custody survives the nap');
	await search.fill('');
	await frame.getByRole('button', { name: 'Close Map', exact: true }).click();
	await frame.getByRole('button', { name: 'Browse walkthroughs', exact: true }).click();
	await expect
		.element(frame.getByRole('option', { name: /How can central sleep without losing delivery/ }))
		.toBeVisible();
	expect(requiredElement(fixture, 'ex-nav').querySelector('nav[slot="pages"]')).toBeNull();
	expect(fixture.querySelector('ex-presentation')).toBeNull();
});
