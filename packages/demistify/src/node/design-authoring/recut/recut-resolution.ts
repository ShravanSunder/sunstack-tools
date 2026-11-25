import type { DeliveryEntry } from '../delivery-model.ts';
import { recordRelationships, type DesignRecord } from '../schema.ts';
import type { CutPlan } from './cut-plan.ts';

/** What the re-cut reads from the initiative as it is: one set per registered project. */
export interface InitiativeSnapshot {
	readonly projects: readonly SnapshotProject[];
	/** Every record, qualified (`mac-side/R1`, `initiative/D1`). */
	readonly records: readonly DesignRecord[];
	/** Every delivery entry, qualified. */
	readonly entries: readonly DeliveryEntry[];
}

export interface SnapshotProject {
	readonly name: string;
	readonly number: number | undefined;
	readonly recordIds: readonly string[];
	readonly deliverableIds: readonly string[];
}

export interface Relocation {
	readonly kind: 'record' | 'deliverable';
	readonly from: string;
	readonly to: string;
	readonly oldId: string;
	readonly newId: string;
}

export interface RecordMove extends Relocation {
	readonly kind: 'record';
	/** The moving deliverables that took it, or the plan line that placed it. */
	readonly reason:
		| { readonly kind: 'scoped'; readonly movers: readonly string[] }
		| { readonly kind: 'plan' };
}

export interface DeliverableMove extends Relocation {
	readonly kind: 'deliverable';
	readonly records: readonly string[];
}

export interface SharedRecord {
	/** Qualified address before the run. */
	readonly record: string;
	readonly outcome:
		| {
				readonly kind: 'stays';
				readonly keptBy: readonly string[];
				readonly movers: readonly string[];
		  }
		| { readonly kind: 'split'; readonly to: string; readonly movers: readonly string[] };
}

export interface AddedRow {
	readonly project: string;
	readonly id: string;
	readonly title: string;
	readonly dependsOn: readonly string[];
	readonly why: string | undefined;
}

export interface WithdrawnRow {
	readonly address: string;
	readonly present: boolean;
	readonly referencedBy: readonly string[];
}

export interface DependsOnEdit {
	/** The deliverable's qualified address after the run. */
	readonly deliverable: string;
	readonly remove: readonly string[];
	readonly add: readonly string[];
	readonly why: string | undefined;
}

export interface RecutResolution {
	readonly recordMoves: ReadonlyMap<string, RecordMove>;
	readonly deliverableMoves: ReadonlyMap<string, DeliverableMove>;
	readonly shared: readonly SharedRecord[];
	readonly added: readonly AddedRow[];
	readonly withdrawn: readonly WithdrawnRow[];
	readonly dependsOnEdits: readonly DependsOnEdit[];
	/** Projects that end with no deliverables, and what stays in them. */
	readonly leftEmpty: readonly { readonly project: string; readonly records: readonly string[] }[];
}

export function projectOf(address: string): string {
	return address.slice(0, address.indexOf('/'));
}

export function localId(address: string): string {
	return address.slice(address.indexOf('/') + 1);
}

/**
 * The next id after the highest numbered id with the same prefix: `R12` → `R13`,
 * `DEL-07` → `DEL-08` (padding kept). An id without digits gets `-2`, `-3`, ….
 */
export function nextFreeId(id: string, taken: ReadonlySet<string>): string {
	const numbered = /^(.*?)(\d+)([A-Za-z]*)$/u.exec(id);
	if (!numbered) {
		for (let counter = 2; ; counter += 1) {
			const candidate = `${id}-${counter}`;
			if (!taken.has(candidate)) return candidate;
		}
	}
	const [, prefix = '', digits = '', suffix = ''] = numbered;
	let highest = 0;
	for (const candidate of taken) {
		const match = /^(.*?)(\d+)([A-Za-z]*)$/u.exec(candidate);
		if (match && match[1] === prefix) highest = Math.max(highest, Number(match[2]));
	}
	for (let next = highest + 1; ; next += 1) {
		const candidate = `${prefix}${String(next).padStart(digits.length, '0')}${suffix}`;
		if (!taken.has(candidate)) return candidate;
	}
}

/** Records a deliverable scopes in its own project: its scope and what those records rest on. */
function scopedClosure(entry: DeliveryEntry, records: ReadonlyMap<string, DesignRecord>): string[] {
	const { deliverable } = entry;
	if (deliverable.kind === 'later') return [];
	const home = projectOf(deliverable.id);
	const { needs, rules, designItems, verificationChecks } = deliverable.scope;
	const scopedRules = new Set(rules);
	const reached = new Set<string>();
	const visit = (address: string): void => {
		if (reached.has(address) || projectOf(address) !== home) return;
		const record = records.get(address);
		if (!record) return;
		reached.add(address);
		for (const { relation, targetId } of recordRelationships(record)) {
			if (relation === 'checks' && !scopedRules.has(targetId)) continue;
			visit(targetId);
		}
	};
	for (const address of [...needs, ...rules, ...designItems, ...verificationChecks]) visit(address);
	return [...reached];
}

