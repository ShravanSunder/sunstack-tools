import { z } from 'zod';

import { decisionActorField, recordActor } from './decision-record.ts';
import { deliverableViewSchema, localDeliveryHref } from './deliverables-view.ts';

const projectRecord = z.looseObject({
	id: z.string().min(1),
	kind: z.enum(['need', 'rule', 'realization', 'planned-check', 'choice', 'ruling', 'assumption']),
	title: z.string().min(1),
	question: z.string().optional(),
	statement: z.string().optional(),
	destination: z.object({ page: localDeliveryHref, fragment: z.string().min(1) }).optional(),
	authority: z
		.looseObject({
			state: z.string(),
			question: z.string().optional(),
			decision: z.string().optional(),
			decidedOn: z.string().optional(),
			lead: z.string().optional(),
			[decisionActorField]: recordActor.optional(),
		})
		.optional(),
	status: z.string().optional(),
	supersedes: z.string().optional(),
	relationships: z.array(z.object({ relation: z.string(), targetId: z.string() })).optional(),
});

export const projectStates = [
	'accepted',
	'dependency-blocked',
	'design-blocked',
	'in-progress',
	'planned',
	'later',
] as const;
export type ProjectState = (typeof projectStates)[number];

/** The generated view of an initiative: its projects, the order their dependencies produce, records and deliverables. */
export const initiativeViewSchema = z.strictObject({
	version: z.literal(2),
	owner: z.string().optional(),
	projects: z.record(
		z.string(),
		z.strictObject({
			path: localDeliveryHref,
			lead: z.string().optional(),
			firstPage: localDeliveryHref.optional(),
			title: z.string().min(1).optional(),
			dependsOn: z.array(z.string()),
			state: z.enum(projectStates),
			reason: z.string().min(1),
		}),
	),
	/** Project names, every dependency before its dependents; ties keep the initiative's authored order. */
	order: z.array(z.string()),
	records: z.array(projectRecord),
	deliverables: z.record(z.string(), deliverableViewSchema),
});

export type InitiativeView = z.infer<typeof initiativeViewSchema>;
export type ProjectRecord = InitiativeView['records'][number];

export interface ProjectDecision {
	readonly record: ProjectRecord;
	readonly authority: string;
	readonly status: 'Open' | 'Decided' | 'Superseded';
	readonly replacement: ProjectRecord | undefined;
	readonly governedRecords: readonly ProjectRecord[];
}

export function parseInitiativeView(source: string): InitiativeView {
	return initiativeViewSchema.parse(JSON.parse(source));
}

export function projectDecisions(records: readonly ProjectRecord[]): ProjectDecision[] {
	const decisions = records.filter(({ kind }): boolean => kind === 'choice' || kind === 'ruling');
	const replacements = new Map<string, ProjectRecord>();
	const governed = new Map<string, Set<ProjectRecord>>();
	for (const decision of decisions) {
		if (
			decision.supersedes &&
			(decision.authority?.state === 'lead-decided' ||
				decision.authority?.state === 'agent-decided' ||
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
		if (
			state !== 'owner-open' &&
			state !== 'owner-decided' &&
			state !== 'lead-decided' &&
			state !== 'agent-decided'
		)
			return [];
		const replacement = replacements.get(record.id);
		return [
			{
				record,
				authority:
					state === 'agent-decided'
						? (record.authority?.by?.alias ?? 'Agent')
						: state === 'lead-decided'
							? 'Lead'
							: 'Owner',
				status: replacement ? 'Superseded' : state === 'owner-open' ? 'Open' : 'Decided',
				replacement,
				governedRecords: Array.from(governed.get(record.id) ?? []),
			},
		];
	});
}
