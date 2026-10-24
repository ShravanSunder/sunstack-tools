import { afterEach, beforeEach, expect, test } from 'vitest';
import { commands, page, userEvent } from 'vitest/browser';

import { commentStorageKey } from '../../../../packages/demistify/src/core/comment-store.ts';
import { assignHeadingFragments } from '../../../../packages/demistify/src/core/heading-fragments.ts';
import { fixtureLocator, openFixture, requiredElement } from './fixture-page.ts';
import {
	openBoundaryReviewFixture,
	readChoiceResponse,
	readCurrentReview,
	textCaretRect,
} from './review-fixture.ts';

beforeEach((): void => {
	localStorage.removeItem(
		commentStorageKey({ documentId: 'demistify-review-boundary-fixture', documentRevision: 'r1' }),
	);
});
afterEach(async (): Promise<void> => {
	document.body.replaceChildren();
	await commands.setReducedMotion('no-preference');
	await page.viewport(1600, 1000);
});

test('detached Markdown finishes with local duplicate and connected heading collisions resolved', async (): Promise<void> => {
	const { document: fixture } = await openBoundaryReviewFixture();
	const occupied = fixture.createElement('h3');
	occupied.id = 'detached-notes--repeat';
	occupied.textContent = 'Authored heading';
	fixture.body.append(occupied);
	const markdown = fixture.createElement('ex-markdown');
	markdown.id = 'detached-notes';
	const source = fixture.createElement('script');
	source.type = 'text/markdown';
	source.textContent = '### Repeat\n\nFirst.\n\n### Repeat\n\nSecond.';
	markdown.append(source);
	fixture.body.append(markdown);
	markdown.remove();
	await expect.poll((): boolean => markdown.hasAttribute('prepared')).toBe(true);
	expect([...markdown.querySelectorAll('h3')].map((heading): string => heading.id)).toEqual([
		'detached-notes--repeat-2',
		'detached-notes--repeat-3',
	]);
	expect(occupied.id).toBe('detached-notes--repeat');
	const retained = fixture.createElement('section');
	retained.id = 'heading-repeat';
	retained.innerHTML =
		'<h3>Repeat</h3><span id="heading-repeat-2">Authored block</span><h3 id="heading-repeat-3">Authored heading</h3><h3>Repeat</h3><a href="#repeat">Read repeat</a>';
	assignHeadingFragments(retained);
	expect([...retained.querySelectorAll('h3')].map((heading): string => heading.id)).toEqual([
		'heading-repeat-4',
		'heading-repeat-3',
		'heading-repeat-5',
	]);
	expect(retained.querySelector('span')?.id).toBe('heading-repeat-2');
	expect(retained.id).toBe('heading-repeat');
	expect(retained.querySelector('a')?.getAttribute('href')).toBe('#heading-repeat-4');
});

test('a live comment draft and focused editor survive both sidebar presentation thresholds', async (): Promise<void> => {
	const { document: fixture, frame } = await openBoundaryReviewFixture();
	await fixtureLocator(
		frame,
		requiredElement(fixture, '#technical > [data-comment-control]'),
	).click();
	await frame.getByRole('textbox', { name: 'Your comment', exact: true }).fill('Keep this draft.');
	const editor = fixture.querySelector<HTMLTextAreaElement>('textarea[aria-label="Your comment"]');
	if (!editor) throw new Error('Comment editor is absent.');
	await page.viewport(390, 900);
	await expect.element(frame.getByRole('dialog', { name: 'Comments', exact: true })).toBeVisible();
	expect(fixture.querySelector('textarea[aria-label="Your comment"]')).toBe(editor);
	expect(fixture.activeElement).toBe(editor);
	expect(editor.value).toBe('Keep this draft.');
	await page.viewport(1600, 1000);
	await expect
		.element(frame.getByRole('complementary', { name: 'Comments', exact: true }))
		.toBeVisible();
	expect(fixture.querySelector('dialog[open]')).toBeNull();
	expect(fixture.activeElement).toBe(editor);
	expect(editor.value).toBe('Keep this draft.');
	await userEvent.keyboard('{Meta>}{Enter}{/Meta}');
	expect(readCurrentReview(fixture).comments[0]?.body).toBe('Keep this draft.');
	await frame.getByRole('button', { name: 'Close comments', exact: true }).click();
	expect(fixture.activeElement?.getAttribute('aria-label')).toBe('Comment on Technical scope (1)');
});

