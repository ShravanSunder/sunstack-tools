import { afterEach, expect, test } from 'vitest';
import { userEvent } from 'vitest/browser';

import { fixtureLocator, insideViewport, openFixture, requiredElement } from './fixture-page.ts';

afterEach((): void => document.body.replaceChildren());

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
	await expect.element(dialog).toBeVisible();
	await dialog.getByRole('combobox').fill('wp');
	await expect
		.element(dialog.getByText('No matching destinations.', { exact: true }))
		.toBeVisible();
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
	await expect.poll((): boolean => !fixture.querySelector('dialog')?.open).toBe(true);
	expect(fixture.defaultView?.location.pathname).toBe('/__demistify/search/index.html');
});
