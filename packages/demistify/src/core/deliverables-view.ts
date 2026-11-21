import { z } from 'zod';

const text = z.string().trim().min(1);
const recordId = z.string().regex(/^[A-Za-z][A-Za-z0-9-]*$/);
const localHref = z
	.string()
	.refine(
		(href) =>
			href.trim().length > 0 &&
			!/^[a-z][a-z0-9+.-]*:|^\//i.test(href.trimStart()) &&
			!/[\\\p{Cc}]/u.test(href),
		'must be a relative link',
	);
export const scopeRevisionSchema = z
	.string()
	.regex(/^[a-f0-9]{64}$/, 'Expected a lowercase SHA-256 hex digest.');

export const deliveryAcceptanceSchema = z.strictObject({
	deliverableId: recordId,
	build: text,
	scopeRevision: scopeRevisionSchema,
	evidence: localHref,
});

const observation = { observation: text, evidence: localHref };
const passedOutcome = z.strictObject({ kind: z.literal('passed'), ...observation });
const failedOutcome = z.strictObject({ kind: z.literal('failed'), ...observation });
const blockedOutcome = z.strictObject({ kind: z.literal('blocked'), reason: text });
const verificationOutcome = z.discriminatedUnion('kind', [
	passedOutcome,
	failedOutcome,
	blockedOutcome,
]);

export const verificationReceiptSchema = z.strictObject({
	build: text,
	scopeRevision: scopeRevisionSchema,
	outcomes: z.record(recordId, verificationOutcome),
});

const destination = z.strictObject({ page: localHref, fragment: recordId });
const recordDestination = destination.extend({ id: recordId });
const check = z.discriminatedUnion('state', [
	z.strictObject({ id: recordId, state: z.literal('missing'), reason: text }),
	z.strictObject({
		id: recordId,
		state: z.literal('stale'),
		reason: text,
		outcome: verificationOutcome,
	}),
	z.strictObject({ id: recordId, state: z.literal('passed'), outcome: passedOutcome }),
	z.strictObject({ id: recordId, state: z.literal('failed'), outcome: failedOutcome }),
	z.strictObject({
		id: recordId,
		state: z.literal('blocked'),
		reason: text,
		outcome: blockedOutcome,
	}),
]);
const openProgressState = z.enum([
	'planned',
	'implementation-in-progress',
	'design-blocked',
	'awaiting-verification',
	'verification-blocked',
	'verification-failed',
	'ready-for-acceptance',
]);
const progress = z.discriminatedUnion('state', [
	z.strictObject({ state: openProgressState, reason: text }),
	z.strictObject({
		state: z.literal('accepted'),
		reason: text,
		acceptance: deliveryAcceptanceSchema,
	}),
]);
const deliverable = z.strictObject({
	id: recordId,
	title: text,
	scopeRevision: scopeRevisionSchema,
	dependsOn: z.array(recordId),
	state: z.union([openProgressState, z.enum(['accepted', 'dependency-blocked'])]),
	reasons: z.array(text),
	progress,
	implementation: z.strictObject({
		candidate: text.optional(),
		tasks: z.array(z.strictObject({ id: recordId, state: z.enum(['completed', 'open']) })),
		evidence: z.array(localHref),
	}),
	verification: z.strictObject({
		build: text.optional(),
		scopeRevision: scopeRevisionSchema.optional(),
		checks: z.array(check),
		history: z.array(verificationReceiptSchema).optional(),
	}),
	links: z.strictObject({
		plan: destination,
		rules: z.array(recordDestination),
		designItems: z.array(recordDestination),
		governingItems: z.array(recordDestination).default([]),
		verificationChecks: z.array(recordDestination),
	}),
});

export const deliverablesViewSchema = z.strictObject({
	version: z.literal(1),
	snapshotRevision: scopeRevisionSchema,
	deliverables: z.record(recordId, deliverable),
});

export type DeliverableView = z.infer<typeof deliverable>;
export type DeliverablesView = z.infer<typeof deliverablesViewSchema>;

/** Read the versioned data emitted by the design checker. */
export function parseDeliverablesView(source: string): DeliverablesView {
	return deliverablesViewSchema.parse(JSON.parse(source));
}
