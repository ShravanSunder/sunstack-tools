import { afterEach, beforeEach, expect, test } from 'vitest';
import { commands, page, userEvent } from 'vitest/browser';

import { commentStorageKey } from '../../../../packages/demistify/src/core/comment-store.ts';
import { fixtureLocator, openFixture, requiredElement } from './fixture-page.ts';
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
		await frame.getByRole('button', { name: 'Close comments', exact: true }).click();
		expect(workspace.matches(':popover-open')).toBe(true);
		await frame.getByRole('button', { name: 'Close workspace', exact: true }).click();
		expect(workspace.matches(':popover-open')).toBe(false);
		expect(fixture.activeElement).toBe(trigger);
	},
);

test.each([1440, 390])(
	'the built requirements workspace keeps calls, Comments and related documents connected at %ipx',
	async (width): Promise<void> => {
		// Arrange: load the prepared collection and the actual application controller.
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture(
			'understanding',
			'',
			'requirements.html?need=U2&stage=program-design',
		);
		await frame.getByRole('button', { name: 'Open call stack', exact: true }).click();
		const workspace = requiredElement(fixture, 'ex-drawer.lab-related-drawer');
		await expect.poll((): boolean => workspace.matches(':popover-open')).toBe(true);
		// Geometry is asserted after the shared entrance transition reaches its final position.
		await expect
			.poll((): number => workspace.getBoundingClientRect().bottom)
			.toBeCloseTo(fixture.defaultView?.innerHeight ?? 0, 0);
		const bounds = workspace.getBoundingClientRect();
		expect(bounds.top).toBeGreaterThan(0);
		expect(bounds.top).toBeCloseTo(
			requiredElement(fixture, '.ex-document-toolbar').getBoundingClientRect().bottom,
			0,
		);
		expect(bounds.bottom).toBeCloseTo(fixture.defaultView?.innerHeight ?? 0, 0);
		expect(bounds.left).toBeGreaterThanOrEqual(0);
		expect(new URL(fixture.URL).searchParams.get('need')).toBe('U2');
		expect(bounds.right).toBeLessThanOrEqual(width);
		fixture.documentElement.dataset['theme'] = 'dark';
		const darkFill = fixture.defaultView?.getComputedStyle(workspace).backgroundColor;
		const darkDocument = fixture.defaultView?.getComputedStyle(
			requiredElement(fixture, '.lab-stage-record'),
		).backgroundColor;
		expect(
			fixture.defaultView?.getComputedStyle(requiredElement(fixture, '.lab-document-header'))
				.backgroundColor,
		).toBe(darkDocument);
		const actionFill = fixture.defaultView?.getComputedStyle(
			requiredElement(fixture, '.lab-document-actions button'),
		).backgroundColor;
		expect(actionFill).not.toBe(darkDocument);
		expect(
			fixture.defaultView?.getComputedStyle(requiredElement(workspace, '.lab-related-header'))
				.backgroundColor,
		).toBe(actionFill);
		fixture.documentElement.dataset['theme'] = 'light';
		expect(fixture.defaultView?.getComputedStyle(workspace).backgroundColor).not.toBe(darkFill);

		// Act: choose a real call, inspect its narrow sheet, and navigate by keyboard.
		const firstCall = requiredElement(workspace, 'button[data-frame="0"]');
		await fixtureLocator(frame, firstCall).click();
		if (width < 900)
			await frame.getByRole('button', { name: 'Close call details', exact: true }).click();
		await userEvent.keyboard('{ArrowDown}');
		expect(workspace.querySelector('button[data-frame="1"]')?.getAttribute('aria-pressed')).toBe(
			'true',
		);
		expect(new URL(fixture.URL).searchParams.get('need')).toBe('U2');
		expect(new URL(fixture.URL).searchParams.has('stack')).toBe(false);
		expect(new URL(fixture.URL).searchParams.has('call')).toBe(false);
		await fixtureLocator(
			frame,
			requiredElement(workspace, '[data-call-index="1"] [data-comment-control]'),
		).click();
		await frame
			.getByRole('textbox', { name: 'Your comment', exact: true })
			.fill('Inspect the routing boundary.');

		// Assert: commenting does not replace the workspace or lose U2's document context.
		expect(fixture.activeElement?.getAttribute('aria-label')).toBe('Your comment');
		expect(workspace.matches(':popover-open')).toBe(true);
		await frame.getByRole('button', { name: 'Close comments', exact: true }).click();
		await frame.getByRole('button', { name: 'Back to Program Design', exact: true }).click();
		await expect
			.element(frame.getByRole('button', { name: 'Program Design', exact: true }))
			.toHaveAttribute('aria-pressed', 'true');
		const viewButtons = [...fixture.querySelectorAll('.lab-document-switcher button')];
		const widths = viewButtons.map((button): number => button.getBoundingClientRect().width);
		expect(Math.max(...widths) - Math.min(...widths)).toBeLessThan(1);
		expect(fixture.querySelectorAll('.lab-document-switcher [aria-pressed="true"]')).toHaveLength(
			1,
		);
		expect(fixture.querySelector('.lab-document-subject small')?.textContent).toBe(
			'U2 · Input ownership',
		);
		await frame.getByRole('button', { name: 'View together', exact: true }).click();
		expect(new URL(fixture.URL).searchParams.get('reading')).toBe('documents');
		const comparison = requiredElement(fixture, 'ex-drawer.lab-related-drawer');
		expect(comparison.matches(':popover-open')).toBe(true);
		expect(comparison.textContent).toContain('S2 · Blocked background actions');
		await fixtureLocator(
			frame,
			requiredElement(
				comparison,
				'.lab-expanded-design .lab-related-record p [data-comment-control]',
			),
		).click();
		await frame
			.getByRole('textbox', { name: 'Your comment', exact: true })
			.fill('Keep the declared design gap visible.');
		expect(fixture.activeElement?.getAttribute('aria-label')).toBe('Your comment');
		expect(comparison.matches(':popover-open')).toBe(true);
	},
);

