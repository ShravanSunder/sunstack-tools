import { realpath } from 'node:fs/promises';
import { dirname, join, relative, resolve, sep } from 'node:path';

import type { DeliverableView } from '../../core/deliverables-view.ts';
import { projectDecisions, projectViewSchema } from '../../core/project-view.ts';
import { checkRecordRelationships, openDesignCheck } from './check-design.ts';
import { type DeliveryEntry, type DeliveryProjection, projectDelivery } from './delivery-model.ts';
import { commitDesignOutputs, planGeneratedJson } from './design-output.ts';
import { checkAuthoringTypes, readDesignItems } from './read-design.ts';
import {
	projectRecordTypeImports,
	readProjectIdentities,
	readProjectManifest,
} from './read-project.ts';
import type { DesignCore, DesignRecord } from './schema.ts';

type EvidenceReceipt = NonNullable<DeliveryEntry['deliverable']['verification']>;
type EvidenceOutcome = EvidenceReceipt['outcomes'][string];

function qualify(name: string, id: string): string {
	return id.includes('/') ? id : `${name}/${id}`;
}

function rebase(href: string, from: string, to: string): string {
	const prefix = relative(to, from);
	return prefix ? `${prefix.split(sep).map(encodeURIComponent).join('/')}/${href}` : href;
}

function rebaseOutcome<TOutcome extends EvidenceOutcome>(
	receipt: TOutcome,
	href: (href: string) => string,
): TOutcome {
	return 'evidence' in receipt ? { ...receipt, evidence: href(receipt.evidence) } : receipt;
}

function projectRecord(
	record: DesignRecord,
	name: string,
	href: (href: string) => string,
): DesignRecord {
	const fields = {
		id: qualify(name, record.id),
		destination: { ...record.destination, page: href(record.destination.page) },
		sources: record.sources.map((ref) => ({ ...ref, href: href(ref.href) })),
	};
	const links = <TLink extends { targetId: string }>(items: readonly TLink[]): TLink[] =>
		items.map((link) => ({ ...link, targetId: qualify(name, link.targetId) }));
	switch (record.kind) {
		case 'rule':
			return { ...record, ...fields, relationships: links(record.relationships) };
		case 'realization':
			return { ...record, ...fields, relationships: links(record.relationships) };
		case 'planned-check':
			return { ...record, ...fields, relationships: links(record.relationships) };
		case 'choice':
			return {
				...record,
				...fields,
				...(record.supersedes && { supersedes: qualify(name, record.supersedes) }),
			};
		case 'assumption':
			return {
				...record,
				...fields,
				...(record.resolution && {
					resolution: { ...record.resolution, href: href(record.resolution.href) },
				}),
			};
		case 'need':
			return { ...record, ...fields };
		default:
			throw new Error('Unsupported project design record.');
	}
}

function projectEntry(
	entry: DeliveryEntry,
	name: string,
	href: (href: string) => string,
): DeliveryEntry {
	const { deliverable: work } = entry;
	const ids = (references: readonly string[]): string[] =>
		references.map((id) => qualify(name, id));
	const verification = (receipt: EvidenceReceipt): EvidenceReceipt => ({
		...receipt,
		outcomes: Object.fromEntries(
			Object.entries(receipt.outcomes).map(([id, result]) => [
				qualify(name, id),
				rebaseOutcome(result, href),
			]),
		),
	});
	return {
		dependsOn: ids(entry.dependsOn),
		deliverable: {
			...work,
			id: qualify(name, work.id),
			destination: { ...work.destination, page: href(work.destination.page) },
			scope: {
				...work.scope,
				rules: ids(work.scope.rules),
				designItems: ids(work.scope.designItems),
				verificationChecks: ids(work.scope.verificationChecks),
			},
			...(work.implementation && {
				implementation: {
					...work.implementation,
					evidence: work.implementation.evidence.map(href),
				},
			}),
			...(work.verification && { verification: verification(work.verification) }),
			verificationHistory: work.verificationHistory.map(verification),
		},
	};
}

