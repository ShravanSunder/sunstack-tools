import { afterEach, expect, test } from 'vitest';
import { page, userEvent } from 'vitest/browser';

import { fixtureLocator, openFixture, requiredElement } from './fixture-page.ts';

afterEach(async (): Promise<void> => {
	document.body.replaceChildren();
	await page.viewport(1600, 1000);
});

test('tab panels and Trace attachments space adjacent blocks without changing nested layouts', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('ui', '', 'content-spacing.html');
	await expect
		.poll((): boolean => requiredElement(fixture, '#spacing-trace').hasAttribute('prepared'))
		.toBe(true);
	const gap = (before: string, after: string): number =>
		requiredElement(fixture, after).getBoundingClientRect().top -
		requiredElement(fixture, before).getBoundingClientRect().bottom;
	const verifySpacing = (): void => {
		for (const [before, after] of [
			['#assessment-heading', '#assessment-event'],
			['#assessment-event', '#assessment-boundary'],
			['#trace-heading', '#trace-limit'],
			['#trace-limit', '#trace-owner'],
			['#trace-owner', '#spacing-code'],
			['#spacing-code', '#trace-grid'],
		]) {
			if (!before || !after) throw new Error('Missing spacing scenario.');
			expect.soft(gap(before, after)).toBeGreaterThanOrEqual(23);
		}
		const grid = requiredElement(fixture, '#trace-grid');
		const cards = grid.querySelectorAll('ex-card');
		if (!cards[0] || !cards[1]) throw new Error('Missing nested grid cards.');
		if (fixture.documentElement.clientWidth > 1000)
			expect
				.soft(Math.abs(cards[0].getBoundingClientRect().top - cards[1].getBoundingClientRect().top))
				.toBeLessThan(2);
		expect
			.soft(fixture.documentElement.scrollWidth)
			.toBeLessThanOrEqual(fixture.documentElement.clientWidth);
	};
	verifySpacing();
	await page.viewport(390, 1000);
	verifySpacing();
	await page.viewport(1600, 1000);
	const trace = requiredElement(fixture, '#spacing-trace');
	trace.scrollIntoView();
	await fixtureLocator(frame, trace).hover();
	await frame.getByRole('button', { name: 'Expand Assessment evidence', exact: true }).click();
	await expect
		.element(frame.getByRole('dialog', { name: 'Assessment evidence', exact: true }))
		.toBeVisible();
	expect(gap('#trace-heading', '#trace-limit')).toBeGreaterThanOrEqual(23);
	await frame.getByRole('dialog').getByRole('button', { name: 'Close', exact: true }).click();
	expect(requiredElement(fixture, '#composition-step').hasAttribute('open')).toBe(true);
	verifySpacing();
});

test('Trace rows center disclosure controls while rich attachment headers keep their own layout', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('ui', '', 'explanation-components.html');
	const summary = requiredElement(fixture, '#source-step > summary');
	await fixtureLocator(frame, summary).click();
	await expect.element(frame.getByText('transport.close();', { exact: false })).toBeVisible();
	await expect
		.poll((): string => fixture.defaultView?.getComputedStyle(summary).display ?? '')
		.toBe('grid');
	const chevron = requiredElement(summary, '.ex-trace-chevron').getBoundingClientRect();
	const title = requiredElement(summary, '.ex-trace-title').getBoundingClientRect();
	expect(Math.abs(chevron.top + chevron.height / 2 - title.top - title.height / 2)).toBeLessThan(2);
	const fileHeader = requiredElement(fixture, '#trace-code .ex-code-file > summary');
	expect(fixture.defaultView?.getComputedStyle(fileHeader).display).toBe('flex');
	expect(fileHeader.querySelector('.ex-trace-chevron')).toBeNull();
	await fixtureLocator(frame, requiredElement(fixture, '#proof-step > summary')).click();
	await expect
		.element(frame.getByRole('heading', { name: 'Expected identity', exact: true }))
		.toBeVisible();
	requiredElement(fixture, '#trace-diagram').scrollIntoView({ block: 'center' });
	await expect
		.poll((): string => fixture.querySelector('#trace-diagram svg')?.textContent ?? '')
		.toContain('Reconnected Session');
	await expect
		.element(frame.getByText('Compare the reconnected identity', { exact: true }))
		.toBeVisible();
	await fixtureLocator(frame, summary).click();
	expect(requiredElement(fixture, '#source-step').hasAttribute('open')).toBe(false);
	await fixtureLocator(frame, requiredElement(fixture, '#evidence-trace')).hover();
	await frame
		.getByRole('button', { name: 'Open Evidence trace in a sidebar', exact: true })
		.click();
	await expect
		.element(frame.getByRole('complementary', { name: 'Evidence trace', exact: true }))
		.toBeVisible();
	expect(requiredElement(fixture, '#source-step').hasAttribute('open')).toBe(false);
	const ids = [...fixture.querySelectorAll('[id]')].map((element): string => element.id);
	expect(new Set(ids).size).toBe(ids.length);
	await userEvent.keyboard('{Escape}');
});

