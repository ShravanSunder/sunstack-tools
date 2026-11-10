import { afterEach, expect, test } from 'vitest';
import { page, userEvent } from 'vitest/browser';

import { browserScreenshotPath } from './browser-evidence.ts';
import { fixtureLocator, openFixture, requiredElement } from './fixture-page.ts';

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

test('JSON Walkthroughs stay available on the page containing their first stop', async (): Promise<void> => {
	const { frame } = await openFixture('shared-navigation', '', 'how-work-runs.html');
	await frame.getByRole('button', { name: 'Browse walkthroughs', exact: true }).click();
	await expect
		.element(frame.getByRole('option', { name: /How can central sleep without losing delivery/ }))
		.toBeVisible();
});

test('Map group search preserves the current Walkthrough address and live page', async (): Promise<void> => {
	const { frame } = await openFixture('shared-navigation', '', 'how-work-runs.html');
	await frame.getByRole('button', { name: 'Browse walkthroughs', exact: true }).click();
	await frame
		.getByRole('option', { name: /How can central sleep without losing delivery/ })
		.click();
	await expect
		.element(frame.getByRole('region', { name: 'Walkthrough player', exact: true }))
		.toBeVisible();
	await frame.getByRole('button', { name: 'Next step', exact: true }).click();
	await expect
		.poll(
			(): string | null | undefined =>
				currentFixture().querySelector('.ex-route-count')?.textContent,
		)
		.toBe('2 / 5');
	await frame.getByRole('button', { name: 'Next step', exact: true }).click();
	await expect
		.poll(
			(): string | null | undefined =>
				currentFixture().querySelector('.ex-route-count')?.textContent,
		)
		.toBe('3 / 5');
	const fixture = currentFixture();
	await frame.getByRole('button', { name: 'Next step', exact: true }).click();
	await expect
		.poll(
			(): string | null | undefined =>
				currentFixture().querySelector('.ex-route-count')?.textContent,
		)
		.toBe('4 / 5');
	expect(currentFixture()).toBe(fixture);
	const before = fixture.location.search;
	expect(new URLSearchParams(before).get('ex-route')).toBe('sleep-and-deliver');
	await frame.getByRole('button', { name: 'Search documents', exact: true }).click();
	const dialog = frame.getByRole('dialog', { name: 'Search documents', exact: true });
	await dialog.getByRole('combobox').fill('sleep and wake');
	await dialog
		.getByRole('option', {
			name: 'Map › Sleep and wake Find when central can sleep and what wakes it.',
			exact: true,
		})
		.click();
	await expect
		.element(frame.getByRole('complementary', { name: 'Map', exact: true }))
		.toBeVisible();
	expect(currentFixture()).toBe(fixture);
	expect(currentFixture().location.search).toBe(before);
	await expect
		.element(frame.getByRole('region', { name: 'Walkthrough player', exact: true }))
		.toBeVisible();
});

test('current and peer page search results keep their authored subject icons', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('shared-navigation', '', 'state.html');
	await frame.getByRole('button', { name: 'Search documents', exact: true }).click();
	expect(
		requiredElement(fixture, '[role="option"][aria-label="How work runs"]').querySelector(
			'.ph-path',
		),
	).not.toBeNull();
	expect(
		requiredElement(
			fixture,
			'[role="option"][aria-label="Where delivery state lives"]',
		).querySelector('.ph-database'),
	).not.toBeNull();
});

test('native Pages icon aliases use the same normalization in collection search', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('shared-navigation', '', 'state.html');
	requiredElement(fixture, 'ex-nav').remove();
	const navigation = fixture.createElement('ex-nav');
	navigation.setAttribute('for', 'content');
	navigation.innerHTML =
		'<nav slot="pages"><a href="how-work-runs.html" icon="path">How work runs</a><a href="state.html" icon="server">Where delivery state lives</a></nav>';
	fixture.body.prepend(navigation);
	await frame.getByRole('button', { name: 'Search documents', exact: true }).click();
	expect(
		requiredElement(
			fixture,
			'[role="option"][aria-label="Where delivery state lives"]',
		).querySelector('.ph-hard-drives'),
	).not.toBeNull();
});

test('reconnecting shared navigation keeps one Map, one launcher and one route player', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('shared-navigation', '', 'state.html');
	const navigation = requiredElement(fixture, 'ex-nav');
	for (let cycle = 0; cycle < 2; cycle += 1) {
		navigation.remove();
		fixture.body.prepend(navigation);
		expect(fixture.querySelectorAll('ex-map')).toHaveLength(1);
		expect(fixture.querySelectorAll('.ex-map-trigger')).toHaveLength(1);
		expect(fixture.querySelectorAll('.ex-route-player')).toHaveLength(1);
	}
	await frame.getByRole('button', { name: 'Map', exact: true }).click();
	await frame
		.getByRole('searchbox', { name: 'Search groups and requirements' })
		.fill('durable facts');
	await expect
		.element(
			frame
				.getByRole('complementary', { name: 'Map', exact: true })
				.getByRole('link', { name: 'Stored custody survives the nap', exact: true }),
		)
		.toBeVisible();
});

