import { z } from 'zod';

export const decisionActorField = 'by';

const text = z.string().trim().min(1);
export const designRecordId = z.string().regex(/^[A-Za-z][A-Za-z0-9-]*$/u);
const recordReference = z.string().regex(/^(?:[A-Za-z][A-Za-z0-9-]*\/)?[A-Za-z][A-Za-z0-9-]*$/u);
const relativeHref = text.refine(
	(value) =>
		!value.startsWith('/') && !value.includes('\\') && !/^[a-z][a-z0-9+.-]*:/iu.test(value),
	'must be a relative link',
);
export const decisionSource = z.strictObject({
	href: relativeHref,
	excerpt: text,
	verbatim: z.literal(true).optional(),
});
export const recordActor = z.strictObject({
	alias: text,
	role: z.enum(['lead', 'implementer', 'verifier', 'reviewer', 'advisor']),
});
const weight = z.enum(['design', 'process', 'minor']);
const ownerAuthority = z.strictObject({
	state: z.literal('owner-decided'),
	sourceKind: z.enum(['owner-words', 'owner-accepted']),
	decidedOn: z.iso.date(),
	source: decisionSource,
});
const agentFields = {
	state: z.literal('agent-decided'),
	[decisionActorField]: recordActor,
	rationale: text,
	decidedOn: z.iso.date(),
	weight,
};
const contractAuthority = z.strictObject({
	...agentFields,
	sourceKind: z.literal('contract'),
	source: decisionSource,
});
const authorAuthority = z.strictObject({ ...agentFields, sourceKind: z.literal('author') });
const identity = {
	id: designRecordId,
	title: text.optional(),
	sources: z.array(decisionSource).optional(),
};
const decisionIdentity = { ...identity, supersedes: recordReference.optional() };
const choiceAnswer = { chosen: designRecordId, decision: text };
export const choiceRecord = z.strictObject({
	...decisionIdentity,
	kind: z.literal('choice'),
	question: text,
	options: z.tuple(
		[
			z.strictObject({ id: designRecordId, title: text }),
			z.strictObject({ id: designRecordId, title: text }),
		],
		z.strictObject({ id: designRecordId, title: text }),
	),
	recommended: z.union([
		z.literal('none'),
		z.strictObject({ option: designRecordId, [decisionActorField]: recordActor }),
	]),
	authority: z.discriminatedUnion('state', [
		z.strictObject({ state: z.literal('owner-open') }),
		ownerAuthority.extend(choiceAnswer),
		z.discriminatedUnion('sourceKind', [
			contractAuthority.extend(choiceAnswer),
			authorAuthority.extend(choiceAnswer),
		]),
	]),
});
export const rulingRecord = z.strictObject({
	...decisionIdentity,
	kind: z.literal('ruling'),
	statement: text,
	authority: z.discriminatedUnion('state', [
		ownerAuthority,
		z.discriminatedUnion('sourceKind', [contractAuthority, authorAuthority]),
	]),
});
export const decisionRecord = z.discriminatedUnion('kind', [choiceRecord, rulingRecord]);
const assumptionIdentity = {
	...identity,
	kind: z.literal('assumption'),
	statement: text,
	[decisionActorField]: recordActor,
	weight,
};
export const assumptionRecord = z.discriminatedUnion('status', [
	z.strictObject({ ...assumptionIdentity, status: z.literal('open'), settles: text }),
	z.strictObject({
		...assumptionIdentity,
		status: z.enum(['confirmed', 'refuted']),
		resolution: decisionSource,
		settledBy: text,
	}),
]);

export type DecisionRecord = z.infer<typeof decisionRecord>;

export function isDecision(
	record: { readonly kind: string } | undefined,
): record is DecisionRecord {
	return record?.kind === 'choice' || record?.kind === 'ruling';
}