test('a cold Trace attachment fragment reveals its row and remains contained on a narrow page', async (): Promise<void> => {
	await page.viewport(390, 900);
	const { document: fixture, frame } = await openFixture(
		'ui',
		'#trace-notes--expected-identity',
		'explanation-components.html',
	);
	await expect
		.element(frame.getByRole('heading', { name: 'Expected identity', exact: true }))
		.toBeVisible();
	expect(requiredElement(fixture, '#proof-step').hasAttribute('open')).toBe(true);
	await expect
		.poll((): boolean => requiredElement(fixture, '#evidence-trace').hasAttribute('prepared'))
		.toBe(true);
	expect(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(390);
	const source = requiredElement(fixture, '#source-step');
	expect(source.hasAttribute('open')).toBe(false);
	await fixtureLocator(frame, requiredElement(fixture, '#evidence-trace')).hover();
	await frame.getByRole('button', { name: 'Expand Evidence trace', exact: true }).click();
	await expect
		.element(frame.getByRole('dialog', { name: 'Evidence trace', exact: true }))
		.toBeVisible();
	const ids = [...fixture.querySelectorAll('[id]')].map((element): string => element.id);
	expect(new Set(ids).size).toBe(ids.length);
	expect(source.hasAttribute('open')).toBe(true);
	await userEvent.keyboard('{Escape}');
	expect(source.hasAttribute('open')).toBe(false);
	expect(requiredElement(fixture, '#source-step')).toBe(source);
});

test('comparison grows with its content, aligns topic rows and keeps page-scroll headings sticky', async (): Promise<void> => {
	await page.viewport(1600, 650);
	const { document: fixture, frame } = await openFixture('ui', '', 'comparison-views.html');
	await expect
		.element(frame.getByRole('button', { name: 'Choose right comparison', exact: true }))
		.toBeVisible();
	const left = requiredElement(fixture, '#current [data-compare-row="disconnect"]');
	const right = requiredElement(fixture, '#proposed [data-compare-row="disconnect"]');
	expect(
		Math.abs(left.getBoundingClientRect().top - right.getBoundingClientRect().top),
	).toBeLessThan(1);
	await frame.getByRole('button', { name: 'Choose right comparison', exact: true }).click();
	await frame.getByRole('button', { name: 'Retain lookup', exact: true }).click();
	const gap = requiredElement(fixture, '#alternative [data-compare-row="disconnect"]');
	expect(gap.textContent).toContain('Not supplied');
	expect(Math.abs(left.getBoundingClientRect().top - gap.getBoundingClientRect().top)).toBeLessThan(
		1,
	);
	// The fixture retains its former scroll attribute: existing HTML must also grow naturally.
	const viewport = requiredElement(fixture, '#models .ex-comparison-viewport');
	expect(viewport.clientHeight).toBeGreaterThanOrEqual(viewport.scrollHeight - 1);
	viewport.scrollTop = 200;
	expect(viewport.scrollTop).toBe(0);
	viewport.scrollIntoView({ block: 'start', behavior: 'instant' });
	fixture.defaultView?.scrollBy({ top: 200, behavior: 'instant' });
	const header = requiredElement(fixture, '#models .ex-comparison-header:not([hidden])');
	expect(Math.abs(header.getBoundingClientRect().top - 56)).toBeLessThan(2);
});

test('comparison fullscreen retains live nested gallery, tabs and input state and returns the same nodes', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('ui', '', 'comparison-views.html');
	const owner = requiredElement(fixture, '#models');
	const input = requiredElement(fixture, '#working-note');
	const gallery = requiredElement(fixture, '#nested-gallery');
	await fixtureLocator(frame, input).fill('Edited before fullscreen');
	const nestedTabs = fixtureLocator(frame, requiredElement(fixture, '#nested-views'));
	await nestedTabs.getByRole('tab', { name: 'Proof', exact: true }).click();
	const nestedGallery = fixtureLocator(frame, gallery);
	await nestedGallery.getByRole('button', { name: 'Grid view', exact: true }).click();
	owner.scrollIntoView({ block: 'start' });
	await fixtureLocator(frame, owner).hover();
	await frame.getByRole('button', { name: 'Expand Session models', exact: true }).click();
	const dialog = frame.getByRole('dialog', { name: 'Session models', exact: true });
	await expect.element(dialog).toBeVisible();
	await expect
		.element(dialog.getByRole('button', { name: 'Grid view', exact: true }))
		.toHaveAttribute('aria-pressed', 'true');
	await expect
		.element(dialog.getByRole('tab', { name: 'Proof', exact: true }))
		.toHaveAttribute('aria-selected', 'true');
	await expect
		.element(dialog.getByLabelText('Working note', { exact: true }))
		.toHaveValue('Edited before fullscreen');
	await dialog.getByRole('button', { name: 'Choose right comparison', exact: true }).click();
	await frame.getByRole('button', { name: 'Retain lookup', exact: true }).click();
	await dialog.getByRole('button', { name: 'Choose right comparison', exact: true }).click();
	await frame.getByRole('button', { name: 'Proposed model', exact: true }).click();
	await userEvent.keyboard('{Escape}');
	expect(requiredElement(fixture, '#working-note')).toBe(input);
	expect(requiredElement(fixture, '#nested-gallery')).toBe(gallery);
	await expect
		.element(nestedGallery.getByRole('button', { name: 'Grid view', exact: true }))
		.toHaveAttribute('aria-pressed', 'true');
});