function scopedDeliverables(
	deliverables: Readonly<Record<string, DeliverableView>>,
	name: string,
	href: (href: string) => string,
): DeliveryProjection {
	const output: Record<string, DeliverableView> = {};
	const destinations: DeliveryProjection['destinations'][number][] = [];
	const destination = <TLink extends { page: string }>(link: TLink): TLink => ({
		...link,
		page: href(link.page),
	});
	for (const [id, entry] of Object.entries(deliverables)) {
		if (!id.startsWith(`${name}/`)) continue;
		const { links, implementation, verification, progress } = entry;
		const plan = destination(links.plan);
		destinations.push(plan);
		output[id] = {
			...entry,
			links: {
				plan,
				rules: links.rules.map(destination),
				designItems: links.designItems.map(destination),
				governingItems: links.governingItems.map(destination),
				verificationChecks: links.verificationChecks.map(destination),
			},
			implementation: { ...implementation, evidence: implementation.evidence.map(href) },
			verification: {
				...verification,
				checks: verification.checks.map((check): typeof check => {
					if (check.state === 'passed')
						return {
							...check,
							outcome: rebaseOutcome(check.outcome, href),
						};
					if (check.state === 'failed')
						return {
							...check,
							outcome: rebaseOutcome(check.outcome, href),
						};
					if (check.state === 'stale')
						return { ...check, outcome: rebaseOutcome(check.outcome, href) };
					return check;
				}),
				...(verification.history && {
					history: verification.history.map((receipt) =>
						Object.assign({}, receipt, {
							outcomes: Object.fromEntries(
								Object.entries(receipt.outcomes).map(([key, result]) => [
									key,
									rebaseOutcome(result, href),
								]),
							),
						}),
					),
				}),
			},
			progress:
				progress.state === 'accepted'
					? {
							...progress,
							acceptance: { ...progress.acceptance, evidence: href(progress.acceptance.evidence) },
						}
					: progress,
		};
	}
	return { deliverables: output, destinations, evidence: [] };
}

interface ProjectCheckOptions {
	readonly siteRoot?: string | undefined;
	readonly write?: boolean | undefined;
}

