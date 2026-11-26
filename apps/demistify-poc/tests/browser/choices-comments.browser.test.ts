import { afterEach, beforeEach, expect, test } from 'vitest';
import { commands, page, userEvent } from 'vitest/browser';

import { commentStorageKey } from '../../../../packages/demistify/src/core/comment-store.ts';
import { browserScreenshotPath } from './browser-evidence.ts';
import { fixtureLocator, requiredElement } from './fixture-page.ts';
import { readChoiceResponse, readCurrentReview, openReviewFixture } from './review-fixture.ts';

const identity = { documentId: 'demistify-choice-fixture', documentRevision: 'r1' };
function controlCenter(control: Element): { x: number; y: number } {
	const bounds = control.getBoundingClientRect();
	return { x: bounds.left + bounds.width / 2, y: bounds.top + bounds.height / 2 };
}
beforeEach((): void => {
	localStorage.removeItem(commentStorageKey(identity));
});
afterEach(async (): Promise<void> => {
	document.body.replaceChildren();
	await commands.setReducedMotion('no-preference');
	await page.viewport(1600, 1000);
});

test('a selected option and its comment share one title in the review rail', async (): Promise<void> => {
	const { document: fixture, frame } = await openReviewFixture();
	await frame.getByRole('radio', { name: 'Layout only', exact: true }).click();
	await fixtureLocator(frame, requiredElement(fixture, '#layout')).hover();
	await fixtureLocator(frame, requiredElement(fixture, '#layout > [data-comment-control]')).click();
	await frame
		.getByRole('textbox', { name: 'Your comment', exact: true })
		.fill('Keep the boundary.');
	await frame.getByRole('button', { name: 'Add comment', exact: true }).click();
	const entries = [...fixture.querySelectorAll('.ex-comments-body > div')]
		.map((element): string => element.textContent ?? '')
		.join('\n');
	expect(entries.match(/Layout only/g)).toHaveLength(1);
	expect(requiredElement(fixture, '.ex-comments-body').textContent).toContain('Keep the boundary.');
	expect(readCurrentReview(fixture).comments).toHaveLength(1);
});

test.each([
	{ width: 1600, theme: 'dark' },
	{ width: 1600, theme: 'light' },
	{ width: 390, theme: 'dark' },
	{ width: 390, theme: 'light' },
])(
	'saved Choice comment count stays inside its circle at $width $theme',
	async ({ width, theme }): Promise<void> => {
		await page.viewport(width, 900);
		const { document: fixture, frame } = await openReviewFixture();
		fixture.documentElement.dataset['theme'] = theme;
		const option = requiredElement(fixture, '#technical');
		const marker = requiredElement(option, ':scope > .ex-entity-comment');
		const original = marker.getBoundingClientRect();
		if (width === 390) {
			await frame.getByRole('button', { name: 'Comments (0)', exact: true }).click();
			await frame
				.getByRole('checkbox', { name: 'Show comment markers on the page', exact: true })
				.click();
		} else await fixtureLocator(frame, option).hover();
		await frame.getByRole('button', { name: 'Comment on Technical scope', exact: true }).click();
		await frame
			.getByRole('textbox', { name: 'Your comment', exact: true })
			.fill('Keep the checked boundary.');
		await frame.getByRole('button', { name: 'Add comment', exact: true }).click();
		await frame.getByRole('button', { name: 'Close comments', exact: true }).click();
		await expect
			.element(frame.getByRole('button', { name: 'Comment on Technical scope (1)', exact: true }))
			.toBeVisible();
		await frame
			.getByRole('button', { name: 'Open details for Technical scope', exact: true })
			.click();
		const bounds = marker.getBoundingClientRect();
		expect(bounds.width).toBe(original.width);
		expect(bounds.height).toBe(original.height);
		expect(marker.scrollWidth).toBeLessThanOrEqual(marker.clientWidth);
		expect(marker.scrollHeight).toBeLessThanOrEqual(marker.clientHeight);
		if (width === 1600) {
			const markerCenter = controlCenter(marker);
			const card = option.getBoundingClientRect();
			expect(Math.abs(markerCenter.x - card.right)).toBeLessThan(2);
			expect(Math.abs(markerCenter.y - card.top)).toBeLessThan(2);
		}
		await page.screenshot({
			path: browserScreenshotPath(`saved-comment-circle-${width}-${theme}.png`),
		});
	},
);