test('comparison choices swap unique live options and restore their nested content state', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('ui', '', 'comparison-views.html');
	const input = requiredElement(fixture, '#working-note');
	await fixtureLocator(frame, input).fill('Keep this edited note');
	await frame.getByRole('button', { name: 'Choose right comparison', exact: true }).click();
	await frame.getByRole('button', { name: 'Current model', exact: true }).click();
	expect(requiredElement(fixture, '#working-note')).toBe(input);
	expect(input.getAttribute('id')).toBe('working-note');
	await expect.element(fixtureLocator(frame, input)).toHaveValue('Keep this edited note');
	expect(requiredElement(fixture, '#proposed').hasAttribute('hidden')).toBe(false);
	const ids = [...fixture.querySelectorAll('[id]')].map((element): string => element.id);
	expect(new Set(ids).size).toBe(ids.length);
	await frame.getByRole('button', { name: 'Choose right comparison', exact: true }).click();
	await frame.getByRole('button', { name: 'Retain lookup', exact: true }).click();
	expect(requiredElement(fixture, '#current').hasAttribute('hidden')).toBe(true);
	await frame.getByRole('button', { name: 'Choose right comparison', exact: true }).click();
	await frame.getByRole('button', { name: 'Current model', exact: true }).click();
	expect(requiredElement(fixture, '#working-note')).toBe(input);
	await expect.element(fixtureLocator(frame, input)).toHaveValue('Keep this edited note');
});

