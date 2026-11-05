import { afterEach, expect, test } from 'vitest';
import { page, userEvent } from 'vitest/browser';

import { fixtureLocator, insideViewport, openFixture, requiredElement } from './fixture-page.ts';

afterEach((): void => document.body.replaceChildren());

for (const width of [1600, 390]) {
	test(`collection walkthroughs stay available without local paths at ${width}px`, async (): Promise<void> => {
		await page.viewport(width, 900);
		const { document: fixture, frame } = await openFixture('search', '', 'reading.html');
		expect(fixture.querySelector('ex-presentation')).toBeNull();
		const launcher = frame.getByRole('button', { name: 'Browse walkthroughs', exact: true });
		await expect.element(launcher).toBeVisible();
		await launcher.click();
		const dialog = frame.getByRole('dialog', { name: 'Search documents', exact: true });
		await expect.element(dialog.getByRole('option', { name: /Review queue/ })).toBeVisible();
		await expect.element(dialog.getByRole('option', { name: /Review captures/ })).toBeVisible();
		await userEvent.keyboard('{Escape}');
		await expect
			.poll((): string | null | undefined => fixture.activeElement?.getAttribute('aria-label'))
			.toBe('Browse walkthroughs');
		await userEvent.keyboard('w');
		await expect.element(dialog).toBeVisible();
		await dialog.getByRole('option', { name: /Review queue/ }).click();
		await expect
			.element(frame.getByRole('region', { name: 'Walkthrough player', exact: true }))
			.toBeVisible();
		const destination = document.querySelector('iframe')?.contentDocument;
		expect(destination?.location.pathname).toBe('/__demistify/search/index.html');
		expect(destination?.querySelector('.ex-route-focus')?.id).toBe('heading-ownership');
		await frame.getByRole('button', { name: 'Close walkthrough', exact: true }).click();
		await frame.getByRole('button', { name: 'Browse pages', exact: true }).click();
		await expect
			.element(frame.getByRole('navigation', { name: 'Pages', exact: true }))
			.toBeVisible();
	});
}

test('a page without local or indexed paths has no empty walkthrough launcher', async (): Promise<void> => {
	const { document: fixture } = await openFixture('review', '', 'choice-only.html');
	expect(fixture.querySelector('ex-presentation')).toBeNull();
	expect(fixture.querySelector('[data-ex-search-index]')).toBeNull();
	expect(fixture.querySelector('.ex-presentations-trigger')).toBeNull();
});

test('command search shows heading ancestry and opens the existing Markdown destination', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('navigation');
	await expect.element(frame.getByRole('heading', { name: 'Recovery', exact: true })).toBeVisible();
	await frame.getByRole('button', { name: 'Browse pages', exact: true }).click();
	await userEvent.keyboard('{Escape}{Control>}k{/Control}');
	const dialog = frame.getByRole('dialog', {
		name: 'Search documents',
		exact: true,
		includeHidden: true,
	});
	await expect.element(dialog).toBeVisible();
	const search = dialog.getByRole('combobox');
	await search.fill('recovery');
	const result = dialog.getByRole('option', { name: /Recovery/ });
	await expect.element(result).toBeVisible();
	expect(requiredElement(fixture, '.ex-command-context').textContent).toContain(
		'Session ownership',
	);
	await userEvent.keyboard('{Enter}');
	await expect.element(dialog).not.toBeVisible();
	await expect
		.poll((): boolean => insideViewport(requiredElement(fixture, '#notes--recovery')))
		.toBe(true);
	expect(fixture.defaultView?.location.hash).toBe('#notes--recovery');
});

test('scoped shortcuts select pages and walkthroughs while ordinary typing keeps ownership', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('navigation');
	await frame.getByRole('button', { name: 'Browse pages', exact: true }).click();
	await userEvent.keyboard('{Escape}w');
	const dialog = frame.getByRole('dialog', {
		name: 'Search documents',
		exact: true,
		includeHidden: true,
	});
	await expect.element(dialog.getByRole('option', { name: /Implementation plan/ })).toBeVisible();
	await userEvent.keyboard('2');
	await expect
		.poll((): boolean => requiredElement(fixture, '#plan').classList.contains('ex-route-focus'))
		.toBe(true);
	await expect.element(dialog).not.toBeVisible();
	await frame.getByRole('button', { name: 'Close walkthrough', exact: true }).click();
	await userEvent.keyboard('p');
	await expect.element(dialog).not.toBeVisible();
	await expect.element(frame.getByRole('navigation', { name: 'Pages', exact: true })).toBeVisible();
	await userEvent.keyboard('{ArrowDown}');
	expect(fixture.activeElement?.textContent).toContain('Implementation detail');
	await userEvent.keyboard('{Escape}');
	expect(fixture.activeElement?.getAttribute('aria-label')).toBe('Browse pages');
});

