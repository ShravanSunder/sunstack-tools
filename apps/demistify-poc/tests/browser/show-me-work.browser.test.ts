import { afterEach, expect, test } from 'vitest';
import { page, userEvent } from 'vitest/browser';

import { browserScreenshotPath } from './browser-evidence.ts';
import { fixtureLocator, openFixture, requiredElement } from './fixture-page.ts';

const readingViews = [
	{ width: 1280, theme: 'light' },
	{ width: 1280, theme: 'dark' },
	{ width: 420, theme: 'light' },
	{ width: 420, theme: 'dark' },
	{ width: 390, theme: 'light' },
	{ width: 390, theme: 'dark' },
] as const;

afterEach(async (): Promise<void> => {
	document.body.replaceChildren();
	await page.viewport(1600, 1000);
});

test.each(readingViews)(
	'stories preserve rich content and supplied native states inside counted sections at $width in $theme',
	async ({ width, theme }): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture } = await openFixture('show-me-work');
		fixture.documentElement.dataset['theme'] = theme;
		await expect
			.poll((): boolean => requiredElement(fixture, 'ex-story').hasAttribute('prepared'))
			.toBe(true);
		expect(fixture.querySelectorAll('ex-section').length).toBe(4);
		expect(requiredElement(fixture, '#delivery > .ex-section-head').textContent).toContain(
			'1 + 1 smaller',
		);
		expect(requiredElement(fixture, '#rich-account').tagName).toBe('STRONG');
		expect(requiredElement(fixture, '#entry-one-body code').textContent).toBe('session.identity');
		expect(fixture.querySelectorAll('#entry-one').length).toBe(1);
		expect(requiredElement(fixture, '#story-main').hasAttribute('open')).toBe(true);
		for (const id of [
			'main-more',
			'main-record',
			'entry-one',
			'entry-two',
			'smaller',
			'story-minor',
			'minor-record',
			'attachment-note',
			'ordinary-notes',
		])
			expect(requiredElement(fixture, `#${id}`).hasAttribute('open')).toBe(false);
		expect(requiredElement(fixture, '#story-main [slot="happened"]').textContent).toContain(
			'Happened',
		);
		expect(requiredElement(fixture, '#story-main [slot="for-you"]').textContent).toContain(
			'For you',
		);
		expect(requiredElement(fixture, '#story-main [slot="next"]').textContent).toContain('Next');
		expect(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(
			fixture.documentElement.clientWidth,
		);
		requiredElement(fixture, '#story-diagram').scrollIntoView({
			block: 'center',
			behavior: 'instant',
		});
		await expect
			.poll((): string => fixture.querySelector('#story-diagram svg')?.textContent ?? '')
			.toContain('Retained record');
		requiredElement(fixture, '#delivery').scrollIntoView({ block: 'start', behavior: 'instant' });
		const headerBounds = requiredElement(
			fixture,
			'#delivery > .ex-section-head',
		).getBoundingClientRect();
		const titleBounds = requiredElement(fixture, '#story-main > summary').getBoundingClientRect();
		expect(titleBounds.top).toBeGreaterThanOrEqual(headerBounds.bottom);
		await page.screenshot({
			path: browserScreenshotPath(`story-${width}-${theme}.png`),
		});
	},
);

test.each(readingViews)(
	'Map replaces Comments through the same live control at $width in $theme',
	async ({ width, theme }): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture('show-me-work');
		fixture.documentElement.dataset['theme'] = theme;
		const map = frame.getByRole('button', { name: 'Map', exact: true });
		await map.click();
		const originalTrigger = requiredElement(fixture, '.ex-map-trigger');
		const originalTree = requiredElement(fixture, '#work-map');
		const mapPanel = frame.getByRole('complementary', { name: 'Map', exact: true });
		await expect.element(mapPanel).toBeVisible();
		const livePanel = requiredElement(fixture, '.ex-drawer[aria-label="Map"]');
		await expect
			.poll((): string | undefined => fixture.defaultView?.getComputedStyle(livePanel).opacity)
			.toBe('1');
		expect(fixture.querySelector('dialog:modal')).toBeNull();
		const panelBounds = requiredElement(
			fixture,
			'.ex-drawer[aria-label="Map"]',
		).getBoundingClientRect();
		if (width < 900)
			expect(panelBounds.top).toBeGreaterThanOrEqual(
				requiredElement(fixture, '.ex-document-toolbar').getBoundingClientRect().bottom,
			);
		await page.screenshot({
			path: browserScreenshotPath(`map-${width}-${theme}.png`),
		});
		await fixtureLocator(frame, requiredElement(fixture, '.ex-comments-trigger')).click();
		await expect
			.element(frame.getByRole('button', { name: 'Close comments', exact: true }))
			.toBeVisible();
		if (width < 900) expect(originalTrigger.closest('dialog:modal > header')).not.toBeNull();
		await map.click();
		await expect.element(mapPanel).toBeVisible();
		expect(requiredElement(fixture, '.ex-map-trigger')).toBe(originalTrigger);
		expect(requiredElement(fixture, '#work-map')).toBe(originalTree);
		expect(fixture.querySelector('dialog:modal')).toBeNull();
		await userEvent.keyboard('{Escape}');
		await expect
			.poll((): string | null => originalTrigger.getAttribute('aria-expanded'))
			.toBe('false');
		expect(livePanel.hasAttribute('hidden')).toBe(true);
		expect(livePanel.matches(':popover-open')).toBe(false);
		expect(fixture.defaultView?.getComputedStyle(livePanel).display).toBe('none');
		expect(originalTrigger.closest('.ex-reader-controls')).not.toBeNull();
		// The existing sidebar policy retains the sheet's original trigger only when invoked inside it.
		expect(fixture.activeElement).toBe(
			width < 900 ? requiredElement(fixture, '.ex-comments-trigger') : originalTrigger,
		);
		await map.click();
		await frame.getByRole('button', { name: 'Close Map', exact: true }).click();
		expect(fixture.activeElement).toBe(originalTrigger);
	},
);

