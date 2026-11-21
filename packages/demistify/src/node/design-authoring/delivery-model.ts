import { createHash } from 'node:crypto';

import { z } from 'zod';

import { scopeRevisionSchema, verificationReceiptSchema } from '../../core/deliverables-view.ts';
import type { DeliverableView } from '../../core/deliverables-view.ts';
import { designSchema } from './schema.ts';
import type { DesignCore, DesignRecord } from './schema.ts';

const [recordSchema] = designSchema.shape.records.element.options;
const { id: recordId, title: text, destination } = recordSchema.shape;
const evidenceHref = recordSchema.shape.sources.element.shape.href;
const scopedIds = z.array(recordId).default([]);
const address = z.string().regex(/^[A-Za-z][A-Za-z0-9-]*(?:\/[A-Za-z][A-Za-z0-9-]*)?$/);
const scopedAddresses = z.array(address).default([]);
const deliveryEntry = z.strictObject({
	dependsOn: scopedAddresses,
	deliverable: z.strictObject({
		id: address,
		title: text,
		destination,
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
});

export type DeliveryEntry = z.infer<typeof deliveryEntry>;
type Deliverable = DeliveryEntry['deliverable'];
type Acceptance = NonNullable<DesignCore['acceptances']>[number];
type DeliveryProgress = DeliverableView['progress'];
type VerificationCheck = DeliverableView['verification']['checks'][number];
type Destination = DeliverableView['links']['plan'];
type RecordDestination = DeliverableView['links']['rules'][number];

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
	readonly links: Omit<DeliverableView['links'], 'governingItems'>;
	readonly coverageGap?: string;
}

function resolveScope(
	deliverable: Deliverable,
	records: ReadonlyMap<string, DesignRecord>,
): ResolvedScope {
	const { id, scope } = deliverable;
	const uncoveredRules = new Set(scope.rules);
	const scopedRules = new Set(scope.rules);
	function resolve(ids: readonly string[], kind: DesignRecord['kind']): RecordDestination[] {
		checkUniqueIds(id, ids, kind);
		return ids.map((scopeId): RecordDestination => {
			const record = records.get(scopeId);
			if (record?.kind !== kind)
				throw new Error(`${id}: ${scopeId} must resolve to a design record of kind ${kind}.`);
			if (record.kind === 'planned-check') {
				const coveredRules = record.relationships.filter(({ targetId }): boolean =>
					scopedRules.has(targetId),
				);
				if (!coveredRules.length)
					throw new Error(`${id}: verification check ${scopeId} must cover a scoped rule.`);
				for (const { targetId } of coveredRules) uncoveredRules.delete(targetId);
			}
			return { id: scopeId, ...record.destination };
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
		...(uncoveredRules.size
			? {
					coverageGap: `Assign planned verification checks for scoped rules ${[...uncoveredRules].join(', ')}.`,
				}
			: {}),
	};
}

interface DesignBlocker {
	readonly reason: string;
	readonly blocksWork: boolean;
}

interface GoverningDesign {
	readonly scopeRevision: string;
	readonly governingItems: RecordDestination[];
	readonly designBlockers: readonly DesignBlocker[];
}

function resolveGoverningDesign(
	{ deliverable, dependsOn }: DeliveryEntry,
	records: ReadonlyMap<string, DesignRecord>,
	recordRevisions: ReadonlyMap<string, string>,
): GoverningDesign {
	const { id, scope, destination: planDestination } = deliverable;
	const linkedRevisions = new Map<string, string>();
	// The checker has validated the relationship kinds before calling the projector.
	function includeRecord(scopeId: string): void {
		if (linkedRevisions.has(scopeId)) return;
		const record = records.get(scopeId);
		if (!record) throw new Error(`${id}: missing governing design record ${scopeId}.`);
		const revision = scopeRevisionSchema.safeParse(recordRevisions.get(scopeId));
		if (!revision.success)
			throw new Error(`${id}: expected a SHA-256 revision for governing design record ${scopeId}.`);
		linkedRevisions.set(scopeId, revision.data);
		if ('relationships' in record) {
			for (const { targetId } of record.relationships) includeRecord(targetId);
		}
	}

	for (const scopeId of [...scope.rules, ...scope.designItems, ...scope.verificationChecks])
		includeRecord(scopeId);
	const revisions = [...linkedRevisions].toSorted(([left], [right]): number => {
		if (left < right) return -1;
		return left === right ? 0 : 1;
	});
	const canonicalScope = {
		rules: scope.rules.toSorted(),
		designItems: scope.designItems.toSorted(),
		implementationTasks: scope.implementationTasks.toSorted(),
		verificationChecks: scope.verificationChecks.toSorted(),
	};
	const scopeRevision = createHash('sha256')
		.update(
			JSON.stringify({
				id,
				scope: canonicalScope,
				destination: planDestination,
				dependsOn: dependsOn.toSorted(),
				revisions,
			}),
		)
		.digest('hex');
	const governingItems: RecordDestination[] = [];
	const designBlockers: DesignBlocker[] = [];
	for (const [scopeId] of revisions) {
		const record = records.get(scopeId);
		if (record?.kind === 'choice') {
			governingItems.push({ id: scopeId, ...record.destination });
			if (record.authority.state === 'owner-open')
				designBlockers.push({ blocksWork: true, reason: `${scopeId} requires an owner decision.` });
		} else if (record?.kind === 'assumption') {
			governingItems.push({ id: scopeId, ...record.destination });
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
	return { scopeRevision, governingItems, designBlockers };
}

function projectImplementation(deliverable: Deliverable): DeliverableView['implementation'] {
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

function projectVerification(
	deliverable: Deliverable,
	scopeRevision: string,
): DeliverableView['verification'] {
	const { id, scope, implementation, verification, verificationHistory } = deliverable;
	const outcomes = new Map(Object.entries(verification?.outcomes ?? {}));
	for (const checkId of outcomes.keys()) {
		if (!scope.verificationChecks.includes(checkId))
			throw new Error(`${id}: unknown verification check ${checkId} in outcomes.`);
	}

	const build = verification?.build;
	const candidate = implementation?.candidate;
	return {
		...(verification === undefined
			? {}
			: { build: verification.build, scopeRevision: verification.scopeRevision }),
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
						? `Outcome is for ${build}. Recheck candidate ${candidate}.`
						: `Outcome is for ${build}. Record the implementation candidate.`,
					outcome,
				};
			if (verification?.scopeRevision !== scopeRevision)
				return {
					id: checkId,
					state: 'stale',
					reason:
						'Outcome covers a different scope revision. Recheck this outcome against the current scope.',
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
	readonly deliverable: Deliverable;
	readonly scopeRevision: string;
	readonly coverageGap: string | undefined;
	readonly designBlockers: readonly DesignBlocker[];
	readonly implementation: DeliverableView['implementation'];
	readonly verification: DeliverableView['verification'];
	readonly acceptances: readonly Acceptance[];
}

function projectProgress({
	deliverable,
	scopeRevision,
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
			reason:
				verification.build === candidate && verification.scopeRevision !== scopeRevision
					? `Recheck candidate ${candidate} against the current scope revision.`
					: `Record every scoped verification outcome for candidate ${candidate}.`,
		};

	const acceptanceBlocker = designBlockers[0];
	if (acceptanceBlocker) return { state: 'design-blocked', reason: acceptanceBlocker.reason };

	const acceptance = acceptances.find(
		(receipt): boolean => receipt.build === candidate && receipt.scopeRevision === scopeRevision,
	);
	if (acceptance)
		return {
			state: 'accepted',
			reason: `The Lead accepted candidate ${candidate}.`,
			acceptance,
		};
	return {
		state: 'ready-for-acceptance',
		reason: `Record Lead acceptance for candidate ${candidate}.`,
	};
}

function deliverablesInDependencyOrder(
	entries: ReadonlyMap<string, DeliveryEntry>,
): readonly DeliveryEntry[] {
	const ready: DeliveryEntry[] = [];
	const currentPath = new Set<string>();
	const visited = new Set<string>();

	function visit(entry: DeliveryEntry): void {
		const {
			deliverable: { id },
			dependsOn,
		} = entry;
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
	entries: unknown,
	recordRevisions: ReadonlyMap<string, string>,
	trackPrerequisiteRevisions = false,
): DeliveryProjection {
	const byId = new Map<string, DeliveryEntry>();
	for (const entry of parseDeliveryEntries(entries)) {
		const { id } = entry.deliverable;
		if (byId.has(id)) throw new Error(`${id}: duplicate deliverable ID.`);
		byId.set(id, entry);
	}

	const acceptancesById = new Map<string, Acceptance[]>();
	const evidence = new Set<string>();
	for (const acceptance of design.acceptances ?? []) {
		const { deliverableId, build, scopeRevision } = acceptance;
		if (!byId.has(deliverableId))
			throw new Error(`${deliverableId}: acceptance names an unknown deliverable.`);
		const previous = acceptancesById.get(deliverableId) ?? [];
		if (
			previous.some(
				(receipt): boolean => receipt.build === build && receipt.scopeRevision === scopeRevision,
			)
		)
			throw new Error(
				`${deliverableId}: duplicate Lead acceptance for build ${build} and scope revision ${scopeRevision}.`,
			);
		acceptancesById.set(deliverableId, [...previous, acceptance]);
		evidence.add(acceptance.evidence);
	}

	const records = new Map(design.records.map((record) => [record.id, record]));
	const destinations: Destination[] = [];
	const byDependencyOrder = new Map<string, DeliverableView>();
	for (const entry of deliverablesInDependencyOrder(byId)) {
		const { deliverable, dependsOn } = entry;
		const { id, title } = deliverable;
		const { links: scopeLinks, coverageGap } = resolveScope(deliverable, records);
		const governing = resolveGoverningDesign(entry, records, recordRevisions);
		const { governingItems, designBlockers } = governing;
		const scopeRevision =
			trackPrerequisiteRevisions && dependsOn.length
				? createHash('sha256')
						.update(
							JSON.stringify([
								governing.scopeRevision,
								dependsOn.toSorted().map((prerequisiteId) => {
									const prerequisite = byDependencyOrder.get(prerequisiteId);
									return [
										prerequisiteId,
										prerequisite?.scopeRevision,
										prerequisite?.implementation.candidate,
									];
								}),
							]),
						)
						.digest('hex')
				: governing.scopeRevision;
		const links = { ...scopeLinks, governingItems };
		const implementation = projectImplementation(deliverable);
		const verification = projectVerification(deliverable, scopeRevision);
		const acceptances = acceptancesById.get(id) ?? [];
		const progress = projectProgress({
			deliverable,
			scopeRevision,
			coverageGap,
			designBlockers,
			implementation,
			verification,
			acceptances,
		});
		const unmetPrerequisites = dependsOn.filter(
			(prerequisite): boolean => byDependencyOrder.get(prerequisite)?.state !== 'accepted',
		);

		const reasons = [...new Set([progress.reason, ...designBlockers.map(({ reason }) => reason)])];
		if (coverageGap && coverageGap !== progress.reason) reasons.push(coverageGap);
		if (unmetPrerequisites.length)
			reasons.unshift(`Awaiting acceptance of prerequisites: ${unmetPrerequisites.join(', ')}.`);
		if (progress.state !== 'accepted') {
			const staleBuilds = new Set(
				acceptances
					.filter(({ build }): boolean => build !== implementation.candidate)
					.map(({ build }): string => build),
			);
			if (staleBuilds.size)
				reasons.push(
					`Lead acceptance for ${[...staleBuilds].join(', ')} does not match the current candidate.`,
				);
			if (acceptances.some((receipt): boolean => receipt.scopeRevision !== scopeRevision))
				reasons.push('A recorded Lead acceptance covers a different scope revision.');
		}

		byDependencyOrder.set(id, {
			id,
			title,
			scopeRevision,
			dependsOn,
			state: unmetPrerequisites.length ? 'dependency-blocked' : progress.state,
			reasons,
			progress,
			implementation,
			verification,
			links,
		});
		destinations.push(
			links.plan,
			...links.rules,
			...links.designItems,
			...links.verificationChecks,
		);
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