test('generated disclosure controls stay out of cross-block text quotes and canonical anchors', async (): Promise<void> => {
	const { document: fixture, frame } = await openBoundaryReviewFixture();
	const first = requiredElement(fixture, '#review-now .ex-choice-summary p');
	const last = requiredElement(fixture, '#first-passage');
	first.scrollIntoView({ block: 'center' });
	const start = textCaretRect(first, 0);
	const end = textCaretRect(last, 17);
	await commands.dragReviewSelection({
		startX: start.left + 0.5,
		startY: start.top + start.height / 2,
		endX: end.left,
		endY: end.top + end.height / 2,
	});
	await frame.getByRole('button', { name: 'Comment on selected text', exact: true }).click();
	await frame
		.getByRole('textbox', { name: 'Your comment', exact: true })
		.fill('Across a local disclosure.');
	await frame.getByRole('button', { name: 'Add comment', exact: true }).click();
	const anchor = readCurrentReview(fixture).comments[0]?.anchor;
	if (anchor?.kind !== 'text') throw new Error('No text anchor was saved.');
	expect(anchor.sourceText).toContain('Resolve the bounded question today.');
	expect(anchor.selectedText).toContain('The first passage');
	expect(anchor.sourceText).not.toContain('Details');
	expect(anchor.selectedText).not.toContain('Details');
});

test('a real cross-block drag saves the exact quote on the document and refuses changed source', async (): Promise<void> => {
	const { document: fixture, frame } = await openBoundaryReviewFixture();
	const first = requiredElement(fixture, '#first-passage');
	const second = requiredElement(fixture, '#second-passage');
	first.scrollIntoView({ block: 'center' });
	const start = textCaretRect(first, 4);
	const end = textCaretRect(second, 18);
	await commands.dragReviewSelection({
		startX: start.left + 0.5,
		startY: start.top + start.height / 2,
		endX: end.left,
		endY: end.top + end.height / 2,
	});
	const quote = 'first passage keeps the original source visible.\n\nThe second passage';
	await expect
		.poll((): string => fixture.defaultView?.getSelection()?.toString() ?? '')
		.toBe(quote);
	await frame.getByRole('button', { name: 'Comment on selected text', exact: true }).click();
	await frame
		.getByRole('textbox', { name: 'Your comment', exact: true })
		.fill('Keep both passages together.');
	await frame.getByRole('button', { name: 'Add comment', exact: true }).click();
	const anchor = readCurrentReview(fixture).comments[0]?.anchor;
	expect(anchor?.kind).toBe('text');
	expect(anchor?.targetId).toBe('content');
	if (anchor?.kind !== 'text') throw new Error('No text anchor was saved.');
	expect(anchor.selectedText).toBe(quote);
	expect(anchor.sourceText).toBe(
		'first passage keeps the original source visible.\n  The second passage',
	);
	expect(readCurrentReview(fixture).answers).toEqual([]);
	first.textContent = 'The source has changed.';
	await frame
		.getByRole('button', { name: 'Show anchor: Choose the review scope', exact: true })
		.click();
	await expect
		.element(frame.getByText('The selected text or its context has changed.', { exact: true }))
		.toBeVisible();
	expect(readCurrentReview(fixture).comments[0]?.anchor).toEqual(anchor);
});

