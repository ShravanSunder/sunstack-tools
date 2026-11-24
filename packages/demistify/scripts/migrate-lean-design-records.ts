import { cp, lstat, mkdir, readFile, readdir, realpath, writeFile } from 'node:fs/promises';
import { basename, dirname, extname, join, relative, resolve, sep } from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { decisionActorField, recordActor } from '../src/core/decision-record.ts';
import {
	declaredDesignPagePath,
	resolveDesignLink,
} from '../src/node/design-authoring/design-location.ts';
import { readProjectManifest } from '../src/node/design-authoring/read-project.ts';
import type { DesignRecord } from '../src/node/design-authoring/schema.ts';
import { containsPath } from '../src/node/local-site.ts';
import {
	checkLeanRecords,
	emitDesign,
	emitSidecar,
	readLegacyDelivery,
	readLegacyDesign,
	readLegacySidecar,
	type LegacyDeliveryGraph,
	type LegacyDesign,
	type LegacyRecord,
	type SourceReader,
} from './lean-design-migration/legacy-records.ts';
import {
	planPage,
	recordEvidence,
	type LocatedRecord,
	type PageChange,
} from './lean-design-migration/pages.ts';
import { planRecordAuthorship } from './lean-design-migration/record-authorship.ts';

interface Sidecar {
	readonly file: string;
	readonly text: string;
	readonly records: readonly LegacyRecord[];
}
interface Milestone {
	readonly namespace: string;
	readonly file: string;
	readonly design: LegacyDesign;
	readonly deliveries: LegacyDeliveryGraph;
	readonly sidecars: readonly Sidecar[];
}
const migrationRecordInput = z.record(
	z.string(),
	z.strictObject({
		[decisionActorField]: recordActor,
		settles: z.string().trim().min(1).optional(),
		settledBy: z.string().trim().min(1).optional(),
	}),
);
type MigrationRecordInput = z.infer<typeof migrationRecordInput>;

interface MigrationInputs {
	readonly source: string;
	readonly project: string;
	readonly generatorOwned: readonly string[];
	readonly recordInput: MigrationRecordInput;
}
interface RecordConversion {
	readonly authored: LocatedRecord;
	readonly page?: PageChange | undefined;
	readonly journey: readonly string[];
	readonly extra?: MigrationRecordInput[string] | undefined;
}

const ignoreGenerated = ['*.generated.json', 'design-sdk.generated.ts', 'design-schema.d.ts'];

function generated(file: string): boolean {
	const name = basename(file);
	return (
		name.endsWith('.generated.json') ||
		name.endsWith('.generated.js') ||
		ignoreGenerated.includes(name)
	);
}

function missing(error: unknown): boolean {
	return error instanceof Error && 'code' in error && error.code === 'ENOENT';
}

async function migrationFileExists(root: string, path: string): Promise<boolean> {
	if (!containsPath(root, path)) throw new Error(`Path escapes the docs root: ${path}.`);
	let current = root;
	try {
		for (const child of relative(root, path).split(sep)) {
			current = join(current, child);
			// oxlint-disable-next-line no-await-in-loop -- reject a linked parent before looking up its child.
			const entry = await lstat(current);
			if (entry.isSymbolicLink()) throw new Error(`A migration input is a symlink: ${current}.`);
			if (current === path && !entry.isFile())
				throw new Error(`Expected a regular migration file: ${path}.`);
		}
		return true;
	} catch (error) {
		if (missing(error)) return false;
		throw error;
	}
}

function guardedReader(root: string): SourceReader {
	return async (path): Promise<string | undefined> =>
		(await migrationFileExists(root, path)) ? readFile(path, 'utf8') : undefined;
}

async function required(read: SourceReader, file: string): Promise<string> {
	const text = await read(file);
	if (text === undefined) throw new Error(`Missing migration input: ${file}.`);
	return text;
}

async function sidecar(
	read: SourceReader,
	path: string | undefined,
	kind: 'choice' | 'assumption',
): Promise<Sidecar[]> {
	if (!path) return [];
	const text = await read(path);
	return text === undefined
		? []
		: [{ file: path, text, records: readLegacySidecar(path, text, kind) }];
}

