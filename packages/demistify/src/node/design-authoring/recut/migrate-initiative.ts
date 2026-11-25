import { readdirSync, statSync } from 'node:fs';
import { cp, mkdtemp, readFile, realpath, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, dirname, join, relative, resolve, sep } from 'node:path';

import { containsPath } from '../../local-site.ts';
import { buildInitiative, loadInitiative } from '../check-initiative.ts';
import { requiredSectionHolds } from '../section-layout.ts';
import type { AddressMaps, AnchorTarget } from './address-rewrite.ts';
import { parseCutPlan, type CutPlan } from './cut-plan.ts';
import { initiativeSource, type InitiativeProjectEntry } from './initiative-source.ts';
import { convertLegacyLayout, isLegacyLayout, type LegacyConversion } from './legacy-layout.ts';
import { MemoryWorkspace } from './memory-workspace.ts';
import { scaffoldProjectSources } from './project-scaffold.ts';
import { projectShellPage, shellRuntime } from './project-shell.ts';
import { planPages, type PageMoveRequest, type PageSource } from './recut-pages.ts';
import { formatRecutReport } from './recut-report.ts';
import { projectOf, resolveRecut, type InitiativeSnapshot } from './recut-resolution.ts';
import { planSources } from './recut-sources.ts';
import { relativeHref, type PageDeclaration, type RecutProject } from './recut-workspace.ts';

export interface MigrateInitiativeOptions {
	readonly plan: string;
	readonly dryRun: boolean;
	readonly siteRoot?: string | undefined;
}

export interface MigrateInitiativeResult {
	readonly report: string;
	readonly applied: boolean;
	readonly checkPassed: boolean | undefined;
}

type LoadedInitiative = Awaited<ReturnType<typeof loadInitiative>>;
type MapNode = LoadedInitiative['design']['mapGroups'][number];

function mapNodeRecordTargets(node: MapNode): string[] {
	return [
		...(node.target && 'record' in node.target ? [node.target.record] : []),
		...node.children.flatMap(mapNodeRecordTargets),
	];
}

/** Record ids a design's Map nodes and Walkthrough steps target. */
function navigationRecordTargets(design: LoadedInitiative['design']): string[] {
	return [
		...design.mapGroups.flatMap(mapNodeRecordTargets),
		...(design.walkthroughs ?? []).flatMap((walkthrough) =>
			walkthrough.steps.flatMap((step) => ('record' in step.target ? [step.target.record] : [])),
		),
	];
}

function htmlFiles(directory: string): string[] {
	const files: string[] = [];
	for (const entry of readdirSync(directory, { withFileTypes: true })) {
		if (entry.name.startsWith('.') || entry.name === 'node_modules') continue;
		const path = join(directory, entry.name);
		if (entry.isDirectory()) files.push(...htmlFiles(path));
		else if (entry.isFile() && /\.html?$/iu.test(entry.name) && !entry.name.includes('.generated.'))
			files.push(path);
	}
	return files;
}

function exists(path: string): boolean {
	try {
		statSync(path);
		return true;
	} catch {
		return false;
	}
}

function commonParent(directories: readonly string[], fallback: string): string {
	const parents = [...new Set(directories.map((directory) => dirname(directory)))];
	return parents.length === 1 && parents[0] ? parents[0] : fallback;
}

function snapshotOf(loaded: LoadedInitiative): InitiativeSnapshot {
	return {
		projects: loaded.sets.map((set) => ({
			name: set.name,
			number: loaded.manifest.projects[set.name]?.number,
			recordIds: set.design.records.map(({ id }) => id),
			deliverableIds: set.entries.map(({ deliverable }) => deliverable.id),
		})),
		records: loaded.design.records,
		entries: loaded.entries,
	};
}

/** Projects as they will be: existing ones keep their folders, the plan's new ones get theirs. */
function recutProjects(loaded: LoadedInitiative, plan: CutPlan): Map<string, RecutProject> {
	const projects = new Map<string, RecutProject>(
		loaded.sets.map((set) => [
			set.name,
			{ name: set.name, directory: set.directory, designFile: set.designFile, created: false },
		]),
	);
	const parent = commonParent(
		loaded.sets.map((set) => set.directory),
		loaded.root,
	);
	for (const planned of plan.projects) {
		if (projects.has(planned.key)) continue;
		const directory = join(parent, planned.folder);
		if (exists(directory))
			throw new Error(
				`${planned.key}: ${relative(loaded.root, directory)} already exists and is not a registered project.`,
			);
		projects.set(planned.key, {
			name: planned.key,
			directory,
			designFile: join(directory, 'design.ts'),
			created: true,
		});
	}
	return projects;
}

