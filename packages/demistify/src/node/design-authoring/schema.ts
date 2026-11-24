import { z } from 'zod';

import {
	assumptionRecord,
	choiceRecord,
	isDecision,
	rulingRecord,
} from '../../core/decision-record.ts';
import { deliveryAcceptanceSchema } from '../../core/deliverables-view.ts';
import {
	navigationTargetSchema,
	sharedMapNodeSchema,
	sharedNavigationSchema,
	sharedWalkthroughSchema,
	sharedWalkthroughStepSchema,
} from '../../core/shared-navigation.ts';

export const nonEmptyText = z.string().trim().min(1);
export const recordId = z.string().regex(/^[A-Za-z][A-Za-z0-9-]*$/);
export const recordReference = z
	.string()
	.regex(/^(?:[A-Za-z][A-Za-z0-9-]*\/)?[A-Za-z][A-Za-z0-9-]*$/);
const authoredNavigationTarget = z.union([
	navigationTargetSchema,
	z.strictObject({ record: recordReference }),
]);
const authoredMapNode = sharedMapNodeSchema
	.pick({ title: true, description: true, icon: true })
	.extend({
		title: nonEmptyText.optional(),
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
export const relativeHref = z
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
export const htmlDestination = z.strictObject({
	page: relativeFile,
	fragment: recordId,
});
const recordBase = {
	id: recordId,
	title: nonEmptyText.optional(),
};
const designStatement = { ...recordBase, statement: nonEmptyText };
const dependencies = {
	governedBy: z.array(recordReference).optional(),
	reliesOn: z.array(recordReference).optional(),
};
const cases = z.array(z.strictObject({ given: nonEmptyText, expect: nonEmptyText }));
const referenceList = z.union([recordReference, z.array(recordReference).min(1)]);

const needRecord = z.strictObject({ ...designStatement, ...dependencies, kind: z.literal('need') });
const ruleRecord = z.strictObject({
	...designStatement,
	...dependencies,
	kind: z.literal('rule'),
	serves: referenceList.optional(),
	cases: cases.optional(),
});
const realizationRecord = z.strictObject({
	...designStatement,
	...dependencies,
	kind: z.literal('realization'),
	realizes: referenceList,
});
const plannedCheckRecord = z.strictObject({
	...designStatement,
	...dependencies,
	kind: z.literal('planned-check'),
	checks: referenceList,
	journey: nonEmptyText,
	unverified: nonEmptyText,
	cases: cases.min(3),
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
		z.discriminatedUnion('kind', [...designRecords, choiceRecord, rulingRecord, assumptionRecord]),
	),
});

export type AuthoredDesign = z.infer<typeof authoredDesignSchema>;
export type DesignCore = z.infer<typeof designSchema>;
export type DesignRecord = DesignCore['records'][number];

export function recordStatement(record: DesignRecord): string {
	if (record.kind !== 'choice') return record.statement;
	return record.authority.state === 'owner-open' ? record.question : record.authority.decision;
}

export function recordLabel(record: DesignRecord): string {
	return record.title ?? recordStatement(record);
}

type RecordRelation = 'serves' | 'governed-by' | 'relies-on' | 'realizes' | 'checks';
export interface RecordRelationship {
	readonly relation: RecordRelation;
	readonly targetId: string;
}

export function recordRelationships(record: DesignRecord): RecordRelationship[] {
	if (isDecision(record) || record.kind === 'assumption') return [];
	const primary =
		record.kind === 'rule'
			? record.serves
			: record.kind === 'realization'
				? record.realizes
				: record.kind === 'planned-check'
					? record.checks
					: undefined;
	const relation: RecordRelation =
		record.kind === 'rule' ? 'serves' : record.kind === 'realization' ? 'realizes' : 'checks';
	return [
		...(typeof primary === 'string' ? [primary] : (primary ?? [])).map(
			(targetId): RecordRelationship => ({ relation, targetId }),
		),
		...(record.governedBy ?? []).map((targetId) => ({
			relation: 'governed-by' as const,
			targetId,
		})),
		...(record.reliesOn ?? []).map((targetId) => ({ relation: 'relies-on' as const, targetId })),
	];
}

export function qualifyRecord(namespace: string | undefined, reference: string): string {
	return namespace && !reference.includes('/') ? `${namespace}/${reference}` : reference;
}
