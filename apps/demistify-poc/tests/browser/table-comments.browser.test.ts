import { afterEach, beforeEach, expect, test } from 'vitest';
import { commands, page } from 'vitest/browser';

import { commentStorageKey } from '../../../../packages/demistify/src/core/comment-store.ts';
import {
	captureCommentAnchor,
	readReviewTargets,
} from '../../../../packages/demistify/src/core/review-anchors.ts';
import { openFixture, requiredElement } from './fixture-page.ts';
import { readCurrentReview, textCaretRect } from './review-fixture.ts';

beforeEach((): void => {
	localStorage.removeItem(
		commentStorageKey({ documentId: 'demistify-targets-fixture', documentRevision: 'r1' }),
	);
});
afterEach(async (): Promise<void> => {
	document.getSelection()?.removeAllRanges();
	document.body.replaceChildren();
	await page.viewport(1600, 1000);
});

test.each([1600, 390])(
	'paragraph and table-cell highlights retain separate comments and icon-labelled actions can delete one at %ipx',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture('review', '', 'targets.html');
		await expect.poll((): number => fixture.querySelectorAll('#table-notes td').length).toBe(4);
		await expect
			.poll((): number => fixture.querySelectorAll('.ex-comments-trigger').length)
			.toBe(1);
		const paragraph = requiredElement(fixture, '#after-views');
		const firstCell = requiredElement(fixture, '#table-notes tbody tr td');
		const otherCell = requiredElement(fixture, '#table-notes tbody tr + tr td');
		async function commentOn(passage: Element, body: string): Promise<void> {
			passage.scrollIntoView({ block: 'center' });
			const start = textCaretRect(passage, 0);
			const end = textCaretRect(passage, passage.firstChild?.textContent?.length ?? 0);
			await commands.dragReviewSelection({
				startX: start.left + 0.5,
				startY: start.top + start.height / 2,
				endX: end.left,
				endY: end.top + end.height / 2,
			});
			await frame.getByRole('button', { name: 'Comment on selected text', exact: true }).click();
			expect(
				fixture.querySelector<HTMLTextAreaElement>('textarea[aria-label="Your comment"]')?.value,
			).toBe('');
			await frame.getByRole('textbox', { name: 'Your comment', exact: true }).fill(body);
			const submit = requiredElement(fixture, '.ex-comment-save');
			expect(submit.querySelector('svg,i')).not.toBeNull();
			await frame.getByRole('button', { name: 'Add comment', exact: true }).click();
			await frame.getByRole('button', { name: 'Close comments', exact: true }).click();
		}
		await commentOn(paragraph, 'Paragraph feedback');
		await commentOn(firstCell, 'Goal feedback');
		await commentOn(otherCell, 'Keep feedback');
		const saved = readCurrentReview(fixture).comments;
		expect(saved.map((comment): string => comment.anchor.targetId)).toEqual([
			paragraph.id,
			firstCell.id,
			otherCell.id,
		]);
		expect(new Set(saved.map((comment): string => comment.anchor.targetId)).size).toBe(3);
		expect(saved.every((comment): boolean => comment.anchor.targetId !== 'content')).toBe(true);
		await frame.getByRole('button', { name: 'Comments (3)', exact: true }).click();
		await frame.getByRole('button', { name: 'Edit comment on Goal', exact: true }).click();
		const save = frame.getByRole('button', { name: 'Save changes', exact: true });
		await expect.element(save).toBeVisible();
		await expect
			.element(frame.getByRole('button', { name: 'Delete comment on Goal', exact: true }))
			.toBeVisible();
		await frame.getByRole('button', { name: 'Cancel', exact: true }).click();
		await frame.getByRole('button', { name: 'Delete comment on Goal', exact: true }).click();
		expect(readCurrentReview(fixture).comments).toEqual([saved[0], saved[2]]);
		document.body.replaceChildren();
		const reopened = await openFixture('review', '', 'targets.html');
		await expect
			.poll((): number => reopened.document.querySelectorAll('.ex-comments-trigger').length)
			.toBe(1);
		expect(readCurrentReview(reopened.document).comments).toEqual([saved[0], saved[2]]);
	},
);

test('a range touching whitespace outside one paragraph stays paragraph-owned', (): void => {
	// Component integration: model the browser's boundary positions without replacing anchor capture.
	const root = document.createElement('ex-document');
	root.id = 'boundary-document';
	root.innerHTML =
		'<div>\n<p id="local-passage">Confirmed with <code>source</code> context.</p>\n<table><tbody><tr><td>Next item</td></tr></tbody></table></div>';
	document.body.append(root);
	const wrapper = requiredElement(root, 'div');
	const passage = requiredElement(root, 'p');
	const range = document.createRange();
	range.setStart(wrapper, 0);
	range.setEnd(wrapper, 3);
	const selection = document.getSelection();
	if (!selection) throw new Error('Selection unavailable');
	selection.addRange(range);
	const captured = captureCommentAnchor(selection, readReviewTargets(root), root);
	expect(captured?.anchor.targetId).toBe(passage.id);
	expect(captured?.anchor.sourceText).toBe('Confirmed with source context.');
});

test('new table targets preserve the existing generated IDs of paragraphs inside cells', (): void => {
	const root = document.createElement('ex-document');
	root.id = 'legacy-root';
	root.innerHTML =
		'<table><tbody><tr><td><p>Existing paragraph feedback.</p></td></tr></tbody></table>';
	document.body.append(root);
	readReviewTargets(root);
	expect(requiredElement(root, 'p').id).toBe('legacy-root--review-table-1-tbody-1-tr-1-td-1-p-1');
});

test.each([
	['first', 'within-cell', 'cell-one'],
	['first', 'next-cell', 'row-one'],
	['first', 'next-row', 'table-owner'],
])('table range from %s through %s uses %s', (startId, endId, expected): void => {
	const root = document.createElement('ex-document');
	root.id = 'table-document';
	root.innerHTML =
		'<table id="table-owner"><tbody><tr id="row-one"><td id="cell-one"><p id="first">First paragraph</p><p id="within-cell">Another paragraph</p></td><td><p id="next-cell">Another cell</p></td></tr><tr><td id="next-row">Another row</td><td>Last cell</td></tr></tbody></table>';
	document.body.append(root);
	const start = requiredElement(root, `#${startId}`).firstChild;
	const end = requiredElement(root, `#${endId}`).firstChild;
	if (!start || !end) throw new Error('Expected table text');
	const range = document.createRange();
	range.setStart(start, 0);
	range.setEnd(end, end.textContent?.length ?? 0);
	const selection = document.getSelection();
	if (!selection) throw new Error('Selection unavailable');
	selection.addRange(range);
	expect(captureCommentAnchor(selection, readReviewTargets(root), root)?.anchor.targetId).toBe(
		expected,
	);
});
