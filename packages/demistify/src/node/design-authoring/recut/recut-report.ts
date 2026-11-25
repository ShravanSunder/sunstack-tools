import type { CutPlan } from './cut-plan.ts';
import type { PagePlan } from './recut-pages.ts';
import type { RecutResolution } from './recut-resolution.ts';
import type { PageDeclaration, RecutProject } from './recut-workspace.ts';

export interface FileChange {
	readonly path: string;
	readonly kind: 'created' | 'changed' | 'removed';
}

interface RecutReportInput {
	readonly root: string;
	readonly plan: CutPlan;
	readonly resolution: RecutResolution;
	readonly pagePlan: PagePlan;
	readonly projects: ReadonlyMap<string, RecutProject>;
	readonly declaredPages: ReadonlyMap<string, readonly PageDeclaration[]>;
	readonly staleProse: readonly {
		readonly from: string;
		readonly to: string;
		readonly files: readonly string[];
	}[];
	readonly files: readonly FileChange[];
	readonly dryRun: boolean;
	readonly check: { readonly passed: boolean; readonly result: string } | undefined;
	readonly applied: boolean;
	readonly relativePath: (file: string) => string;
}

function section(title: string, lines: readonly string[]): string[] {
	return lines.length ? ['', title, ...lines.map((line) => `  ${line}`)] : [];
}

/** The printed plan: every change, every collision, and what the Lead still places by hand. */
export function formatRecutReport(input: RecutReportInput): string {
	const { plan, resolution, pagePlan, relativePath } = input;
	const newAddress = (address: string): string =>
		resolution.recordMoves.get(address)?.to ??
		resolution.deliverableMoves.get(address)?.to ??
		address;
	const renames = [
		...resolution.deliverableMoves.values(),
		...resolution.recordMoves.values(),
	].filter((move) => move.oldId !== move.newId);
	const header = input.dryRun
		? `Re-cut of ${input.root} (dry run: nothing written)`
		: input.applied
			? `Re-cut of ${input.root} (written; the check passed)`
			: `Re-cut of ${input.root} (nothing written: the check failed on the staged copy)`;
	const projectLines = plan.projects.map((planned) => {
		const project = input.projects.get(planned.key);
		const where = project ? relativePath(project.directory) : planned.folder;
		return `${project?.created ? 'create' : 'keep  '} ${planned.number} ${planned.key} (${where})`;
	});
	const emptyLines = resolution.leftEmpty.map(({ project, records }) => {
		const removed = plan.remove.includes(project);
		return `${project}: no deliverables left; ${
			records.length ? `still holds ${records.join(', ')}` : 'no records left'
		}; ${removed ? 'removed from initiative.ts (its folder stays on disk)' : 'kept in initiative.ts; add it to "remove" to drop it'}`;
	});
	// Each record is listed once, under the first move that takes it.
	const moveLines: string[] = [];
	const listed = new Set<string>();
	for (const move of resolution.deliverableMoves.values()) {
		moveLines.push(`${move.from} -> ${move.to}`);
		const records = move.records.filter((address) => !listed.has(address));
		for (const address of records) listed.add(address);
		if (records.length)
			moveLines.push(
				`  records: ${records.map((address) => `${address} -> ${newAddress(address)}`).join(', ')}`,
			);
	}
	const placedRecords = [...resolution.recordMoves.values()]
		.filter((move) => move.reason.kind === 'plan')
		.map((move) => `${move.from} -> ${move.to} (plan "records")`);
	const sharedLines = resolution.shared.map(({ record, outcome }) =>
		outcome.kind === 'stays'
			? `${record} stays: ${outcome.keptBy.join(', ')} ${outcome.keptBy.length === 1 ? 'still scopes' : 'still scope'} it; ${outcome.movers.map(newAddress).join(', ')} ${outcome.movers.length === 1 ? 'names' : 'name'} it ${record}`
			: `${record} -> ${newAddress(record)}: scoped by ${outcome.movers.join(', ')}, which go to different projects; it goes with the lowest-numbered one and the others name it qualified`,
	);
	const renameLines = renames.map((move) => `${move.kind} ${move.from} -> ${move.to}`);
	const addedLines = resolution.added.flatMap((row) => [
		`${row.project}/${row.id} (later) · ${row.title}`,
		`  when you scope it: dependsOn ${row.dependsOn.length ? row.dependsOn.join(', ') : 'none'}${row.why ? `; why: ${row.why}` : ''}`,
	]);
	const withdrawnLines = resolution.withdrawn.map(
		(row) =>
			`${row.address}: ${row.present ? 'removed (module, entry and acceptance)' : 'already absent'}${
				row.referencedBy.length
					? `; dependsOn removed from ${row.referencedBy.map(newAddress).join(', ')}`
					: ''
			}`,
	);
	const dependsLines = resolution.dependsOnEdits.map(
		(edit) =>
			`${edit.deliverable}: ${[
				...edit.remove.map((address) => `- ${address}`),
				...edit.add.map((address) => `+ ${address}`),
			].join(' ')}${edit.why ? ` (${edit.why})` : ''}`,
	);
	const movedPageLines = pagePlan.moved.map(
		(unit) =>
			`${relativePath(unit.page)} "${unit.heading}" -> ${relativePath(unit.target)} (${[
				...unit.records,
				...unit.deliverables,
			]
				.map(newAddress)
				.join(', ')})`,
	);
	const declaredLines = [...input.declaredPages].flatMap(([project, pages]) =>
		pages.map((page) => `${project} declares ${page.href}: ${page.reason}`),
	);
	const unplacedLines = pagePlan.unplaced.map(
		(item) =>
			`${newAddress(item.address)}${item.file ? ` in ${relativePath(item.file)}` : ''}: ${item.reason}`,
	);
	const proseLines = input.staleProse.map(
		(item) =>
			`${item.to} (was ${item.from}) is still named ${item.from.split('/')[1] ?? item.from} in ${item.files.join(', ')}`,
	);
	const fileLines = input.files.map(
		(file) => `${file.kind === 'created' ? '+' : file.kind === 'removed' ? '-' : '~'} ${file.path}`,
	);
	const counts = ['created', 'changed', 'removed'].map(
		(kind) => `${input.files.filter((file) => file.kind === kind).length} ${kind}`,
	);
	const checkLines = input.check
		? [
				'',
				input.check.passed
					? 'Initiative check on the staged tree: passed'
					: 'Initiative check on the staged tree: failed',
				...input.check.result.split('\n').map((line) => `  ${line}`),
			]
		: [
				'',
				'Initiative check: not run in a dry run. The real run checks a staged copy before writing.',
			];
	const next = input.applied
		? [
				'',
				'Next',
				'  Run demistify design types <project> for each new project, then demistify initiative check --write.',
				'  Move each "for you to place" passage by hand; references already name the new addresses.',
			]
		: [];
	return [
		header,
		...section('Projects', [...projectLines, ...emptyLines]),
		...section('Moves', moveLines),
		...section('Records placed by the plan', placedRecords),
		...section('Shared records', sharedLines),
		...section('Renamed on collision', renameLines),
		...section('Added as later deliverables', addedLines),
		...section('Withdrawn', withdrawnLines),
		...section('dependsOn changes', dependsLines),
		...section('Passages moved', movedPageLines),
		...section('Pages declared', declaredLines),
		...section('For you to place', unplacedLines),
		...section('Prose that still names an old id (never rewritten)', proseLines),
		...section('Open in the plan (not applied)', plan.open),
		...section(`Files: ${counts.join(', ')}`, fileLines),
		...checkLines,
		...next,
	].join('\n');
}
