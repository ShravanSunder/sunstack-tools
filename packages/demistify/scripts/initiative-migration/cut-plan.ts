import { z } from 'zod';

/**
 * The Lead's re-cut plan: which projects the initiative has afterwards, and which
 * whole deliverables move between them. Addresses name the tree as it is before
 * the run (`queue/DEL-07`); the command translates them.
 */
const projectKey = z.string().regex(/^[A-Za-z][A-Za-z0-9-]*$/);
const qualifiedAddress = z
	.string()
	.regex(
		/^[A-Za-z][A-Za-z0-9-]*\/[A-Za-z][A-Za-z0-9-]*$/,
		'must be a qualified address such as queue/DEL-07',
	);
const sentence = z.string().trim().min(1);

const plannedProjectSchema = z.strictObject({
	key: projectKey,
	/** Folder name for a project the run creates, beside the existing project folders. */
	folder: z
		.string()
		.regex(/^[A-Za-z0-9][A-Za-z0-9._-]*$/, 'must be one folder name without slashes'),
	/** The initiative's order: lower first; the same number means side by side. */
	number: z.number().int().nonnegative(),
	/** Page title for a project the run creates. */
	title: sentence,
	achieves: sentence.optional(),
	acceptance: sentence.optional(),
	/** Lead alias for a project the run creates. */
	lead: z.string().min(1).optional(),
});

const deliverableMoveSchema = z.strictObject({ from: qualifiedAddress, to: projectKey });
const recordPlacementSchema = z.strictObject({
	/** A record of the tree before the run, such as `queue/R14`. */
	record: qualifiedAddress,
	/** The project that owns it afterwards; its own project keeps it in place. */
	to: projectKey,
});
const addedDeliverableSchema = z.strictObject({
	project: projectKey,
	title: sentence,
	dependsOn: z.array(qualifiedAddress).default([]),
	why: sentence.optional(),
});
const dependsOnChangeSchema = z.strictObject({
	deliverable: qualifiedAddress,
	from: z.array(qualifiedAddress).default([]),
	to: z.array(qualifiedAddress).default([]),
	why: sentence.optional(),
});

export const cutPlanSchema = z.strictObject({
	note: z.string().optional(),
	projects: z.array(plannedProjectSchema).min(1),
	moves: z.array(deliverableMoveSchema).default([]),
	records: z.array(recordPlacementSchema).default([]),
	add: z.array(addedDeliverableSchema).default([]),
	withdraw: z.array(qualifiedAddress).default([]),
	dependsOnChanges: z.array(dependsOnChangeSchema).default([]),
	/** Projects to drop from initiative.ts once nothing is left in them. */
	remove: z.array(projectKey).default([]),
	/** Open questions the Lead keeps with the plan; printed back, never applied. */
	open: z.array(z.string()).default([]),
});

export type CutPlan = z.infer<typeof cutPlanSchema>;
export type PlannedProject = CutPlan['projects'][number];
export type AddedDeliverable = CutPlan['add'][number];
export type DependsOnChange = CutPlan['dependsOnChanges'][number];

const projectKeyInput = z
	.object({ projects: z.array(z.object({ key: z.string() }).loose()) })
	.loose();

/** The key of the planned project an issue points into, for a readable message. */
function plannedProjectKey(input: unknown, path: readonly PropertyKey[]): string | undefined {
	const [field, index] = path;
	if (field !== 'projects' || typeof index !== 'number') return undefined;
	const parsed = projectKeyInput.safeParse(input);
	return parsed.success ? parsed.data.projects[index]?.key : undefined;
}

export function parseCutPlan(file: string, text: string): CutPlan {
	let input: unknown;
	try {
		input = JSON.parse(text);
	} catch (error) {
		throw new Error(`${file}: ${error instanceof Error ? error.message : String(error)}`, {
			cause: error,
		});
	}
	const parsed = cutPlanSchema.safeParse(input);
	if (!parsed.success)
		throw new Error(
			parsed.error.issues
				.map(({ path, message }): string => {
					const named = plannedProjectKey(input, path);
					return `${file} ${path.join('.') || 'plan'}${named ? ` (${named})` : ''}: ${message}`;
				})
				.join('\n'),
		);
	const plan = parsed.data;
	const keys = new Set<string>();
	for (const { key } of plan.projects) {
		if (key === 'initiative')
			throw new Error(`${file}: the initiative namespace is reserved for shared records.`);
		if (keys.has(key)) throw new Error(`${file}: project ${key} is listed twice.`);
		keys.add(key);
	}
	const known = (key: string, where: string): void => {
		if (!keys.has(key))
			throw new Error(`${file}: ${where} names ${key}, which is not in projects.`);
	};
	const moved = new Set<string>();
	for (const { from, to } of plan.moves) {
		known(to, `the move of ${from}`);
		const home = from.slice(0, from.indexOf('/'));
		if (to === home)
			throw new Error(
				`${file}: the move of ${from} names its own project ${to}; leave it out of moves.`,
			);
		if (moved.has(from)) throw new Error(`${file}: ${from} is moved twice.`);
		moved.add(from);
	}
	for (const address of plan.withdraw)
		if (moved.has(address)) throw new Error(`${file}: ${address} is both moved and withdrawn.`);
	const changed = new Set<string>();
	for (const { deliverable } of plan.dependsOnChanges) {
		if (changed.has(deliverable))
			throw new Error(`${file}: ${deliverable} has two dependsOn changes; write them as one row.`);
		changed.add(deliverable);
	}
	for (const { record, to } of plan.records) known(to, `the record placement of ${record}`);
	for (const { project, title } of plan.add) known(project, `the added row "${title}"`);
	for (const key of plan.remove)
		if (keys.has(key))
			throw new Error(`${file}: ${key} is both a project of the plan and in remove.`);
	return plan;
}