test('reader commands share the top toolbar with Footprints immediately before Map', async (): Promise<void> => {
	const { document: fixture } = await openFixture('shared-navigation', '', 'state.html');
	const toolbar = requiredElement(fixture, '.ex-document-toolbar');
	const controls = requiredElement(fixture, '.ex-reader-controls');
	expect(toolbar.contains(controls)).toBe(true);
	const walkthrough = requiredElement(controls, '[aria-label="Browse walkthroughs"]');
	expect(walkthrough.querySelector('.ph-footprints')).not.toBeNull();
	expect(walkthrough.nextElementSibling?.getAttribute('aria-label')).toBe('Map');
	const map = requiredElement(controls, '[aria-label="Map"]');
	expect(fixture.defaultView?.getComputedStyle(walkthrough).borderRadius).toBe(
		fixture.defaultView?.getComputedStyle(map).borderRadius,
	);
	expect(walkthrough.getBoundingClientRect().width).toBe(
		walkthrough.getBoundingClientRect().height,
	);
	expect(controls.getBoundingClientRect().bottom).toBeLessThanOrEqual(
		toolbar.getBoundingClientRect().bottom,
	);
});

test('Cmd-K groups actual destinations, puts titles first, and reveals a filtered Map group', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('shared-navigation', '', 'state.html');
	await frame.getByRole('button', { name: 'Map', exact: true }).click();
	await frame.getByRole('searchbox', { name: 'Search groups and requirements' }).fill('backlog');
	await userEvent.keyboard('{Control>}k{/Control}');
	const dialog = frame.getByRole('dialog', { name: 'Search documents', exact: true });
	await expect.element(dialog).toBeVisible();
	expect(
		Array.from(fixture.querySelectorAll('.ex-command-group-title')).map(
			(label): string => label.textContent ?? '',
		),
	).toEqual(['Pages and headings', 'Walkthroughs', 'Map']);
	const heading = requiredElement(
		fixture,
		'.ex-command-results [role="option"]:has(.ex-command-context)',
	);
	expect(heading.querySelector('div')?.firstElementChild?.tagName).toBe('STRONG');
	await dialog.getByRole('combobox').fill('sleep and wake');
	await dialog
		.getByRole('option', {
			name: 'Map › Sleep and wake Find when central can sleep and what wakes it.',
			exact: true,
		})
		.click();
	await expect
		.poll(
			(): boolean | undefined =>
				fixture.querySelector<HTMLDialogElement>('.ex-command-dialog')?.open,
		)
		.toBe(false);
	const map = frame.getByRole('complementary', { name: 'Map', exact: true });
	await expect
		.element(map.getByRole('link', { name: 'Stored custody survives the nap', exact: true }))
		.toBeVisible();
	expect(fixture.querySelector<HTMLInputElement>('.ex-navigation-map-search input')?.value).toBe(
		'',
	);
	expect(fixture.activeElement?.getAttribute('aria-label')).toBe('Sleep and wake');
	await userEvent.keyboard('{ArrowRight}{ArrowLeft}');
	expect(fixture.activeElement?.getAttribute('aria-label')).toBe('Sleep and wake');
});

test('Map searches descriptions and Enter opens the matching content', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('shared-navigation', '', 'state.html');
	await frame.getByRole('button', { name: 'Map', exact: true }).click();
	await frame
		.getByRole('searchbox', { name: 'Search groups and requirements' })
		.fill('durable facts');
	const link = frame
		.getByRole('complementary', { name: 'Map', exact: true })
		.getByRole('link', { name: 'Stored custody survives the nap', exact: true });
	await expect.element(link).toBeVisible();
	expect(requiredElement(fixture, '.ex-navigation-map-description').textContent).not.toBe('');
	await userEvent.keyboard('{Enter}');
	await expect.poll((): string => currentFixture().location.hash).toBe('#stored-custody');
});

