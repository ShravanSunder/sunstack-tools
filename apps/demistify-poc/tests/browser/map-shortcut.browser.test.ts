import { afterEach, expect, test } from 'vitest';
import { page, userEvent } from 'vitest/browser';

import { fixtureLocator, openFixture, requiredElement } from './fixture-page.ts';

afterEach(async (): Promise<void> => {
	document.body.replaceChildren();
	await page.viewport(1600, 1000);
});

test.each([1280, 390])(
	'M toggles the same Map and replaces Comments in both directions at %ipx',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture('show-me-work');
		await fixtureLocator(frame, requiredElement(fixture, 'h1')).click();
		await userEvent.keyboard('m');
		await expect
			.poll((): string | null =>
				requiredElement(fixture, '.ex-map-trigger').getAttribute('aria-expanded'),
			)
			.toBe('true');
		const trigger = requiredElement(fixture, '.ex-map-trigger');
		expect(trigger.getAttribute('aria-keyshortcuts')).toBe('M');
		expect(trigger.getAttribute('title')).toContain('(M)');
		await userEvent.keyboard('m');
		expect(trigger.getAttribute('aria-expanded')).toBe('false');
		await userEvent.keyboard('c');
		await expect
			.element(frame.getByRole('button', { name: 'Close comments', exact: true }))
			.toBeVisible();
		if (width < 900) expect(trigger.closest('dialog:modal > header')).not.toBeNull();
		await userEvent.keyboard('M');
		expect(trigger.getAttribute('aria-expanded')).toBe('true');
		expect(fixture.querySelector('dialog:modal')).toBeNull();
		await userEvent.keyboard('c');
		expect(trigger.getAttribute('aria-expanded')).toBe('false');
		await expect
			.element(frame.getByRole('button', { name: 'Close comments', exact: true }))
			.toBeVisible();
		await userEvent.keyboard('m');
		expect(requiredElement(fixture, '.ex-map-trigger')).toBe(trigger);
		expect(trigger.getAttribute('aria-expanded')).toBe('true');
		await userEvent.keyboard('{Escape}');
		expect(trigger.getAttribute('aria-expanded')).toBe('false');
	},
);

test('M preserves editable input and rejects protected key events without a second listener', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('show-me-work');
	const trigger = requiredElement(fixture, '.ex-map-trigger');
	const input = fixture.createElement('input');
	input.setAttribute('aria-label', 'Shortcut typing control');
	requiredElement(fixture, 'h1').after(input);
	await fixtureLocator(frame, input).click();
	await userEvent.keyboard('m');
	expect(input.value).toBe('m');
	expect(trigger.getAttribute('aria-expanded')).toBe('false');
	const editable = fixture.createElement('div');
	editable.contentEditable = 'true';
	editable.textContent = 'Edit here';
	input.after(editable);
	await fixtureLocator(frame, editable).click();
	await userEvent.keyboard('m');
	expect(editable.textContent).toContain('m');
	expect(trigger.getAttribute('aria-expanded')).toBe('false');
	const view = fixture.defaultView;
	if (!view) throw new Error('Missing fixture window.');
	// Integration policy checks for flags whose physical delivery is outside this harness.
	for (const flags of [
		{ repeat: true },
		{ isComposing: true },
		{ metaKey: true },
		{ ctrlKey: true },
		{ altKey: true },
	]) {
		const event = new view.KeyboardEvent('keydown', {
			key: 'm',
			bubbles: true,
			cancelable: true,
			...flags,
		});
		fixture.body.dispatchEvent(event);
		expect(event.defaultPrevented).toBe(false);
		expect(trigger.getAttribute('aria-expanded')).toBe('false');
	}
	const prevented = new view.KeyboardEvent('keydown', {
		key: 'm',
		bubbles: true,
		cancelable: true,
	});
	prevented.preventDefault();
	fixture.body.dispatchEvent(prevented);
	expect(trigger.getAttribute('aria-expanded')).toBe('false');
	const owner = requiredElement(fixture, 'ex-map'),
		root = requiredElement(fixture, 'ex-document');
	owner.remove();
	await fixtureLocator(frame, requiredElement(fixture, 'h1')).click();
	await userEvent.keyboard('m');
	expect(fixture.querySelector('.ex-map-trigger')).toBeNull();
	root.append(owner);
	await userEvent.keyboard('m');
	expect(requiredElement(fixture, '.ex-map-trigger')).toBe(trigger);
	expect(trigger.getAttribute('aria-expanded')).toBe('true');
	await userEvent.keyboard('m');
	expect(trigger.getAttribute('aria-expanded')).toBe('false');
	owner.remove();
	const invalid = fixture.createElement('ex-map');
	invalid.setAttribute('for', 'missing-map-scope');
	invalid.append(fixture.createElement('ex-tree'));
	root.append(invalid);
	await userEvent.keyboard('m');
	expect(fixture.querySelector('.ex-map-trigger')).toBeNull();
});

test('M retains the existing C policy while a fullscreen non-editing control has focus', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('show-me-work');
	const diagram = requiredElement(fixture, '#story-diagram');
	diagram.scrollIntoView({ block: 'center', behavior: 'instant' });
	await expect.poll((): boolean => diagram.hasAttribute('prepared')).toBe(true);
	await frame.getByRole('button', { name: 'Expand Supplied record path', exact: true }).click();
	const dialog = requiredElement(fixture, 'dialog:modal');
	await userEvent.keyboard('m');
	expect(requiredElement(fixture, '.ex-map-trigger').getAttribute('aria-expanded')).toBe('true');
	expect(fixture.querySelector('dialog:modal')).toBe(dialog);
	await userEvent.keyboard('m');
	expect(requiredElement(fixture, '.ex-map-trigger').getAttribute('aria-expanded')).toBe('false');
	expect(fixture.querySelector('dialog:modal')).toBe(dialog);
	await userEvent.keyboard('{Escape}');
	expect(fixture.querySelector('dialog:modal')).toBeNull();
});
