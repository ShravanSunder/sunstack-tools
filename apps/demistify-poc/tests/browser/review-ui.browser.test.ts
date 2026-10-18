import { afterEach, expect, test } from 'vitest';
import { page, userEvent } from 'vitest/browser';

import { fixtureLocator, openFixture, requiredElement } from './fixture-page.ts';

afterEach(async (): Promise<void> => {
	document.body.replaceChildren();
	await page.viewport(1600, 1000);
});

test('explicit destination IDs win over generated heading aliases', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('ui');
	const link = requiredElement(fixture, '#exact-link');
	expect(link.getAttribute('href')).toBe('#connections');
	await fixtureLocator(frame, link).click();
	await expect.poll((): string => fixture.location.hash).toBe('#connections');
	await expect
		.poll((): number => requiredElement(fixture, '#connections').getBoundingClientRect().top)
		.toBeLessThan(150);
});

test('explicit lower outline targets stay current at the natural scroll limit', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('ui');
	const outline = frame.getByRole('navigation', { name: 'Page navigation', exact: true });
	await outline.getByRole('link', { name: 'Lower last', exact: true }).click();
	await expect
		.poll(
			(): boolean =>
				Math.abs(
					(fixture.defaultView?.scrollY ?? 0) -
						(fixture.documentElement.scrollHeight - (fixture.defaultView?.innerHeight ?? 0)),
				) < 2,
		)
		.toBe(true);
	await expect
		.element(outline.getByRole('link', { name: 'Lower last', exact: true }))
		.toHaveAttribute('aria-current', 'location');
	await outline.getByRole('link', { name: 'Lower first', exact: true }).click();
	await expect
		.element(outline.getByRole('link', { name: 'Lower first', exact: true }))
		.toHaveAttribute('aria-current', 'location');
	await fixtureLocator(frame, requiredElement(fixture, '#lower-first')).click();
	await userEvent.keyboard('{Home}');
	await expect
		.poll(
			(): string => fixture.querySelector('.ex-heading-panel [aria-current]')?.textContent ?? '',
		)
		.toBe('Connections');
});

test('mixed tree rows align their labels and keep disclosure, meaning and navigation separate', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('ui');
	const tree = requiredElement(fixture, '#mixed-tree');
	await expect.poll((): number => tree.querySelectorAll('.ex-tree-label').length).toBe(5);
	const topLabels = [
		...tree.querySelectorAll('.ex-tree-body > ul > li > .ex-tree-node .ex-tree-label'),
	];
	expect(
		Math.max(...topLabels.map((label): number => label.getBoundingClientRect().left)) -
			Math.min(...topLabels.map((label): number => label.getBoundingClientRect().left)),
	).toBeLessThan(1);
	await expect
		.poll((): number => fixture.querySelectorAll('#plain-tree .ex-tree-label').length)
		.toBe(2);
	expect(fixture.querySelector('#plain-tree .ex-tone-icon')).toBeNull();
	expect(tree.querySelector('.ex-tree-node .ph-folder')).not.toBeNull();
	expect(tree.querySelector('li[tone=done] > .ex-tree-node .ph-check-circle')).not.toBeNull();
	expect(tree.querySelector('li:not([tone]) > .ex-tree-node .ph-file')).not.toBeNull();
	for (const row of tree.querySelectorAll('.ex-tree-node'))
		expect(row.querySelector('.ex-tone-icon')).not.toBeNull();
	const toggle = frame.getByRole('button', { name: 'Toggle Session ownership', exact: true });
	await toggle.click();
	await expect.element(toggle).toHaveAttribute('aria-expanded', 'false');
	expect(fixture.location.hash).toBe('');
	await toggle.click();
	await frame.getByRole('link', { name: 'Go to Session ownership', exact: true }).click();
	await expect.poll((): string => fixture.location.hash).toBe('#connections');
	await expect.element(toggle).toHaveAttribute('aria-expanded', 'true');
});

test('actual page movement releases an explicit section without wheel or keyboard events', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('ui');
	const fixtureWindow = fixture.defaultView;
	if (!fixtureWindow) throw new Error('Missing fixture window.');
	const outline = frame.getByRole('navigation', { name: 'Page navigation', exact: true });
	await outline.getByRole('link', { name: 'Lower last', exact: true }).click();
	await expect
		.poll(
			(): boolean =>
				Math.abs(
					fixtureWindow.scrollY -
						(fixture.documentElement.scrollHeight - fixtureWindow.innerHeight),
				) < 2,
		)
		.toBe(true);
	await expect
		.element(outline.getByRole('link', { name: 'Lower last', exact: true }))
		.toHaveAttribute('aria-current', 'location');
	fixtureWindow.scrollTo({ top: 0, behavior: 'instant' });
	await expect.poll((): number => fixtureWindow.scrollY).toBe(0);
	await expect
		.poll(
			(): string => fixture.querySelector('.ex-heading-panel [aria-current]')?.textContent ?? '',
		)
		.toBe('Connections');
});