async function milestone(
	namespace: string,
	registered: string,
	read: SourceReader,
): Promise<Milestone> {
	const dir = dirname(registered);
	const file = join(dir, 'design.ts');
	if (registered !== file) await required(read, registered);
	const text = await required(read, file);
	const [deliveries, decisions, assumptions] = await Promise.all([
		readLegacyDelivery(file, text, read),
		sidecar(read, join(dir, 'design-decisions.ts'), 'choice'),
		sidecar(read, join(dir, 'design-assumptions.ts'), 'assumption'),
	]);
	const inventory = await readdir(join(dir, 'deliverables'), {
		recursive: true,
		withFileTypes: true,
	}).catch((error: unknown) => {
		if (missing(error)) return [];
		throw error;
	});
	for (const entry of inventory) {
		const module = join(entry.parentPath, entry.name);
		if (!entry.isDirectory() && entry.name.endsWith('.ts') && !deliveries.modules.has(module))
			throw new Error(`Unregistered deliverable module: ${module}.`);
	}
	return {
		namespace,
		file,
		design: readLegacyDesign(file, text),
		deliveries,
		sidecars: [...decisions, ...assumptions],
	};
}

function identifier(owner: string, id: string): string {
	return id.includes('/') ? id : `${owner}/${id}`;
}

interface RelationshipChanges {
	readonly id: string;
	readonly original: string;
	readonly next: string;
	readonly target: string;
}
function convertRelationships(
	authored: LocatedRecord,
	registry: ReadonlyMap<string, LocatedRecord>,
	changes: RelationshipChanges[],
): {
	serves?: string[];
	governedBy?: string[];
	reliesOn?: string[];
	realizes?: string[];
	checks?: string[];
} {
	const { namespace, record } = authored;
	if (!('relationships' in record)) return {};
	const output: ReturnType<typeof convertRelationships> = {};
	for (const edge of record.relationships) {
		const bound = registry.get(identifier(namespace, edge.targetId))?.record;
		const id = identifier(namespace, record.id);
		if (!bound) throw new Error(`${id}: unresolved relationship ${edge.targetId}.`);
		if (edge.relation === 'governed-by' || edge.relation === 'relies-on') {
			if (bound.kind !== 'choice' && bound.kind !== 'assumption')
				throw new Error(`${id}: ${edge.relation} points to ${bound.kind} ${edge.targetId}.`);
			const next = bound.kind === 'choice' ? 'governedBy' : 'reliesOn';
			if ((edge.relation === 'governed-by') !== (bound.kind === 'choice'))
				changes.push({ id, original: edge.relation, next, target: edge.targetId });
			(output[next] ??= []).push(edge.targetId);
		} else {
			const kind = edge.relation === 'serves' ? 'need' : 'rule';
			if (bound.kind !== kind)
				throw new Error(`${id}: ${edge.relation} must name a ${kind}: ${edge.targetId}.`);
			(output[edge.relation] ??= []).push(edge.targetId);
		}
	}
	return output;
}

function convertRecord(
	{ authored, page, journey, extra }: RecordConversion,
	registry: ReadonlyMap<string, LocatedRecord>,
	dependencyChanges: RelationshipChanges[],
): DesignRecord {
	const { record } = authored;
	const { destination: _destination, sources, ...data } = record;
	const retarget = (source: (typeof sources)[number]): (typeof sources)[number] => {
		const replacement = page?.sourceSplit?.retargeted.find(({ from }) => from === source.href);
		return replacement ? { ...source, href: replacement.to } : source;
	};
	if (data.kind === 'assumption') {
		if (!extra) throw new Error(`${data.id}: supply the assumption author in --record-input.`);
		const shared = {
			kind: 'assumption' as const,
			id: data.id,
			title: data.title,
			[decisionActorField]: extra.by,
			statement: data.statement,
			weight: 'design' as const,
			sources: sources.map(retarget),
		};
		if (data.status === 'open' && extra.settles)
			return { ...shared, status: data.status, settles: extra.settles };
		if (data.status !== 'open' && extra.settledBy && data.resolution)
			return {
				...shared,
				status: data.status,
				settledBy: extra.settledBy,
				resolution: retarget(data.resolution),
			};
		throw new Error(
			`${data.id}: supply the missing assumption settlement information in --record-input.`,
		);
	}
	if (data.kind === 'choice') {
		if (data.authority.state === 'owner-open')
			throw new Error(`${data.id}: supply at least two real options before conversion.`);

		const primary = sources[0];
		if (!primary) throw new Error(`${data.id}: supply a decision source before conversion.`);
		const { decision, decidedOn, sourceKind } = data.authority;
		const basis = { decidedOn, source: retarget(primary) };
		const authority =
			data.authority.state === 'owner-decided'
				? { state: 'owner-decided' as const, ...basis, sourceKind: data.authority.sourceKind }
				: {
						state: 'agent-decided' as const,
						decidedOn,
						[decisionActorField]: { alias: data.authority.lead, role: 'lead' as const },
						rationale: data.authority.rationale,
						weight: 'design' as const,
						...(sourceKind === 'contract'
							? { sourceKind, source: basis.source }
							: { sourceKind: 'author' as const }),
					};
		return {
			kind: 'ruling',
			id: data.id,
			title: data.title,
			statement: decision,
			authority,
			sources: sources.map(retarget),
			...(data.supersedes && { supersedes: data.supersedes }),
		};
	}
	if (data.kind === 'need') return data;
	const { id, title } = data;
	const links = convertRelationships(authored, registry, dependencyChanges);
	const dependencies = {
		...(links.governedBy && { governedBy: links.governedBy }),
		...(links.reliesOn && { reliesOn: links.reliesOn }),
	};
	switch (data.kind) {
		case 'rule':
			return {
				id,
				title,
				kind: 'rule',
				statement: data.statement,
				...dependencies,
				...(links.serves && { serves: links.serves }),
			};
		case 'realization':
			return {
				id,
				title,
				kind: 'realization',
				statement: data.explanation,
				...dependencies,
				realizes: links.realizes ?? [],
			};
		case 'planned-check':
			return {
				id,
				title,
				kind: 'planned-check',
				statement: page?.statement ?? title,
				journey: journey.length === 1 ? (journey[0] ?? title) : title,
				unverified: 'Not recorded before conversion.',
				cases: data.cases,
				...dependencies,
				checks: links.checks ?? [],
			};
		default:
			return data satisfies never;
	}
}

