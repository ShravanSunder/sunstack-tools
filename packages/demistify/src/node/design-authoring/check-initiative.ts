import { realpath, stat } from 'node:fs/promises';
import { dirname, join, relative, resolve, sep } from 'node:path';

import type { DeliverableView } from '../../core/deliverables-view.ts';
import { projectDecisions, initiativeViewSchema } from '../../core/initiative-view.ts';
import {
	checkRecordRelationships,
	generatedViewNotes,
	type LoadedDesign,
	openDesignCheck,
	writeDesignBuild,
} from './check-design.ts';
import {
	type CoreDeliverable,
	type DeliveryEntry,
	type DeliveryProjection,
	type DeliveryViews,
	designCheckNotes,
	projectDelivery,
} from './delivery-model.ts';
import { compileDesignNavigation } from './design-navigation.ts';
import { designTypeSource } from './design-type-source.ts';
import { projectIgnoreDirectory } from './generated-ignore.ts';
import { initiativeWork, projectProgress } from './initiative-order.ts';
import { checkAuthoringTypes, readDesignItems } from './read-design.ts';
import {
	projectRecordTypeImports,
	readInitiativeIdentities,
	readInitiativeManifest,
} from './read-initiative.ts';
import {
	attachDecisionDeliverables,
	projectedDestinations,
	qualifyRecord,
} from './record-projection.ts';
import { recordLabel, recordRelationships, type DesignCore, type DesignRecord } from './schema.ts';

type EvidenceReceipt = NonNullable<CoreDeliverable['verification']>;
type EvidenceOutcome = EvidenceReceipt['outcomes'][string];