export async function checkProject(
	directory: string,
	options: ProjectCheckOptions = {},
): Promise<string> {
	const root = await realpath(resolve(directory));
	const projectFile = join(root, 'project.ts');
	const check = await openDesignCheck(options.siteRoot ?? root);
	const readAuthoringInput = (file: string): Promise<string> => check.read(file, root);
	const manifest = readProjectManifest(projectFile, await readAuthoringInput(projectFile));
	const identities =
		manifest.identities &&
		(await readProjectIdentities(manifest.identities, root, readAuthoringInput));
	const sets = await Promise.all(
		Object.entries(manifest.designs).map(async ([name, file]) => {
			await readAuthoringInput(file);
			const set = await check.prepare(dirname(file), { root, namespace: name });
			if (set.lead !== undefined && (!identities || !Object.hasOwn(identities.agents, set.lead)))
				throw new Error(
					`${name}: unknown Lead alias ${set.lead}. Register it in agents-identity.ts.`,
				);
			for (const {
				deliverable: { id, builder },
			} of set.entries) {
				if (builder && (!identities || !Object.hasOwn(identities.agents, builder)))
					throw new Error(
						`${name}/${id}: unknown Builder alias ${builder}. Register it in agents-identity.ts.`,
					);
			}
			return Object.assign({ name }, set);
		}),
	);
	if (new Set(sets.map((set) => set.directory)).size !== sets.length)
		throw new Error('Register each design directory only once.');

	const design: DesignCore = { pages: [], mapGroups: [], records: [], acceptances: [] };
	const revisions = new Map<string, string>();
	async function loadShared(
		file: string | undefined,
		kind: 'choice' | 'assumption',
	): Promise<{ records: DesignRecord[]; revisions: Map<string, string> } | undefined> {
		if (file === undefined) return undefined;
		const text = await readAuthoringInput(file);
		const records = readDesignItems(file, text, kind);
		const declarations = await Promise.all(
			projectRecordTypeImports(file, text, root).map(async (path): Promise<[string, string]> => [
				path,
				await readAuthoringInput(path),
			]),
		);
		checkAuthoringTypes([file], new Map([[file, text], ...declarations]));
		const scope = await check.inspectRecords(records, file, { namespace: 'project' });
		return {
			records: records.map((record) =>
				projectRecord(record, 'project', (href) => rebase(href, dirname(file), root)),
			),
			revisions: scope,
		};
	}
	const shared = await Promise.all([
		loadShared(manifest.decisions, 'choice'),
		loadShared(manifest.assumptions, 'assumption'),
	]);
	for (const source of shared) {
		if (!source) continue;
		design.records.push(...source.records);
		for (const [id, revision] of source.revisions) revisions.set(qualify('project', id), revision);
	}
	const entries: DeliveryEntry[] = [];
	for (const set of sets) {
		const href = (path: string): string => rebase(path, set.directory, root);
		design.records.push(
			...set.design.records.map((record) => projectRecord(record, set.name, href)),
		);
		for (const [id, revision] of set.recordRevisions)
			revisions.set(qualify(set.name, id), revision);
		for (const acceptance of set.design.acceptances ?? [])
			design.acceptances?.push({
				...acceptance,
				deliverableId: qualify(set.name, acceptance.deliverableId),
				evidence: href(acceptance.evidence),
			});
		entries.push(...set.entries.map((entry) => projectEntry(entry, set.name, href)));
	}
	checkRecordRelationships(design.records);
	const delivery = projectDelivery(design, entries, revisions, true);
	await Promise.all(delivery.evidence.map((href) => check.checkEvidence(href, projectFile)));
	const output = {
		version: 1,
		...(identities ? { owner: identities.owner } : {}),
		designs: Object.fromEntries(
			sets.map((set) => [
				set.name,
				{
					path: rebase('design.ts', set.directory, root),
					firstPage: rebase(set.firstPage.href, set.directory, root),
					title: set.firstPage.title,
					...(set.lead ? { lead: set.lead } : {}),
				},
			]),
		),
		records: design.records,
		deliverables: delivery.deliverables,
	};
	const view = projectViewSchema.parse(output);
	const projectSource = {
		file: join(root, 'project.generated.json'),
		text: JSON.stringify(view),
		records: new Map(design.records.map((record) => [record.id, record])),
	};
	const plans = await Promise.all(
		sets.map((set) =>
			set.plan(
				scopedDeliverables(delivery.deliverables, set.name, (href) =>
					rebase(href, root, set.directory),
				),
				projectSource,
			),
		),
	);
	if (options.write) {
		await commitDesignOutputs(
			[root, ...sets.map((set) => set.directory)],
			[
				{
					verify: check.verify,
					plan: async () => [await planGeneratedJson(projectSource.file, view)],
				},
				...plans.map(({ outputs }) => ({ verify: async (): Promise<void> => {}, plan: outputs })),
			],
		);
	} else {
		await check.verify();
	}
	const questions = projectDecisions(view.records).filter(({ status }) => status === 'Open');
	return [
		`Checked ${sets.length} project design(s).`,
		options.write
			? 'Generated views refreshed.'
			: 'Read-only check. Add --write to refresh generated views.',
		...(questions.length
			? [
					'Waiting on owner',
					...questions.map(
						({ record: { id, title, authority } }): string =>
							`  ${id} · ${authority?.question ?? title}`,
					),
				]
			: []),
		...plans.map(({ result }) => result),
	].join('\n');
}