test('typing navigation shortcut letters in Map search keeps editing ownership', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('shared-navigation', '', 'state.html');
	await frame.getByRole('button', { name: 'Map', exact: true }).click();
	await userEvent.keyboard('wpc');
	expect(fixture.querySelector<HTMLInputElement>('.ex-navigation-map-search input')?.value).toBe(
		'wpc',
	);
	expect(fixture.querySelector<HTMLDialogElement>('.ex-command-dialog')?.open).toBe(false);
	expect(fixture.querySelector('.ex-pages-panel')?.matches(':popover-open')).toBe(false);
	expect(fixture.querySelector('.ex-map-trigger')?.getAttribute('aria-expanded')).toBe('true');
	await frame.getByRole('searchbox', { name: 'Search groups and requirements' }).fill('');
	await userEvent.keyboard('{ArrowDown}wpcm');
	expect(fixture.querySelector<HTMLDialogElement>('.ex-command-dialog')?.open).toBe(false);
	expect(fixture.querySelector('.ex-map-trigger')?.getAttribute('aria-expanded')).toBe('true');
	await userEvent.keyboard('{Escape}');
	await fixtureLocator(frame, requiredElement(fixture, '.ex-comments-trigger')).click();
	await userEvent.keyboard('wpcm');
	expect(fixture.querySelector<HTMLDialogElement>('.ex-command-dialog')?.open).toBe(false);
	expect(fixture.querySelector('.ex-pages-panel')?.matches(':popover-open')).toBe(false);
	expect(fixture.querySelector('.ex-drawer[aria-label="Comments"]')?.getClientRects().length).toBe(
		1,
	);
});

test('Option page digits preserve content editing and work from Map search', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture(
		'shared-navigation',
		'',
		'how-work-runs.html',
	);
	const editor = fixture.createElement('textarea');
	editor.setAttribute('aria-label', 'Draft');
	requiredElement(fixture, 'ex-document').prepend(editor);
	await fixtureLocator(frame, editor).click();
	await userEvent.keyboard('{Alt>}2{/Alt}');
	expect(fixture.location.pathname).toContain('how-work-runs.html');
	await frame.getByRole('button', { name: 'Map', exact: true }).click();
	await userEvent.keyboard('{Alt>}2{/Alt}');
	await expect
		.element(frame.getByRole('heading', { name: 'Where delivery state lives', exact: true }))
		.toBeVisible();
});

test('Map focuses search, supports arrows and Enter, and Escape restores its trigger', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('shared-navigation', '', 'state.html');
	await frame.getByRole('button', { name: 'Map', exact: true }).click();
	expect(fixture.activeElement).toBe(fixture.querySelector('.ex-navigation-map-search input'));
	await userEvent.keyboard('{ArrowDown}{ArrowDown}{Enter}');
	await expect.poll((): string => currentFixture().location.hash).toBe('#hibernate');
	await frame.getByRole('button', { name: 'Map', exact: true }).click();
	await expect
		.element(frame.getByRole('complementary', { name: 'Map', exact: true }))
		.toBeVisible();
	expect(currentFixture().activeElement).toBe(
		currentFixture().querySelector('.ex-navigation-map-search input'),
	);
	await userEvent.keyboard('{Escape}');
	await expect
		.poll(
			(): number | undefined =>
				currentFixture().querySelector('.ex-drawer[data-presentation="map"]')?.getClientRects()
					.length,
		)
		.toBe(0);
	expect(currentFixture().activeElement?.getAttribute('aria-label')).toBe('Map');
});

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
		const toolbar = requiredElement(fixture, '.ex-document-toolbar');
		const controls = requiredElement(fixture, '.ex-reader-controls');
		expect(controls.getBoundingClientRect().right).toBeLessThanOrEqual(width);
		expect(controls.getBoundingClientRect().bottom).toBeLessThanOrEqual(
			toolbar.getBoundingClientRect().bottom,
		);
		expect(
			fixture.defaultView?.getComputedStyle(requiredElement(fixture, '.ex-document-breadcrumbs'))
				.display === 'none',
		).toBe(width < 900);
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
		await frame.getByRole('button', { name: 'Search documents', exact: true }).click();
		const dialog = requiredElement(fixture, '.ex-command-dialog');
		const inputSearch = requiredElement(dialog, 'input');
		await userEvent.keyboard('{ArrowUp}');
		const results = requiredElement(dialog, '.ex-command-results');
		const footer = requiredElement(dialog, 'footer');
		expect(results.getBoundingClientRect().bottom).toBeLessThanOrEqual(
			footer.getBoundingClientRect().top,
		);
		expect(dialog.getBoundingClientRect().bottom).toBeLessThanOrEqual(1000);
		// Walk every result across section boundaries; the selected last row must remain above the footer.
		const count = dialog.querySelectorAll('[role="option"]').length;
		await userEvent.keyboard('{ArrowDown}'.repeat(count));
		const selected = requiredElement(dialog, '[aria-selected="true"]');
		expect(selected.getBoundingClientRect().bottom).toBeLessThanOrEqual(
			footer.getBoundingClientRect().top,
		);
		expect(inputSearch.getAttribute('aria-activedescendant')).toBe(selected.id);
		await page.screenshot({ path: browserScreenshotPath(`shared-command-${width}-${theme}.png`) });
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
