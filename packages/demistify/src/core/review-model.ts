import { z } from 'zod';

const identityText = z.string().trim().min(1);
export const documentIdentitySchema = z.object({
	documentId: identityText,
	documentRevision: identityText,
});
export const choiceResponseSchema = z.object({
	questionId: identityText,
	decisionId: identityText.optional(),
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
		...(/^sha256:[a-f0-9]{64}$/.test(review.documentRevision)
			? []
			: [`Document revision: ${review.documentRevision}`]),
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
		if (answer.decisionId) lines.push(`Decision ID: ${answer.decisionId}`);
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

/** One answer read back from a copied review, with the comments the owner left on it. */
export interface ReviewedAnswer {
	readonly question: string;
	readonly questionId: string;
	readonly decisionId?: string;
	readonly selected: readonly { readonly optionId: string; readonly label: string }[];
	/** Comments on the question or a selected option, in the order the review lists them. */
	readonly comments: readonly { readonly targetId: string; readonly body: string }[];
}

const reviewOptionLine = /^- (.+) \(([^()\s]+)\)$/u;

/**
 * Read the answers back from the text {@link formatReview} writes, which is what an
 * agent receives when the owner copies a review. Comments not attached to an answer
 * are ignored.
 */
export function parseReviewAnswers(text: string): {
	reviewId: string;
	answers: ReviewedAnswer[];
} {
	const lines = text.replace(/\r\n?/gu, '\n').split('\n');
	const reviewId = lines
		.find((line) => line.startsWith('Review ID: '))
		?.slice('Review ID: '.length)
		.trim();
	if (!reviewId) throw new Error('Not a copied Demistify review: it has no "Review ID:" line.');
	const answerStarts = (index: number): boolean =>
		Boolean(lines[index]?.startsWith('## ') && lines[index + 1]?.startsWith('Question ID: '));
	const looseCommentStarts = (index: number): boolean =>
		Boolean(
			lines[index]?.startsWith('## ') &&
			lines[index + 1] === '' &&
			lines[index + 2]?.startsWith('Target ID: '),
		);
	const commentStarts = (index: number): boolean =>
		lines[index] === '' && Boolean(lines[index + 1]?.startsWith('Target ID: '));
	const answerEnds = (index: number): boolean =>
		index >= lines.length ||
		(lines[index] === '' && (answerStarts(index + 1) || looseCommentStarts(index + 1)));

	const answers: ReviewedAnswer[] = [];
	let index = lines.indexOf('# Selected answers') + 1;
	if (index === 0) return { reviewId, answers };
	while (index < lines.length && !looseCommentStarts(index)) {
		if (!answerStarts(index)) {
			index += 1;
			continue;
		}
		const question = lines[index]?.slice(3) ?? '';
		const questionId = lines[index + 1]?.slice('Question ID: '.length).trim() ?? '';
		index += 2;
		const decisionLine = lines[index];
		const decisionId = decisionLine?.startsWith('Decision ID: ')
			? decisionLine.slice('Decision ID: '.length).trim()
			: undefined;
		if (decisionId) index += 1;
		const selected: { optionId: string; label: string }[] = [];
		const comments: { targetId: string; body: string }[] = [];
		while (!answerEnds(index)) {
			const option = reviewOptionLine.exec(lines[index] ?? '');
			if (option?.[1] && option[2]) {
				selected.push({ label: option[1], optionId: option[2] });
				index += 1;
			} else if (commentStarts(index)) {
				const targetId = lines[index + 1]?.slice('Target ID: '.length).trim() ?? '';
				index += 2;
				while (index < lines.length && lines[index] !== '') index += 1;
				index += 1;
				const body: string[] = [];
				while (
					!answerEnds(index) &&
					!commentStarts(index) &&
					!reviewOptionLine.test(lines[index] ?? '')
				) {
					body.push(lines[index] ?? '');
					index += 1;
				}
				comments.push({ targetId, body: body.join('\n').trim() });
			} else index += 1;
		}
		answers.push({ question, questionId, ...(decisionId && { decisionId }), selected, comments });
	}
	return { reviewId, answers };
}
