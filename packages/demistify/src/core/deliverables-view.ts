import { z } from 'zod';

const text = z.string().trim().min(1);
const recordId = z.string().regex(/^[A-Za-z][A-Za-z0-9-]*(?:\/[A-Za-z][A-Za-z0-9-]*)?$/);
export const localDeliveryHref = z
	.string()
	.refine(
		(href) =>
			href.trim().length > 0 &&
			!/^[a-z][a-z0-9+.-]*:|^\//i.test(href.trimStart()) &&
			!/[\\\p{Cc}]/u.test(href),
		'must be a relative link',
	);
export const deliveryAcceptanceSchema = z.strictObject({
	deliverableId: recordId,
	build: text,
	evidence: localDeliveryHref,
});

const observation = { observation: text, evidence: localDeliveryHref };
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
	outcomes: z.record(recordId, verificationOutcome),
});

const destination = z.strictObject({ page: localDeliveryHref, fragment: recordId });
const recordDestination = z.strictObject({
	id: recordId,
	label: text,
	destination: destination.optional(),
});
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
const coreDeliverableView = z.strictObject({
	kind: z.literal('core'),
	id: recordId,
	title: text,
	implementer: z.string().min(1).optional(),
	verifier: z.string().min(1).optional(),
	reviewer: z.string().min(1).optional(),
	blockedBy: z.array(recordId),
	state: z.union([openProgressState, z.enum(['accepted', 'dependency-blocked'])]),
	reasons: z.array(text),
	progress,
	implementation: z.strictObject({
		candidate: text.optional(),
		issues: z.array(
			z.strictObject({ id: recordId, title: text, state: z.enum(['closed', 'open']) }),
		),
		evidence: z.array(localDeliveryHref),
	}),
	verification: z.strictObject({
		build: text.optional(),
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

const laterDeliverableView = z.strictObject({
	kind: z.literal('later'),
	id: recordId,
	title: text,
	state: z.literal('later'),
});
export const deliverablePlanSummarySchema = z.discriminatedUnion('kind', [
	coreDeliverableView
		.pick({ kind: true, id: true, title: true, state: true })
		.extend({ destination }),
	laterDeliverableView,
]);

export const deliverableViewSchema = z.discriminatedUnion('kind', [
	coreDeliverableView,
	laterDeliverableView,
]);

export const deliverablesViewSchema = z.strictObject({
	version: z.literal(2),
	deliverables: z.record(recordId, deliverableViewSchema),
});

export type DeliverableView = z.infer<typeof deliverableViewSchema>;
export type CoreDeliverableView = Extract<DeliverableView, { kind: 'core' }>;
export type DeliverablesView = z.infer<typeof deliverablesViewSchema>;

/** Read the versioned data emitted by the design checker. */
export function parseDeliverablesView(source: string): DeliverablesView {
	return deliverablesViewSchema.parse(JSON.parse(source));
}
