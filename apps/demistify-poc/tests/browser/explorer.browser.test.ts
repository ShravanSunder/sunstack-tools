import { afterEach, expect, test } from 'vitest';
import { page, userEvent } from 'vitest/browser';

import { fixtureLocator, openFixture, requiredElement } from './fixture-page.ts';

afterEach((): void => {
	document.body.replaceChildren();
});

test('full frozen explorer keeps search, selection/history, graph/table correspondence and one document theme', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('gallery', '', 'explorer.html');
	const owner = requiredElement(fixture, '#planning');
	await expect.poll((): boolean => owner.hasAttribute('data-ready')).toBe(true);
	expect(fixture.querySelectorAll('button[data-ex-theme]')).toHaveLength(1);
	const browser = frame.getByRole('navigation', { name: 'Explore graph nodes', exact: true });
	const search = browser.getByRole('searchbox', { name: 'Find a node', exact: true });
	await search.fill('program-design');
	await browser.getByRole('button', { name: 'program-design', exact: true }).click();
	await expect
		.element(frame.getByRole('heading', { name: 'program-design', exact: true }))
		.toBeVisible();
	await frame.getByRole('button', { name: 'Back to previous selection', exact: true }).click();
	await expect
		.element(frame.getByRole('heading', { name: 'spec-design', exact: true }))
		.toBeVisible();
	await search.fill('');
	await expect
		.poll((): number => owner.querySelectorAll('.ex-explorer-browser details button').length)
		.toBe(49);
	await frame.getByRole('button', { name: 'Current', exact: true }).click();
	expect(owner.querySelectorAll('.ex-explorer-proposed')).toHaveLength(0);
	await frame.getByRole('button', { name: 'With proposals', exact: true }).click();
	expect(owner.querySelectorAll('.ex-explorer-proposed').length).toBeGreaterThan(0);
	const node = requiredElement(owner, '[data-node-id="spec-design"]');
	await fixtureLocator(frame, node).hover();
	await expect
		.poll((): number => owner.querySelectorAll('.ex-explorer-edge.is-traced').length)
		.toBeGreaterThan(0);
	const edge = requiredElement(owner, 'g[data-edge-id="current-213"] .ex-explorer-edge-hit');
	await fixtureLocator(frame, edge).click();
	await expect
		.poll((): boolean => !!owner.querySelector('tr[data-edge-id="current-213"].is-selected'))
		.toBe(true);
	await frame.getByRole('tab', { name: 'Used by', exact: true }).click();
	expect(owner.querySelectorAll('.ex-explorer-panel tbody tr').length).toBeGreaterThan(0);
	const assertEvidenceTableRows = async (tab: string, rows: number): Promise<void> => {
		await frame.getByRole('tab', { name: tab, exact: true }).click();
		expect(owner.querySelectorAll('.ex-explorer-panel tbody tr')).toHaveLength(rows);
	};
	await assertEvidenceTableRows('Authored changes', 18);
	await assertEvidenceTableRows('Earlier proposals', 6);
	await assertEvidenceTableRows('Source inventory', 173);
	await frame.getByRole('button', { name: 'Sources & details', exact: true }).click();
	await expect
		.poll((): number => owner.querySelectorAll('.ex-explorer-sources a').length)
		.toBeGreaterThan(0);
});

test('explorer keeps modest arrowheads, soft routes and a fixed search header above its scrolling list', async (): Promise<void> => {
	await page.viewport(1600, 1000);
	const { document: fixture, frame } = await openFixture('gallery', '', 'explorer.html');
	const owner = requiredElement(fixture, '#planning');
	await expect.poll((): boolean => owner.hasAttribute('data-ready')).toBe(true);
	const marker = requiredElement(owner, 'marker');
	expect(marker.getAttribute('markerUnits')).toBe('userSpaceOnUse');
	expect(Number(marker.getAttribute('markerWidth'))).toBeLessThanOrEqual(8);
	expect(
		[...owner.querySelectorAll('.ex-explorer-line')].some(
			(path): boolean => path.getAttribute('d')?.includes('Q') ?? false,
		),
	).toBe(true);
	const line = requiredElement(owner, '.ex-explorer-line');
	const normalWidth = fixture.defaultView?.getComputedStyle(line).strokeWidth;
	await fixtureLocator(frame, requiredElement(owner, '[data-node-id="spec-design"]')).hover();
	await expect.poll((): number => owner.querySelectorAll('.is-traced').length).toBeGreaterThan(0);
	expect(
		fixture.defaultView?.getComputedStyle(requiredElement(owner, '.is-traced .ex-explorer-line'))
			.strokeWidth,
	).not.toBe(normalWidth);
	expect(marker.getAttribute('markerUnits')).toBe('userSpaceOnUse');
	expect(owner.querySelector('.ex-explorer-proposal-dot')).toBeNull();
	await expect.element(frame.getByText('Proposed connection', { exact: true })).toBeVisible();
	const list = requiredElement(owner, '.ex-explorer-node-list');
	expect(list.scrollHeight).toBeGreaterThan(list.clientHeight);
	const search = requiredElement(owner, '.ex-control-search');
	const searchTop = search.getBoundingClientRect().top;
	list.scrollTop = list.scrollHeight;
	expect(search.getBoundingClientRect().top).toBe(searchTop);
	let lastInput = 0;
	let settledAfter = 0;
	owner.addEventListener(
		'input',
		(): void => {
			lastInput = fixture.defaultView?.performance.now() ?? 0;
		},
		{ capture: true },
	);
	const observer = new MutationObserver((): void => {
		settledAfter = (fixture.defaultView?.performance.now() ?? 0) - lastInput;
	});
	observer.observe(list, { childList: true });
	await frame.getByRole('searchbox', { name: 'Find a node', exact: true }).fill('program-design');
	await expect.poll((): number => list.querySelectorAll('details button').length).toBe(1);
	observer.disconnect();
	expect(settledAfter).toBeGreaterThanOrEqual(199);
	await frame.getByRole('button', { name: 'Current', exact: true }).click();
	await expect
		.element(frame.getByRole('button', { name: 'Current', exact: true }))
		.toHaveAttribute('aria-pressed', 'true');
	expect(
		requiredElement(owner, '.ex-explorer-browser-header').getBoundingClientRect().top,
	).toBeGreaterThanOrEqual(
		requiredElement(fixture, '.ex-document-toolbar').getBoundingClientRect().bottom,
	);
	await frame.getByRole('tab', { name: 'Used by', exact: true }).click();
	await userEvent.keyboard('{Home}');
	await expect
		.element(frame.getByRole('tab', { name: /^Connections/ }))
		.toHaveAttribute('aria-selected', 'true');
});