function initiativeEntries(
	loaded: LoadedInitiative,
	plan: CutPlan,
	projects: ReadonlyMap<string, RecutProject>,
): InitiativeProjectEntry[] {
	const planned = plan.projects.map((project): InitiativeProjectEntry => {
		const existing = loaded.manifest.projects[project.key];
		const created = projects.get(project.key);
		return {
			key: project.key,
			design: existing?.design ?? created?.designFile ?? '',
			number: project.number,
			achieves: project.achieves ?? existing?.achieves,
			acceptance: project.acceptance ?? existing?.acceptance,
		};
	});
	const kept = Object.entries(loaded.manifest.projects)
		.filter(
			([key]) =>
				!plan.projects.some((project) => project.key === key) && !plan.remove.includes(key),
		)
		.map(([key, project]): InitiativeProjectEntry => ({
			key,
			design: project.design,
			number: project.number,
			achieves: project.achieves,
			acceptance: project.acceptance,
		}));
	return [...planned, ...kept];
}

function copyAuthoredTree(from: string, to: string): Promise<void> {
	return cp(from, to, {
		recursive: true,
		filter: (source) => !basename(source).includes('.generated.') || source === from,
	});
}

/** Stage the changes on a sibling copy and run the initiative check there. */
async function checkStaged(
	root: string,
	siteRoot: string | undefined,
	workspace: MemoryWorkspace,
): Promise<{ readonly passed: boolean; readonly result: string }> {
	const stage = join(dirname(root), `.${basename(root)}.recut-${process.pid}`);
	const unstaged = (text: string): string =>
		text.replaceAll(stage, root).replaceAll(basename(stage), basename(root));
	await rm(stage, { recursive: true, force: true });
	try {
		await copyAuthoredTree(root, stage);
		await workspace.applyTo(stage);
		const site = siteRoot && containsPath(siteRoot, stage) ? siteRoot : stage;
		const build = await buildInitiative(stage, { siteRoot: site });
		return { passed: true, result: unstaged(build.result) };
	} catch (error) {
		return {
			passed: false,
			result: unstaged(error instanceof Error ? error.message : String(error)),
		};
	} finally {
		await rm(stage, { recursive: true, force: true });
	}
}

export async function migrateInitiative(
	directory: string,
	options: MigrateInitiativeOptions,
): Promise<MigrateInitiativeResult> {
	const root = await realpath(resolve(directory));
	const siteRoot =
		options.siteRoot === undefined ? undefined : await realpath(resolve(options.siteRoot));
	const planFile = resolve(options.plan);
	const plan = parseCutPlan(planFile, await readFile(planFile, 'utf8'));
	if (!isLegacyLayout(root))
		return recut({ root, workRoot: root, siteRoot, plan, dryRun: options.dryRun });
	// A project.ts tree is converted on a private copy, so the reader sees initiative.ts.
	const scratch = await realpath(await mkdtemp(join(tmpdir(), 'demistify-recut-')));
	try {
		const workRoot = join(scratch, basename(root));
		await copyAuthoredTree(root, workRoot);
		return await recut({ root, workRoot, siteRoot, plan, dryRun: options.dryRun, legacy: true });
	} finally {
		await rm(scratch, { recursive: true, force: true });
	}
}

interface RecutRun {
	/** The initiative as it is: originals are read here and the result is written here. */
	readonly root: string;
	/** Where the run reads the tree from: the initiative, or a converted copy. */
	readonly workRoot: string;
	readonly siteRoot: string | undefined;
	readonly plan: CutPlan;
	readonly dryRun: boolean;
	readonly legacy?: true;
}