test('comparison tabs support their normal keys and a cold hidden option target updates content choice', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture(
		'ui',
		'#alternative-evidence',
		'comparison-views.html',
	);
	await expect
		.element(
			frame.getByText('No disconnect implementation was supplied for this option.', {
				exact: true,
			}),
		)
		.toBeVisible();
	await expect
		.element(frame.getByRole('button', { name: 'Choose right comparison', exact: true }))
		.toHaveTextContent('Retain lookup');
	const tabset = fixtureLocator(frame, requiredElement(fixture, '#tabbed-models'));
	await tabset.getByRole('tab', { name: 'First', exact: true }).click();
	await tabset.getByLabelText('Tab note', { exact: true }).fill('Edited tab note');
	await tabset.getByRole('tab', { name: 'First', exact: true }).click();
	await userEvent.keyboard('{End}');
	await expect.element(tabset.getByText('Second view.', { exact: false })).toBeVisible();
	await userEvent.keyboard('{Home}');
	await expect
		.element(tabset.getByLabelText('Tab note', { exact: true }))
		.toHaveValue('Edited tab note');
	await page.viewport(390, 900);
	await expect.poll((): number => fixture.documentElement.scrollWidth).toBeLessThanOrEqual(390);
	const firstOption = requiredElement(fixture, '#current');
	const secondHeader = requiredElement(fixture, '#models .ex-comparison-header:last-of-type');
	await expect
		.poll(
			(): number =>
				secondHeader.getBoundingClientRect().top - firstOption.getBoundingClientRect().bottom,
		)
		.toBeGreaterThanOrEqual(0);
	expect(
		Math.abs(
			firstOption.getBoundingClientRect().left -
				requiredElement(fixture, '#alternative').getBoundingClientRect().left,
		),
	).toBeLessThan(1);
});

test('comparison sidebar choices are independent and closing a nested gallery copy returns ordinary keys to the main gallery', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('ui', '', 'comparison-views.html');
	const owner = requiredElement(fixture, '#models');
	const input = requiredElement(fixture, '#working-note');
	await fixtureLocator(frame, input).fill('Main note');
	const originalGallery = fixtureLocator(frame, requiredElement(fixture, '#nested-gallery'));
	await originalGallery.getByRole('button', { name: 'Hide thumbnails', exact: true }).click();
	owner.scrollIntoView({ block: 'start' });
	await fixtureLocator(frame, owner).hover();
	await frame
		.getByRole('button', { name: 'Open Session models in a sidebar', exact: true })
		.click();
	const drawer = frame.getByRole('complementary', { name: 'Session models', exact: true });
	await expect.element(drawer).toBeVisible();
	await expect
		.element(drawer.getByRole('button', { name: 'Show thumbnails', exact: true }))
		.toHaveAttribute('aria-expanded', 'false');
	await drawer.getByRole('button', { name: 'Show thumbnails', exact: true }).click();
	await expect
		.element(originalGallery.getByRole('button', { name: 'Show thumbnails', exact: true }))
		.toHaveAttribute('aria-expanded', 'false');
	await drawer.getByLabelText('Working note', { exact: true }).fill('Copy note');
	await expect.element(fixtureLocator(frame, input)).toHaveValue('Main note');
	await drawer.getByRole('button', { name: 'Choose right comparison', exact: true }).click();
	await frame.getByRole('button', { name: 'Retain lookup', exact: true }).click();
	expect(requiredElement(fixture, '#proposed').hasAttribute('hidden')).toBe(false);
	await drawer.getByRole('button', { name: 'Choose right comparison', exact: true }).click();
	await frame.getByRole('button', { name: 'Proposed model', exact: true }).click();
	await drawer.getByRole('button', { name: 'Grid view', exact: true }).click();
	const ids = [...fixture.querySelectorAll('[id]')].map((element): string => element.id);
	expect(ids.filter((id, index): boolean => ids.indexOf(id) !== index)).toEqual([]);
	await userEvent.keyboard('{Escape}');
	await frame.getByRole('heading', { name: 'Compare corresponding topics', exact: true }).click();
	await userEvent.keyboard('{ArrowRight}');
	const mainGallery = fixtureLocator(frame, requiredElement(fixture, '#nested-gallery'));
	await expect
		.element(mainGallery.getByRole('img', { name: 'Second synthetic fixture', exact: true }))
		.toBeVisible();
});

