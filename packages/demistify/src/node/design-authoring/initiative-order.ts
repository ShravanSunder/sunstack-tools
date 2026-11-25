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
		for (let next = pending.pop(); next !== undefined; next = pending.pop()) {
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

export interface ProjectProgress {
	readonly state: ProjectState;
	readonly reason: string;
}

/**
 * A project's state. Projects are sequenced by the initiative: a project waits
 * until every project it depends on is accepted. Inside a project, its own
 * deliverables' states decide the rest.
 */
export function projectProgress(
	name: string,
	dependsOn: readonly string[],
	deliverables: readonly DeliverableView[],
	isAccepted: (dependency: string) => boolean,
	loop: readonly string[] | undefined,
): ProjectProgress {
	if (loop)
		return {
			state: 'dependency-blocked',
			reason: `Depends on itself through ${loop.filter((other) => other !== name).join(', ')}; no order exists until the loop is broken.`,
		};
	const waiting = dependsOn.filter((dependency) => !isAccepted(dependency));
	if (waiting.length)
		return { state: 'dependency-blocked', reason: `Waits for ${waiting.join(', ')} to finish.` };
	const core = deliverables.filter((deliverable) => deliverable.kind === 'core');
	if (!core.length) return { state: 'later', reason: 'Only later deliverables are named.' };
	if (core.every((deliverable) => deliverable.state === 'accepted'))
		return { state: 'accepted', reason: 'Every core deliverable is accepted.' };
	const open = core.filter((deliverable) => deliverable.state !== 'accepted');
	const designBlocked = open.filter((deliverable) => deliverable.state === 'design-blocked');
	if (designBlocked.length === open.length)
		return {
			state: 'design-blocked',
			reason: `${designBlocked.map(({ id }) => id).join(', ')} wait on an open owner decision.`,
		};
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
	return { state: 'planned', reason: 'Ready to start; no core deliverable has started.' };
}

interface MilestoneInput {
	readonly title: string;
	readonly check: string;
	readonly includes: readonly string[];
}

/** Each milestone with its included deliverables expanded and its state from theirs. */
export function initiativeMilestones(
	milestones: Readonly<Record<string, MilestoneInput>>,
	deliverables: Readonly<Record<string, DeliverableView>>,
): Record<
	string,
	MilestoneInput & {
		readonly deliverables: string[];
		readonly state: 'accepted' | 'open';
		readonly reason: string;
	}
> {
	const expand = (key: string, path: readonly string[]): string[] => {
		if (path.includes(key))
			throw new Error(`Milestones include each other: ${[...path, key].join(' -> ')}.`);
		const milestone = milestones[key];
		if (!milestone) throw new Error(`Unknown milestone ${key}.`);
		return milestone.includes.flatMap((included) => {
			if (!included.includes('/')) return expand(included, [...path, key]);
			if (!deliverables[included])
				throw new Error(`Milestone ${key} includes unknown deliverable ${included}.`);
			return [included];
		});
	};
	return Object.fromEntries(
		Object.entries(milestones).map(([key, milestone]) => {
			const needed = [...new Set(expand(key, []))];
			const open = needed.filter((id) => deliverables[id]?.state !== 'accepted');
			return [
				key,
				{
					title: milestone.title,
					check: milestone.check,
					includes: [...milestone.includes],
					deliverables: needed,
					state: open.length ? 'open' : 'accepted',
					reason: open.length
						? `${open.length} of ${needed.length} deliverables not accepted: ${open.join(', ')}.`
						: 'Every included deliverable is accepted.',
				},
			];
		}),
	);
}

/** What may start now, what blocks the most open work, and what is built but not accepted. */
export function initiativeWork(deliverables: Readonly<Record<string, DeliverableView>>): {
	readyNow: string[];
	blocksMost: { id: string; waiting: number }[];
	builtNotAccepted: string[];
} {
	const views = Object.values(deliverables);
	const open = views.filter((view) => view.kind === 'core' && view.state !== 'accepted');
	const dependents = new Map<string, string[]>();
	for (const view of open)
		if (view.kind === 'core')
			for (const prerequisite of view.dependsOn)
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
		readyNow: open.filter((view) => view.state === 'planned').map(({ id }) => id),
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
