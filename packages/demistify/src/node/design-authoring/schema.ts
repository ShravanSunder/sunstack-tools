import { z } from 'zod';

import { deliveryAcceptanceSchema } from '../../core/deliverables-view.ts';
import {
	navigationTargetSchema,
	sharedMapNodeSchema,
	sharedNavigationSchema,
	sharedWalkthroughSchema,
	sharedWalkthroughStepSchema,
} from '../../core/shared-navigation.ts';

const nonEmptyText = z.string().trim().min(1);
const recordId = z.string().regex(/^[A-Za-z][A-Za-z0-9-]*$/);
const recordReference = z.string().regex(/^(?:[A-Za-z][A-Za-z0-9-]*\/)?[A-Za-z][A-Za-z0-9-]*$/);
const authoredNavigationTarget = z.union([
	navigationTargetSchema,
	z.strictObject({ record: recordReference }),
]);
const authoredMapNode = sharedMapNodeSchema
	.pick({ title: true, description: true, icon: true })
	.extend({
		target: authoredNavigationTarget.optional(),
		get children(): z.ZodDefault<z.ZodArray<typeof authoredMapNode>> {
			return z.array(authoredMapNode).default([]);
		},
	});
const authoredWalkthrough = sharedWalkthroughSchema.extend({
	steps: z.array(sharedWalkthroughStepSchema.extend({ target: authoredNavigationTarget })).min(1),
});
export type AuthoredNavigationTarget = z.infer<typeof authoredNavigationTarget>;
export type AuthoredMapNode = z.infer<typeof authoredMapNode>;
const relativeHref = z
	.string()
	.min(1)
	.refine(
		(value) =>
			!value.startsWith('/') && !value.includes('\\') && !/^[a-z][a-z0-9+.-]*:/i.test(value),
		'must be a relative link',
	);
const relativeFile = relativeHref.refine(
	(value) => !/[?#]/.test(value),
	'must be a safe relative path',
);
const htmlDestination = z.strictObject({
	page: relativeFile,
	fragment: recordId,
});
const sourceReference = z.strictObject({
	href: relativeHref,
	excerpt: nonEmptyText,
	verbatim: z.literal(true).optional(),
});
const servesRelationship = z.strictObject({
	relation: z.literal('serves'),
	targetId: recordReference,
});
const designDependencies = [
	z.strictObject({ relation: z.literal('governed-by'), targetId: recordReference }),
	z.strictObject({ relation: z.literal('relies-on'), targetId: recordReference }),
] as const;
const realizesRelationship = z.strictObject({
	relation: z.literal('realizes'),
	targetId: recordReference,
});
const checksRelationship = z.strictObject({
	relation: z.literal('checks'),
	targetId: recordReference,
});
const recordBase = {
	id: recordId,
	title: nonEmptyText,
	destination: htmlDestination,
	sources: z.array(sourceReference).min(1),
};

const needRecord = z.strictObject({
	...recordBase,
	kind: z.literal('need'),
	statement: nonEmptyText,
});
const ruleRecord = z.strictObject({
	...recordBase,
	kind: z.literal('rule'),
	statement: nonEmptyText,
	relationships: z
		.array(z.discriminatedUnion('relation', [servesRelationship, ...designDependencies]))
		.refine(
			(links) => links.some((link) => link.relation === 'serves'),
			'a rule must serve at least one need',
		),
});
const realizationRecord = z.strictObject({
	...recordBase,
	kind: z.literal('realization'),
	relationships: z
		.array(z.discriminatedUnion('relation', [realizesRelationship, ...designDependencies]))
		.refine(
			(links) => links.some((link) => link.relation === 'realizes'),
			'a realization must realize at least one rule',
		),
	explanation: nonEmptyText,
	status: z.literal('proposed'),
});
const plannedCheckRecord = z.strictObject({
	...recordBase,
	kind: z.literal('planned-check'),
	relationships: z
		.array(z.discriminatedUnion('relation', [checksRelationship, ...designDependencies]))
		.refine(
			(links) => links.some((link) => link.relation === 'checks'),
			'a planned check must check at least one rule',
		),
	status: z.literal('planned'),
	cases: z
		.array(
			z.strictObject({
				given: nonEmptyText,
				expect: nonEmptyText,
			}),
		)
		.min(3),
});
const settledDecision = {
	decision: nonEmptyText,
	decidedOn: z.iso.date(),
};
const choiceAuthority = z.discriminatedUnion('state', [
	z.strictObject({
		state: z.literal('owner-decided'),
		owner: z.literal('owner'),
		sourceKind: z.enum(['owner-words', 'owner-accepted']),
		...settledDecision,
	}),
	z.strictObject({
		state: z.literal('owner-open'),
		owner: z.literal('owner'),
		question: nonEmptyText,
	}),
	z.strictObject({
		state: z.literal('lead-decided'),
		lead: nonEmptyText,
		sourceKind: z.enum(['author', 'contract']),
		rationale: nonEmptyText,
		...settledDecision,
	}),
]);
export const choiceRecord = z.strictObject({
	...recordBase,
	kind: z.literal('choice'),
	authority: choiceAuthority,
	supersedes: recordReference.optional(),
});
export const assumptionRecord = z
	.strictObject({
		...recordBase,
		kind: z.literal('assumption'),
		statement: nonEmptyText,
		status: z.enum(['open', 'confirmed', 'refuted']),
		resolution: sourceReference.optional(),
	})
	.refine(({ status, resolution }) => status === 'open' || resolution !== undefined, {
		path: ['resolution'],
		message: 'confirmed and refuted assumptions require a resolution source',
	})
	.refine(({ status, resolution }) => status !== 'open' || resolution === undefined, {
		path: ['resolution'],
		message: 'remove the resolution source or mark the assumption confirmed or refuted',
	});

const designRecords = [needRecord, ruleRecord, realizationRecord, plannedCheckRecord] as const;

export const authoredDesignSchema = z.strictObject({
	pages: sharedNavigationSchema.shape.pages,
	mapGroups: z.array(authoredMapNode),
	walkthroughs: z.array(authoredWalkthrough).optional(),
	acceptances: z.array(deliveryAcceptanceSchema.extend({ deliverableId: recordId })).optional(),
	records: z.array(z.discriminatedUnion('kind', designRecords)),
});

export const designSchema = authoredDesignSchema.extend({
	records: z.array(
		z.discriminatedUnion('kind', [...designRecords, choiceRecord, assumptionRecord]),
	),
});

export type AuthoredDesign = z.infer<typeof authoredDesignSchema>;
export type DesignCore = z.infer<typeof designSchema>;
export type DesignRecord = DesignCore['records'][number];
