import { expect } from 'vitest';

import {
	choiceResponseSchema,
	reviewEnvelopeSchema,
	type ChoiceResponse,
	type ReviewEnvelope,
} from '../../../../packages/demistify/src/core/review-model.ts';
import { openFixture, requiredElement } from './fixture-page.ts';

export const reviewFixtureIdentity = {
	documentId: 'demistify-choice-fixture',
	documentRevision: 'r1',
};
export function readChoiceResponse(element: Element): ChoiceResponse {
	if (!('response' in element)) throw new Error('Choice response getter is absent.');
	return choiceResponseSchema.parse(element.response);
}
export function readCurrentReview(fixture: Document): ReviewEnvelope {
	const element = requiredElement(fixture, 'ex-comments');
	if (!('review' in element)) throw new Error('Review getter is absent.');
	return reviewEnvelopeSchema.parse(element.review);
}
export async function openReviewFixture(file = 'index.html'): ReturnType<typeof openFixture> {
	const opened = await openFixture('review', '', file);
	await expect
		.poll((): number => opened.document.querySelectorAll('ex-choice input').length)
		.toBe(file === 'independent.html' ? 2 : 8);
	await expect
		.poll((): number => opened.document.querySelectorAll('.ex-comments-trigger').length)
		.toBe(1);
	return opened;
}
export async function openBoundaryReviewFixture(): ReturnType<typeof openFixture> {
	return openReviewFixture('boundary.html');
}
export function textCaretRect(element: Element, offset: number): DOMRect {
	const text = element.firstChild;
	if (!text || text.nodeType !== Node.TEXT_NODE)
		throw new Error('Expected a literal text passage.');
	const range = element.ownerDocument.createRange();
	range.setStart(text, offset);
	range.collapse(true);
	return range.getBoundingClientRect();
}
