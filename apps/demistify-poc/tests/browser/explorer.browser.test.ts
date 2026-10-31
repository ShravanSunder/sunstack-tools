import { afterEach, expect, test } from 'vitest';
import { page, userEvent } from 'vitest/browser';

import { fixtureLocator, insideViewport, openFixture, requiredElement } from './fixture-page.ts';

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

test('narrow Explorer starts with nodes, preserves filtered selection on Back, and restores desktop graph', async (): Promise<void> => {
	await page.viewport(390, 900);
	const { document: fixture, frame } = await openFixture('gallery', '', 'explorer.html');
	const owner = requiredElement(fixture, '#planning');
	await expect.poll((): boolean => owner.hasAttribute('data-ready')).toBe(true);
	const browser = frame.getByRole('navigation', { name: 'Explore graph nodes', exact: true });
	await expect.element(browser).toBeVisible();
	const title = requiredElement(owner, '.ex-explorer-main > header h1').textContent;
	if (!title) throw new Error('Missing Explorer title');
	await expect
		.element(browser.getByRole('heading', { name: title, level: 1, exact: true }))
		.toBeVisible();
	expect(owner.querySelectorAll('h1')).toHaveLength(1);
	expect(
		fixture.defaultView?.getComputedStyle(requiredElement(owner, '.ex-explorer-main')).display,
	).toBe('none');
	const search = browser.getByRole('searchbox', { name: 'Find a node', exact: true });
	await search.fill('program-design');
	const node = browser.getByRole('button', { name: 'program-design', exact: true });
	await expect.element(node).toBeVisible();
	expect(requiredElement(owner, '.ex-explorer-node-caret').getAttribute('aria-hidden')).toBe(
		'true',
	);
	await node.click();
	await expect
		.element(frame.getByRole('button', { name: 'Back to nodes', exact: true }))
		.toHaveFocus();
	await expect
		.element(frame.getByRole('heading', { name: 'program-design', exact: true }))
		.toBeVisible();
	expect(
		fixture.defaultView?.getComputedStyle(requiredElement(owner, '.ex-explorer-browser')).display,
	).toBe('none');
	await frame.getByRole('button', { name: 'Back to nodes', exact: true }).click();
	await expect.element(search).toHaveValue('program-design');
	await expect.element(node).toHaveFocus();
	// Reenter detail: a graph sized while hidden must have real visible node boxes.
	await node.click();
	const selectedNode = (): Element => requiredElement(owner, '[data-node-id="program-design"]');
	await expect.poll((): number => selectedNode().getBoundingClientRect().height).toBeGreaterThan(0);
	expect(owner.getAttribute('data-mobile-view')).toBe('detail');
	await fixtureLocator(frame, selectedNode()).click();
	expect(owner.getAttribute('data-mobile-view')).toBe('detail');
	// A graph selection outside the active filter has no visible selected list row.
	const otherNode = requiredElement(
		owner,
		'.ex-explorer-node:not([data-node-id="program-design"])',
	);
	await fixtureLocator(frame, otherNode).click();
	expect(owner.getAttribute('data-mobile-view')).toBe('detail');
	await frame.getByRole('button', { name: 'Back to nodes', exact: true }).click();
	await expect.element(search).toHaveValue('program-design');
	await expect.element(search).toHaveFocus();
	expect(owner.querySelector('.ex-explorer-node-list [aria-current="true"]')).toBeNull();
	await node.click();
	await expect
		.element(frame.getByRole('button', { name: 'Back to nodes', exact: true }))
		.toHaveFocus();
	const narrowCanvas = requiredElement(owner, '.ex-explorer-canvas');
	const viewport = requiredElement(owner, '.ex-explorer-viewport');
	// At this wide width the viewport exceeds the renderer's minimum column canvas.
	await page.viewport(2300, 1000);
	await expect
		.poll((): boolean => requiredElement(owner, '.ex-explorer-canvas') !== narrowCanvas)
		.toBe(true);
	await expect
		.poll((): number =>
			Math.abs(
				requiredElement(owner, '.ex-explorer-canvas').getBoundingClientRect().width -
					viewport.clientWidth,
			),
		)
		.toBeLessThan(2);
	await expect.element(browser).toBeVisible();
	expect(owner.querySelectorAll('h1.ex-explorer-list-title')).toHaveLength(0);
});

test.each(['program-design', '__issues'])(
	'a cold narrow Explorer selection %s opens detail',
	async (selection): Promise<void> => {
		await page.viewport(390, 900);
		const { document: fixture, frame } = await openFixture(
			'gallery',
			'',
			`explorer.html?ex.planning=${selection}`,
		);
		const owner = requiredElement(fixture, '#planning');
		await expect.poll((): boolean => owner.hasAttribute('data-ready')).toBe(true);
		// Model an ordinary long document before a cold-linked Explorer.
		const introduction = fixture.createElement('div');
		introduction.style.height = '1600px';
		owner.before(introduction);
		owner.scrollIntoView({ block: 'start' });
		expect(owner.getAttribute('data-mobile-view')).toBe('detail');
		await expect
			.element(frame.getByRole('button', { name: 'Back to nodes', exact: true }))
			.toBeVisible();
		expect(
			fixture.defaultView?.getComputedStyle(requiredElement(owner, '.ex-explorer-browser')).display,
		).toBe('none');
		await frame.getByRole('button', { name: 'Back to nodes', exact: true }).click();
		expect(owner.getAttribute('data-mobile-view')).toBe('list');
		const search = requiredElement(owner, '.ex-explorer-browser input');
		await expect.poll((): number => search.getBoundingClientRect().top).toBeGreaterThanOrEqual(0);
		await expect.poll((): number => search.getBoundingClientRect().bottom).toBeLessThanOrEqual(900);
	},
);