test.each(readingViews)(
	'Map links reveal a closed routine story again when its fragment is already current at $width in $theme',
	async ({ width, theme }): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture('show-me-work');
		fixture.documentElement.dataset['theme'] = theme;
		const activateRoutine = async (): Promise<void> => {
			await frame.getByRole('link', { name: 'Go to Routine story', exact: true }).click();
			await expect
				.poll((): boolean => requiredElement(fixture, '#story-minor').hasAttribute('open'))
				.toBe(true);
			expect(requiredElement(fixture, '#smaller').hasAttribute('open')).toBe(true);
			expect(requiredElement(fixture, '#entry-one').hasAttribute('open')).toBe(false);
			const title = requiredElement(fixture, '#story-minor > summary').getBoundingClientRect();
			const header = requiredElement(
				fixture,
				'#delivery > .ex-section-head',
			).getBoundingClientRect();
			expect(title.top).toBeGreaterThanOrEqual(header.bottom);
			expect(title.bottom).toBeLessThanOrEqual(1000);
			if (width < 900)
				expect(requiredElement(fixture, '.ex-map-trigger').getAttribute('aria-expanded')).toBe(
					'false',
				);
		};
		await frame.getByRole('button', { name: 'Map', exact: true }).click();
		await activateRoutine();
		await fixtureLocator(frame, requiredElement(fixture, '#story-minor > summary')).click();
		await fixtureLocator(frame, requiredElement(fixture, '#smaller > summary')).click();
		if (width < 900) await frame.getByRole('button', { name: 'Map', exact: true }).click();
		await activateRoutine();
	},
);

test.each(readingViews.filter((view): boolean => view.width !== 420))(
	'existing Trace and card surfaces retain the observed base typography at $width in $theme',
	async ({ width, theme }): Promise<void> => {
		// Arrange: these literal values were inspected on frozen base 26b316c before feature edits.
		await page.viewport(width, 1000);
		const { document: fixture } = await openFixture('ui', '', 'content-spacing.html');
		fixture.documentElement.dataset['theme'] = theme;
		await expect
			.poll((): boolean => requiredElement(fixture, '#spacing-trace').hasAttribute('prepared'))
			.toBe(true);
		const view = fixture.defaultView;
		if (!view) throw new Error('Fixture window unavailable.');
		for (const [selector, padding, background] of [
			['#spacing-trace', '0px', 'rgba(0, 0, 0, 0)'],
			['#composition-step > summary', '12px 8px', 'rgba(0, 0, 0, 0)'],
			['#trace-owner', '20px', theme === 'dark' ? 'rgb(23, 29, 38)' : 'rgb(255, 255, 255)'],
		]) {
			if (!selector) throw new Error('Missing baseline selector.');
			// Act: inspect the real built components, not the new story's style values.
			const style = view.getComputedStyle(requiredElement(fixture, selector));
			// Assert: a global quiet-surface or compact-type override would fail these reads.
			expect(style.fontFamily).toBe('"Source Sans 3", system-ui, sans-serif');
			expect(style.fontSize).toBe('18px');
			expect(style.fontWeight).toBe('400');
			expect(style.lineHeight).toBe('27.9px');
			expect(style.padding).toBe(padding);
			expect(style.backgroundColor).toBe(background);
		}
	},
);

test.each(readingViews)(
	'page and section controls derive actions from every native member while preserving exclusions at $width in $theme',
	async ({ width, theme }): Promise<void> => {
		// Arrange: membership is literal contract data, not the production collector's output.
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture('show-me-work');
		fixture.documentElement.dataset['theme'] = theme;
		const deliveryMembers = [
			'story-main',
			'main-more',
			'main-record',
			'entry-one',
			'entry-two',
			'smaller',
			'story-minor',
			'minor-record',
		];
		const verifyState = (ids: readonly string[], open: boolean): void => {
			for (const id of ids)
				expect(requiredElement(fixture, `#${id}`).hasAttribute('open')).toBe(open);
		};
		await frame
			.getByRole('button', { name: "Expand What's done / what's next", exact: true })
			.click();
		verifyState(deliveryMembers, true);
		requiredElement(fixture, '#main-record').scrollIntoView({
			block: 'center',
			behavior: 'instant',
		});
		await page.screenshot({
			path: browserScreenshotPath(`record-${width}-${theme}.png`),
		});
		expect(requiredElement(fixture, '#other-record').hasAttribute('open')).toBe(false);
		await frame.getByRole('button', { name: 'Expand all stories', exact: true }).click();
		verifyState([...deliveryMembers, 'story-other', 'other-record'], true);
		await frame
			.getByRole('button', { name: "Collapse What's done / what's next", exact: true })
			.click();
		verifyState(deliveryMembers, false);
		verifyState(['story-other', 'other-record'], true);
		await frame.getByRole('button', { name: 'Expand all stories', exact: true }).click();
		await frame.getByRole('button', { name: 'Collapse all stories', exact: true }).click();
		verifyState([...deliveryMembers, 'story-other', 'other-record'], false);
		verifyState(['attachment-note', 'ordinary-notes'], false);
	},
);
