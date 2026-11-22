import { z } from 'zod';

import { deliverableViewSchema, localDeliveryHref } from './deliverables-view.ts';

const projectRecord = z.looseObject({
	id: z.string().min(1),
	kind: z.enum(['need', 'rule', 'realization', 'planned-check', 'choice', 'assumption']),
	title: z.string().min(1),
	destination: z.object({ page: localDeliveryHref, fragment: z.string().min(1) }),
	authority: z
		.looseObject({
			state: z.string(),
			question: z.string().optional(),
			decision: z.string().optional(),
			decidedOn: z.string().optional(),
			lead: z.string().optional(),
		})
		.optional(),
	status: z.string().optional(),
	supersedes: z.string().optional(),
	relationships: z.array(z.object({ relation: z.string(), targetId: z.string() })).optional(),
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
export type ProjectRecord = ProjectView['records'][number];

export interface ProjectDecision {
	readonly record: ProjectRecord;
	readonly authority: 'Lead' | 'Owner';
	readonly status: 'Open' | 'Decided' | 'Superseded';
	readonly replacement: ProjectRecord | undefined;
	readonly governedRecords: readonly ProjectRecord[];
}

export function parseProjectView(source: string): ProjectView {
	return projectViewSchema.parse(JSON.parse(source));
}

export function projectDecisions(records: readonly ProjectRecord[]): ProjectDecision[] {
	const decisions = records.filter(({ kind }): boolean => kind === 'choice');
	const replacements = new Map<string, ProjectRecord>();
	const governed = new Map<string, Set<ProjectRecord>>();
	for (const decision of decisions) {
		if (
			decision.supersedes &&
			(decision.authority?.state === 'lead-decided' ||
				decision.authority?.state === 'owner-decided')
		)
			replacements.set(decision.supersedes, decision);
	}
	for (const record of records) {
		for (const { relation, targetId } of record.relationships ?? []) {
			if (relation !== 'governed-by') continue;
			const related = governed.get(targetId) ?? new Set<ProjectRecord>();
			related.add(record);
			governed.set(targetId, related);
		}
	}
	return decisions.flatMap((record): ProjectDecision[] => {
		const state = record.authority?.state;
		if (state !== 'owner-open' && state !== 'owner-decided' && state !== 'lead-decided') return [];
		const replacement = replacements.get(record.id);
		return [
			{
				record,
				authority: state === 'lead-decided' ? 'Lead' : 'Owner',
				status: replacement ? 'Superseded' : state === 'owner-open' ? 'Open' : 'Decided',
				replacement,
				governedRecords: Array.from(governed.get(record.id) ?? []),
			},
		];
	});
}
