import { afterEach, expect, test } from 'vitest';

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
	expect(owner.querySelectorAll('.ex-explorer-browser details button')).toHaveLength(49);
	await frame.getByRole('button', { name: 'Current', exact: true }).click();
	expect(owner.querySelectorAll('.ex-explorer-proposal-dot')).toHaveLength(0);
	await frame.getByRole('button', { name: 'With proposals', exact: true }).click();
	expect(owner.querySelectorAll('.ex-explorer-proposal-dot').length).toBeGreaterThan(0);
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