test.each([1440, 390])(
	'the document reader keeps source links, expanded proof and call context at %ipx',
	async (width): Promise<void> => {
		// Arrange: the related workspace has one document view, without an experiment selector.
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture(
			'understanding',
			'',
			'requirements.html?need=U2&reading=documents',
		);
		await expect
			.poll((): boolean => !!fixture.querySelector('ex-drawer.lab-related-drawer'))
			.toBe(true);
		const workspace = requiredElement(fixture, 'ex-drawer.lab-related-drawer');
		await expect.poll((): boolean => workspace.matches(':popover-open')).toBe(true);
		expect(workspace.querySelector('.lab-reading-experiment')).toBeNull();
		expect(workspace.querySelector('.lab-expanded-grid')).not.toBeNull();
		if (width < 760) {
			const designCard = requiredElement(workspace, '.lab-expanded-design').getBoundingClientRect();
			const proofCard = requiredElement(workspace, '.lab-expanded-proof').getBoundingClientRect();
			expect(designCard.width).toBeGreaterThan(width * 0.8);
			expect(proofCard.left).toBeCloseTo(designCard.left, 0);
			expect(proofCard.top).toBeGreaterThanOrEqual(designCard.bottom);
			expect(
				requiredElement(workspace, '.lab-specification-cases').getBoundingClientRect().width,
			).toBeGreaterThan(width * 0.8);
		}

		// Act: reveal the design's source locally, then inspect its call browser.
		await fixtureLocator(frame, requiredElement(workspace, 'a[href="#related-record-C1"]')).click();
		const design = requiredElement(workspace, '#related-record-C1');
		expect(design.closest('details')?.open).toBe(true);
		expect(design.textContent).toContain('Pointer entry, menu ownership');
		expect(fixture.activeElement).toBe(design);
		await fixtureLocator(
			frame,
			requiredElement(workspace, '.lab-expanded-design [data-action="open-calls"]'),
		).click();
		await fixtureLocator(frame, requiredElement(workspace, 'button[data-frame="0"]')).click();
		if (width < 900)
			await frame.getByRole('button', { name: 'Close call details', exact: true }).click();
		expect(new URL(fixture.URL).searchParams.get('need')).toBe('U2');
		expect(new URL(fixture.URL).searchParams.get('reading')).toBe('documents');
		await frame.getByRole('button', { name: 'Back to related documents', exact: true }).click();

		// Assert: return retains the same need; each source link reveals its own record.
		expect(workspace.querySelector('.lab-expanded-grid')).not.toBeNull();
		await fixtureLocator(frame, requiredElement(workspace, 'a[href="#related-record-V2"]')).click();
		const pointer = requiredElement(workspace, '#related-record-V2');
		expect(fixture.activeElement).toBe(pointer);
		expect(pointer.textContent).toContain('Example run: the background action fired.');
		const proofSteps = [
			...pointer.querySelectorAll<HTMLDetailsElement>('ex-trace > ol > li > details'),
		];
		expect(proofSteps).toHaveLength(3);
		expect(proofSteps.map((step) => step.open)).toEqual([true, true, true]);
		expect(pointer.querySelector<HTMLDetailsElement>('.lab-proof-design > details')?.open).toBe(
			false,
		);
		const keyboard = requiredElement(workspace, '#related-record-V2-keyboard');
		expect(keyboard.textContent).toContain('Not run');
		await fixtureLocator(
			frame,
			requiredElement(workspace, 'a[href="#related-record-V2-keyboard"]'),
		).click();
		expect(fixture.activeElement).toBe(keyboard);
		const readingBounds = requiredElement(
			workspace,
			'.lab-related-content',
		).getBoundingClientRect();
		expect(keyboard.getBoundingClientRect().top).toBeGreaterThanOrEqual(readingBounds.top);
		expect(
			requiredElement(keyboard, '.lab-related-record-title').getBoundingClientRect().bottom,
		).toBeLessThanOrEqual(readingBounds.bottom);
		await fixtureLocator(
			frame,
			requiredElement(pointer, '.lab-example-outcome p [data-comment-control]'),
		).click();
		await frame
			.getByRole('textbox', { name: 'Your comment', exact: true })
			.fill('Which target produced this example?');
		expect(fixture.activeElement?.getAttribute('aria-label')).toBe('Your comment');
		expect(workspace.matches(':popover-open')).toBe(true);
		await frame.getByRole('button', { name: 'Close comments', exact: true }).click();
		await frame.getByRole('button', { name: 'Close related documents', exact: true }).click();
		expect(new URL(fixture.URL).searchParams.has('reading')).toBe(false);
		expect(fixture.querySelector('.lab-document-subject small')?.textContent).toBe(
			'U2 · Input ownership',
		);
	},
);