function evidenceReferences(sets: readonly Milestone[]): Map<string, Set<string>> {
	const references = new Map<string, Set<string>>();
	for (const set of sets) {
		for (const { deliverable } of set.deliveries.entries) {
			if (deliverable.kind !== 'core')
				throw new Error(`${deliverable.id}: unexpected legacy later work.`);
			const receipts = [deliverable.verification, ...deliverable.verificationHistory];
			for (const receipt of receipts) {
				for (const [reference, outcome] of Object.entries(receipt?.outcomes ?? {})) {
					if (!('evidence' in outcome)) continue;
					const id = identifier(set.namespace, reference);
					const known = references.get(id) ?? new Set<string>();
					known.add(outcome.evidence);
					references.set(id, known);
				}
			}
		}
	}
	return references;
}

function retargetNavigation(
	set: Milestone,
	registry: ReadonlyMap<string, LocatedRecord>,
	placements: ReadonlyMap<string, PageChange>,
	site: string,
): void {
	const pathsById = new Map(
		set.design.pages.map(({ id, href }) => [id, declaredDesignPagePath(href, set.file, site)]),
	);
	const idsByAnchor = new Map<string, string>();
	for (const [id, { file, record }] of registry) {
		const destination = declaredDesignPagePath(record.destination.page, file, site);
		if (record.destination.fragment === record.id && placements.has(id))
			idsByAnchor.set(`${destination}#${record.id}`, id);
	}
	function targetFor<TTarget extends NonNullable<LegacyDesign['mapGroups'][number]['target']>>(
		target: TTarget,
	): TTarget | { record: string } {
		if ('record' in target || !target.fragment) return target;
		const file = pathsById.get(target.page);
		const id = file && idsByAnchor.get(`${file}#${target.fragment}`);
		return id
			? { record: id.startsWith(`${set.namespace}/`) ? id.slice(set.namespace.length + 1) : id }
			: target;
	}
	function updateMap(nodes: LegacyDesign['mapGroups']): void {
		for (const node of nodes) {
			if (node.target) node.target = targetFor(node.target);
			updateMap(node.children);
		}
	}
	updateMap(set.design.mapGroups);
	for (const route of set.design.walkthroughs ?? [])
		for (const step of route.steps) step.target = targetFor(step.target);
}

function wordCount(source: string): number {
	return source.match(/\S+/gu)?.length ?? 0;
}