test('dragging actual card body text creates an option anchor without selecting its answer', async (): Promise<void> => {
	const { document: fixture, frame } = await openBoundaryReviewFixture();
	const passage = requiredElement(fixture, '#layout .ex-choice-summary li');
	passage.scrollIntoView({ block: 'center' });
	const start = textCaretRect(passage, 0);
	const end = textCaretRect(passage, 19);
	await commands.dragReviewSelection({
		startX: start.left + 0.5,
		startY: start.top + start.height / 2,
		endX: end.left,
		endY: end.top + end.height / 2,
	});
	await expect
		.poll((): string => fixture.defaultView?.getSelection()?.toString() ?? '')
		.toBe('Desktop containment');
	expect(readChoiceResponse(requiredElement(fixture, '#scope-question')).selected).toEqual([]);
	await frame.getByRole('button', { name: 'Comment on selected text', exact: true }).click();
	await frame
		.getByRole('textbox', { name: 'Your comment', exact: true })
		.fill('Preserve this item.');
	await frame.getByRole('button', { name: 'Add comment', exact: true }).click();
	const anchor = readCurrentReview(fixture).comments[0]?.anchor;
	expect(anchor?.kind).toBe('text');
	expect(anchor?.targetId).toBe('layout');
	if (anchor?.kind !== 'text') throw new Error('No passage anchor was saved.');
	expect(anchor.selectedText).toBe('Desktop containment');
	expect(readCurrentReview(fixture).answers).toEqual([]);
});

