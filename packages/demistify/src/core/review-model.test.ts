import { expect, test } from 'vitest';

import { commentStorageKey, createCommentStore, type CommentStorage } from './comment-store.ts';
import {
	commentSnapshotSchema,
	createCommentSnapshot,
	formatReview,
	reviewEnvelopeSchema,
	textAnchorMatches,
	textCommentAnchorSchema,
} from './review-model.ts';

const identity = { documentId: 'scope', documentRevision: 'r1' };
const anchor = { kind: 'entity', targetId: 'layout', label: 'Layout' } as const;

test('a review carries current answers, exact comments and isolated returned values', (): void => {
	const values = new Map<string, string>();
	const storage: CommentStorage = {
		getItem: (key): string | null => values.get(key) ?? null,
		setItem: (key, value): void => {
			values.set(key, value);
		},
	};
	const store = createCommentStore(identity, (): CommentStorage => storage);
	store.save(anchor, 'Keep this.\nCheck the phone.');
	const before = store.snapshot();
	const copiedComment = before.comments[0];
	if (copiedComment) copiedComment.body = 'Mutated copy';
	const restored = createCommentStore(identity, (): CommentStorage => storage).snapshot();
	expect(restored.comments[0]?.body).toBe('Keep this.\nCheck the phone.');
	expect(restored.reviewId).toBe(before.reviewId);
	const originalId = restored.comments[0]?.commentId;
	store.save(anchor, 'Edited comment', originalId);
	expect(store.snapshot().comments[0]?.commentId).toBe(originalId);
	const review = reviewEnvelopeSchema.parse({
		...store.snapshot(),
		answers: [
			{
				questionId: 'scope-question',
				question: 'Which scope?',
				selected: [{ optionId: 'layout', label: 'Layout' }],
			},
		],
	});
	expect(formatReview(review)).toContain('Question ID: scope-question\n- Layout (layout)');
	expect(formatReview(review)).toContain(`Comment ID: ${originalId}`);
	expect(
		createCommentStore(
			{ ...identity, documentRevision: 'r2' },
			(): CommentStorage => storage,
		).snapshot().comments,
	).toEqual([]);
	expect(commentStorageKey({ documentId: 'a:b', documentRevision: 'c' })).not.toBe(
		commentStorageKey({ documentId: 'a', documentRevision: 'b:c' }),
	);
});

test.each([
	'invalid json',
	JSON.stringify({ ...createCommentSnapshot(identity), format: 'experimental-review-1' }),
	JSON.stringify(createCommentSnapshot({ ...identity, documentRevision: 'other' })),
])('invalid saved bytes remain intact while new comments stay copyable: %s', (raw): void => {
	let written = raw;
	const store = createCommentStore(identity, (): CommentStorage => ({
		getItem: (): string => raw,
		setItem: (_key, value): void => {
			written = value;
		},
	}));
	store.save(anchor, 'Memory only');
	expect(store.persistence().kind).toBe('invalid');
	expect(written).toBe(raw);
	expect(store.snapshot().comments[0]?.body).toBe('Memory only');
});

test.each(['read', 'write'])(
	'storage %s failure is explicit and preserves in-memory feedback',
	(failure): void => {
		const store = createCommentStore(identity, (): CommentStorage => ({
			getItem: (): null => {
				if (failure === 'read') throw new Error('Denied');
				return null;
			},
			setItem: (): never => {
				throw new Error('Denied');
			},
		}));
		store.save(anchor, 'Unsaved comment');
		expect(store.persistence().kind).toBe('unavailable');
		expect(store.snapshot().comments[0]?.body).toBe('Unsaved comment');
	},
);

test('the schema rejects duplicate comments and malformed or empty boundaries', (): void => {
	const snapshot = createCommentSnapshot(identity);
	const comment = {
		commentId: 'one',
		anchor,
		body: 'Comment',
		createdAt: '2026-10-24T10:00:00Z',
		updatedAt: '2026-10-24T10:00:00Z',
	};
	expect(
		commentSnapshotSchema.safeParse({ ...snapshot, comments: [comment, comment] }).success,
	).toBe(false);
	for (const invalid of [
		{ ...comment, body: '  ' },
		{ ...comment, anchor: { ...anchor, targetId: '' } },
	])
		expect(commentSnapshotSchema.safeParse({ ...snapshot, comments: [invalid] }).success).toBe(
			false,
		);
	const store = createCommentStore(identity, (): CommentStorage => ({
		getItem: (): null => null,
		setItem: (): void => {},
	}));
	expect((): void => store.save(anchor, ' ')).toThrow();
	expect(store.snapshot().comments).toEqual([]);
});

test('exact text verification retains source whitespace and refuses moved or changed passages', (): void => {
	const text = 'Before customer\n\tpilot after';
	const exact = textCommentAnchorSchema.parse({
		kind: 'text',
		targetId: 'document',
		label: 'Document',
		selectedText: 'customer pilot',
		sourceText: 'customer\n\tpilot',
		start: 7,
		end: 22,
		prefix: 'Before ',
		suffix: ' after',
	});
	expect(textAnchorMatches(text, exact)).toBe(true);
	expect(textAnchorMatches('Before customer pilot after', exact)).toBe(false);
	expect(textAnchorMatches('Before other customer\n\tpilot after', exact)).toBe(false);
	for (const range of [{ start: -1 }, { end: 7 }, { start: 8 }, { sourceText: '' }])
		expect(textCommentAnchorSchema.safeParse({ ...exact, ...range }).success).toBe(false);
});

test('a carried review rejects duplicate or unanswered question records', (): void => {
	const snapshot = createCommentSnapshot(identity);
	const answer = {
		questionId: 'scope-question',
		question: 'Which scope?',
		selected: [{ optionId: 'layout', label: 'Layout' }],
	};
	expect(reviewEnvelopeSchema.safeParse({ ...snapshot, answers: [answer] }).success).toBe(true);
	for (const answers of [[answer, answer], [{ ...answer, selected: [] }]])
		expect(reviewEnvelopeSchema.safeParse({ ...snapshot, answers }).success).toBe(false);
});
