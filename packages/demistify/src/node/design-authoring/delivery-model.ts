import { z } from 'zod';

import {
	verificationReceiptSchema,
	type CoreDeliverableView,
	type DeliverableView,
} from '../../core/deliverables-view.ts';
import {
	htmlDestination,
	nonEmptyText as text,
	recordId,
	relativeHref as evidenceHref,
	recordReference as address,
	recordLabel,
	recordRelationships,
} from './schema.ts';
import type { DesignCore, DesignRecord } from './schema.ts';

const scopedIds = z.array(recordId).default([]);
const scopedAddresses = z.array(address).default([]);
const deliveryEntry = z.strictObject({
	dependsOn: scopedAddresses,
	deliverable: z.discriminatedUnion('kind', [
		z.strictObject({
			kind: z.literal('core'),
			id: address,
			title: text,
			builder: recordId.optional(),
			destination: htmlDestination,
			scope: z.strictObject({
				rules: scopedAddresses,
				designItems: scopedAddresses,
				implementationTasks: scopedIds,
				verificationChecks: scopedAddresses,
			}),
			implementation: z
				.strictObject({
					completedTasks: scopedIds,
					candidate: text.optional(),
					evidence: z.array(evidenceHref).default([]),
				})
				.optional(),
			verification: verificationReceiptSchema
				.extend({ outcomes: verificationReceiptSchema.shape.outcomes.default({}) })
				.optional(),
			verificationHistory: z.array(verificationReceiptSchema).default([]),
		}),
		z.strictObject({ kind: z.literal('later'), id: address, title: text }),
	]),
});

export type DeliveryEntry = z.infer<typeof deliveryEntry>;
export type CoreDeliverable = Extract<DeliveryEntry['deliverable'], { kind: 'core' }>;
type Acceptance = NonNullable<DesignCore['acceptances']>[number];
type DeliveryProgress = CoreDeliverableView['progress'];
type VerificationCheck = CoreDeliverableView['verification']['checks'][number];
export type Destination = CoreDeliverableView['links']['plan'];
type RecordDestination = CoreDeliverableView['links']['rules'][number];

export interface DeliveryProjection {
	readonly deliverables: Readonly<Record<string, DeliverableView>>;
	readonly destinations: readonly { readonly page: string; readonly fragment?: string }[];
	readonly evidence: readonly string[];
}

function parseEntry(input: unknown, index: number): DeliveryEntry {
	const result = deliveryEntry.safeParse(input);
	if (result.success) return result.data;

	const identity = z
		.object({
			deliverable: z.object({ id: recordId }),
		})
		.safeParse(input);
	const label = identity.success ? identity.data.deliverable.id : `Delivery entry ${index + 1}`;
	const issues = result.error.issues.map(
		({ path, message }): string => `${path.join('.') || 'entry'}: ${message}`,
	);
	throw new Error(`${label}: ${issues.join('; ')}`);
}

function checkUniqueIds(deliverableId: string, ids: readonly string[], label: string): void {
	const seen = new Set<string>();
	for (const id of ids) {
		if (seen.has(id)) throw new Error(`${deliverableId}: duplicate ${label} ${id}.`);
		seen.add(id);
	}
}

interface ResolvedScope {
	readonly links: Omit<CoreDeliverableView['links'], 'governingItems'>;
	readonly coverageGaps: readonly string[];
}

function resolveScope(
	deliverable: CoreDeliverable,
	records: ReadonlyMap<string, DesignRecord>,
	destinations: ReadonlyMap<string, Destination>,
): ResolvedScope {
	const { id, scope } = deliverable;
	const uncoveredRules = new Set(scope.rules);
	const scopedRules = new Set(scope.rules);
	const rulesWithoutNeeds: string[] = [];
	function resolve(ids: readonly string[], kind: DesignRecord['kind']): RecordDestination[] {
		checkUniqueIds(id, ids, kind);
		return ids.map((scopeId): RecordDestination => {
			const record = records.get(scopeId);
			if (record?.kind !== kind)
				throw new Error(`${id}: ${scopeId} must resolve to a design record of kind ${kind}.`);
			if (record.kind === 'rule' && record.serves === undefined) rulesWithoutNeeds.push(scopeId);
			if (record.kind === 'planned-check') {
				const coveredRules = recordRelationships(record).filter(
					({ relation, targetId }): boolean => relation === 'checks' && scopedRules.has(targetId),
				);
				if (!coveredRules.length)
					throw new Error(`${id}: verification check ${scopeId} must cover a scoped rule.`);
				for (const { targetId } of coveredRules) uncoveredRules.delete(targetId);
			}
			return deliveryRecordLink(record, destinations);
		});
	}
	const links = {
		plan: deliverable.destination,
		rules: resolve(scope.rules, 'rule'),
		designItems: resolve(scope.designItems, 'realization'),
		verificationChecks: resolve(scope.verificationChecks, 'planned-check'),
	};
	return {
		links,
		coverageGaps: [
			...(rulesWithoutNeeds.length
				? [`Link scoped rules to a need: ${rulesWithoutNeeds.join(', ')}.`]
				: []),
			...(uncoveredRules.size
				? [`Assign planned verification checks for scoped rules ${[...uncoveredRules].join(', ')}.`]
				: []),
		],
	};
}