async function recut(run: RecutRun): Promise<MigrateInitiativeResult> {
	const { plan, siteRoot } = run;
	const root = run.workRoot;
	const workspace = new MemoryWorkspace(run.root, run.workRoot);
	let conversion: LegacyConversion | undefined;
	if (run.legacy) {
		conversion = convertLegacyLayout(root, plan, workspace);
		await workspace.applyTo(root);
	}
	let loaded: LoadedInitiative;
	try {
		loaded = await loadInitiative(root, run.legacy ? undefined : siteRoot);
	} catch (error) {
		throw new Error(
			`The initiative must check before a re-cut. ${error instanceof Error ? error.message : String(error)}`,
			{ cause: error },
		);
	}
	const resolution = resolveRecut(snapshotOf(loaded), plan);
	const maps: AddressMaps = {
		records: new Map([...resolution.recordMoves].map(([from, move]) => [from, move.to])),
		deliverables: new Map([...resolution.deliverableMoves].map(([from, move]) => [from, move.to])),
	};
	const projects = recutProjects(loaded, plan);

	// New projects: typed files and a page that declares the record sections.
	const runtime = shellRuntime(
		loaded.sets
			.map((set) => set.design.pages[0] && resolve(set.directory, set.design.pages[0].href))
			.flatMap((file) => (file ? [workspace.read(file) ?? ''] : [])),
	);
	const rootIndex = join(root, 'index.html');
	for (const planned of plan.projects) {
		const project = projects.get(planned.key);
		if (!project?.created) continue;
		scaffoldProjectSources(project, planned.title, planned.lead, workspace);
		workspace.write(
			join(project.directory, 'index.html'),
			projectShellPage({
				title: planned.title,
				achieves: planned.achieves,
				runtime,
				backLink: exists(rootIndex) ? relativeHref(project.directory, rootIndex) : undefined,
			}),
		);
	}

	// Pages: where each project's passages live, and what each page's bare references mean.
	const declared = new Map<string, Map<string, { title: string; icon: string; id: string }>>();
	for (const set of loaded.sets)
		declared.set(
			set.name,
			new Map(
				set.design.pages.map((page) => [
					resolve(dirname(set.designFile), decodeURIComponent(page.href)),
					{ title: page.title, icon: page.icon, id: page.id },
				]),
			),
		);
	const ownedPages = new Map<string, string>();
	const targetPages = new Map<string, string>();
	for (const project of projects.values()) {
		const pages = project.created
			? new Map([[join(project.directory, 'index.html'), { title: '', icon: 'file', id: 'home' }]])
			: (declared.get(project.name) ??
				new Map<string, { title: string; icon: string; id: string }>());
		for (const file of pages.keys())
			if (containsPath(project.directory, file) && !ownedPages.has(file))
				ownedPages.set(file, project.name);
		const first = [...pages.keys()].find((file) => containsPath(project.directory, file));
		if (first) targetPages.set(project.name, first);
	}
	const ownerOf = (file: string): string | undefined =>
		[...projects.values()]
			.filter((project) => containsPath(project.directory, file))
			.toSorted((left, right) => right.directory.length - left.directory.length)[0]?.name;
	const pageFiles = new Set([
		...htmlFiles(root),
		...[...workspace.changes.keys()].filter((file) => file.endsWith('.html')),
	]);
	const pageSources: PageSource[] = [...pageFiles].flatMap((file): PageSource[] => {
		const text = workspace.read(file);
		return text === undefined ? [] : [{ file, text, owner: ownerOf(file) }];
	});
	const records = new Map(loaded.design.records.map((record) => [record.id, record]));
	const entries = new Map(loaded.entries.map((entry) => [entry.deliverable.id, entry]));
	const destinationOf = (address: string): AnchorTarget | undefined => {
		const entry = entries.get(address);
		// Loaded entries carry destinations relative to the initiative root.
		if (!entry || entry.deliverable.kind !== 'core') return undefined;
		return {
			file: resolve(root, decodeURIComponent(entry.deliverable.destination.page)),
			fragment: entry.deliverable.destination.fragment,
		};
	};
	const withdrawn = new Set(plan.withdraw);
	const request: PageMoveRequest = {
		records: new Map(
			[...resolution.recordMoves].flatMap(([from, move]) => {
				const record = records.get(from);
				return record
					? [[from, { to: projectOf(move.to), holds: requiredSectionHolds(record) }]]
					: [];
			}),
		),
		deliverables: new Map(
			[...resolution.deliverableMoves].flatMap(([from, move]) => {
				const destination = destinationOf(from);
				return destination
					? [[from, { to: projectOf(move.to), newAddress: move.to, destination }]]
					: [];
			}),
		),
		stayingDestinations: loaded.entries.flatMap(({ deliverable }) => {
			if (resolution.deliverableMoves.has(deliverable.id) || withdrawn.has(deliverable.id))
				return [];
			const destination = destinationOf(deliverable.id);
			return destination ? [destination] : [];
		}),
		targetPages,
		ownedPages,
		label: (file) => relative(root, file).split(sep).join('/'),
	};
	const pagePlan = planPages(pageSources, request, maps);
	for (const [file, text] of pagePlan.files) workspace.write(file, text);

	// A project declares any page it must link to: a moved deliverable's destination, or
	// the new home of a record its Map or Walkthrough targets.
	const destinations = new Map<string, { page: string; fragment: string }>();
	const declaredPages = new Map<string, PageDeclaration[]>();
	const pageInfo = (file: string): { title: string; icon: string; id: string } | undefined => {
		for (const pages of declared.values()) {
			const info = pages.get(file);
			if (info) return info;
		}
		const created = plan.projects.find(
			(planned) => join(projects.get(planned.key)?.directory ?? '', 'index.html') === file,
		);
		return created ? { title: created.title, icon: 'file', id: created.key } : undefined;
	};
	const declarePage = (projectName: string, file: string, reason: string): void => {
		const project = projects.get(projectName);
		if (!project) return;
		const already = project.created
			? new Set([join(project.directory, 'index.html')])
			: new Set(declared.get(projectName)?.keys() ?? []);
		const pending = declaredPages.get(projectName) ?? [];
		const href = relativeHref(project.directory, file);
		if (already.has(file) || pending.some((page) => page.href === href)) return;
		const info = pageInfo(file);
		const usedIds = new Set([
			...[...(declared.get(projectName)?.values() ?? [])].map(({ id }) => id),
			...pending.map(({ id }) => id),
			...(project.created ? ['home'] : []),
		]);
		const base = info?.id ?? basename(file).replace(/\.html?$/iu, '');
		const owner = ownerOf(file) ?? 'page';
		let id = usedIds.has(base) ? `${owner}-${base}` : base;
		for (let counter = 2; usedIds.has(id); counter += 1) id = `${owner}-${base}-${counter}`;
		pending.push({
			id,
			title: info?.title ?? basename(file),
			href,
			icon: info?.icon ?? 'file',
			reason,
		});
		declaredPages.set(projectName, pending);
	};
	for (const [from, move] of resolution.deliverableMoves) {
		const target = projects.get(projectOf(move.to));
		const destination = pagePlan.destinations.get(from);
		if (!target || !destination) continue;
		destinations.set(from, {
			page: relativeHref(target.directory, destination.file),
			fragment: destination.fragment,
		});
		declarePage(target.name, destination.file, `${move.to} keeps its destination there`);
	}
	for (const set of loaded.sets) {
		for (const reference of navigationRecordTargets(set.design)) {
			const address = reference.includes('/') ? reference : `${set.name}/${reference}`;
			const file = pagePlan.recordPlacements.get(address);
			if (file)
				declarePage(
					set.name,
					file,
					`its Map or Walkthrough targets ${maps.records.get(address) ?? address} there`,
				);
		}
	}

	const sharedFiles = [loaded.manifest.decisions, loaded.manifest.assumptions].flatMap((file) =>
		file ? [file] : [],
	);
	planSources(
		{
			projects,
			records,
			entries: loaded.entries,
			resolution,
			maps,
			anchors: pagePlan.anchors,
			destinations,
			declaredPages,
			sharedFiles,
		},
		workspace,
	);
	workspace.write(
		loaded.initiativeFile,
		initiativeSource({
			root,
			projects: initiativeEntries(loaded, plan, projects),
			decisions: loaded.manifest.decisions,
			assumptions: loaded.manifest.assumptions,
			identities: loaded.manifest.identities,
		}),
	);

	// Prose is never rewritten: say where a renamed item's old id still appears.
	const staleProse = [...resolution.deliverableMoves.values(), ...resolution.recordMoves.values()]
		.filter((move) => move.oldId !== move.newId)
		.flatMap((move) => {
			const word = new RegExp(
				`(?<![\\w-])${move.oldId.replace(/[.*+?^${}()|[\]\\]/gu, '\\$&')}(?![\\w-])`,
				'u',
			);
			const files = pageSources
				.map(({ file }) => file)
				.filter((file) => {
					const owner = ownerOf(file);
					if (owner !== projectOf(move.to) && owner !== projectOf(move.from)) return false;
					return word.test((workspace.read(file) ?? '').replace(/<[^>]*>/gu, ' '));
				});
			return files.length
				? [{ from: move.from, to: move.to, files: files.map((file) => relative(root, file)) }]
				: [];
		});

	const check = run.dryRun ? undefined : await checkStaged(run.root, siteRoot, workspace);
	const applied = check?.passed === true;
	if (applied) await workspace.applyTo(run.root);
	const report = formatRecutReport({
		root: run.root,
		conversion: conversion && {
			...conversion,
			renamed: conversion.renamed.map(({ from, to }) => ({
				from: relative(root, from),
				to: relative(root, to),
			})),
		},
		plan,
		resolution,
		pagePlan,
		projects,
		declaredPages,
		staleProse,
		files: workspace.fileChanges(),
		dryRun: run.dryRun,
		check,
		applied,
		relativePath: (file) => relative(root, file).split(sep).join('/'),
	});
	return { report, applied, checkPassed: check?.passed };
}
