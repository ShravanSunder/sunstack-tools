import { z } from 'zod';

import { deliverableViewSchema, localDeliveryHref } from './deliverables-view.ts';

const projectRecord = z.looseObject({
	id: z.string().min(1),
	kind: z.enum(['need', 'rule', 'realization', 'planned-check', 'choice', 'assumption']),
	title: z.string().min(1),
	destination: z.object({ page: localDeliveryHref, fragment: z.string().min(1) }),
	authority: z.looseObject({ state: z.string(), question: z.string().optional() }).optional(),
	supersedes: z.string().optional(),
});

export const projectViewSchema = z.strictObject({
	version: z.literal(1),
	owner: z.string().optional(),
	designs: z.record(
		z.string(),
		z.strictObject({
			path: localDeliveryHref,
			lead: z.string().optional(),
			firstPage: localDeliveryHref.optional(),
			title: z.string().min(1).optional(),
		}),
	),
	records: z.array(projectRecord),
	deliverables: z.record(z.string(), deliverableViewSchema),
});

export type ProjectView = z.infer<typeof projectViewSchema>;

export function parseProjectView(source: string): ProjectView {
	return projectViewSchema.parse(JSON.parse(source));
}

export function ownerQuestions(records: ProjectView['records']): ProjectView['records'] {
	const superseded = new Set(records.flatMap(({ supersedes }) => (supersedes ? [supersedes] : [])));
	return records.filter(
		(record): boolean =>
			record.kind === 'choice' &&
			record.authority?.state === 'owner-open' &&
			!superseded.has(record.id),
	);
}