test.each(['dark', 'light'])(
	'desktop %s review centers diagram controls on the boundary and reveals only the hovered target',
	async (theme): Promise<void> => {
		const { document: fixture, frame } = await openReviewFixture();
		fixture.documentElement.dataset['theme'] = theme;
		const paragraph = requiredElement(fixture, '#first-passage');
		const diagram = fixture.createElement('ex-mermaid');
		diagram.setAttribute('label', 'Review diagram');
		diagram.innerHTML = '<script type="text/plain">flowchart LR\nA[Start] --> B[Finish]</script>';
		paragraph.after(diagram);
		diagram.scrollIntoView({ block: 'center' });
		await expect.poll(() => diagram.querySelector(':scope > svg') !== null).toBe(true);
		const marker = requiredElement(diagram, ':scope > .ex-entity-comment');
		const fullscreen = requiredElement(diagram, ':scope > .ex-inspect-controls');
		const paragraphMarker = requiredElement(paragraph, ':scope > .ex-entity-comment');
		const opacity = (element: Element): string =>
			fixture.defaultView?.getComputedStyle(element).opacity ?? '';

		await frame.getByRole('button', { name: 'Comments (0)', exact: true }).hover();
		await expect.poll(() => opacity(marker)).toBe('0');
		await expect.poll(() => opacity(fullscreen)).toBe('0');
		await fixtureLocator(frame, diagram).hover();
		await expect.poll(() => opacity(marker)).toBe('1');
		await expect.poll(() => opacity(fullscreen)).toBe('1');
		expect(opacity(paragraphMarker)).toBe('0');
		const edge = diagram.getBoundingClientRect();
		expect(controlCenter(marker).x).toBeCloseTo(edge.right, 0);
		expect(controlCenter(marker).y).toBeCloseTo(edge.top, 0);
		expect(controlCenter(fullscreen).x).toBeCloseTo(edge.right, 0);
		expect(controlCenter(fullscreen).y).toBeGreaterThan(controlCenter(marker).y);
		const expand = requiredElement(fullscreen, 'button');
		const optionMarker = requiredElement(fixture, '#technical > .ex-entity-comment');
		for (const control of [marker, expand]) {
			const bounds = control.getBoundingClientRect();
			expect(bounds.width).toBeCloseTo(optionMarker.getBoundingClientRect().width, 0);
			expect(bounds.height).toBeCloseTo(bounds.width, 0);
			expect(fixture.defaultView?.getComputedStyle(control).borderRadius).toBe('50%');
		}
		expect(controlCenter(paragraphMarker).x).toBeCloseTo(
			paragraph.getBoundingClientRect().right,
			0,
		);
		expect(controlCenter(paragraphMarker).y).toBeCloseTo(paragraph.getBoundingClientRect().top, 0);
		await page.screenshot({
			path: browserScreenshotPath(`reader-comment-gutter-desktop-${theme}.png`),
		});

		await fixtureLocator(frame, marker).click();
		await frame.getByRole('textbox', { name: 'Your comment', exact: true }).fill('A diagram note.');
		await frame.getByRole('button', { name: 'Add comment', exact: true }).click();
		await frame.getByRole('button', { name: 'Close comments', exact: true }).click();
		await fixtureLocator(frame, paragraph).click();
		await expect.poll(() => opacity(marker)).toBe('1');
		await expect.poll(() => opacity(fullscreen)).toBe('0');
		await expect
			.poll(() => Math.abs(controlCenter(marker).x - diagram.getBoundingClientRect().right))
			.toBeLessThan(1);
		expect(readCurrentReview(fixture).comments[0]?.anchor.targetId).toBe(diagram.id);
	},
);