test('tree go-to absolute URLs retain the lower destination at the page clamp', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('ui');
	const tree = fixtureLocator(frame, requiredElement(fixture, '#lower-links'));
	const destination = tree.getByRole('link', { name: 'Go to Lower last destination', exact: true });
	await expect.element(destination).toHaveAttribute('href', `${fixture.location.href}#lower-last`);
	await destination.click();
	await expect
		.poll(
			(): boolean =>
				Math.abs(
					(fixture.defaultView?.scrollY ?? 0) -
						(fixture.documentElement.scrollHeight - (fixture.defaultView?.innerHeight ?? 0)),
				) < 2,
		)
		.toBe(true);
	await expect
		.element(
			frame
				.getByRole('navigation', { name: 'Page navigation', exact: true })
				.getByRole('link', { name: 'Lower last', exact: true }),
		)
		.toHaveAttribute('aria-current', 'location');
});

test('a blocked downward page key retains the clamped selection until actual movement', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('ui');
	const fixtureWindow = fixture.defaultView;
	if (!fixtureWindow) throw new Error('Missing fixture window.');
	const lower = frame
		.getByRole('navigation', { name: 'Page navigation', exact: true })
		.getByRole('link', { name: 'Lower last', exact: true });
	await lower.click();
	await expect
		.poll(
			(): boolean =>
				Math.abs(
					fixtureWindow.scrollY -
						(fixture.documentElement.scrollHeight - fixtureWindow.innerHeight),
				) < 2,
		)
		.toBe(true);
	await fixtureLocator(frame, requiredElement(fixture, '#lower-last')).click();
	const clampedPosition = fixtureWindow.scrollY;
	await userEvent.keyboard('{ArrowDown}');
	// Give native input and queued visual updates time to paint even when no scroll event fires.
	await new Promise<void>((resolve): void => {
		fixtureWindow.requestAnimationFrame((): void => {
			fixtureWindow.requestAnimationFrame((): void => resolve());
		});
	});
	expect(fixtureWindow.scrollY).toBe(clampedPosition);
	await expect.element(lower).toHaveAttribute('aria-current', 'location');
	await userEvent.keyboard('{ArrowUp}');
	await expect.poll((): number => fixtureWindow.scrollY).toBeLessThan(clampedPosition);
	await expect.element(lower).not.toHaveAttribute('aria-current', 'location');
});

test('tabs carry meaningful icons, layer switches expose checked state and Home has its own icon', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('ui');
	const tab = frame.getByRole('tab', { name: 'Current', exact: true });
	await tab.click();
	await userEvent.keyboard('{ArrowRight}');
	await expect
		.element(frame.getByRole('tabpanel', { name: 'Proposed', exact: true }))
		.toBeVisible();
	expect(fixture.querySelectorAll('#models [role=tab] .ph')).toHaveLength(2);
	const toggle = frame.getByRole('switch', { name: 'planned', exact: true });
	await expect.element(toggle).toHaveAttribute('aria-checked', 'false');
	await toggle.click();
	await expect
		.element(frame.getByText('Recovery needs identity evidence.', { exact: true }))
		.toBeVisible();
	await frame.getByRole('button', { name: 'Browse pages', exact: true }).click();
	expect(fixture.querySelector('.ex-pages-panel a .ph-house')).not.toBeNull();
	const home = requiredElement(fixture, '.ex-pages-panel a');
	expect(home.getAttribute('aria-current')).toBe('page');
	expect(home.getAttribute('href')).toBe(new URL('index.html', fixture.location.href).href);
});

test('timeline and layer-band content share deliberate type metrics and aligned markers', async (): Promise<void> => {
	const { document: fixture } = await openFixture('ui');
	const fixtureWindow = fixture.defaultView;
	if (!fixtureWindow) throw new Error('Missing fixture window.');
	const styles = (selector: string): CSSStyleDeclaration =>
		fixtureWindow.getComputedStyle(requiredElement(fixture, selector));
	expect(styles('ex-timeline section small').fontSize).toBe('13px');
	expect(styles('ex-timeline section h3').fontSize).toBe('17px');
	expect(styles('ex-timeline section p').fontSize).toBe('15px');
	expect(styles('ex-timeline section p').fontFamily).toBe(
		styles('ex-timeline section h3').fontFamily,
	);
	expect(fixture.querySelectorAll('#status-timeline .ex-timeline-marker')).toHaveLength(2);
	const marker = requiredElement(fixture, '#plain-timeline .ex-timeline-marker');
	expect(marker.querySelector('.ex-tone-icon')).toBeNull();
	expect(marker.getBoundingClientRect().width).toBe(32);
	expect(marker.getBoundingClientRect().height).toBe(32);
	expect(styles('#plain-timeline .ex-timeline-marker').borderRadius).toBe('50%');
	expect(styles('ex-layer > strong').fontSize).toBe('15px');
	expect(styles('ex-layer > p').lineHeight).toBe('22.5px');
	expect(styles('ex-layer > strong').paddingTop).toBe(styles('ex-layer > p').paddingTop);
	const label = requiredElement(fixture, 'ex-layer > strong');
	const description = requiredElement(fixture, 'ex-layer > p');
	expect(styles('ex-layer > strong').fontSize).toBe(styles('ex-layer > p').fontSize);
	expect(styles('ex-layer > strong').lineHeight).toBe(styles('ex-layer > p').lineHeight);
	expect(
		Math.abs(
			label.getBoundingClientRect().top +
				label.getBoundingClientRect().height / 2 -
				description.getBoundingClientRect().top -
				description.getBoundingClientRect().height / 2,
		),
	).toBeLessThan(1);
});