test('choice-only and reviewed documents share the ordinary 8px phone gutter', async (): Promise<void> => {
	await page.viewport(390, 900);
	const { document: fixture, frame } = await openFixture('review', '', 'choice-only.html');
	await expect.poll((): number => fixture.querySelectorAll('ex-choice input').length).toBe(2);
	expect(fixture.defaultView?.getComputedStyle(fixture.body).paddingLeft).toBe('8px');
	expect(fixture.defaultView?.getComputedStyle(fixture.body).paddingRight).toBe('8px');
	expect(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(391);
	expect(fixture.querySelector('.ex-comments-trigger')).toBeNull();
	await frame.getByRole('radio', { name: 'Concise', exact: true }).click();
	expect(
		readChoiceResponse(requiredElement(fixture, '#format-question')).selected[0]?.optionId,
	).toBe('concise-format');
});

test('editable shortcuts keep native newlines and unified manual-copy/export content retains identities', async (): Promise<void> => {
	const { document: fixture, frame } = await openBoundaryReviewFixture();
	await frame.getByRole('textbox', { name: 'Scratch note', exact: true }).click();
	await userEvent.keyboard('c');
	expect(requiredElement(fixture, '.ex-comments-trigger').getAttribute('aria-expanded')).toBe(
		'false',
	);
	await fixtureLocator(frame, requiredElement(fixture, '#layout .ex-choice-summary li')).click();
	await fixtureLocator(
		frame,
		requiredElement(fixture, '#technical > [data-comment-control]'),
	).click();
	await frame.getByRole('textbox', { name: 'Your comment', exact: true }).click();
	await userEvent.keyboard('First{Enter}Second{Shift>}{Enter}{/Shift}Third');
	expect(readCurrentReview(fixture).comments).toEqual([]);
	await userEvent.keyboard('{Meta>}{Enter}{/Meta}');
	const saved = readCurrentReview(fixture);
	expect(saved.comments[0]?.body).toBe('First\nSecond\nThird');
	const navigator = fixture.defaultView?.navigator;
	if (!navigator) throw new Error('Fixture navigator is unavailable.');
	Object.defineProperty(navigator, 'clipboard', {
		configurable: true,
		value: {
			writeText: async (): Promise<never> => {
				throw new Error('Clipboard denied');
			},
		},
	});
	await frame.getByRole('button', { name: 'Copy review', exact: true }).click();
	const manual = frame.getByRole('textbox', { name: 'Review for manual copy', exact: true });
	await expect.element(manual).toBeVisible();
	const manualText =
		fixture.querySelector<HTMLTextAreaElement>('textarea[aria-label="Review for manual copy"]')
			?.value ?? '';
	for (const text of [
		'demistify-review-boundary-fixture',
		'Document revision: r1',
		`Review ID: ${saved.reviewId}`,
		`Comment ID: ${saved.comments[0]?.commentId}`,
		'Target ID: technical',
		'Question ID: scope-question',
		'Layout only (layout)',
		'First\nSecond\nThird',
	])
		expect(manualText).toContain(text);
	const view = fixture.defaultView;
	if (!view) throw new Error('Fixture window is unavailable.');
	let exported: Blob | undefined;
	Object.defineProperty(view.URL, 'createObjectURL', {
		configurable: true,
		value: (blob: Blob): string => {
			exported = blob;
			return 'blob:review-test-unavailable';
		},
	});
	await frame.getByRole('button', { name: 'Export review', exact: true }).click();
	if (!exported) throw new Error('Export did not create a review Blob.');
	expect(JSON.parse(await exported.text())).toEqual(saved);
});

test('phone anchor reveal dismisses Comments, missing targets stay honest, and old disconnect cannot close new Details', async (): Promise<void> => {
	await page.viewport(390, 900);
	const { document: fixture, frame } = await openBoundaryReviewFixture();
	const context = requiredElement(fixture, '#context');
	context.scrollIntoView({ block: 'center' });
	await fixtureLocator(frame, requiredElement(context, ':scope > [data-comment-control]')).click();
	await frame.getByRole('textbox', { name: 'Your comment', exact: true }).fill('Context feedback');
	await frame.getByRole('button', { name: 'Add comment', exact: true }).click();
	await frame.getByRole('button', { name: 'Show anchor: Review context', exact: true }).click();
	expect(fixture.querySelector('dialog[open]')).toBeNull();
	await userEvent.keyboard('c');
	context.remove();
	await expect.poll((): boolean => context.isConnected).toBe(false);
	await frame.getByRole('button', { name: 'Show anchor: Review context', exact: true }).click();
	await expect
		.element(frame.getByText('The original item is not on this page.', { exact: true }))
		.toBeVisible();
	// Direct component integration covers the behind-modal replacement edge.
	const details = fixture.querySelector<HTMLButtonElement>('#technical .ex-choice-details-button');
	if (!details) throw new Error('Details is absent.');
	details.click();
	const comments = requiredElement(fixture, 'ex-comments');
	comments.remove();
	await expect
		.element(frame.getByRole('dialog', { name: 'Technical scope', exact: true }))
		.toBeVisible();
	await frame.getByRole('button', { name: 'Close', exact: true }).click();
	expect(fixture.activeElement?.isConnected).toBe(true);
});

test.each([1600, 390])(
	'fullscreen restores active Comments before borrowing original choice state at width %i',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openBoundaryReviewFixture();
		const choice = requiredElement(fixture, '#scope-question');
		await fixtureLocator(frame, requiredElement(fixture, '#layout .ex-choice-summary li')).click();
		await frame.getByRole('button', { name: 'Comments (0)', exact: true }).click();
		choice.scrollIntoView({ block: 'center' });
		if (width < 900) {
			// Component integration reaches the modal handoff; native pointer input is inert behind the sheet.
			const expand = choice.querySelector<HTMLButtonElement>(
				'button[aria-label="Expand Which scope?"]',
			);
			if (!expand) throw new Error('Expand control is absent.');
			expand.click();
		} else {
			await fixtureLocator(frame, choice).hover();
			await frame.getByRole('button', { name: 'Expand Which scope?', exact: true }).click();
		}
		await expect
			.element(frame.getByRole('dialog', { name: 'Which scope?', exact: true }))
			.toBeVisible();
		expect(requiredElement(fixture, '.ex-comments-trigger').getAttribute('aria-expanded')).toBe(
			'false',
		);
		await userEvent.keyboard('{Escape}');
		expect(readChoiceResponse(choice).selected[0]?.optionId).toBe('layout');
		if (width >= 900)
			expect(fixture.activeElement?.getAttribute('aria-label')).toBe('Expand Which scope?');
		else expect(fixture.activeElement?.isConnected).toBe(true);
	},
);