test('removing the document browser closes its workspace and clears its opening hint', async (): Promise<void> => {
	// Arrange: open the real workspace inside the owning document component.
	await page.viewport(1440, 1000);
	const { document: fixture, frame } = await openFixture(
		'understanding',
		'',
		'requirements.html?need=U2',
	);
	await frame.getByRole('button', { name: 'View together', exact: true }).click();
	expect(new URL(fixture.URL).searchParams.get('reading')).toBe('documents');
	const workspace = requiredElement(fixture, 'ex-drawer.lab-related-drawer');

	// Act: exercise the browser's real DOM disconnect lifecycle, as a host replacement does.
	requiredElement(fixture, 'lab-requirement-browser').remove();

	// Assert: the closing hint is removed without rewriting the current requirement.
	expect(workspace.matches(':popover-open')).toBe(false);
	expect(new URL(fixture.URL).searchParams.has('reading')).toBe(false);
	expect(new URL(fixture.URL).searchParams.get('need')).toBe('U2');
});

test.each(['specification', 'program-design', 'verification'])(
	'a standalone document page opens call details from its narrow workspace: %s',
	async (documentPage): Promise<void> => {
		// Arrange: use the real stage page, including its own stylesheet bindings.
		await page.viewport(390, 1000);
		const { document: fixture, frame } = await openFixture(
			'understanding',
			'',
			`${documentPage}.html?need=U2`,
		);
		await frame.getByRole('button', { name: 'Program Design', exact: true }).click();
		await frame.getByRole('button', { name: 'Open call stack', exact: true }).click();
		const workspace = requiredElement(fixture, 'ex-drawer.lab-related-drawer');

		// Act: select a call through the same control used on the Requirements page.
		await fixtureLocator(frame, requiredElement(workspace, 'button[data-frame="0"]')).click();
		const detail = requiredElement(workspace, 'ex-drawer.lab-call-detail-sheet');
		await expect.poll((): boolean => detail.matches(':popover-open')).toBe(true);
		expect(detail.textContent).toContain('handleKey');

		// Assert: the compact detail sheet and its parent close independently.
		await frame.getByRole('button', { name: 'Close call details', exact: true }).click();
		expect(workspace.matches(':popover-open')).toBe(true);
		await frame.getByRole('button', { name: 'Back to Program Design', exact: true }).click();
		expect(fixture.querySelector('ex-drawer.lab-related-drawer')).toBeNull();
		expect(fixture.querySelector('.lab-document-subject small')?.textContent).toBe(
			'U2 · Input ownership',
		);
	},
);