export function resolveRecut(snapshot: InitiativeSnapshot, plan: CutPlan): RecutResolution {
	const projectsByName = new Map(snapshot.projects.map((project) => [project.name, project]));
	const records = new Map(snapshot.records.map((record) => [record.id, record]));
	const entries = new Map(snapshot.entries.map((entry) => [entry.deliverable.id, entry]));
	const planned = new Map(
		plan.projects.map((project, position) => [project.key, { ...project, position }]),
	);
	const deliverableTarget = new Map<string, string>();
	for (const { from, to } of plan.moves) {
		if (!projectsByName.has(projectOf(from)))
			throw new Error(`Move ${from}: no project ${projectOf(from)} in the initiative.`);
		if (!entries.has(from)) throw new Error(`Move ${from}: no such deliverable.`);
		deliverableTarget.set(from, to);
	}
	const withdrawnAddresses = new Set(plan.withdraw);
	const afterProject = (address: string): string =>
		deliverableTarget.get(address) ?? projectOf(address);

	// Which project owns each record afterwards.
	const scopers = new Map<string, string[]>();
	for (const entry of snapshot.entries) {
		if (withdrawnAddresses.has(entry.deliverable.id)) continue;
		for (const address of scopedClosure(entry, records)) {
			const list = scopers.get(address) ?? [];
			list.push(entry.deliverable.id);
			scopers.set(address, list);
		}
	}
	const planOrder = (key: string): [number, number] => {
		const project = planned.get(key);
		return [
			project?.number ?? Number.MAX_SAFE_INTEGER,
			project?.position ?? Number.MAX_SAFE_INTEGER,
		];
	};
	const overrides = new Map(plan.records.map((placement) => [placement.record, placement.to]));
	for (const address of overrides.keys())
		if (!records.has(address) || projectOf(address) === 'initiative')
			throw new Error(`Record placement ${address}: no such project record.`);
	const recordTarget = new Map<string, { to: string; reason: RecordMove['reason'] }>();
	const shared: SharedRecord[] = [];
	for (const record of snapshot.records) {
		const home = projectOf(record.id);
		if (home === 'initiative') continue;
		const override = overrides.get(record.id);
		if (override !== undefined) {
			if (override !== home)
				recordTarget.set(record.id, { to: override, reason: { kind: 'plan' } });
			continue;
		}
		const all = scopers.get(record.id) ?? [];
		const local = all.filter((address) => projectOf(address) === home);
		const movers = local.filter((address) => afterProject(address) !== home);
		const keptBy = all.filter((address) => afterProject(address) === home);
		if (!movers.length) continue;
		if (keptBy.length) {
			shared.push({ record: record.id, outcome: { kind: 'stays', keptBy, movers } });
			continue;
		}
		const targets = [...new Set(movers.map(afterProject))].toSorted((left, right) => {
			const [leftNumber, leftPosition] = planOrder(left);
			const [rightNumber, rightPosition] = planOrder(right);
			return leftNumber - rightNumber || leftPosition - rightPosition;
		});
		const [target] = targets;
		if (!target) continue;
		recordTarget.set(record.id, { to: target, reason: { kind: 'scoped', movers } });
		if (targets.length > 1)
			shared.push({ record: record.id, outcome: { kind: 'split', to: target, movers } });
	}

	// Ids already used in a project, before the run: an incoming item never reuses one.
	const takenIds = (project: string, kind: 'record' | 'deliverable'): Set<string> =>
		new Set(
			kind === 'record'
				? (projectsByName.get(project)?.recordIds ?? [])
				: (projectsByName.get(project)?.deliverableIds ?? []),
		);
	const taken = new Map<string, Set<string>>();
	const takenFor = (project: string, kind: 'record' | 'deliverable'): Set<string> => {
		const key = `${kind}:${project}`;
		const existing = taken.get(key);
		if (existing) return existing;
		const created = takenIds(project, kind);
		taken.set(key, created);
		return created;
	};
	const assign = (address: string, to: string, kind: 'record' | 'deliverable'): string => {
		const ids = takenFor(to, kind);
		const id = localId(address);
		const assigned = ids.has(id) ? nextFreeId(id, ids) : id;
		ids.add(assigned);
		return assigned;
	};

	const deliverableMoves = new Map<string, DeliverableMove>();
	for (const { from, to } of plan.moves) {
		const newId = assign(from, to, 'deliverable');
		deliverableMoves.set(from, {
			kind: 'deliverable',
			from,
			to: `${to}/${newId}`,
			oldId: localId(from),
			newId,
			records: [],
		});
	}
	// Records in plan order of the deliverables that take them, then authored order.
	const recordOrder = (address: string): number => {
		const target = recordTarget.get(address);
		if (target?.reason.kind !== 'scoped') return Number.MAX_SAFE_INTEGER;
		return Math.min(
			...target.reason.movers.map((mover) => plan.moves.findIndex(({ from }) => from === mover)),
		);
	};
	const recordMoves = new Map<string, RecordMove>();
	const authored = snapshot.records.map(({ id }) => id);
	for (const address of [...recordTarget.keys()].toSorted(
		(left, right) =>
			recordOrder(left) - recordOrder(right) || authored.indexOf(left) - authored.indexOf(right),
	)) {
		const target = recordTarget.get(address);
		if (!target) continue;
		const newId = assign(address, target.to, 'record');
		recordMoves.set(address, {
			kind: 'record',
			from: address,
			to: `${target.to}/${newId}`,
			oldId: localId(address),
			newId,
			reason: target.reason,
		});
		if (target.reason.kind === 'scoped')
			for (const mover of target.reason.movers) {
				const move = deliverableMoves.get(mover);
				if (move && projectOf(move.to) === target.to)
					deliverableMoves.set(mover, { ...move, records: [...move.records, address] });
			}
	}

	const withdrawn = plan.withdraw.map((address): WithdrawnRow => ({
		address,
		present: entries.has(address),
		referencedBy: snapshot.entries
			.filter(({ dependsOn }) => dependsOn.includes(address))
			.map(({ deliverable }) => deliverable.id),
	}));

	const translateDeliverable = (address: string): string =>
		deliverableMoves.get(address)?.to ?? address;
	const added = plan.add.map((row): AddedRow => {
		const ids = takenFor(row.project, 'deliverable');
		const template = [...ids].find((id) => /^DEL-\d+$/u.test(id)) ?? 'DEL-00';
		const id = nextFreeId(template, ids.size ? ids : new Set([template]));
		ids.add(id);
		return {
			project: row.project,
			id,
			title: row.title,
			dependsOn: row.dependsOn.map(translateDeliverable),
			why: row.why,
		};
	});

	const dependsOnEdits = plan.dependsOnChanges.map((change): DependsOnEdit => {
		if (!entries.has(change.deliverable))
			throw new Error(`dependsOn change for ${change.deliverable}: no such deliverable.`);
		if (withdrawnAddresses.has(change.deliverable))
			throw new Error(`dependsOn change for ${change.deliverable}: it is withdrawn.`);
		for (const address of [...change.from, ...change.to])
			if (!entries.has(address) && !withdrawnAddresses.has(address))
				throw new Error(`dependsOn change for ${change.deliverable}: no deliverable ${address}.`);
		return {
			deliverable: translateDeliverable(change.deliverable),
			remove: change.from.map(translateDeliverable),
			add: change.to
				.filter((address) => !withdrawnAddresses.has(address))
				.map(translateDeliverable),
			why: change.why,
		};
	});

	const leftEmpty = snapshot.projects.flatMap((project) => {
		const remaining = snapshot.entries.some(
			({ deliverable }) =>
				projectOf(deliverable.id) === project.name &&
				!withdrawnAddresses.has(deliverable.id) &&
				!deliverableMoves.has(deliverable.id),
		);
		const incoming = [...deliverableMoves.values()].some(
			({ to }) => projectOf(to) === project.name,
		);
		const addedHere = added.some((row) => row.project === project.name);
		if (remaining || incoming || addedHere) return [];
		return [
			{
				project: project.name,
				records: project.recordIds
					.map((id) => `${project.name}/${id}`)
					.filter((address) => !recordMoves.has(address)),
			},
		];
	});
	for (const key of plan.remove) {
		if (!projectsByName.has(key))
			throw new Error(`remove names ${key}, which is not in the initiative.`);
		const empty = leftEmpty.find(({ project }) => project === key);
		if (!empty) throw new Error(`remove names ${key}, which still has deliverables after the run.`);
		if (empty.records.length)
			throw new Error(
				`remove names ${key}, which still holds ${empty.records.join(', ')} after the run. Place them with "records" first.`,
			);
	}
	return { recordMoves, deliverableMoves, shared, added, withdrawn, dependsOnEdits, leftEmpty };
}
