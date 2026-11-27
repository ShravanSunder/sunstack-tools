import type { DeliverableView } from '../../core/deliverables-view.ts';
import type { ProjectState } from '../../core/initiative-view.ts';

export interface ProjectProgress {
	readonly state: ProjectState;
	readonly reason: string;
}

/** A project's state from its own deliverables. */
export function projectProgress(deliverables: readonly DeliverableView[]): ProjectProgress {
	const core = deliverables.filter((deliverable) => deliverable.kind === 'core');
	if (!core.length) return { state: 'later', reason: 'Only later deliverables are named.' };
	if (core.every((deliverable) => deliverable.state === 'accepted'))
		return { state: 'accepted', reason: 'Every core deliverable is accepted.' };
	const open = core.filter((deliverable) => deliverable.state !== 'accepted');
	const designBlocked = open.filter((deliverable) => deliverable.state === 'design-blocked');
	const dependencyBlocked = open.filter(
		(deliverable) => deliverable.state === 'dependency-blocked',
	);
	if (designBlocked.length + dependencyBlocked.length === open.length) {
		const waitingOn = [
			...new Set(dependencyBlocked.flatMap((deliverable) => deliverable.blockedBy)),
		];
		if (!designBlocked.length)
			return {
				state: 'dependency-blocked',
				reason: `Every open core deliverable waits on another: ${waitingOn.join(', ')}.`,
			};
		return {
			state: 'design-blocked',
			reason: `${designBlocked.map(({ id }) => id).join(', ')} wait on an open owner decision${
				dependencyBlocked.length ? `; the rest wait on ${waitingOn.join(', ')}` : ''
			}.`,
		};
	}
	const started = core.filter(
		(deliverable) =>
			deliverable.state !== 'planned' &&
			deliverable.state !== 'dependency-blocked' &&
			deliverable.state !== 'design-blocked',
	);
	if (started.length)
		return {
			state: 'in-progress',
			reason: `${started.length} of ${core.length} core deliverables started or done.`,
		};
	return { state: 'planned', reason: 'A core deliverable can start; none has started.' };
}

/**
 * What may start now (planned, with its steps written, lower project numbers first),
 * what still needs its plan, what blocks the most open work, and what is built but
 * not accepted.
 */
export function initiativeWork(
	deliverables: Readonly<Record<string, DeliverableView>>,
	projectNumber: (deliverableId: string) => number = (): number => 0,
): {
	readyNow: string[];
	needsPlan: string[];
	blocksMost: { id: string; waiting: number }[];
	builtNotAccepted: string[];
} {
	const views = Object.values(deliverables);
	const open = views.filter((view) => view.kind === 'core' && view.state !== 'accepted');
	const dependents = new Map<string, string[]>();
	for (const view of open)
		if (view.kind === 'core')
			for (const prerequisite of view.blockedBy)
				dependents.set(prerequisite, [...(dependents.get(prerequisite) ?? []), view.id]);
	const waitingOn = (id: string): number => {
		const seen = new Set<string>();
		const pending = [...(dependents.get(id) ?? [])];
		for (let next = pending.pop(); next !== undefined; next = pending.pop()) {
			if (seen.has(next)) continue;
			seen.add(next);
			pending.push(...(dependents.get(next) ?? []));
		}
		return seen.size;
	};
	return {
		readyNow: open
			.filter(
				(view) =>
					view.kind === 'core' && view.state === 'planned' && view.implementation.issues.length > 0,
			)
			.map(({ id }) => id)
			.toSorted((left, right) => projectNumber(left) - projectNumber(right)),
		needsPlan: open
			.filter((view) => view.kind === 'core' && view.implementation.issues.length === 0)
			.map(({ id }) => id)
			.toSorted((left, right) => projectNumber(left) - projectNumber(right)),
		blocksMost: open
			.map(({ id }) => ({ id, waiting: waitingOn(id) }))
			.filter(({ waiting }) => waiting > 0)
			.toSorted((left, right) => right.waiting - left.waiting || left.id.localeCompare(right.id))
			.slice(0, 10),
		builtNotAccepted: open
			.filter((view) => view.kind === 'core' && view.implementation.candidate !== undefined)
			.map(({ id }) => id),
	};
}
