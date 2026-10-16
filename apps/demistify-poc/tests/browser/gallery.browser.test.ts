import { afterEach, expect, test } from 'vitest';
import { commands, page, userEvent } from 'vitest/browser';

import { fixtureLocator, openFixture, requiredElement } from './fixture-page.ts';

afterEach(async (): Promise<void> => {
	document.body.replaceChildren();
	await page.viewport(1600, 1000);
});

test('presentation crosses files, returns to its home and reports an unavailable stop visibly', async (): Promise<void> => {
	const { frame } = await openFixture('gallery', '#cross-page', 'navigation.html');
	await expect
		.element(
			frame
				.getByRole('region', { name: 'Presentation player', exact: true })
				.getByText('The shared decision.', { exact: true }),
		)
		.toBeVisible();
	await frame.getByRole('button', { name: 'Next step', exact: true }).click();
	await expect
		.element(frame.getByRole('heading', { name: 'Handoff evidence', exact: true }))
		.toBeVisible();
	await expect.element(frame.getByText('The second file.', { exact: true })).toBeVisible();
	await frame.getByRole('button', { name: 'Next step', exact: true }).click();
	await expect
		.poll(
			(): string =>
				document
					.querySelector<HTMLIFrameElement>('iframe')
					?.contentDocument?.querySelector('.ex-route-narration')?.textContent ?? '',
			{ timeout: 8000 },
		)
		.toBe('Target unavailable: unavailable-target');
	await expect
		.element(frame.getByRole('heading', { name: 'Presentations and deep links', exact: true }))
		.toBeVisible();
});

test('screenshot modal dismisses through backdrop and Escape without losing main selection', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('gallery');
	const gallery = requiredElement(fixture, '#proof-gallery');
	const local = fixtureLocator(frame, gallery);
	const expand = local.getByRole('button', { name: 'Expand screenshot', exact: true });
	await expand.click();
	const dialog = frame.getByRole('dialog', { name: 'Home Mobile', exact: true });
	await expect.element(dialog).toBeVisible();
	await commands.clickInspectionBackdrop();
	await expect.element(dialog).not.toBeInTheDocument();
	expect(fixture.activeElement?.getAttribute('aria-label')).toBe('Expand screenshot');
	await expand.click();
	await userEvent.keyboard('{Escape}');
	await expect.element(dialog).not.toBeInTheDocument();
	await expect.element(local.getByRole('img', { name: 'Home Mobile', exact: true })).toBeVisible();
});

test('narrow gallery contains its toolbar and scrolling thumbnails within the page', async (): Promise<void> => {
	await page.viewport(390, 900);
	const { document: fixture, frame } = await openFixture('gallery');
	const gallery = requiredElement(fixture, '#proof-gallery');
	await expect
		.poll(
			(): number =>
				gallery.querySelector<HTMLImageElement>('.ex-gallery-preview img')?.naturalWidth ?? 0,
		)
		.toBe(1000);
	expect(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(391);
	const strip = requiredElement(gallery, '.ex-gallery-strip');
	expect(strip.scrollWidth).toBeGreaterThan(strip.clientWidth);
	expect(fixture.defaultView?.getComputedStyle(strip).overflowX).toBe('auto');
	await expect
		.element(
			frame.getByRole('button', { name: 'Browse page headings', exact: true, includeHidden: true }),
		)
		.not.toBeVisible();
});

test('gallery switches between review and grid without changing selection or another island', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('gallery');
	const gallery = requiredElement(fixture, '#proof-gallery');
	const local = fixtureLocator(frame, gallery);
	await expect.element(local.getByRole('button', { name: 'Grid view', exact: true })).toBeVisible();
	await local.getByRole('button', { name: 'Grid view', exact: true }).click();
	await expect
		.element(local.getByRole('button', { name: 'Grid view', exact: true }))
		.toHaveAttribute('aria-pressed', 'true');
	expect(gallery.querySelectorAll('.ex-gallery-grid figure')).toHaveLength(4);
	expect(requiredElement(fixture, '#second-gallery').getAttribute('view')).toBe('review');
	await local.getByRole('button', { name: 'Review view', exact: true }).click();
	await expect.element(local.getByRole('img', { name: 'Home Mobile', exact: true })).toBeVisible();
});