test('generated collection search distinguishes duplicate headings and reveals a hidden cross-page Markdown target', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('search');
	await frame.getByRole('button', { name: 'Browse pages', exact: true }).click();
	await userEvent.keyboard('{Escape}{Meta>}k{/Meta}');
	const dialog = frame.getByRole('dialog', { name: 'Search documents', exact: true });
	const input = dialog.getByRole('combobox');
	await input.fill('summary');
	expect(fixture.querySelectorAll('[role="option"]')).toHaveLength(2);
	expect(
		[...fixture.querySelectorAll('.ex-command-context')].map(
			(element): string | null => element.textContent,
		),
	).toEqual(['Queue design › Ownership', 'Queue design › Retries']);
	await userEvent.keyboard('{ArrowDown}{Enter}');
	expect(fixture.defaultView?.location.hash).toBe('#heading-summary-2');
	await userEvent.keyboard('{Control>}k{/Control}');
	await input.fill('durable receipt');
	await userEvent.keyboard('{Enter}');
	await expect
		.element(frame.getByRole('tab', { name: 'Planned', exact: true }))
		.toHaveAttribute('aria-selected', 'true');
	await expect
		.element(frame.getByRole('heading', { name: 'Durable receipt', exact: true }))
		.toBeVisible();
	const iframe = document.querySelector<HTMLIFrameElement>('iframe');
	const destination = iframe?.contentDocument;
	if (!destination) throw new Error('Destination page did not load.');
	expect(destination.location.pathname).toBe('/__demistify/search/evidence.html');
	expect(destination.location.hash).toBe('#planned-proof--durable-receipt');
	expect(destination.querySelector('details')?.open).toBe(true);
});

test('typing and native editing retain keys; Option hints and Escape restore the document controls', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('search');
	await frame.getByRole('textbox', { name: 'Reader notes', exact: true }).click();
	await userEvent.keyboard('pw');
	await expect
		.element(frame.getByRole('textbox', { name: 'Reader notes', exact: true }))
		.toHaveValue('pw');
	expect(fixture.querySelector('dialog')?.open).toBe(false);
	const editable = requiredElement(fixture, '[contenteditable]');
	await fixtureLocator(frame, editable).click();
	await userEvent.keyboard('wp');
	expect(editable.textContent).toContain('wp');
	expect(fixture.querySelector('dialog')?.open).toBe(false);
	await frame.getByRole('button', { name: 'Browse pages', exact: true }).click();
	await userEvent.keyboard('{Escape}{Alt>}');
	try {
		expect(fixture.documentElement.hasAttribute('data-shortcut-hints')).toBe(true);
		expect(
			fixture.defaultView?.getComputedStyle(requiredElement(fixture, '.ex-pages-trigger kbd'))
				.display,
		).not.toBe('none');
	} finally {
		await userEvent.keyboard('{/Alt}');
	}
	expect(fixture.documentElement.hasAttribute('data-shortcut-hints')).toBe(false);
	await userEvent.keyboard('p1');
	await expect
		.poll(
			(): string | undefined =>
				document.querySelector('iframe')?.contentDocument?.location.pathname,
		)
		.toBe('/__demistify/search/index.html');
});

test('query-bearing documents keep live headings searchable and open them without reloading', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('search', '?review=planned');
	await frame.getByRole('button', { name: 'Browse pages', exact: true }).click();
	await userEvent.keyboard('{Escape}{Control>}k{/Control}');
	const dialog = frame.getByRole('dialog', { name: 'Search documents', exact: true });
	await dialog.getByRole('combobox').fill('retries summary');
	await expect
		.element(dialog.getByRole('option', { name: 'Queue design › Retries › Summary', exact: true }))
		.toBeVisible();
	await userEvent.keyboard('{Enter}');
	await expect
		.poll((): string | undefined => fixture.defaultView?.location.hash)
		.toBe('#heading-summary-2');
	expect(fixture.defaultView?.location.search).toBe('?review=planned');
	expect(document.querySelector('iframe')?.contentDocument).toBe(fixture);
});