function deliveryRecordLink(
	record: DesignRecord,
	destinations: ReadonlyMap<string, Destination>,
): RecordDestination {
	const destination = destinations.get(record.id);
	return { id: record.id, label: recordLabel(record), ...(destination && { destination }) };
}

interface DesignBlocker {
	readonly reason: string;
	readonly blocksWork: boolean;
}

interface GoverningDesign {
	readonly governingItems: RecordDestination[];
	readonly designBlockers: readonly DesignBlocker[];
}

export function governingRecords(
	deliverable: CoreDeliverable,
	records: ReadonlyMap<string, DesignRecord>,
): DesignRecord[] {
	const linkedRecords = new Map<string, DesignRecord>();
	const { rules, designItems, verificationChecks } = deliverable.scope;
	const scopedRules = new Set(rules);
	function includeRecord(scopeId: string): void {
		if (linkedRecords.has(scopeId)) return;
		const record = records.get(scopeId);
		if (!record) throw new Error(`${deliverable.id}: missing governing design record ${scopeId}.`);
		linkedRecords.set(scopeId, record);
		for (const { relation, targetId } of recordRelationships(record)) {
			if ((relation === 'checks' || relation === 'realizes') && !scopedRules.has(targetId))
				continue;
			includeRecord(targetId);
		}
	}

	for (const scopeId of [...rules, ...designItems, ...verificationChecks]) includeRecord(scopeId);
	return [...linkedRecords.values()];
}

function resolveGoverningDesign(
	deliverable: CoreDeliverable,
	records: ReadonlyMap<string, DesignRecord>,
	destinations: ReadonlyMap<string, Destination>,
): GoverningDesign {
	const governingItems: RecordDestination[] = [];
	const designBlockers: DesignBlocker[] = [];
	for (const record of governingRecords(deliverable, records)) {
		const scopeId = record.id;
		if (record?.kind === 'choice') {
			governingItems.push(deliveryRecordLink(record, destinations));
			if (record.authority.state === 'owner-open')
				designBlockers.push({ blocksWork: true, reason: `${scopeId} requires an owner decision.` });
		} else if (record?.kind === 'assumption') {
			governingItems.push(deliveryRecordLink(record, destinations));
			if (record.status === 'refuted')
				designBlockers.push({
					blocksWork: true,
					reason: `Assumption ${scopeId} was refuted. Await Lead review.`,
				});
			else if (record.status === 'open')
				designBlockers.push({
					blocksWork: false,
					reason: `Confirm assumption ${scopeId} before acceptance.`,
				});
		}
	}
	return { governingItems, designBlockers };
}

function projectImplementation(
	deliverable: CoreDeliverable,
): CoreDeliverableView['implementation'] {
	const { id, scope, implementation } = deliverable;
	const completedTasks = implementation?.completedTasks ?? [];
	checkUniqueIds(id, scope.implementationTasks, 'implementation task in scope');
	checkUniqueIds(id, completedTasks, 'completed task');
	for (const taskId of completedTasks) {
		if (!scope.implementationTasks.includes(taskId))
			throw new Error(`${id}: unknown implementation task ${taskId} in completedTasks.`);
	}
	const completed = new Set(completedTasks);
	return {
		...(implementation?.candidate === undefined ? {} : { candidate: implementation.candidate }),
		evidence: implementation?.evidence ?? [],
		tasks: scope.implementationTasks.map((taskId) => ({
			id: taskId,
			state: completed.has(taskId) ? 'completed' : 'open',
		})),
	};
}