test.each(['dark', 'light'])(
	'phone-width %s review hides markers until requested and can compose from a tapped paragraph',
	async (theme): Promise<void> => {
		await page.viewport(390, 850);
		const { document: fixture, frame } = await openReviewFixture();
		fixture.documentElement.dataset['theme'] = theme;
		const paragraph = requiredElement(fixture, '#first-passage');
		const paragraphMarker = requiredElement(paragraph, ':scope > .ex-entity-comment');
		const choiceMarker = requiredElement(fixture, '#technical > .ex-entity-comment');
		const opacity = (element: Element): string =>
			fixture.defaultView?.getComputedStyle(element).opacity ?? '';
		expect(opacity(paragraphMarker)).toBe('0');
		expect(opacity(choiceMarker)).toBe('0');
		expect(fixture.defaultView?.getComputedStyle(paragraph).paddingInlineEnd).toBe('0px');
		await fixtureLocator(frame, requiredElement(fixture, '#technical')).hover();
		expect(opacity(choiceMarker)).toBe('0');

		await frame.getByRole('button', { name: 'Comments (0)', exact: true }).click();
		await frame
			.getByRole('checkbox', { name: 'Show comment markers on the page', exact: true })
			.click();
		await expect.poll(() => opacity(paragraphMarker)).toBe('1');
		await expect.poll(() => opacity(choiceMarker)).toBe('1');
		await frame.getByRole('button', { name: 'Comments (0)', exact: true }).click();
		await frame
			.getByRole('checkbox', { name: 'Show comment markers on the page', exact: true })
			.click();
		await expect.poll(() => opacity(paragraphMarker)).toBe('0');
		expect(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(
			fixture.documentElement.clientWidth,
		);
		await page.screenshot({
			path: browserScreenshotPath(`reader-comment-markers-phone-${theme}.png`),
		});

		await commands.tapReviewTarget('#first-passage');
		const tapAction = frame.getByRole('button', { name: 'Comment on this block', exact: true });
		await expect.element(tapAction).toBeVisible();
		const button = requiredElement(fixture, '.ex-block-comment-action');
		expect(button.getBoundingClientRect().bottom).toBeLessThanOrEqual(
			requiredElement(fixture, '#second-passage').getBoundingClientRect().top,
		);
		await page.screenshot({ path: browserScreenshotPath(`reader-comment-tap-phone-${theme}.png`) });
		await tapAction.click();
		await frame
			.getByRole('textbox', { name: 'Your comment', exact: true })
			.fill('A paragraph note.');
		await frame.getByRole('button', { name: 'Add comment', exact: true }).click();
		expect(readCurrentReview(fixture).comments[0]?.anchor.targetId).toBe(paragraph.id);
		await frame.getByRole('button', { name: 'Close comments', exact: true }).click();
		await fixtureLocator(frame, requiredElement(fixture, 'h1')).hover();
		await expect.poll(() => opacity(paragraphMarker)).toBe('0');
		const view = fixture.defaultView;
		if (!view) throw new Error('Missing review window');
		// Legacy touch clicks can arrive as a plain MouseEvent.
		paragraph.dispatchEvent(
			new view.PointerEvent('pointerdown', { bubbles: true, pointerType: 'touch' }),
		);
		paragraph.dispatchEvent(new view.MouseEvent('click', { bubbles: true, detail: 1 }));
		await expect.element(tapAction).toBeVisible();
	},
);

test('a code comment leaves its inspection buttons clear', async (): Promise<void> => {
	const { document: fixture, frame } = await openReviewFixture();
	const code = fixture.createElement('ex-code');
	code.setAttribute('label', 'Review command');
	code.innerHTML = '<script type="text/plain">cargo test</script>';
	requiredElement(fixture, '#first-passage').after(code);
	await expect.poll(() => code.querySelector(':scope > .ex-entity-comment')).not.toBeNull();
	await fixtureLocator(frame, code).hover();
	const marker = requiredElement(code, ':scope > .ex-entity-comment');
	const actions = requiredElement(code, ':scope > .ex-inspect-controls');
	expect(actions.getBoundingClientRect().right).toBeLessThan(marker.getBoundingClientRect().left);
	await frame.getByRole('button', { name: 'Expand Review command', exact: true }).click();
	await expect
		.element(frame.getByRole('dialog', { name: 'Review command', exact: true }))
		.toBeVisible();
});

test('a compact choice detail keeps an unmarked diagram’s Fullscreen button available', async (): Promise<void> => {
	await page.viewport(390, 850);
	const { document: fixture, frame } = await openReviewFixture();
	await frame
		.getByRole('button', { name: 'Open details for Technical scope', exact: true })
		.click();
	const diagram = requiredElement(fixture, '#scope-diagram');
	diagram.scrollIntoView({ block: 'center' });
	await expect.poll(() => diagram.querySelector('svg')).not.toBeNull();
	expect(diagram.querySelector(':scope > .ex-entity-comment')).toBeNull();
	const actions = requiredElement(diagram, ':scope > .ex-inspect-controls');
	expect(fixture.defaultView?.getComputedStyle(actions).opacity).toBe('1');
	await frame.getByRole('button', { name: 'Expand Review ownership', exact: true }).click();
	await expect
		.element(frame.getByRole('dialog', { name: 'Review ownership', exact: true }))
		.toBeVisible();
});

test('tapping a labelled table cell puts the comment action below the table', async (): Promise<void> => {
	await page.viewport(390, 850);
	const { document: fixture, frame } = await openReviewFixture();
	const table = fixture.createElement('table');
	table.innerHTML =
		'<tbody><tr><td id="tap-cell" data-comment-label="Selected cell">Cell contents</td></tr></tbody>';
	requiredElement(fixture, 'ex-document').append(table);
	await expect.poll(() => table.querySelector('#tap-cell > .ex-entity-comment')).not.toBeNull();
	await commands.tapReviewTarget('#tap-cell');
	const action = frame.getByRole('button', { name: 'Comment on this block', exact: true });
	await expect.element(action).toBeVisible();
	const button = requiredElement(fixture, '.ex-block-comment-action');
	expect(button.closest('table')).toBeNull();
	expect(button.getBoundingClientRect().top).toBeGreaterThanOrEqual(
		table.getBoundingClientRect().bottom,
	);
	await action.click();
	await expect
		.element(frame.getByRole('heading', { name: 'Selected cell', exact: true }))
		.toBeVisible();
});

test('reopening a node edits its saved comment instead of adding another', async (): Promise<void> => {
	const { document: fixture, frame } = await openReviewFixture();
	const node = requiredElement(fixture, '#layout > [data-comment-control]');
	await fixtureLocator(frame, requiredElement(fixture, '#layout')).hover();
	await fixtureLocator(frame, node).click();
	await frame.getByRole('textbox', { name: 'Your comment', exact: true }).fill('First note');
	await frame.getByRole('button', { name: 'Add comment', exact: true }).click();
	const original = readCurrentReview(fixture).comments[0];
	await frame.getByRole('button', { name: 'Close comments', exact: true }).click();
	await fixtureLocator(frame, node).click();
	expect(
		fixture.querySelector<HTMLTextAreaElement>('textarea[aria-label="Your comment"]')?.value,
	).toBe('First note');
	await frame
		.getByRole('textbox', { name: 'Your comment', exact: true })
		.fill('Edited note\nSecond line');
	await userEvent.keyboard('{Meta>}{Enter}{/Meta}');
	expect(readCurrentReview(fixture).comments).toHaveLength(1);
	expect(readCurrentReview(fixture).comments[0]).toMatchObject({
		commentId: original?.commentId,
		body: 'Edited note\nSecond line',
	});
});

test('choice cards, table rows and inline details keep one stable option comment target', async (): Promise<void> => {
	const { document: fixture } = await openReviewFixture();
	const options = [...fixture.querySelectorAll('.ex-choice-option')];
	expect(options).toHaveLength(8);
	for (const option of options) {
		expect(
			option.querySelectorAll(
				':scope > [data-comment-control],:scope > td > [data-comment-control]',
			),
		).toHaveLength(1);
		expect(
			option.querySelectorAll(':is(p,li,h1,h2,h3,h4,h5,h6) > [data-comment-control]'),
		).toHaveLength(0);
	}
});

test('body, native controls and repainted Comment SVG keep answer and action semantics separate', async (): Promise<void> => {
	const { document: fixture, frame } = await openReviewFixture();
	const choice = requiredElement(fixture, '#scope-question');
	await fixtureLocator(frame, requiredElement(fixture, '#layout .ex-choice-summary li')).click();
	expect(readChoiceResponse(choice).selected).toEqual([
		{ optionId: 'layout', label: 'Layout only' },
	]);
	await fixtureLocator(frame, requiredElement(fixture, '#technical')).hover();
	await fixtureLocator(
		frame,
		requiredElement(fixture, '#technical > [data-comment-control] svg'),
	).click();
	await expect
		.element(frame.getByRole('textbox', { name: 'Your comment', exact: true }))
		.toBeVisible();
	expect(readChoiceResponse(choice).selected).toEqual([
		{ optionId: 'layout', label: 'Layout only' },
	]);
	await frame
		.getByRole('textbox', { name: 'Your comment', exact: true })
		.fill('Keep this unselected option.');
	await userEvent.keyboard('{Meta>}{Enter}{/Meta}');
	expect(readCurrentReview(fixture).comments[0]?.anchor).toEqual({
		kind: 'entity',
		targetId: 'technical',
		label: 'Technical scope',
	});
	expect(readCurrentReview(fixture).answers[0]?.selected[0]?.optionId).toBe('layout');
	await frame.getByRole('button', { name: 'Close comments', exact: true }).click();
	await frame.getByRole('radio', { name: 'Layout only', exact: true }).click();
	await userEvent.keyboard('{ArrowRight}');
	expect(readChoiceResponse(choice).selected[0]?.optionId).toBe('technical');
	await frame.getByRole('checkbox', { name: 'Desktop', exact: true }).click();
	await frame.getByRole('checkbox', { name: 'Phone', exact: true }).click();
	expect(
		readChoiceResponse(requiredElement(fixture, '#checks-question')).selected.map(
			(option): string => option.optionId,
		),
	).toEqual(['desktop-check', 'phone-check']);
});

test('explicit sidebar Details stays right-aligned and preserves the outline', async (): Promise<void> => {
	const { document: fixture, frame } = await openReviewFixture('sidebar.html');
	const target = requiredElement(fixture, '#technical');
	const label = requiredElement(target, '.ex-choice-label');
	const button = requiredElement(target, '.ex-choice-details-button');
	const view = fixture.defaultView;
	expect(view?.getComputedStyle(label).fontSize).toBe('18px');
	expect(view?.getComputedStyle(requiredElement(target, '.ex-choice-summary')).fontSize).toBe(
		'15px',
	);
	expect(button.textContent?.trim()).toBe('Details');
	expect(button.querySelector('.ph-article')).not.toBeNull();
	expect(button.getBoundingClientRect().width).toBeGreaterThan(32);
	expect(button.getBoundingClientRect().height).toBe(32);
	expect(button.getBoundingClientRect().right).toBeCloseTo(
		requiredElement(target, '.ex-choice-title-row').getBoundingClientRect().right,
		0,
	);
	expect(button.getBoundingClientRect().left - label.getBoundingClientRect().right).toBeCloseTo(
		8,
		0,
	);
	const headings = [...fixture.querySelectorAll('.ex-heading-outline a')].map(
		(heading): string => heading.textContent ?? '',
	);
	await frame
		.getByRole('button', { name: 'Open details for Technical scope', exact: true })
		.click();
	await expect
		.poll((): string => fixture.querySelector('.ex-drawer svg')?.textContent ?? '')
		.toContain('Comments');
	expect(target.getBoundingClientRect().height).toBeGreaterThan(40);
	expect(view?.getComputedStyle(target).outlineWidth).toBe('2px');
	expect(readChoiceResponse(requiredElement(fixture, '#scope-question')).selected).toEqual([]);
	expect(
		[...fixture.querySelectorAll('.ex-heading-outline a')].map(
			(heading): string => heading.textContent ?? '',
		),
	).toEqual(headings);
	await frame.getByRole('button', { name: 'Close', exact: true }).click();
	expect(fixture.activeElement?.getAttribute('aria-label')).toBe(
		'Open details for Technical scope',
	);
});

test('default beside-card Details closes back to the same trigger and leaves selection unchanged', async (): Promise<void> => {
	const { document: fixture, frame } = await openReviewFixture();
	const choice = requiredElement(fixture, '#scope-question');
	const option = requiredElement(choice, '#technical');
	const marker = requiredElement(option, ':scope > .ex-entity-comment');
	expect(choice.getBoundingClientRect().width).toBeLessThanOrEqual(761);
	const card = option.getBoundingClientRect();
	expect(Math.abs(controlCenter(marker).x - card.right)).toBeLessThan(2);
	expect(Math.abs(controlCenter(marker).y - card.top)).toBeLessThan(2);
	expect(fixture.defaultView?.getComputedStyle(marker).opacity).toBe('0');
	const trigger = frame.getByRole('button', {
		name: 'Open details for Technical scope',
		exact: true,
	});
	const close = frame.getByRole('button', { name: 'Close details', exact: true });
	await frame.getByRole('radio', { name: 'Layout only', exact: true }).click();
	await trigger.click();
	await expect.element(close).toBeVisible();
	expect(choice.getBoundingClientRect().width).toBeGreaterThan(761);
	const panel = requiredElement(fixture, '#technical--choice-details');
	expect(marker.getBoundingClientRect().right).toBeLessThan(panel.getBoundingClientRect().left);
	expect(
		requiredElement(fixture, '#technical .ex-choice-details-button').getAttribute('aria-expanded'),
	).toBe('true');
	await close.click();
	expect(panel.hasAttribute('hidden')).toBe(true);
	expect(choice.getBoundingClientRect().width).toBeLessThanOrEqual(761);
	expect(fixture.activeElement?.getAttribute('aria-label')).toBe(
		'Open details for Technical scope',
	);
	expect(
		readChoiceResponse(requiredElement(fixture, '#scope-question')).selected[0]?.optionId,
	).toBe('layout');
	const view = fixture.defaultView;
	if (!view) throw new Error('Choice frame has no window.');
	view.location.hash = 'scope-diagram';
	await expect.element(close).toBeVisible();
	await expect
		.poll((): boolean => Boolean(fixture.querySelector('#scope-diagram > svg')))
		.toBe(true);
	const paragraph = requiredElement(fixture, '#technical--choice-details p');
	const quote = fixture.createRange();
	quote.selectNodeContents(paragraph);
	view.getSelection()?.addRange(quote);
	await frame.getByRole('button', { name: 'Comment on selected text', exact: true }).click();
	await frame.getByRole('textbox', { name: 'Your comment', exact: true }).fill('Detail quote.');
	await frame.getByRole('button', { name: 'Add comment', exact: true }).click();
	expect(readCurrentReview(fixture).comments[0]?.anchor).toMatchObject({
		kind: 'text',
		targetId: 'technical--choice-details',
	});
	document.body.replaceChildren();
	const reopened = await openReviewFixture();
	expect(reopened.document.querySelector('#scope-diagram > svg')).toBeNull();
	await reopened.frame.getByRole('button', { name: 'Comments (1)', exact: true }).click();
	await reopened.frame
		.getByRole('button', { name: 'Show anchor: Details for Technical scope', exact: true })
		.click();
	expect(
		requiredElement(reopened.document, '#technical--choice-details').classList.contains(
			'ex-comment-active',
		),
	).toBe(true);
});

test('selectable table and inline details retain their authored views at phone width', async (): Promise<void> => {
	await page.viewport(390, 900);
	const { document: fixture, frame } = await openReviewFixture();
	const table = requiredElement(fixture, '#delivery-question table');
	expect(table.querySelectorAll('tbody tr')).toHaveLength(2);
	await fixtureLocator(frame, requiredElement(fixture, '#source-copy .ex-choice-summary')).click();
	expect(
		readChoiceResponse(requiredElement(fixture, '#delivery-question')).selected[0]?.optionId,
	).toBe('source-copy');
	const scroll = requiredElement(fixture, '#delivery-question .ex-table-scroll');
	expect(scroll.scrollWidth).toBeGreaterThan(scroll.clientWidth);
	expect(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(391);
	const before = fixture.querySelectorAll('.ex-heading-outline a').length;
	await fixtureLocator(frame, requiredElement(fixture, '#review-now details > summary')).click();
	await expect
		.element(
			frame.getByText('Keep this supporting explanation beside its option.', { exact: true }),
		)
		.toBeVisible();
	expect(fixture.querySelector('dialog[open],.ex-drawer:not([hidden])')).toBeNull();
	expect(fixture.querySelectorAll('.ex-heading-outline a')).toHaveLength(before);
	expect(fixture.querySelector('#review-now h3')).toBeNull();
	await fixtureLocator(frame, requiredElement(fixture, '#review-now details > summary')).click();
	await expect
		.element(
			frame.getByText('Keep this supporting explanation beside its option.', { exact: true }),
		)
		.not.toBeVisible();
});

test.each([1600, 390])(
	'Comments and explicit sidebar Details replace each other at width %i',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openReviewFixture('sidebar.html');
		await frame.getByRole('button', { name: 'Comments (0)', exact: true }).click();
		await expect
			.element(
				frame.getByRole(width < 900 ? 'dialog' : 'complementary', {
					name: 'Comments',
					exact: true,
				}),
			)
			.toBeVisible();
		if (width < 900) {
			// This unreachable-behind-modal edge is component integration, not native pointer proof.
			const details = fixture.querySelector<HTMLButtonElement>(
				'#technical .ex-choice-details-button',
			);
			if (!details) throw new Error('Technical Details is absent.');
			details.click();
		} else
			await frame
				.getByRole('button', { name: 'Open details for Technical scope', exact: true })
				.click();
		await expect
			.element(
				frame.getByRole(width < 900 ? 'dialog' : 'complementary', {
					name: 'Technical scope',
					exact: true,
				}),
			)
			.toBeVisible();
		expect(requiredElement(fixture, '.ex-comments-trigger').getAttribute('aria-expanded')).toBe(
			'false',
		);
		await userEvent.keyboard('c');
		await expect
			.element(
				frame.getByRole(width < 900 ? 'dialog' : 'complementary', {
					name: 'Technical scope',
					exact: true,
				}),
			)
			.toBeVisible();
		if (width < 900) {
			// Match the first transition's component integration seam: the phone sheet hides this trigger.
			const comments = fixture.querySelector<HTMLButtonElement>('.ex-comments-trigger');
			if (!comments) throw new Error('Comments trigger is absent.');
			comments.click();
		} else await frame.getByRole('button', { name: 'Comments (0)', exact: true }).click();
		await expect
			.element(
				frame.getByRole(width < 900 ? 'dialog' : 'complementary', {
					name: 'Comments',
					exact: true,
				}),
			)
			.toBeVisible();
		await frame.getByRole('button', { name: 'Close comments', exact: true }).click();
	},
);

test('same-revision reload retains comment identities and bodies while answers remain current page inputs', async (): Promise<void> => {
	const { document: fixture, frame } = await openReviewFixture();
	await fixtureLocator(frame, requiredElement(fixture, '#technical')).hover();
	await fixtureLocator(
		frame,
		requiredElement(fixture, '#technical > [data-comment-control]'),
	).click();
	await frame
		.getByRole('textbox', { name: 'Your comment', exact: true })
		.fill('Saved body\nSecond line');
	await frame.getByRole('button', { name: 'Add comment', exact: true }).click();
	const saved = readCurrentReview(fixture);
	document.body.replaceChildren();
	const reopened = await openReviewFixture();
	expect(readCurrentReview(reopened.document)).toEqual(saved);
	expect(readCurrentReview(reopened.document).comments[0]?.body).toBe('Saved body\nSecond line');
	expect(readCurrentReview(reopened.document).answers).toEqual([]);
});

test('invalid stored data stays untouched and is visibly unsaved after a new comment', async (): Promise<void> => {
	const key = commentStorageKey(identity);
	localStorage.setItem(key, '{broken');
	const { document: fixture, frame } = await openReviewFixture();
	await fixtureLocator(frame, requiredElement(fixture, '#layout')).hover();
	await fixtureLocator(frame, requiredElement(fixture, '#layout > [data-comment-control]')).click();
	await frame.getByRole('textbox', { name: 'Your comment', exact: true }).fill('Memory feedback');
	await frame.getByRole('button', { name: 'Add comment', exact: true }).click();
	await expect
		.element(frame.getByRole('status').filter({ hasText: 'Saved comments are invalid' }))
		.toBeVisible();
	expect(localStorage.getItem(key)).toBe('{broken');
	expect(readCurrentReview(fixture).comments[0]?.body).toBe('Memory feedback');
});

test('reconnect retains one original controller and readonly inspection cannot change its radio answer', async (): Promise<void> => {
	const { document: fixture, frame } = await openReviewFixture();
	const choice = requiredElement(fixture, '#scope-question');
	const comments = requiredElement(fixture, 'ex-comments');
	const root = requiredElement(fixture, '#content');
	await fixtureLocator(frame, requiredElement(fixture, '#layout .ex-choice-summary')).click();
	choice.remove();
	root.prepend(choice);
	comments.remove();
	fixture.body.append(comments);
	await expect.poll((): number => fixture.querySelectorAll('.ex-comments-trigger').length).toBe(1);
	expect(choice.querySelectorAll('fieldset')).toHaveLength(1);
	expect(readChoiceResponse(choice).selected[0]?.optionId).toBe('layout');
	await fixtureLocator(frame, choice).hover();
	await frame.getByRole('button', { name: 'Open Which scope? in a sidebar', exact: true }).click();
	await expect
		.poll((): number => fixture.querySelectorAll('.ex-drawer ex-choice input').length)
		.toBe(2);
	const original = choice.querySelector<HTMLInputElement>('input');
	const copied = fixture.querySelector<HTMLInputElement>('.ex-drawer ex-choice input');
	expect(copied?.disabled).toBe(true);
	expect(copied?.name).not.toBe(original?.name);
	expect(readChoiceResponse(choice).selected[0]?.optionId).toBe('layout');
	await frame.getByRole('button', { name: 'Close', exact: true }).click();
	await fixtureLocator(frame, choice).hover();
	await frame.getByRole('button', { name: 'Expand Which scope?', exact: true }).click();
	await expect
		.element(frame.getByRole('dialog', { name: 'Which scope?', exact: true }))
		.toBeVisible();
	await userEvent.keyboard('{Escape}');
	expect(choice.querySelector('input')).toBe(original);
	expect(readChoiceResponse(choice).selected[0]?.optionId).toBe('layout');
	expect(fixture.querySelectorAll('.ex-comments-trigger')).toHaveLength(1);
	let responseChanges = 0;
	let reviewChanges = 0;
	choice.addEventListener('choice-change', (): void => {
		responseChanges += 1;
	});
	comments.addEventListener('review-change', (): void => {
		reviewChanges += 1;
	});
	await fixtureLocator(frame, requiredElement(fixture, '#technical .ex-choice-summary')).click();
	expect(responseChanges).toBe(1);
	expect(reviewChanges).toBe(1);
});

test('a second independently authored page uses the same response and local review components', async (): Promise<void> => {
	const { document: fixture, frame } = await openReviewFixture('independent.html');
	await frame.getByRole('checkbox', { name: 'Source', exact: true }).click();
	expect(readCurrentReview(fixture).answers[0]?.selected).toEqual([
		{ optionId: 'include-source', label: 'Source' },
	]);
	expect(readCurrentReview(fixture).documentId).toBe('demistify-independent-review');
});

test.each([1600, 390])(
	'a non-modal bottom workspace keeps its comment composer usable at %ipx',
	async (width): Promise<void> => {
		// Arrange: the live document owns both the workspace content and its comment target.
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openReviewFixture();
		const root = requiredElement(fixture, 'ex-document');
		const trigger = fixture.createElement('button');
		trigger.textContent = 'Open workspace';
		const workspace = fixture.createElement('ex-drawer');
		workspace.setAttribute('label', 'Related documents');
		workspace.innerHTML =
			'<header class="ex-inspect-header"><strong>Related documents</strong><button type="button">Close workspace</button></header><div class="ex-inspect-body"><p id="workspace-rule">Background actions stay blocked.</p></div>';
		trigger.addEventListener('click', (): void => workspace.setAttribute('open', ''));
		workspace
			.querySelector('button')
			?.addEventListener('click', (): void => workspace.removeAttribute('open'));
		const enlargement = fixture.createElement('ex-card');
		enlargement.setAttribute('label', 'Workspace host');
		enlargement.setAttribute('zoom', '');
		enlargement.append(trigger, workspace);
		root.append(enlargement);

		// Act: use the visible workspace and its ordinary Comments control.
		await fixtureLocator(frame, enlargement).hover();
		await frame.getByRole('button', { name: 'Expand Workspace host', exact: true }).click();
		await fixtureLocator(frame, trigger).click();
		await expect.poll((): boolean => workspace.matches(':popover-open')).toBe(true);
		expect(fixture.querySelector('dialog:modal')).toBeNull();
		expect(fixture.defaultView?.getComputedStyle(workspace).animationName).toBe('ex-drawer-enter');
		await expect
			.poll((): boolean => !!workspace.querySelector('#workspace-rule > [data-comment-control]'))
			.toBe(true);
		const workspaceMarker = workspace.querySelector<HTMLButtonElement>(
			'#workspace-rule > button[data-comment-control]',
		);
		if (!workspaceMarker) throw new Error('Missing workspace comment button');
		workspaceMarker.focus();
		await userEvent.keyboard('{Enter}');
		await frame
			.getByRole('textbox', { name: 'Your comment', exact: true })
			.fill('Check the pointer path.');

		// Assert: Comments has real editable focus, while the same workspace remains open.
		expect(fixture.activeElement?.getAttribute('aria-label')).toBe('Your comment');
		expect(
			fixture.querySelector<HTMLTextAreaElement>('textarea[aria-label="Your comment"]')?.value,
		).toBe('Check the pointer path.');
		expect(workspace.matches(':popover-open')).toBe(true);
		expect(root.contains(requiredElement(workspace, '#workspace-rule'))).toBe(true);
		await frame.getByRole('button', { name: 'Add comment', exact: true }).click();
		expect(readCurrentReview(fixture).comments).toEqual([
			expect.objectContaining({
				body: 'Check the pointer path.',
				anchor: {
					kind: 'entity',
					targetId: 'workspace-rule',
					label: 'Background actions stay blocked.',
				},
			}),
		]);
		const commentsPanel = requiredElement(fixture, '.ex-comments-body').parentElement;
		if (!commentsPanel) throw new Error('Comments panel is missing.');
		expect(fixture.defaultView?.getComputedStyle(commentsPanel).animationName).toBe(
			'ex-drawer-enter',
		);
		await commands.setReducedMotion('reduce');
		expect(fixture.defaultView?.getComputedStyle(workspace).animationName).toBe('none');
		expect(fixture.defaultView?.getComputedStyle(commentsPanel).animationName).toBe('none');
		if (width >= 900) {
			const comments = requiredElement(fixture, 'aside.ex-drawer');
			expect(workspace.getBoundingClientRect().right).toBeLessThanOrEqual(
				comments.getBoundingClientRect().left + 1,
			);
		}
		fixture.querySelector<HTMLButtonElement>('button[aria-label="Close comments"]')?.focus();
		expect(fixture.activeElement?.getAttribute('aria-label')).toBe('Close comments');
		await userEvent.keyboard('{Escape}');
		expect(fixture.querySelector('.ex-comments-body')?.parentElement?.hidden ?? true).toBe(true);
		expect(workspace.matches(':popover-open')).toBe(true);
		await frame.getByRole('button', { name: 'Close workspace', exact: true }).click();
		expect(workspace.matches(':popover-open')).toBe(false);
		expect(fixture.activeElement).toBe(trigger);
	},
);
