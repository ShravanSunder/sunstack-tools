import { afterEach, beforeEach, expect, test } from 'vitest';
import { commands, page, userEvent } from 'vitest/browser';

import { commentStorageKey } from '../../../../packages/demistify/src/core/comment-store.ts';
import { fixtureLocator, requiredElement } from './fixture-page.ts';
import { readChoiceResponse, readCurrentReview, openReviewFixture } from './review-fixture.ts';

const identity = { documentId: 'demistify-choice-fixture', documentRevision: 'r1' };
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

test('reopening a node edits its saved comment instead of adding another', async (): Promise<void> => {
	const { document: fixture, frame } = await openReviewFixture();
	const node = requiredElement(fixture, '#layout > [data-comment-control]');
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

test('choice typography and title-adjacent Details use the real native inspector and preserve the outline', async (): Promise<void> => {
	const { document: fixture, frame } = await openReviewFixture();
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
	'Comments and Details replace each other through the native owner at width %i',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openReviewFixture();
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
		const enlargement = fixture.createElement('ex-zoom');
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
		await fixtureLocator(
			frame,
			requiredElement(workspace, '#workspace-rule > [data-comment-control]'),
		).click();
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