test('malformed declarations reject visibly without another Comments authority', async (): Promise<void> => {
	const { document: fixture, frame } = await openBoundaryReviewFixture();
	const invalid = fixture.createElement('ex-choice');
	invalid.id = 'bad-choice';
	invalid.setAttribute('title', 'Bad input');
	invalid.innerHTML =
		'<ex-option id="duplicate-option" title="First"><template slot="summary">First.</template></ex-option><ex-option id="duplicate-option" title="Second"><template slot="summary">Second.</template></ex-option>';
	requiredElement(fixture, '#content').append(invalid);
	await expect
		.element(frame.getByRole('alert').filter({ hasText: 'Choice could not load' }))
		.toBeVisible();
	const duplicate = fixture.createElement('ex-comments');
	duplicate.setAttribute('for', 'content');
	fixture.body.append(duplicate);
	await expect
		.element(frame.getByRole('alert').filter({ hasText: 'only one Comments surface' }))
		.toBeVisible();
	const unidentified = fixture.createElement('ex-document');
	unidentified.id = 'unidentified';
	fixture.body.append(unidentified);
	const missingIdentity = fixture.createElement('ex-comments');
	missingIdentity.setAttribute('for', 'unidentified');
	fixture.body.append(missingIdentity);
	await expect
		.poll((): number => missingIdentity.querySelectorAll('[role="alert"]').length)
		.toBe(1);
	expect(fixture.querySelectorAll('.ex-comments-trigger')).toHaveLength(1);
});

test('reconnecting one Comments instance under another revision rejects scope mixing', async (): Promise<void> => {
	const { document: fixture, frame } = await openBoundaryReviewFixture();
	await fixtureLocator(
		frame,
		requiredElement(fixture, '#technical > [data-comment-control]'),
	).click();
	await frame
		.getByRole('textbox', { name: 'Your comment', exact: true })
		.fill('Original revision comment.');
	await frame.getByRole('button', { name: 'Add comment', exact: true }).click();
	const saved = readCurrentReview(fixture);
	const comments = requiredElement(fixture, 'ex-comments');
	comments.remove();
	requiredElement(fixture, '#content').setAttribute('revision', 'r2');
	fixture.body.append(comments);
	await expect
		.element(
			frame.getByRole('alert').filter({ hasText: 'cannot change document identity or revision' }),
		)
		.toBeVisible();
	expect(fixture.querySelectorAll('.ex-comments-trigger')).toHaveLength(0);
	expect(readCurrentReview(fixture)).toEqual(saved);
	expect(
		localStorage.getItem(
			commentStorageKey({
				documentId: 'demistify-review-boundary-fixture',
				documentRevision: 'r2',
			}),
		),
	).toBeNull();
});

test.each(['dark', 'light'])(
	'shared reduced motion and usable controls retain annotation roles in %s phone theme',
	async (theme): Promise<void> => {
		await page.viewport(390, 900);
		await commands.setReducedMotion('reduce');
		const { document: fixture, frame } = await openBoundaryReviewFixture();
		if (fixture.documentElement.dataset['theme'] !== theme)
			await frame.getByRole('button', { name: 'Switch light or dark theme', exact: true }).click();
		await fixtureLocator(frame, requiredElement(fixture, '#layout .ex-choice-summary li')).click();
		await fixtureLocator(
			frame,
			requiredElement(fixture, '#technical > [data-comment-control]'),
		).click();
		await frame.getByRole('textbox', { name: 'Your comment', exact: true }).fill('Theme feedback');
		await frame.getByRole('button', { name: 'Add comment', exact: true }).click();
		const panel = requiredElement(fixture, 'dialog[open]');
		expect(fixture.defaultView?.getComputedStyle(panel).animationName).toBe('none');
		expect(panel.getBoundingClientRect().width).toBeLessThanOrEqual(390);
		const trigger = requiredElement(fixture, '.ex-comments-trigger');
		expect(trigger.querySelector('.ex-comment-symbol-filled')).not.toBeNull();
		expect(trigger.getAttribute('aria-label')).toBe('Comments (1)');
		const annotation = fixture.defaultView?.getComputedStyle(trigger).color;
		expect(annotation).toBe(
			fixture.defaultView?.getComputedStyle(
				requiredElement(fixture, '#technical > [data-comment-control]'),
			).color,
		);
		expect(annotation).not.toBe(
			fixture.defaultView?.getComputedStyle(requiredElement(fixture, '#layout')).borderColor,
		);
		await frame.getByRole('button', { name: 'Close comments', exact: true }).click();
		expect(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(391);
	},
);