test('gallery uses existing sidebar inspection while its main image and caption remain', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('gallery');
	const gallery = requiredElement(fixture, '#proof-gallery');
	const local = fixtureLocator(frame, gallery);
	await local.getByRole('button', { name: 'Open screenshot in sidebar', exact: true }).click();
	const panel = frame.getByRole('complementary', { name: 'Home Mobile', exact: true });
	await expect.element(panel).toBeVisible();
	await expect.element(panel.getByRole('img', { name: 'Home Mobile', exact: true })).toBeVisible();
	expect(gallery.querySelector('.ex-gallery-preview img')).not.toBeNull();
	expect(gallery.querySelector('.ex-inspect-placeholder')).toBeNull();
	await expect.element(local.getByText('SessionId', { exact: true })).toBeVisible();
	expect(fixture.querySelectorAll('button[data-ex-theme]')).toHaveLength(1);
	expect(gallery.querySelectorAll('dialog')).toHaveLength(0);
});

test('gallery viewport keys share filter state and leave text fields alone', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('gallery');
	const gallery = requiredElement(fixture, '#proof-gallery');
	const local = fixtureLocator(frame, gallery);
	await local.getByRole('button', { name: 'Natural size', exact: true }).click();
	await userEvent.keyboard('{ArrowRight}');
	await expect
		.element(local.getByRole('combobox', { name: 'Viewport category', exact: true }))
		.toHaveValue('Laptop');
	await userEvent.keyboard('{ArrowDown}');
	await expect
		.element(local.getByRole('img', { name: 'Settings Laptop', exact: true }))
		.toBeVisible();
	await userEvent.keyboard('{ArrowLeft}');
	await expect
		.element(local.getByRole('img', { name: 'Settings Mobile', exact: true }))
		.toBeVisible();
	const search = local.getByRole('searchbox', { name: 'Search screenshots', exact: true });
	await search.fill('Settings');
	await userEvent.keyboard('{ArrowRight}');
	await expect
		.element(local.getByRole('combobox', { name: 'Viewport category', exact: true }))
		.toHaveValue('Mobile');
	expect(requiredElement(fixture, '#second-gallery .ex-gallery-count').textContent).toBe(
		'4 screenshots',
	);
});

test('200ms settled filtering retains the exact selected image node and natural zoom', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('gallery');
	const gallery = requiredElement(fixture, '#proof-gallery');
	const local = fixtureLocator(frame, gallery);
	await expect
		.poll(
			(): number =>
				gallery.querySelector<HTMLImageElement>('.ex-gallery-preview img')?.naturalWidth ?? 0,
		)
		.toBe(1000);
	await local.getByRole('button', { name: 'Natural size', exact: true }).click();
	const image = requiredElement(gallery, '.ex-gallery-preview img');
	let lastInput = 0;
	let settledAfter = 0;
	gallery.addEventListener(
		'input',
		(): void => {
			lastInput = fixture.defaultView?.performance.now() ?? 0;
		},
		{ capture: true },
	);
	const observer = new MutationObserver((): void => {
		if (gallery.querySelectorAll('.ex-gallery-thumbnail').length === 1)
			settledAfter = (fixture.defaultView?.performance.now() ?? 0) - lastInput;
	});
	observer.observe(requiredElement(gallery, '.ex-gallery-strip'), { childList: true });
	try {
		const search = local.getByRole('searchbox', { name: 'Search screenshots', exact: true });
		await search.fill('Home');
		await search.fill('Home Mobile');
		await expect
			.poll((): number => gallery.querySelectorAll('.ex-gallery-thumbnail').length)
			.toBe(1);
		expect(settledAfter).toBeGreaterThanOrEqual(199);
		expect(requiredElement(gallery, '.ex-gallery-preview img')).toBe(image);
		expect(requiredElement(gallery, '.ex-gallery-zoom').textContent).toBe('100%');
	} finally {
		observer.disconnect();
	}
});

