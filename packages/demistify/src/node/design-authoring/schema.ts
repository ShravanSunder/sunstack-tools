import { z } from 'zod';

import { sharedNavigationSchema } from '../../core/shared-navigation.ts';

const nonEmptyText = z.string().trim().min(1);
const recordId = z.string().regex(/^[A-Za-z][A-Za-z0-9-]*$/);
const relativeHref = z
	.string()
	.min(1)
	.refine(
		(value) =>
			!value.startsWith('/') && !value.includes('\\') && !/^[a-z][a-z0-9+.-]*:/i.test(value),
		'must be a relative link',
	);
const relativeFile = relativeHref.refine(
	(value) => !/[?#]/.test(value) && !value.split('/').includes('..'),
	'must be a safe relative path',
);
const htmlDestination = z.strictObject({
	page: relativeFile,
	fragment: recordId,
});
const sourceReference = z.strictObject({
	href: relativeHref,
	excerpt: nonEmptyText,
});
const servesRelationship = z.strictObject({ relation: z.literal('serves'), targetId: recordId });
const governingRelationship = z.strictObject({
	relation: z.literal('governed-by'),
	targetId: recordId,
});
const realizesRelationship = z.strictObject({
	relation: z.literal('realizes'),
	targetId: recordId,
});
const checksRelationship = z.strictObject({
	relation: z.literal('checks'),
	targetId: recordId,
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
		.array(z.discriminatedUnion('relation', [servesRelationship, governingRelationship]))
		.refine(
			(links) => links.some((link) => link.relation === 'serves'),
			'a rule must serve at least one need',
		),
});
const realizationRecord = z.strictObject({
	...recordBase,
	kind: z.literal('realization'),
	relationships: z.array(realizesRelationship).min(1),
	explanation: nonEmptyText,
	status: z.literal('proposed'),
});
const plannedCheckRecord = z.strictObject({
	...recordBase,
	kind: z.literal('planned-check'),
	relationships: z.array(checksRelationship).min(1),
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
const choiceAuthority = z.discriminatedUnion('state', [
	z.strictObject({
		state: z.literal('owner-decided'),
		owner: z.literal('owner'),
		decision: nonEmptyText,
	}),
	z.strictObject({
		state: z.literal('owner-open'),
		owner: z.literal('owner'),
		question: nonEmptyText,
	}),
	z.strictObject({
		state: z.literal('agent-rationale'),
		author: nonEmptyText,
		rationale: nonEmptyText,
		tradeoff: nonEmptyText,
	}),
]);
const choiceRecord = z.strictObject({
	...recordBase,
	kind: z.literal('choice'),
	authority: choiceAuthority,
});

export const designSchema = z.strictObject({
	pages: sharedNavigationSchema.shape.pages,
	mapGroups: sharedNavigationSchema.shape.map.unwrap(),
	walkthroughs: sharedNavigationSchema.shape.walkthroughs.unwrap().optional(),
	records: z.array(
		z.discriminatedUnion('kind', [
			needRecord,
			ruleRecord,
			realizationRecord,
			plannedCheckRecord,
			choiceRecord,
		]),
	),
});

export type DesignCore = z.infer<typeof designSchema>;
export type DesignRecord = DesignCore['records'][number];
