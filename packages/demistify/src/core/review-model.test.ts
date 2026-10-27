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

test('saving through either node entry updates one comment and preserves its identity', (): void => {
	const values = new Map<string, string>();
	const storage: CommentStorage = {
		getItem: (key): string | null => values.get(key) ?? null,
		setItem: (key, value): void => {
			values.set(key, value);
		},
	};
	const store = createCommentStore(identity, (): CommentStorage => storage);
	store.save(anchor, 'First note');
	const original = store.snapshot().comments[0];
	store.save(anchor, 'Updated note');
	expect(store.snapshot().comments).toHaveLength(1);
	expect(store.snapshot().comments[0]).toMatchObject({
		commentId: original?.commentId,
		createdAt: original?.createdAt,
		body: 'Updated note',
	});
	store.save({ ...anchor, targetId: 'technical', label: 'Technical' }, 'Another node');
	expect(store.snapshot().comments).toHaveLength(2);
	expect(createCommentStore(identity, (): CommentStorage => storage).snapshot()).toEqual(
		store.snapshot(),
	);
});

test('selected answer copy includes its comment without repeating the option title', (): void => {
	const store = createCommentStore(identity, (): CommentStorage => ({
		getItem: (): null => null,
		setItem: (): void => {},
	}));
	store.save(anchor, 'Keep this boundary.');
	store.save(
		{ ...anchor, targetId: 'other', label: 'Unselected option' },
		'Do not lose this note.',
	);
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
	const copied = formatReview(review);
	expect(copied.match(/Layout/g)).toHaveLength(1);
	expect(copied).toContain('Keep this boundary.');
	expect(copied).toContain('Unselected option');
	expect(copied).toContain('Do not lose this note.');
	for (const comment of review.comments)
		expect(copied).toContain(`Comment ID: ${comment.commentId}`);
});

test('a new selected passage updates the same comment without replacing its saved identity', (): void => {
	const store = createCommentStore(identity, (): CommentStorage => ({
		getItem: (): null => null,
		setItem: (): void => {},
	}));
	store.save(anchor, 'Whole item note');
	const original = store.snapshot().comments[0];
	const passage = textCommentAnchorSchema.parse({
		...anchor,
		kind: 'text',
		selectedText: 'Layout',
		sourceText: 'Layout',
		start: 0,
		end: 6,
		prefix: '',
		suffix: '',
	});
	store.save(passage, 'Precise note', original?.commentId);
	expect(store.snapshot().comments).toHaveLength(1);
	expect(store.snapshot().comments[0]).toMatchObject({
		commentId: original?.commentId,
		createdAt: original?.createdAt,
		anchor: passage,
		body: 'Precise note',
	});
});

test('historical same-node records remain readable and are not deleted or appended on save', (): void => {
	const prior = commentSnapshotSchema.parse({
		...createCommentSnapshot(identity),
		comments: ['first', 'second'].map((commentId) => ({
			commentId,
			anchor,
			body: commentId,
			createdAt: '2026-10-24T10:00:00Z',
			updatedAt: '2026-10-24T10:00:00Z',
		})),
	});
	let stored = JSON.stringify(prior);
	const store = createCommentStore(identity, (): CommentStorage => ({
		getItem: (): string => stored,
		setItem: (_key, value): void => {
			stored = value;
		},
	}));
	expect(store.snapshot()).toEqual(prior);
	store.save(anchor, 'Edited first');
	expect(store.snapshot().comments.map((comment): string => comment.commentId)).toEqual([
		'first',
		'second',
	]);
	expect(store.snapshot().comments[1]).toEqual(prior.comments[1]);
	expect(JSON.parse(stored)).toEqual(store.snapshot());
});

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