function projectVerification(deliverable: CoreDeliverable): CoreDeliverableView['verification'] {
	const { id, scope, implementation, verification, verificationHistory } = deliverable;
	const outcomes = new Map(Object.entries(verification?.outcomes ?? {}));
	for (const checkId of outcomes.keys()) {
		if (!scope.verificationChecks.includes(checkId))
			throw new Error(`${id}: unknown verification check ${checkId} in outcomes.`);
	}

	const build = verification?.build;
	const candidate = implementation?.candidate;
	return {
		...(verification === undefined ? {} : { build: verification.build }),
		...(verificationHistory.length ? { history: verificationHistory } : {}),
		checks: scope.verificationChecks.map((checkId): VerificationCheck => {
			const outcome = outcomes.get(checkId);
			if (!outcome)
				return { id: checkId, state: 'missing', reason: 'Record an outcome for this check.' };
			if (build !== candidate)
				return {
					id: checkId,
					state: 'stale',
					reason: candidate
						? 'Recheck this outcome on the current build.'
						: 'Record the implementation candidate.',
					outcome,
				};
			if (outcome.kind === 'blocked')
				return { id: checkId, state: 'blocked', reason: outcome.reason, outcome };
			if (outcome.kind === 'failed') return { id: checkId, state: 'failed', outcome };
			return { id: checkId, state: 'passed', outcome };
		}),
	};
}

interface ProgressInputs {
	readonly deliverable: CoreDeliverable;
	readonly coverageGap: string | undefined;
	readonly designBlockers: readonly DesignBlocker[];
	readonly implementation: CoreDeliverableView['implementation'];
	readonly verification: CoreDeliverableView['verification'];
	readonly acceptances: readonly Acceptance[];
}

function projectProgress({
	deliverable,
	coverageGap,
	designBlockers,
	implementation,
	verification,
	acceptances,
}: ProgressInputs): DeliveryProgress {
	const workBlocker = designBlockers.find(({ blocksWork }): boolean => blocksWork);
	if (workBlocker) return { state: 'design-blocked', reason: workBlocker.reason };

	const { scope } = deliverable;
	const { candidate, tasks, evidence } = implementation;
	const missingScope = [
		...(!scope.rules.length ? ['governing rules'] : []),
		...(!tasks.length ? ['implementation tasks'] : []),
	];
	const started =
		candidate !== undefined ||
		evidence.length > 0 ||
		tasks.some(({ state }): boolean => state === 'completed');
	if (missingScope.length)
		return {
			state: started ? 'implementation-in-progress' : 'planned',
			reason: `Define the scoped ${missingScope.join(', ')}.`,
		};
	if (!started) return { state: 'planned', reason: 'Start the scoped implementation tasks.' };
	const openTasks = tasks.filter(({ state }): boolean => state === 'open');
	if (openTasks.length)
		return {
			state: 'implementation-in-progress',
			reason: `Complete implementation tasks: ${openTasks.map(({ id }): string => id).join(', ')}.`,
		};
	if (!candidate)
		return { state: 'implementation-in-progress', reason: 'Record the implementation candidate.' };
	if (coverageGap) return { state: 'awaiting-verification', reason: coverageGap };

	const failed = verification.checks.filter(({ state }): boolean => state === 'failed');
	const blocked = verification.checks.filter(
		(check): check is Extract<VerificationCheck, { state: 'blocked' }> => check.state === 'blocked',
	);
	if (failed.length)
		return {
			state: 'verification-failed',
			reason: `Verification failed: ${failed.map(({ id }): string => id).join(', ')}.`,
		};
	if (blocked.length)
		return {
			state: 'verification-blocked',
			reason: blocked.map(({ id, reason }): string => `${id} is blocked. ${reason}`).join(' '),
		};
	if (verification.checks.some(({ state }): boolean => state !== 'passed'))
		return {
			state: 'awaiting-verification',
			reason: 'Record the verification results for the current build.',
		};

	const acceptanceBlocker = designBlockers[0];
	if (acceptanceBlocker) return { state: 'design-blocked', reason: acceptanceBlocker.reason };

	const acceptance = acceptances.find(({ build }) => build === candidate);
	if (acceptance)
		return {
			state: 'accepted',
			reason: `The Lead accepted candidate ${candidate}.`,
			acceptance,
		};
	return {
		state: 'ready-for-acceptance',
		reason: 'Record the Lead’s acceptance.',
	};
}

function deliverablesInDependencyOrder(
	entries: ReadonlyMap<string, DeliveryEntry>,
): readonly DeliveryEntry[] {
	const ready: DeliveryEntry[] = [];
	const currentPath = new Set<string>();
	const visited = new Set<string>();

	function visit(entry: DeliveryEntry): void {
		const { deliverable, dependsOn } = entry;
		const { id } = deliverable;
		if (deliverable.kind === 'later' && dependsOn.length)
			throw new Error(`${id}: only core deliverables can declare prerequisites.`);
		if (visited.has(id)) return;
		if (currentPath.has(id)) {
			const path = [...currentPath];
			const cycle = path.slice(path.indexOf(id));
			throw new Error(`Delivery dependency cycle: ${[...cycle, id].join(' -> ')}.`);
		}
		currentPath.add(id);
		checkUniqueIds(id, dependsOn, 'prerequisite');
		for (const prerequisite of dependsOn) {
			const dependency = entries.get(prerequisite);
			if (!dependency) throw new Error(`${id}: unknown prerequisite ${prerequisite}.`);
			visit(dependency);
		}
		currentPath.delete(id);
		visited.add(id);
		ready.push(entry);
	}

	for (const entry of entries.values()) visit(entry);
	return ready;
}

