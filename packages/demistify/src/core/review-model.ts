import { z } from 'zod';

const identityText = z.string().trim().min(1);
export const documentIdentitySchema = z.object({
	documentId: identityText,
	documentRevision: identityText,
});
export const choiceResponseSchema = z.object({
	questionId: identityText,
	question: identityText,
	selected: z
		.array(z.object({ optionId: identityText, label: identityText }))
		.refine(
			(options): boolean =>
				new Set(options.map((option): string => option.optionId)).size === options.length,
			'Option IDs must be unique within an answer.',
		),
});
const targetFields = { targetId: identityText, label: identityText };
export const textCommentAnchorSchema = z
	.object({
		kind: z.literal('text'),
		...targetFields,
		selectedText: z
			.string()
			.refine((value): boolean => value.trim().length > 0, 'Select readable text.'),
		sourceText: z
			.string()
			.refine((value): boolean => value.trim().length > 0, 'Select readable source text.'),
		start: z.number().int().nonnegative(),
		end: z.number().int().positive(),
		prefix: z.string(),
		suffix: z.string(),
	})
	.refine(
		(anchor): boolean => anchor.end - anchor.start === anchor.sourceText.length,
		'The source text must match the UTF-16 range length.',
	);
export const commentAnchorSchema = z.union([
	z.object({ kind: z.literal('entity'), ...targetFields }),
	textCommentAnchorSchema,
]);
export const pageCommentSchema = z.object({
	commentId: identityText,
	anchor: commentAnchorSchema,
	body: z.string().trim().min(1),
	createdAt: z.iso.datetime(),
	updatedAt: z.iso.datetime(),
});
const commentsSchema = z
	.array(pageCommentSchema)
	.refine(
		(comments): boolean =>
			new Set(comments.map((comment): string => comment.commentId)).size === comments.length,
		'Comment IDs must be unique within a review.',
	);
export const reviewEnvelopeSchema = documentIdentitySchema.extend({
	format: z.literal('demistify-review-1'),
	reviewId: identityText,
	comments: commentsSchema,
	answers: z
		.array(
			choiceResponseSchema.refine(
				(answer): boolean => answer.selected.length > 0,
				'Unanswered questions are omitted from a review.',
			),
		)
		.refine(
			(answers): boolean =>
				new Set(answers.map((answer): string => answer.questionId)).size === answers.length,
			'A review has one current answer per question.',
		),
});
export const commentSnapshotSchema = reviewEnvelopeSchema.omit({ answers: true });
export type DocumentIdentity = z.infer<typeof documentIdentitySchema>;
export type ChoiceResponse = z.infer<typeof choiceResponseSchema>;
export type CommentAnchor = z.infer<typeof commentAnchorSchema>;
export type TextCommentAnchor = z.infer<typeof textCommentAnchorSchema>;
export type PageComment = z.infer<typeof pageCommentSchema>;
export type ReviewEnvelope = z.infer<typeof reviewEnvelopeSchema>;
export type CommentSnapshot = z.infer<typeof commentSnapshotSchema>;

export function createCommentSnapshot(identity: DocumentIdentity): CommentSnapshot {
	return commentSnapshotSchema.parse({
		...identity,
		format: 'demistify-review-1',
		reviewId: crypto.randomUUID(),
		comments: [],
	});
}

export function textAnchorMatches(text: string, anchor: TextCommentAnchor): boolean {
	return (
		text.slice(anchor.start, anchor.end) === anchor.sourceText &&
		text.slice(Math.max(0, anchor.start - anchor.prefix.length), anchor.start) === anchor.prefix &&
		text.slice(anchor.end, anchor.end + anchor.suffix.length) === anchor.suffix
	);
}

export function formatReview(review: ReviewEnvelope): string {
	const lines = [
		`# Review of ${review.documentId}`,
		`Document revision: ${review.documentRevision}`,
		`Review ID: ${review.reviewId}`,
	];
	const rendered = new Set<string>();
	const appendComment = (comment: PageComment, heading: boolean): void => {
		if (heading) lines.push('', `## ${comment.anchor.label}`);
		lines.push('', `Target ID: ${comment.anchor.targetId}`, `Comment ID: ${comment.commentId}`);
		if (comment.anchor.kind === 'text')
			lines.push(
				`Selected text: ${JSON.stringify(comment.anchor.selectedText)}`,
				`Text range (UTF-16): ${comment.anchor.start}–${comment.anchor.end}`,
			);
		else lines.push('Anchor: whole item');
		lines.push('', comment.body);
		rendered.add(comment.commentId);
	};
	const appendTargetComments = (targetId: string): void => {
		for (const comment of review.comments)
			if (comment.anchor.targetId === targetId) appendComment(comment, false);
	};
	if (review.answers.length) lines.push('', '# Selected answers');
	for (const answer of review.answers) {
		lines.push('', `## ${answer.question}`, `Question ID: ${answer.questionId}`);
		appendTargetComments(answer.questionId);
		for (const option of answer.selected) {
			lines.push(`- ${option.label} (${option.optionId})`);
			appendTargetComments(option.optionId);
		}
	}
	for (const comment of review.comments)
		if (!rendered.has(comment.commentId)) appendComment(comment, true);
	return lines.join('\n');
}