test('a narrow Explorer main heading link reveals detail through document target resolution', async (): Promise<void> => {
	await page.viewport(390, 900);
	const { document: fixture, frame } = await openFixture('gallery', '', 'explorer.html');
	const owner = requiredElement(fixture, '#planning');
	await expect.poll((): boolean => owner.hasAttribute('data-ready')).toBe(true);
	const heading = requiredElement(owner, '.ex-explorer-main > header h1');
	expect(heading.id).toBe('heading-how-the-job-is-composed');
	expect(owner.getAttribute('data-mobile-view')).toBe('list');
	const link = fixture.createElement('a');
	link.href = `#${heading.id}`;
	link.textContent = 'Open Explorer heading';
	requiredElement(owner, '.ex-explorer-browser-header').append(link);
	const introduction = fixture.createElement('div');
	introduction.style.height = '1600px';
	owner.before(introduction);
	const view = fixture.defaultView;
	if (!view) throw new Error('Missing fixture window');
	view.scrollTo({ top: owner.getBoundingClientRect().top + view.scrollY + 100 });
	const list = requiredElement(owner, '.ex-explorer-node-list');
	// Narrow lists use the page scroller rather than an inner overflow box.
	const listPosition = list.scrollTop;
	const pagePosition = view.scrollY;
	expect(pagePosition).toBeGreaterThan(1000);
	expect(insideViewport(link)).toBe(true);
	await frame.getByRole('link', { name: 'Open Explorer heading', exact: true }).click();
	await expect.poll((): number => heading.getBoundingClientRect().width).toBeGreaterThan(0);
	expect(owner.getAttribute('data-mobile-view')).toBe('detail');
	expect(new URL(fixture.URL).searchParams.has('ex.planning')).toBe(false);
	const repeatLink = fixture.createElement('a');
	repeatLink.href = `#${heading.id}`;
	repeatLink.textContent = 'Reveal the same detail again';
	heading.after(repeatLink);
	await expect.poll((): number => Math.abs(view.scrollY - pagePosition)).toBeGreaterThan(2);
	await frame.getByRole('link', { name: 'Reveal the same detail again', exact: true }).click();
	await frame.getByRole('button', { name: 'Back to nodes', exact: true }).click();
	await expect
		.element(frame.getByRole('navigation', { name: 'Explore graph nodes', exact: true }))
		.toBeVisible();
	await expect.poll((): number => list.scrollTop).toBe(listPosition);
	await expect.poll((): number => Math.abs(view.scrollY - pagePosition)).toBeLessThan(2);
});

test('a cold narrow Explorer main heading fragment waits for its controller and returns a visible list', async (): Promise<void> => {
	await page.viewport(390, 900);
	const { document: fixture, frame } = await openFixture(
		'gallery',
		'#heading-how-the-job-is-composed',
		'explorer.html',
	);
	const owner = requiredElement(fixture, '#planning');
	await expect.poll((): boolean => owner.hasAttribute('data-ready')).toBe(true);
	const heading = requiredElement(owner, '.ex-explorer-main > header h1');
	await expect.poll((): number => heading.getBoundingClientRect().width).toBeGreaterThan(0);
	expect(owner.getAttribute('data-mobile-view')).toBe('detail');
	expect(owner.getAttribute('aria-busy')).toBeNull();
	await frame.getByRole('button', { name: 'Back to nodes', exact: true }).click();
	const search = requiredElement(owner, '.ex-explorer-browser input');
	await expect.poll((): number => search.getBoundingClientRect().top).toBeGreaterThanOrEqual(0);
	await expect.poll((): number => search.getBoundingClientRect().bottom).toBeLessThanOrEqual(900);
});

test('a cold authored Explorer intro fragment stays revealed after lazy mounting', async (): Promise<void> => {
	await page.viewport(390, 900);
	const { frame } = await openFixture('gallery', '', 'explorer.html');
	const iframe = document.querySelector<HTMLIFrameElement>('iframe[data-testid="explainer"]');
	if (!iframe) throw new Error('Missing fixture frame');
	// The real built consumer carries this authored target before the lazy controller loads.
	iframe.src = '/__demistify/content-surfaces/index.html#explorer-reading-guide';
	await expect
		.poll((): boolean => !!iframe.contentDocument?.querySelector('#team-explorer[data-ready]'))
		.toBe(true);
	const fixture = iframe.contentDocument;
	if (!fixture) throw new Error('Missing same-origin built demo');
	const owner = requiredElement(fixture, '#team-explorer');
	const target = requiredElement(owner, '#explorer-reading-guide');
	await expect.poll((): number => target.getBoundingClientRect().width).toBeGreaterThan(0);
	expect(owner.getAttribute('data-mobile-view')).toBe('detail');
	expect(new URL(fixture.URL).searchParams.has('ex.team-explorer')).toBe(false);
	await frame.getByRole('button', { name: 'Back to nodes', exact: true }).click();
	const search = requiredElement(owner, '.ex-explorer-browser input');
	await expect.poll((): number => search.getBoundingClientRect().top).toBeGreaterThanOrEqual(0);
	await expect.poll((): number => search.getBoundingClientRect().bottom).toBeLessThanOrEqual(900);
});
