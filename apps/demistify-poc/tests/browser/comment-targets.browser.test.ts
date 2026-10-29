import { afterEach, beforeEach, expect, test } from 'vitest';
import { commands, page } from 'vitest/browser';

import { commentStorageKey } from '../../../../packages/demistify/src/core/comment-store.ts';
import { createCommentSnapshot } from '../../../../packages/demistify/src/core/review-model.ts';
import { fixtureLocator, openFixture, requiredElement } from './fixture-page.ts';
import { readCurrentReview, textCaretRect } from './review-fixture.ts';

beforeEach((): void => {
	localStorage.removeItem(
		commentStorageKey({ documentId: 'demistify-targets-fixture', documentRevision: 'r1' }),
	);
});
afterEach(async (): Promise<void> => {
	document.body.replaceChildren();
	await page.viewport(1600, 1000);
});

test('Code and individual Mermaid nodes retain independent comments instead of sharing the document', async (): Promise<void> => {
	const legacy = {
		commentId: 'existing-page-note',
		anchor: { kind: 'entity', targetId: 'content', label: 'Page feedback' },
		body: 'Keep this existing page note.',
		createdAt: '2026-10-29T10:00:00Z',
		updatedAt: '2026-10-29T10:00:00Z',
	};
	const snapshot = createCommentSnapshot({
		documentId: 'demistify-targets-fixture',
		documentRevision: 'r1',
	});
	localStorage.setItem(
		commentStorageKey(snapshot),
		JSON.stringify({ ...snapshot, comments: [legacy] }),
	);
	const { document: fixture, frame } = await openFixture('review', '', 'targets.html');
	await expect.poll((): boolean => !!fixture.querySelector('#process g.node text')).toBe(true);
	await expect.poll((): number => fixture.querySelectorAll('.ex-comments-trigger').length).toBe(1);
	const code = requiredElement(fixture, 'ex-code');
	expect(code.id).not.toBe('');
	async function savePassage(passage: Element, index: number): Promise<void> {
		passage.scrollIntoView({ block: 'center' });
		// Native SVG word selection may cross adjacent tspans within the same diagram node.
		const svgText = passage.namespaceURI === 'http://www.w3.org/2000/svg';
		const start = svgText ? passage.getBoundingClientRect() : textCaretRect(passage, 0);
		const end = svgText
			? passage.getBoundingClientRect()
			: textCaretRect(passage, passage.firstChild?.textContent?.length ?? 0);
		const hit = fixture.elementFromPoint(start.left + 0.5, start.top + start.height / 2);
		const diagnostics = JSON.stringify({
			bounds: { left: start.left, top: start.top, width: start.width, height: start.height },
			hit: hit?.localName,
			hitNode: hit?.closest('[data-id]')?.getAttribute('data-id'),
			selection: fixture.defaultView?.getComputedStyle(passage).userSelect,
			pointerEvents: fixture.defaultView?.getComputedStyle(passage).pointerEvents,
			viewport: [fixture.defaultView?.innerWidth, fixture.defaultView?.innerHeight],
		});
		if (svgText) await fixtureLocator(frame, passage).dblClick();
		else
			await commands.dragReviewSelection({
				startX: start.left + 0.5,
				startY: start.top + start.height / 2,
				endX: end.left,
				endY: end.top + end.height / 2,
			});
		const selectedText = expect.poll(
			(): string => fixture.defaultView?.getSelection()?.toString().trim() ?? '',
			{ message: diagnostics },
		);
		if (svgText) await selectedText.toContain(passage.textContent?.trim());
		else await selectedText.toBe(passage.textContent?.trim());
		await frame.getByRole('button', { name: 'Comment on selected text', exact: true }).click();
		expect(
			fixture.querySelector<HTMLTextAreaElement>('textarea[aria-label="Your comment"]')?.value,
		).toBe('');
		await frame
			.getByRole('textbox', { name: 'Your comment', exact: true })
			.fill(`Feedback ${index}`);
		await frame.getByRole('button', { name: 'Add comment', exact: true }).click();
		await frame.getByRole('button', { name: 'Close comments', exact: true }).click();
	}
	await savePassage(requiredElement(code, '.ex-code-text:nth-child(5)'), 0);
	await savePassage(requiredElement(fixture, '#process g.node[data-id="request"] text tspan'), 1);
	await savePassage(requiredElement(fixture, '#process g.node[data-id="receipt"] text tspan'), 2);
	const saved = readCurrentReview(fixture).comments;
	expect(saved[0]).toEqual(legacy);
	expect(saved.map((comment): string => comment.anchor.targetId)).toEqual([
		'content',
		code.id,
		'process--request',
		'process--receipt',
	]);
	expect(saved.map((comment): string => comment.body)).toEqual([
		legacy.body,
		'Feedback 0',
		'Feedback 1',
		'Feedback 2',
	]);
	expect(fixture.querySelector('svg [data-comment-control]')).toBeNull();
	document.body.replaceChildren();
	const reopened = await openFixture('review', '', 'targets.html');
	await expect
		.poll((): number => reopened.document.querySelectorAll('.ex-comments-trigger').length)
		.toBe(1);
	expect(readCurrentReview(reopened.document).comments).toEqual(saved);
});

test.each([1600, 390])(
	'editing one comment shows one copy and Cancel restores its saved body at %ipx',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture('review', '', 'targets.html');
		await expect
			.poll((): number => fixture.querySelectorAll('.ex-comments-trigger').length)
			.toBe(1);
		const paragraph = requiredElement(fixture, '#after-views');
		await fixtureLocator(frame, paragraph).hover();
		await fixtureLocator(frame, requiredElement(paragraph, '[data-comment-control]')).click();
		await frame.getByRole('textbox', { name: 'Your comment', exact: true }).fill('Original body');
		await frame.getByRole('button', { name: 'Add comment', exact: true }).click();
		const original = readCurrentReview(fixture).comments[0];
		await frame
			.getByRole('button', {
				name: 'Edit comment on Keep the original source and feedback together.',
				exact: true,
			})
			.click();
		expect(fixture.querySelectorAll('.ex-saved-comment')).toHaveLength(0);
		await frame.getByRole('textbox', { name: 'Your comment', exact: true }).fill('Unsaved body');
		await frame.getByRole('button', { name: 'Cancel', exact: true }).click();
		expect(fixture.querySelectorAll('.ex-saved-comment')).toHaveLength(1);
		expect(readCurrentReview(fixture).comments).toEqual([original]);
	},
);