async function counts(
	root: string,
	read: SourceReader,
	edits: ReadonlyMap<string, string>,
): Promise<{ before: number; after: number; files: number }> {
	const total = { before: 0, after: 0, files: 0 };
	const entries = await readdir(root, { withFileTypes: true });
	for (const entry of entries) {
		const file = join(root, entry.name);
		if (generated(file) || entry.name.startsWith('.')) continue;
		if (entry.isDirectory()) {
			// oxlint-disable-next-line no-await-in-loop -- walk the authored word boundary one directory at a time.
			const nested = await counts(file, read, edits);
			total.before += nested.before;
			total.after += nested.after;
			total.files += nested.files;
		} else if (entry.isFile() && ['.ts', '.html', '.md'].includes(extname(file))) {
			// oxlint-disable-next-line no-await-in-loop -- count the old and planned bytes for the same path.
			const original = await required(read, file);
			total.before += wordCount(original);
			total.after += wordCount(edits.get(file) ?? original);
			total.files++;
		}
	}
	return total;
}

async function planMigration({
	source,
	project,
	generatorOwned,
	recordInput,
}: MigrationInputs): Promise<{
	readonly edits: ReadonlyMap<string, string>;
	readonly report: Record<string, unknown>;
}> {
	const read = guardedReader(source);
	const manifestFile = join(project, 'project.ts');
	const manifest = readProjectManifest(manifestFile, await required(read, manifestFile));
	const generatorMilestones = [...new Set(generatorOwned)].map((name) => {
		const registered = Object.hasOwn(manifest.designs, name) ? manifest.designs[name] : undefined;
		if (!registered) throw new Error(`Unknown generator-owned milestone namespace: ${name}.`);
		return { name, directory: relative(source, dirname(registered)) };
	});
	const sets = await Promise.all(
		Object.entries(manifest.designs).map(([namespace, file]) => milestone(namespace, file, read)),
	);
	if (new Set(sets.map(({ file }) => file)).size !== sets.length)
		throw new Error('Register each design directory only once.');
	const shared = (
		await Promise.all([
			sidecar(read, manifest.decisions, 'choice'),
			sidecar(read, manifest.assumptions, 'assumption'),
		])
	).flat();
	const locations = [
		...sets.flatMap(({ namespace, file, design, sidecars }) => [
			...design.records.map((record) => ({ namespace, file, record })),
			...sidecars.flatMap((entry) =>
				entry.records.map((record) => ({ namespace, file: entry.file, record })),
			),
		]),
		...shared.flatMap((entry) =>
			entry.records.map((record) => ({ namespace: 'project', file: entry.file, record })),
		),
	];
	const registry = new Map(
		locations.map((value) => [identifier(value.namespace, value.record.id), value]),
	);
	if (registry.size !== locations.length)
		throw new Error('The old design records contain a duplicate ID.');
	const pageOwners = new Map<string, string>();
	const declaredPages = new Set([join(project, 'index.html')]);
	const preservedAnchors = new Map<string, Map<string, Set<string>>>();
	function retainAnchor(file: string, fragment: string, owner: string): void {
		const anchors = preservedAnchors.get(file) ?? new Map<string, Set<string>>();
		const owners = anchors.get(fragment) ?? new Set<string>();
		owners.add(owner);
		anchors.set(fragment, owners);
		preservedAnchors.set(file, anchors);
	}
	for (const set of sets) {
		for (const { href } of set.design.pages) {
			const file = declaredDesignPagePath(href, set.file, source);
			declaredPages.add(file);
			if (containsPath(dirname(set.file), file)) pageOwners.set(file, set.namespace);
		}
		for (const { deliverable } of set.deliveries.entries) {
			if (deliverable.kind !== 'core') continue;
			const { page, fragment } = deliverable.destination;
			const file = declaredDesignPagePath(page, set.file, source);
			retainAnchor(file, fragment, identifier(set.namespace, deliverable.id));
		}
	}
	const byPage = new Map<string, LocatedRecord[]>();
	for (const location of locations) {
		const { record, file, namespace } = location;
		for (const sourceReference of recordEvidence(record)) {
			if (!sourceReference.verbatim) continue;
			const target = resolveDesignLink(sourceReference.href, file, source);
			if (target?.fragment)
				retainAnchor(target.file, target.fragment, identifier(namespace, record.id));
		}
		const page = declaredDesignPagePath(location.record.destination.page, location.file, source);
		const group = byPage.get(page) ?? [];
		group.push(location);
		byPage.set(page, group);
	}
	const edits = new Map<string, string>();
	const pageChanges = new Map<string, PageChange & { readonly page: string }>();
	const changedPages = new Set<string>();
	let checkedIds = 0;
	for (const [file, records] of byPage) {
		if (!declaredPages.has(file))
			throw new Error(`Record placement page is not declared: ${file}.`);
		// oxlint-disable-next-line no-await-in-loop -- each authored page has one source-preserving edit plan.
		const html = await required(read, file);
		const plan = planPage({
			html,
			file,
			site: source,
			namespace: pageOwners.get(file),
			records,
			preservedAnchors: preservedAnchors.get(file) ?? new Map(),
		});
		for (const change of plan.changes)
			pageChanges.set(change.id, { ...change, page: relative(source, file) });
		checkedIds += plan.retainedIds;
		if (html !== plan.html) {
			edits.set(file, plan.html);
			changedPages.add(file);
		}
	}
	const references = evidenceReferences(sets);
	const dependencyChanges: RelationshipChanges[] = [];
	const defaults: { id: string; field: string; source: string; references?: string[] }[] = [];
	const blockedInputs: string[] = [];
	for (const [id, { record }] of registry) {
		if (record.kind === 'choice' && record.authority.state === 'owner-open')
			blockedInputs.push(`${id}: supply two real options before conversion.`);
		if (record.kind === 'assumption') {
			const extra = recordInput[id];
			if (!extra?.by) blockedInputs.push(`${id}: supply the assumption author.`);
			if (record.status === 'open' && !extra?.settles)
				blockedInputs.push(`${id}: supply what would settle the assumption.`);
			if (record.status !== 'open' && !extra?.settledBy)
				blockedInputs.push(`${id}: supply who settled the assumption.`);
		}
	}
	if (blockedInputs.length) throw new Error(blockedInputs.join('\n'));
	const converted = new Map<string, DesignRecord>();
	for (const [id, authored] of registry) {
		const page = pageChanges.get(id);
		const journey = [...(references.get(id) ?? [])].toSorted();
		const record = convertRecord(
			{ authored, page, journey, extra: recordInput[id] },
			registry,
			dependencyChanges,
		);
		converted.set(id, record);
		if (record.kind !== 'planned-check') continue;
		if (!page?.statement) defaults.push({ id, field: 'statement', source: 'old title' });
		if (journey.length !== 1)
			defaults.push({ id, field: 'journey', source: 'old title', references: journey });
		defaults.push({ id, field: 'unverified', source: record.unverified });
	}
	checkLeanRecords([...converted.values()]);
	function select(namespace: string, records: readonly LegacyRecord[]): DesignRecord[] {
		return records.map(({ id }): DesignRecord => {
			const record = converted.get(identifier(namespace, id));
			if (!record) throw new Error(`Missing converted record ${namespace}/${id}.`);
			return record;
		});
	}
	for (const set of sets) {
		retargetNavigation(set, registry, pageChanges, source);
		edits.set(set.file, emitDesign(set.design, select(set.namespace, set.design.records)));
		for (const entry of set.sidecars)
			edits.set(entry.file, emitSidecar(select(set.namespace, entry.records), entry.text, false));
		for (const [file, text] of set.deliveries.modules) edits.set(file, text);
	}
	for (const entry of shared)
		edits.set(entry.file, emitSidecar(select('project', entry.records), entry.text, true));
	const ignoreFile = join(project, '.gitignore');
	const previousIgnore = (await read(ignoreFile)) ?? '';
	const retainedPatterns = new Set(previousIgnore.split(/\r?\n/u));
	const patterns = ignoreGenerated.filter((pattern) => !retainedPatterns.has(pattern));
	if (patterns.length)
		edits.set(
			ignoreFile,
			[previousIgnore.trimEnd(), ...patterns].filter(Boolean).join('\n') + '\n',
		);
	const statements = locations.flatMap(({ namespace, file, record }) => {
		if (record.kind !== 'rule') return [];
		const id = identifier(namespace, record.id);
		const migrated = converted.get(id);
		if (migrated?.kind !== 'rule' || migrated.statement !== record.statement)
			throw new Error(`${id}: rule statement changed during conversion.`);
		const passage = pageChanges.get(id);
		return [
			{
				id,
				source: relative(source, file),
				page: passage?.page,
				oldAnchor: passage?.oldAnchor,
				statementWords: wordCount(record.statement),
				paraphraseWords: passage?.paraphraseWords ?? null,
			},
		];
	});
	const milestones = await Promise.all(
		sets.map(async ({ namespace, file }) => {
			const directory = dirname(file);
			const pages = [...pageChanges.values()].filter((change): boolean =>
				change.id.startsWith(`${namespace}/`),
			);
			return {
				namespace,
				directory: relative(source, directory),
				records: pages.length,
				placements: pages.length,
				pagesChanged: [...changedPages].filter((path): boolean => containsPath(directory, path))
					.length,
				words: await counts(directory, read, edits),
			};
		}),
	);
	return {
		edits,
		report: {
			milestones,
			totalWords: milestones.reduce(
				(total, { words }) => ({
					before: total.before + words.before,
					after: total.after + words.after,
					files: total.files + words.files,
				}),
				{ before: 0, after: 0, files: 0 },
			),
			sharedRecords: shared.reduce((total, entry) => total + entry.records.length, 0),
			declaredPages: declaredPages.size,
			retainedIdsOnEditedPages: checkedIds,
			deliverables: sets.reduce((total, set) => total + set.deliveries.modules.size, 0),
			unchangedRuleStatements: statements.length,
			statements,
			defaults,
			dependencyChanges,
			passages: [...pageChanges.values()],
			...(generatorMilestones.length > 0 && {
				generatorFollowUp: {
					milestones: generatorMilestones,
					requiredBeforeRegeneration:
						'Update the listed generators for lean records before regenerating.',
				},
			}),
			wordBoundary:
				'Whitespace-delimited words in milestone .ts, .html and .md files, with generated JSON, SDK, and declarations excluded.',
		},
	};
}