test('search launcher sits beside the bottom theme control at 1600px', async (): Promise<void> => {
	await page.viewport(1600, 900);
	const { document: fixture, frame } = await openFixture('navigation');
	const launcher = frame.getByRole('button', { name: 'Search pages and headings', exact: true });
	await expect.element(launcher).toBeVisible();
	const searchBounds = requiredElement(fixture, '.ex-search-trigger').getBoundingClientRect();
	const themeBounds = requiredElement(fixture, '.ex-theme-toggle').getBoundingClientRect();
	expect(searchBounds.left).toBeGreaterThan(themeBounds.right);
	expect(searchBounds.left - themeBounds.right).toBeLessThan(20);
	expect(Math.abs(searchBounds.bottom - themeBounds.bottom)).toBeLessThan(2);
	expect(searchBounds.bottom).toBeGreaterThan(850);
	expect(insideViewport(requiredElement(fixture, '.ex-search-trigger'))).toBe(true);
	await expect.element(launcher.getByText(/⌘K|Ctrl-K/)).toBeVisible();
	await launcher.click();
	const dialog = frame.getByRole('dialog', {
		name: 'Search documents',
		exact: true,
		includeHidden: true,
	});
	await expect.element(dialog).toBeVisible();
	await userEvent.keyboard('{Escape}');
	expect(fixture.activeElement?.getAttribute('aria-label')).toBe('Search pages and headings');
});

test('search and theme controls join the top toolbar at 390px', async (): Promise<void> => {
	await page.viewport(390, 900);
	const { document: fixture, frame } = await openFixture('navigation');
	const launcher = frame.getByRole('button', { name: 'Search pages and headings', exact: true });
	await expect.element(launcher).toBeVisible();
	const toolbar = requiredElement(fixture, '.ex-document-toolbar').getBoundingClientRect();
	const searchBounds = requiredElement(fixture, '.ex-search-trigger').getBoundingClientRect();
	const themeBounds = requiredElement(fixture, '.ex-theme-toggle').getBoundingClientRect();
	expect(searchBounds.top).toBeGreaterThanOrEqual(toolbar.top);
	expect(searchBounds.bottom).toBeLessThanOrEqual(toolbar.bottom);
	expect(Math.abs(searchBounds.top - themeBounds.top)).toBeLessThan(2);
	expect(themeBounds.right).toBeLessThan(searchBounds.left);
	expect(searchBounds.right).toBeLessThanOrEqual(390);
	expect(insideViewport(requiredElement(fixture, '.ex-search-trigger'))).toBe(true);
	await launcher.click();
	const dialog = frame.getByRole('dialog', {
		name: 'Search documents',
		exact: true,
		includeHidden: true,
	});
	await expect.element(dialog).toBeVisible();
	await userEvent.keyboard('{Escape}');
	expect(fixture.activeElement?.getAttribute('aria-label')).toBe('Search pages and headings');
});

test('Pages is a native link dropdown, separate from search, with keyboard selection', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('navigation');
	await frame.getByRole('button', { name: 'Browse pages', exact: true }).click();
	const pages = frame.getByRole('navigation', { name: 'Pages', exact: true, includeHidden: true });
	await expect.element(pages).toBeVisible();
	expect(fixture.querySelector('dialog')?.open).toBe(false);
	await expect.element(pages.getByRole('link', { name: 'Home', exact: true })).toBeVisible();
	await expect
		.element(pages.getByRole('link', { name: 'Implementation detail', exact: true }))
		.toHaveAttribute('href', 'details.html');
	await userEvent.keyboard('{Escape}p{End}');
	expect(fixture.activeElement?.textContent).toContain('Implementation detail');
	await userEvent.keyboard('{Home}');
	expect(fixture.activeElement?.textContent).toContain('Home');
	await userEvent.keyboard('2');
	await expect
		.poll(
			(): string | undefined =>
				document.querySelector('iframe')?.contentDocument?.location.pathname,
		)
		.toBe('/__demistify/navigation/details.html');
});

test('walkthrough results retain page context, explanations and distinct path or image icons', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('search');
	await frame.getByRole('button', { name: 'Browse walkthroughs', exact: true }).click();
	const dialog = frame.getByRole('dialog', { name: 'Search documents', exact: true });
	const path = dialog.getByRole('option', { name: /Review queue/ });
	await expect.element(path.getByText('Queue design', { exact: true })).toBeVisible();
	await expect
		.element(path.getByText('Follow ownership into its planned proof.', { exact: true }))
		.toBeVisible();
	expect(requiredElement(fixture, 'a[href$="#queue-review"] .ph-path')).toBeTruthy();
	const journey = dialog.getByRole('option', { name: /Review captures/ });
	await expect.element(journey.getByText('Evidence', { exact: true })).toBeVisible();
	await expect
		.element(journey.getByText('Inspect the archived wide and narrow views.', { exact: true }))
		.toBeVisible();
	expect(requiredElement(fixture, 'a[href$="#capture-review"] .ph-images')).toBeTruthy();
});