export function parseDeliveryEntries(entries: unknown): DeliveryEntry[] {
	const parsed = z.array(z.unknown()).safeParse(entries);
	if (!parsed.success) throw new Error('Delivery entries must be an array.');
	return parsed.data.map(parseEntry);
}

export function projectDelivery(
	design: DesignCore,
	input: unknown,
	recordDestinations: ReadonlyMap<string, Destination>,
): DeliveryProjection {
	const byId = new Map<string, DeliveryEntry>();
	for (const entry of parseDeliveryEntries(input)) {
		const { id } = entry.deliverable;
		if (byId.has(id)) throw new Error(`${id}: duplicate deliverable ID.`);
		byId.set(id, entry);
	}

	const acceptancesById = new Map<string, Acceptance[]>();
	const evidence = new Set<string>();
	for (const acceptance of design.acceptances ?? []) {
		const { deliverableId, build } = acceptance;
		if (byId.get(deliverableId)?.deliverable.kind !== 'core')
			throw new Error(`${deliverableId}: acceptance must name a core deliverable.`);
		const previous = acceptancesById.get(deliverableId) ?? [];
		if (previous.some((receipt) => receipt.build === build))
			throw new Error(`${deliverableId}: duplicate Lead acceptance for the same build.`);
		acceptancesById.set(deliverableId, [...previous, acceptance]);
		evidence.add(acceptance.evidence);
	}

	const records = new Map(design.records.map((record) => [record.id, record]));
	const destinations: Destination[] = [];
	const byDependencyOrder = new Map<string, DeliverableView>();
	for (const { deliverable, dependsOn } of deliverablesInDependencyOrder(byId)) {
		const { id, title } = deliverable;
		if (deliverable.kind === 'later') {
			byDependencyOrder.set(id, { kind: 'later', id, title, state: 'later' });
			continue;
		}
		const { builder } = deliverable;
		const { links: scopeLinks, coverageGaps } = resolveScope(
			deliverable,
			records,
			recordDestinations,
		);
		const { governingItems, designBlockers } = resolveGoverningDesign(
			deliverable,
			records,
			recordDestinations,
		);
		const links = { ...scopeLinks, governingItems };
		const implementation = projectImplementation(deliverable);
		const verification = projectVerification(deliverable);
		const acceptances = acceptancesById.get(id) ?? [];
		const progress = projectProgress({
			deliverable,
			coverageGap: coverageGaps[0],
			designBlockers,
			implementation,
			verification,
			acceptances,
		});
		const unmetPrerequisites = dependsOn.filter(
			(prerequisite): boolean => byDependencyOrder.get(prerequisite)?.state !== 'accepted',
		);

		const reasons = [...new Set([progress.reason, ...designBlockers.map(({ reason }) => reason)])];
		for (const gap of coverageGaps) if (gap !== progress.reason) reasons.push(gap);
		if (unmetPrerequisites.length)
			reasons.unshift(`Awaiting acceptance of prerequisites: ${unmetPrerequisites.join(', ')}.`);
		if (progress.state !== 'accepted') {
			if (acceptances.some(({ build }) => build !== implementation.candidate))
				reasons.push('The earlier Lead acceptance is for a different build.');
		}

		byDependencyOrder.set(id, {
			kind: 'core',
			id,
			title,
			...(builder === undefined ? {} : { builder }),
			dependsOn,
			state: unmetPrerequisites.length ? 'dependency-blocked' : progress.state,
			reasons,
			progress,
			implementation,
			verification,
			links,
		});
		destinations.push(links.plan);
		for (const href of implementation.evidence) evidence.add(href);
		for (const check of verification.checks) {
			if ('outcome' in check && 'evidence' in check.outcome) evidence.add(check.outcome.evidence);
		}
		for (const receipt of verification.history ?? []) {
			for (const outcome of Object.values(receipt.outcomes)) {
				if ('evidence' in outcome) evidence.add(outcome.evidence);
			}
		}
	}
	return {
		deliverables: Object.fromEntries(byDependencyOrder),
		destinations: destinations.map(({ page, fragment }) => ({ page, fragment })),
		evidence: [...evidence],
	};
}