test('the image slider still moves its actual reveal mask through native range keys', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('ui', '', 'comparison-views.html');
	const owner = requiredElement(fixture, '#image-reveal');
	const slider = fixtureLocator(frame, owner).getByRole('slider', {
		name: 'Reveal before and after',
		exact: true,
	});
	await slider.click();
	const after = requiredElement(owner, '[data-side="After"]');
	await userEvent.keyboard('{Home}');
	await expect.element(slider).toHaveValue('0');
	const beforeMask = fixture.defaultView?.getComputedStyle(after).clipPath;
	await userEvent.keyboard('{End}');
	await expect.element(slider).toHaveValue('100');
	expect(fixture.defaultView?.getComputedStyle(after).clipPath).not.toBe(beforeMask);
	expect(requiredElement(owner, '[data-side="After"]')).toBe(after);
});

test.each(['#alternative', '#second-tab'])(
	'a direct alternative fragment %s reveals the option itself',
	async (fragment: string): Promise<void> => {
		const { document: fixture, frame } = await openFixture('ui', fragment, 'comparison-views.html');
		if (fragment === '#alternative') {
			await expect
				.element(frame.getByRole('button', { name: 'Choose right comparison', exact: true }))
				.toHaveTextContent('Retain lookup');
			expect(requiredElement(fixture, '#alternative').hasAttribute('hidden')).toBe(false);
		} else {
			await expect
				.element(frame.getByRole('tab', { name: 'Second', exact: true }))
				.toHaveAttribute('aria-selected', 'true');
			expect(requiredElement(fixture, '#second-tab').hasAttribute('hidden')).toBe(false);
		}
	},
);

test('stacked comparison inspection controls leave the first content choice reachable', async (): Promise<void> => {
	await page.viewport(390, 900);
	const { document: fixture, frame } = await openFixture('ui', '', 'comparison-views.html');
	const owner = requiredElement(fixture, '#models');
	owner.scrollIntoView({ block: 'start' });
	await fixtureLocator(frame, owner).hover();
	const choice = requiredElement(fixture, '#models .ex-comparison-header:first-of-type > button');
	const bounds = choice.getBoundingClientRect();
	const controls = [...fixture.querySelectorAll('#models > .ex-inspect-controls > button')];
	expect(controls).toHaveLength(2);
	for (const control of controls) {
		const button = control.getBoundingClientRect();
		const overlapWidth = Math.min(bounds.right, button.right) - Math.max(bounds.left, button.left);
		const overlapHeight = Math.min(bounds.bottom, button.bottom) - Math.max(bounds.top, button.top);
		expect(overlapWidth > 0 && overlapHeight > 0).toBe(false);
	}
	await fixtureLocator(frame, choice).click();
	await expect
		.element(frame.getByRole('button', { name: 'Retain lookup', exact: true }))
		.toBeVisible();
});

test('expanding a source call keeps the next comparison topic aligned and return branches remain separate', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('ui', '', 'call-paths.html');
	const proposed = fixtureLocator(frame, requiredElement(fixture, '#new-calls'));
	await proposed.getByText('transport.close()', { exact: true }).click();
	await expect
		.element(proposed.getByText('The replacement call closes this connection.', { exact: false }))
		.toBeVisible();
	const currentLookup = requiredElement(fixture, '#old-calls [data-compare-row="lookup"]');
	const proposedLookup = requiredElement(fixture, '#new-calls [data-compare-row="lookup"]');
	expect(
		Math.abs(
			currentLookup.getBoundingClientRect().top - proposedLookup.getBoundingClientRect().top,
		),
	).toBeLessThan(1);
	expect(
		requiredElement(fixture, '#new-calls [data-compare-row="outcomes"]').querySelector('ex-trace'),
	).toBeNull();
	await expect
		.element(proposed.getByText('No tenant check is supplied.', { exact: false }))
		.toBeVisible();
	expect(requiredElement(fixture, '#call-source').textContent).toContain(
		"if (session.expired) return { kind: 'expired' };",
	);
});