test('dedicated gallery fills available space and its short grid scrolls without shrinking rows', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('ui', '', 'gallery.html');
	const gallery = requiredElement(fixture, 'ex-gallery');
	const local = fixtureLocator(frame, gallery);
	await expect
		.poll(
			(): number =>
				gallery.querySelector<HTMLImageElement>('.ex-gallery-preview img')?.naturalWidth ?? 0,
		)
		.toBe(1000);
	expect(
		Math.abs(gallery.getBoundingClientRect().bottom - (fixture.defaultView?.innerHeight ?? 0)),
	).toBeLessThan(50);
	expect(fixture.documentElement.scrollHeight).toBeLessThanOrEqual(1002);
	expect(
		requiredElement(gallery, '.ex-control-search').getBoundingClientRect().width,
	).toBeGreaterThan(400);
	expect(gallery.querySelector('select')).toBeNull();
	await local.getByRole('button', { name: 'Keyboard shortcuts', exact: true }).click();
	await expect.element(local.getByText('Up / Down', { exact: true })).toBeVisible();
	await userEvent.keyboard('{Escape}');
	await page.viewport(375, 600);
	expect(
		requiredElement(gallery, '.ex-gallery-preview').getBoundingClientRect().height,
	).toBeGreaterThanOrEqual(140);
	await local.getByRole('radio', { name: 'Grid view', exact: true }).click();
	const grid = requiredElement(gallery, '.ex-gallery-grid');
	await expect.poll((): boolean => grid.scrollHeight > grid.clientHeight).toBe(true);
	expect(requiredElement(grid, 'figure').getBoundingClientRect().height).toBeGreaterThan(190);
	expect(fixture.defaultView?.getComputedStyle(grid).overflowY).toBe('auto');
	expect(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(376);
});

test('embedded gallery keeps surrounding content in normal page flow', async (): Promise<void> => {
	const { document: fixture } = await openFixture('ui', '', 'embedded.html');
	expect(requiredElement(fixture, 'ex-document').classList.contains('ex-gallery-document')).toBe(
		false,
	);
	const footer = requiredElement(fixture, 'footer');
	footer.scrollIntoView();
	await expect
		.poll(
			(): boolean =>
				footer.getBoundingClientRect().bottom <= (fixture.defaultView?.innerHeight ?? 0) + 1,
		)
		.toBe(true);
});

test('a bare gallery also receives dedicated space without an author sizing flag', async (): Promise<void> => {
	const { document: fixture } = await openFixture('ui', '', 'bare-gallery.html');
	const gallery = requiredElement(fixture, 'ex-gallery');
	expect(requiredElement(fixture, 'ex-document').classList.contains('ex-gallery-document')).toBe(
		true,
	);
	expect(gallery.getBoundingClientRect().height).toBeGreaterThan(700);
	expect(fixture.documentElement.scrollHeight).toBeLessThanOrEqual(1002);
});

test('the actual formatted guide literal stays exact in the page and sidebar', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('ui', '', 'literal-formatted.html');
	const drawing = requiredElement(fixture, 'ex-pre');
	const expected = 'first line\n</script>\n<widget>& value\nlast line';
	await expect.poll((): string => drawing.querySelector('pre')?.textContent ?? '').toBe(expected);
	await userEvent.hover(fixtureLocator(frame, drawing));
	await fixtureLocator(frame, drawing)
		.getByRole('button', { name: 'Open Literal markup in a sidebar', exact: true })
		.click();
	await expect
		.element(frame.getByRole('complementary', { name: 'Literal markup', exact: true }))
		.toBeVisible();
	expect(requiredElement(fixture, '.ex-drawer pre').textContent).toBe(expected);
	expect(drawing.querySelector('pre')?.textContent).toBe(expected);
});

test('ordinary paragraphs and lead text fill their section unless reading measure is explicit', async (): Promise<void> => {
	await page.viewport(1600, 1000);
	const { document: fixture } = await openFixture('ui', '', 'widths.html');
	const canvas = requiredElement(fixture, '#width-canvas').getBoundingClientRect();
	for (const selector of ['#lead-proof', '#ordinary-proof', '#lede-proof']) {
		const paragraph = requiredElement(fixture, selector).getBoundingClientRect();
		expect(Math.abs(paragraph.width - canvas.width)).toBeLessThan(1);
	}
	expect(requiredElement(fixture, '#narrow-proof').getBoundingClientRect().width).toBeLessThan(
		canvas.width * 0.8,
	);
	await page.viewport(390, 900);
	await expect.poll((): number => fixture.documentElement.scrollWidth).toBeLessThanOrEqual(391);
});