function qualify(name: string, id: string): string {
	return qualifyRecord(name, id);
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

function projectRecord(record: DesignRecord, name: string): DesignRecord {
	const identifier = { id: qualify(name, record.id) };
	const references = (value: string | string[]): string[] =>
		(typeof value === 'string' ? [value] : value).map((id) => qualify(name, id));
	const dependencies = (value: {
		governedBy?: string[] | undefined;
		reliesOn?: string[] | undefined;
	}): { governedBy?: string[]; reliesOn?: string[] } => ({
		...(value.governedBy && { governedBy: references(value.governedBy) }),
		...(value.reliesOn && { reliesOn: references(value.reliesOn) }),
	});
	switch (record.kind) {
		case 'rule':
			return {
				...record,
				...identifier,
				...dependencies(record),
				...(record.serves && { serves: references(record.serves) }),
			};
		case 'realization':
			return {
				...record,
				...identifier,
				...dependencies(record),
				realizes: references(record.realizes),
			};
		case 'planned-check':
			return {
				...record,
				...identifier,
				...dependencies(record),
				checks: references(record.checks),
			};
		case 'choice':
		case 'ruling':
			return {
				...record,
				...identifier,
				...(record.supersedes && { supersedes: qualify(name, record.supersedes) }),
			};
		case 'assumption':
			return { ...record, ...identifier };
		case 'need':
			return { ...record, ...identifier, ...dependencies(record) };
		default:
			return record satisfies never;
	}
}

function projectEntry(
	entry: DeliveryEntry,
	name: string,
	href: (href: string) => string,
): DeliveryEntry {
	const { deliverable: work } = entry;
	if (work.kind === 'later')
		return { ...entry, deliverable: { ...work, id: qualify(name, work.id) } };
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
				needs: ids(work.scope.needs),
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
): DeliveryViews {
	const output: Record<string, DeliverableView> = {};
	const destinations: DeliveryViews['destinations'][number][] = [];
	const destination = <TLink extends { page: string }>(link: TLink): TLink => ({
		...link,
		page: href(link.page),
	});
	for (const [id, entry] of Object.entries(deliverables)) {
		if (!id.startsWith(`${name}/`)) continue;
		if (entry.kind === 'later') {
			output[id] = entry;
			continue;
		}
		const { links, implementation, verification, progress } = entry;
		const plan = destination(links.plan);
		const recordLink = (link: (typeof links.rules)[number]): typeof link =>
			link.destination ? { ...link, destination: destination(link.destination) } : link;
		destinations.push(plan);
		output[id] = {
			...entry,
			links: {
				plan,
				rules: links.rules.map(recordLink),
				designItems: links.designItems.map(recordLink),
				governingItems: links.governingItems.map(recordLink),
				verificationChecks: links.verificationChecks.map(recordLink),
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

interface InitiativeCheckOptions {
	readonly siteRoot?: string | undefined;
	readonly write?: boolean | undefined;
}

export interface CheckedInitiativeBuild {
	readonly projects: ReadonlyMap<string, string>;
	readonly result: string;
	readonly generated: ReadonlyMap<string, unknown>;
	readonly preparedPages: ReadonlyMap<string, string>;
	readonly design: DesignCore;
	readonly entries: readonly DeliveryEntry[];
	readonly delivery: DeliveryProjection;
}

interface SharedRecordSource {
	readonly file: string;
	readonly records: readonly DesignRecord[];
}

interface LoadedInitiative {
	readonly root: string;
	readonly initiativeFile: string;
	readonly check: Awaited<ReturnType<typeof openDesignCheck>>;
	readonly owner: string | undefined;
	readonly sets: readonly (LoadedDesign & { readonly name: string })[];
	readonly shared: readonly SharedRecordSource[];
	readonly design: DesignCore;
	readonly entries: readonly DeliveryEntry[];
	/** Project directory to project name. */
	readonly projects: ReadonlyMap<string, string>;
	readonly manifest: ReturnType<typeof readInitiativeManifest>;
}

export async function loadInitiative(
	directory: string,
	siteRoot?: string,
): Promise<LoadedInitiative> {
	const root = await realpath(resolve(directory));
	const initiativeFile = join(root, 'initiative.ts');
	const check = await openDesignCheck(siteRoot ?? root);
	const readAuthoringInput = (file: string): Promise<string> => check.read(file, root);
	const manifest = readInitiativeManifest(initiativeFile, await readAuthoringInput(initiativeFile));
	const identities =
		manifest.identities &&
		(await readInitiativeIdentities(manifest.identities, root, readAuthoringInput));
	const sets = await Promise.all(
		Object.entries(manifest.projects).map(async ([name, { design: file }]) => {
			await readAuthoringInput(file);
			const set = await check.load(dirname(file), root);
			if (set.lead !== undefined && (!identities || !Object.hasOwn(identities.agents, set.lead)))
				throw new Error(
					`${name}: unknown Lead alias ${set.lead}. Register it in agents-identity.ts.`,
				);
			for (const { deliverable } of set.entries) {
				if (
					deliverable.kind === 'core' &&
					deliverable.builder &&
					(!identities || !Object.hasOwn(identities.agents, deliverable.builder))
				)
					throw new Error(
						`${name}/${deliverable.id}: unknown Builder alias ${deliverable.builder}. Register it in agents-identity.ts.`,
					);
			}
			return Object.assign({ name }, set);
		}),
	);
	if (new Set(sets.map((set) => set.directory)).size !== sets.length)
		throw new Error('Register each design directory only once.');

	const design: DesignCore = { pages: [], mapGroups: [], records: [], acceptances: [] };
	async function loadShared(
		file: string | undefined,
		kind: 'choice' | 'assumption',
	): Promise<SharedRecordSource[]> {
		if (file === undefined) return [];
		const text = await readAuthoringInput(file);
		const records = readDesignItems(file, text, kind, root);
		const declarations = projectRecordTypeImports(file, text, root).map(
			(path): [string, string] => [path, designTypeSource],
		);
		checkAuthoringTypes([file], new Map([[file, text], ...declarations]));
		return [{ file, records }];
	}
	const shared = (
		await Promise.all([
			loadShared(manifest.decisions, 'choice'),
			loadShared(manifest.assumptions, 'assumption'),
		])
	).flat();
	for (const { records } of shared)
		design.records.push(...records.map((record) => projectRecord(record, 'initiative')));
	const entries: DeliveryEntry[] = [];
	for (const set of sets) {
		const href = (path: string): string => rebase(path, set.directory, root);
		design.records.push(...set.design.records.map((record) => projectRecord(record, set.name)));
		for (const acceptance of set.design.acceptances ?? [])
			design.acceptances?.push({
				...acceptance,
				deliverableId: qualify(set.name, acceptance.deliverableId),
				evidence: href(acceptance.evidence),
			});
		entries.push(...set.entries.map((entry) => projectEntry(entry, set.name, href)));
	}
	const qualifiedIds = design.records.map(({ id }) => id);
	const records = checkRecordRelationships(design.records);
	for (const set of sets) {
		set.checkTypes([...qualifiedIds, ...set.design.records.map(({ id }) => id)]);
		compileDesignNavigation(set.design, { source: set.designFile, records, namespace: set.name });
	}
	return {
		root,
		initiativeFile,
		check,
		owner: identities ? identities.owner : undefined,
		sets,
		shared,
		design,
		entries,
		projects: new Map(sets.map((set) => [set.directory, set.name])),
		manifest,
	};
}

function projectOf(qualifiedId: string): string {
	const slash = qualifiedId.indexOf('/');
	return slash === -1 ? qualifiedId : qualifiedId.slice(0, slash);
}

/** Projects by their number, then in the initiative's authored order. */
function projectsByNumber(
	projects: Readonly<Record<string, { readonly number: number }>>,
): string[] {
	return Object.entries(projects)
		.map(([name, { number }], position) => ({ name, number, position }))
		.toSorted((left, right) => left.number - right.number || left.position - right.position)
		.map(({ name }) => name);
}

/**
 * A deliverable that depends on one in a later-numbered project contradicts the
 * initiative's order; the cut or the number is wrong. A warning, not a refusal.
 */
function orderWarnings(
	entries: readonly DeliveryEntry[],
	projects: Readonly<Record<string, { readonly number: number }>>,
): string[] {
	const warnings: string[] = [];
	for (const { deliverable, dependsOn } of entries) {
		const own = projects[projectOf(deliverable.id)]?.number;
		for (const prerequisite of dependsOn) {
			const other = projects[projectOf(prerequisite)]?.number;
			if (own !== undefined && other !== undefined && other > own)
				warnings.push(
					`${deliverable.id} (project number ${own}) depends on ${prerequisite} (project number ${other}); the cut or the number is wrong.`,
				);
		}
	}
	return warnings;
}

export async function buildInitiative(
	directory: string,
	options: InitiativeCheckOptions = {},
): Promise<CheckedInitiativeBuild> {
	const { root, initiativeFile, check, owner, sets, shared, design, entries, projects, manifest } =
		await loadInitiative(directory, options.siteRoot);
	const projectOutput = join(root, 'initiative.generated.json');
	const [preparedSets] = await Promise.all([
		Promise.all(sets.map(async (set) => Object.assign({}, set, await check.prepare(set)))),
		Promise.all(shared.map(({ file, records }) => check.checkRecordSources(records, file))),
	]);

	const pageNamespaces = new Map<string, string | undefined>();
	for (const set of preparedSets) {
		for (const { file, owned } of set.pages) {
			if (owned) {
				if (pageNamespaces.get(file)) throw new Error(`Multiple design sets own ${file}.`);
				pageNamespaces.set(file, set.name);
			} else if (!pageNamespaces.has(file)) pageNamespaces.set(file, undefined);
		}
	}
	const projectIndex = join(root, 'index.html');
	const hasProjectIndex = await stat(projectIndex).then(
		(file): boolean => file.isFile(),
		(error: unknown): boolean => {
			if (error instanceof Error && 'code' in error && error.code === 'ENOENT') return false;
			throw error;
		},
	);
	if (hasProjectIndex && !pageNamespaces.has(projectIndex))
		pageNamespaces.set(projectIndex, undefined);
	const outputDirectories = [root, ...sets.map((set) => set.directory)];
	const { projection, preparedPages, recordOutputs } = await check.projectPages(
		design.records,
		pageNamespaces,
		outputDirectories,
	);
	const destinations = projectedDestinations(projection, root);
	const delivery = projectDelivery(design, entries, destinations);
	attachDecisionDeliverables(recordOutputs, delivery, root, projection);
	await Promise.all(delivery.evidence.map((href) => check.checkEvidence(href, initiativeFile)));
	const order = projectsByNumber(manifest.projects);
	const progressByProject = new Map(
		order.map((name) => [
			name,
			projectProgress(
				Object.values(delivery.deliverables).filter(({ id }) => id.startsWith(`${name}/`)),
			),
		]),
	);
	const output = {
		version: 2,
		...(owner === undefined ? {} : { owner }),
		projects: Object.fromEntries(
			preparedSets.map((set) => {
				const progress = progressByProject.get(set.name);
				if (!progress) throw new Error(`${set.name}: project missing from the initiative order.`);
				return [
					set.name,
					{
						path: rebase('design.ts', set.directory, root),
						firstPage: rebase(set.firstPage.href, set.directory, root),
						title: set.firstPage.title,
						...(set.lead ? { lead: set.lead } : {}),
						number: manifest.projects[set.name]?.number ?? 1,
						...(manifest.projects[set.name]?.achieves
							? { achieves: manifest.projects[set.name]?.achieves }
							: {}),
						...(manifest.projects[set.name]?.acceptance
							? { acceptance: manifest.projects[set.name]?.acceptance }
							: {}),
						state: progress.state,
						reason: progress.reason,
					},
				];
			}),
		),
		order: [...order],
		work: initiativeWork(delivery.deliverables),
		records: design.records.map((record) => ({
			...record,
			title: recordLabel(record),
			destination: destinations.get(record.id),
			relationships: recordRelationships(record),
		})),
		deliverables: delivery.deliverables,
	};
	const view = initiativeViewSchema.parse(output);
	const projectSource = { file: projectOutput, text: JSON.stringify(view) };
	const plans = await Promise.all(
		preparedSets.map((set) =>
			set.plan(
				scopedDeliverables(delivery.deliverables, set.name, (href) =>
					rebase(href, root, set.directory),
				),
				{ projection, preparedPages, namespace: set.name, projectSource },
			),
		),
	);
	const generated = new Map<string, unknown>([
		[projectOutput, view],
		...plans.flatMap((plan) => [...plan.generated]),
	]);
	if (options.write)
		await writeDesignBuild(
			outputDirectories,
			check.verify,
			generated,
			recordOutputs,
			projectIgnoreDirectory(root),
		);
	else await check.verify();
	const questions = projectDecisions(view.records).filter(({ status }) => status === 'Open');
	const result = [
		`Checked ${sets.length} project(s) by number: ${order.map((name) => `${manifest.projects[name]?.number ?? '?'} ${name}`).join(', ')}.`,
		...orderWarnings(entries, manifest.projects).map((warning) => `Warning: ${warning}`),
		options.write ? 'Generated views refreshed.' : 'Read-only check.',
		...(options.write ? [] : await generatedViewNotes('initiative', generated, recordOutputs)),
		...(questions.length
			? [
					'Waiting on owner',
					...questions.map(
						({ record: { id, title, question, authority } }): string =>
							`  ${id} · ${question ?? authority?.question ?? title}`,
					),
				]
			: []),
		...plans.map((plan) => plan.result),
		...designCheckNotes(design.records, delivery),
	].join('\n');
	return {
		result,
		generated,
		preparedPages,
		design,
		entries,
		delivery,
		projects,
	};
}

export async function checkInitiative(
	directory: string,
	options: InitiativeCheckOptions = {},
): Promise<string> {
	return (await buildInitiative(directory, options)).result;
}