test('cold presentation and duplicate diagram node fragments resolve to the intended owner', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture(
		'gallery',
		'#walkthrough',
		'navigation.html',
	);
	await expect
		.poll((): boolean =>
			requiredElement(fixture, '#requirements--required-behavior').classList.contains(
				'ex-route-focus',
			),
		)
		.toBe(true);
	await frame.getByRole('button', { name: 'Next step', exact: true }).click();
	await expect
		.poll(
			(): string => fixture.querySelector('#after .ex-route-focus')?.getAttribute('data-id') ?? '',
		)
		.toBe('Registry');
	expect(fixture.querySelector('#before .ex-route-focus')).toBeNull();
	await expect
		.element(fixtureLocator(frame, requiredElement(fixture, '.ex-presentation-marker')))
		.toBeVisible();
	expect(
		requiredElement(fixture, '.ex-presentation-marker').getBoundingClientRect().right,
	).toBeLessThan(requiredElement(fixture, '#after').getBoundingClientRect().left);
	await frame.getByRole('button', { name: 'Next step', exact: true }).click();
	await expect
		.poll((): boolean => requiredElement(fixture, '#lifetime').classList.contains('ex-route-focus'))
		.toBe(true);
	fixture.defaultView?.history.back();
	await expect
		.poll(
			(): string => fixture.querySelector('#after .ex-route-focus')?.getAttribute('data-id') ?? '',
		)
		.toBe('Registry');
	fixture.defaultView?.history.forward();
	await expect
		.poll((): boolean => requiredElement(fixture, '#lifetime').classList.contains('ex-route-focus'))
		.toBe(true);
});

test('file Markdown renders tasks, literal code and relative images; tree links and preformatted text retain their roles', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('gallery', '', 'navigation.html');
	const notes = requiredElement(fixture, '#requirements');
	await expect
		.element(frame.getByRole('cell', { name: 'Session lookup', exact: true }))
		.toBeVisible();
	expect(notes.querySelectorAll('input[type="checkbox"]')).toHaveLength(4);
	const tasks = notes.querySelectorAll(
		'li:has(> input[type="checkbox"]), li:has(> p > input[type="checkbox"])',
	);
	expect(tasks).toHaveLength(4);
	for (const task of tasks)
		expect(fixture.defaultView?.getComputedStyle(task).listStyleType).toBe('none');
	expect(notes.querySelector('pre code')?.textContent).toContain('count < 3');
	await expect
		.poll((): number => notes.querySelector<HTMLImageElement>('img')?.naturalWidth ?? 0)
		.toBe(1000);
	const tree = requiredElement(fixture, 'ex-tree');
	await expect.poll((): number => tree.querySelectorAll('.ex-tree-go').length).toBe(3);
	expect(tree.querySelector('.ex-tree-label a')).toBeNull();
	await frame.getByRole('link', { name: 'Go to Registry node', exact: true }).click();
	await expect.poll((): string => fixture.location.hash).toBe('#after--Registry');
	await expect
		.poll((): number => requiredElement(fixture, '#after--Registry').getBoundingClientRect().width)
		.toBeGreaterThan(0);
	expect(requiredElement(fixture, '#terminal-layout pre').textContent).toBe(
		'┌─ Sessions ──────┐\n│ > review       │\n└────────────────┘',
	);
	const outline = requiredElement(fixture, '.ex-heading-outline');
	expect(outline.scrollHeight).toBeGreaterThan(outline.clientHeight);
	expect(fixture.defaultView?.getComputedStyle(outline).overflowY).toBe('auto');
});

test('cold Markdown fence node links reveal the lazy diagram before it renders', async (): Promise<void> => {
	const { document: fixture } = await openFixture(
		'gallery',
		'#requirements--diagram-1--Registry',
		'navigation.html',
	);
	await expect
		.poll(
			(): number =>
				fixture.querySelector('#requirements--diagram-1--Registry')?.getBoundingClientRect()
					.width ?? 0,
			{ timeout: 8000 },
		)
		.toBeGreaterThan(0);
	const target = requiredElement(fixture, '#requirements--diagram-1--Registry');
	expect(target.getAttribute('data-id')).toBe('Registry');
	const bounds = target.getBoundingClientRect();
	expect(bounds.top).toBeGreaterThanOrEqual(0);
	expect(bounds.bottom).toBeLessThanOrEqual(fixture.defaultView?.innerHeight ?? 0);
});
