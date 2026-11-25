import type { DeliverableView } from '../../core/deliverables-view.ts';
import type { ProjectState } from '../../core/initiative-view.ts';

export interface InitiativeOrder {
	/** Every project, each after the projects it depends on; ties keep the authored order. */
	readonly order: readonly string[];
	/** Groups of projects that depend on each other, so no order exists among them. */
	readonly loops: readonly (readonly string[])[];
}

/** The order an initiative's project dependencies produce, and any loops that prevent one. */
export function orderInitiativeProjects(
	projects: ReadonlyMap<string, readonly string[]>,
): InitiativeOrder {
	const authored = [...projects.keys()];
	const placed = new Set<string>();
	const order: string[] = [];
	let progressed = true;
	while (progressed) {
		progressed = false;
		for (const name of authored) {
			if (placed.has(name)) continue;
			if ((projects.get(name) ?? []).every((dependency) => placed.has(dependency))) {
				placed.add(name);
				order.push(name);
				progressed = true;
			}
		}
	}
	const remaining = authored.filter((name) => !placed.has(name));
	return { order: [...order, ...remaining], loops: dependencyLoops(projects, remaining) };
}

function dependencyLoops(
	projects: ReadonlyMap<string, readonly string[]>,
	remaining: readonly string[],
): string[][] {
	const reaches = (from: string, to: string): boolean => {
		const seen = new Set<string>();
		const pending = [...(projects.get(from) ?? [])];
		while (pending.length) {
			const next = pending.pop() as string;
			if (next === to) return true;
			if (seen.has(next)) continue;
			seen.add(next);
			pending.push(...(projects.get(next) ?? []));
		}
		return false;
	};
	const grouped = new Set<string>();
	const loops: string[][] = [];
	for (const name of remaining) {
		if (grouped.has(name) || !reaches(name, name)) continue;
		const loop = remaining.filter(
			(other) => other === name || (reaches(name, other) && reaches(other, name)),
		);
		for (const member of loop) grouped.add(member);
		loops.push(loop);
	}
	return loops;
}

/** Every project a project depends on, directly or through others. */
export function transitiveDependencies(
	projects: ReadonlyMap<string, readonly string[]>,
	name: string,
): Set<string> {
	const reached = new Set<string>();
	const pending = [...(projects.get(name) ?? [])];
	while (pending.length) {
		const next = pending.pop() as string;
		if (reached.has(next)) continue;
		reached.add(next);
		pending.push(...(projects.get(next) ?? []));
	}
	return reached;
}

export interface ProjectProgress {
	readonly state: ProjectState;
	readonly reason: string;
}

/**
 * A project's state from its own deliverables and the projects it depends on.
 * A project waits on a dependency until every core deliverable there is accepted.
 */
export function projectProgress(
	name: string,
	dependsOn: readonly string[],
	deliverables: readonly DeliverableView[],
	accepted: ReadonlySet<string>,
	loop: readonly string[] | undefined,
): ProjectProgress {
	if (loop)
		return {
			state: 'dependency-blocked',
			reason: `Depends on itself through ${loop.filter((other) => other !== name).join(', ')}; no order exists until the loop is broken.`,
		};
	const waiting = dependsOn.filter((dependency) => !accepted.has(dependency));
	if (waiting.length)
		return { state: 'dependency-blocked', reason: `Waits for ${waiting.join(', ')} to finish.` };
	const core = deliverables.filter((deliverable) => deliverable.kind === 'core');
	if (!core.length) return { state: 'later', reason: 'Only later deliverables are named.' };
	if (core.every((deliverable) => deliverable.state === 'accepted'))
		return { state: 'accepted', reason: 'Every core deliverable is accepted.' };
	const designBlocked = core.filter((deliverable) => deliverable.state === 'design-blocked');
	if (designBlocked.length)
		return {
			state: 'design-blocked',
			reason: `${designBlocked.map(({ id }) => id).join(', ')} wait on an open owner decision.`,
		};
	const started = core.filter(
		(deliverable) => deliverable.state !== 'planned' && deliverable.state !== 'dependency-blocked',
	);
	if (started.length)
		return {
			state: 'in-progress',
			reason: `${started.length} of ${core.length} core deliverables started or done.`,
		};
	return { state: 'planned', reason: 'No core deliverable has started.' };
}