async function newOutput(raw: string, source: string): Promise<string> {
	const path = resolve(raw);
	const canonical = join(await realpath(dirname(path)), basename(path));
	if (containsPath(source, canonical) || containsPath(canonical, source))
		throw new Error('The output and source trees must be separate.');
	const exists = await lstat(canonical).then(
		(): boolean => true,
		(error: unknown): boolean => {
			if (missing(error)) return false;
			throw error;
		},
	);
	if (exists) throw new Error(`Output already exists: ${canonical}.`);
	return canonical;
}

async function main(): Promise<void> {
	const { values } = parseArgs({
		options: {
			source: { type: 'string' },
			project: { type: 'string' },
			output: { type: 'string' },
			'dry-run': { type: 'boolean' },
			'generator-owned': { type: 'string', multiple: true },
			'record-input': { type: 'string' },
			'record-authorship': { type: 'boolean' },
			help: { type: 'boolean', short: 'h' },
		},
	});
	if (values.help) {
		process.stdout.write(
			'Usage: node migrate-lean-design-records.ts --source DOCS --project RELATIVE_PROJECT [--output NEW_DOCS | --dry-run] [--generator-owned REGISTERED_NAMESPACE]... [--record-input INPUT.json] [--record-authorship]\n',
		);
		return;
	}
	if (!values.source || !values.project || (!values.output && !values['dry-run']))
		throw new Error('Provide --source, --project, and either --output or --dry-run. See --help.');
	const source = await realpath(resolve(values.source));
	const project = resolve(source, values.project);
	if (!containsPath(source, project))
		throw new Error('The project must be within the source docs tree.');
	const output = values.output && (await newOutput(values.output, source));
	const recordInput = values['record-input']
		? migrationRecordInput.parse(JSON.parse(await readFile(values['record-input'], 'utf8')))
		: {};
	const { edits, report } = values['record-authorship']
		? await planRecordAuthorship({
				source,
				project,
				read: guardedReader(source),
				actors: new Map(Object.entries(recordInput).map(([id, record]) => [id, record.by])),
			})
		: await planMigration({
				source,
				project,
				generatorOwned: values['generator-owned'] ?? [],
				recordInput,
			});
	if (!values['dry-run'] && output) {
		await mkdir(output);
		await Promise.all(
			(await readdir(source)).map((child) =>
				cp(join(source, child), join(output, child), {
					recursive: true,
					force: false,
					errorOnExist: true,
					verbatimSymlinks: true,
					filter: (file): boolean =>
						basename(file) !== '.git' && (!containsPath(project, file) || !generated(file)),
				}),
			),
		);
		const outputEdits = [...edits].map(
			([file, text]) => [join(output, relative(source, file)), text] as const,
		);
		await Promise.all(outputEdits.map(([file]) => migrationFileExists(output, file)));
		for (const [file, text] of outputEdits) {
			// oxlint-disable-next-line no-await-in-loop -- write only the planned paths in the newly copied tree.
			await writeFile(file, text);
		}
	}
	process.stdout.write(
		`${JSON.stringify({ dryRun: !!values['dry-run'], changedFiles: edits.size, ...report }, null, 2)}\n`,
	);
}

await main().catch((error: unknown): void => {
	process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
	process.exitCode = 1;
});
