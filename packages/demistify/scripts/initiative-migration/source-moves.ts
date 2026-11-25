import { basename, dirname, join } from 'node:path';

import typescript from 'typescript';

import type { DeliveryEntry } from '../../src/node/design-authoring/delivery-model.ts';
import type { DesignRecord } from '../../src/node/design-authoring/schema.ts';
import type { AddressMaps, AnchorRelocations } from './address-rewrite.ts';
import { readDeliveryFile, updateDelivery } from './delivery-sources.ts';
import {
	acceptanceRules,
	deliverableRules,
	designRules,
	helperForKind,
	helperToken,
	helperTokenPattern,
	literalEdits,
	recordOrPagePath,
	recordRules,
} from './literal-rules.ts';
import {
	sdkFilename,
	type PageDeclaration,
	type MigrationProject,
	type SourceWorkspace,
} from './migration-workspace.ts';
import { localId, projectOf, type MigrationResolution } from './move-resolution.ts';
import { camelBinding, laterDeliverableSource, recordsTemplate, slug } from './project-scaffold.ts';
import {
	applyTextEdits,
	defaultExport,
	ensureNamedImport,
	helperArgument,
	insertFirstProperty,
	nodeText,
	parseModule,
	propertyValue,
	rebuildArray,
	stringProperty,
	type TextEdit,
} from './source-edits.ts';

/** Everything the TypeScript pass needs to rewrite authored inputs. */
export interface SourceInputs {
	readonly projects: ReadonlyMap<string, MigrationProject>;
	readonly records: ReadonlyMap<string, DesignRecord>;
	readonly entries: readonly DeliveryEntry[];
	readonly resolution: MigrationResolution;
	readonly maps: AddressMaps;
	readonly anchors: AnchorRelocations;
	/** Destination of each moved deliverable: page relative to its new project, and fragment. */
	readonly destinations: ReadonlyMap<string, { readonly page: string; readonly fragment: string }>;
	/** Pages a target project declares so a moved deliverable can keep its destination. */
	readonly declaredPages: ReadonlyMap<string, readonly PageDeclaration[]>;
	/** Shared record files at the initiative root. */
	readonly sharedFiles: readonly string[];
}

interface RecordElement {
	readonly file: string;
	readonly id: string;
	readonly element: typescript.Expression;
}

/** The array a record file exports, or design.ts's `records`. */
function recordArray(
	file: string,
	source: typescript.SourceFile,
): typescript.ArrayLiteralExpression {
	const exported = defaultExport(source);
	const value =
		basename(file) === 'design.ts' ? propertyValue(helperArgument(exported), 'records') : exported;
	if (!value || !typescript.isArrayLiteralExpression(value))
		throw new Error(`${file}: expected a literal array of records.`);
	return value;
}

/** Record files of one design: design.ts, its imported records/*.ts, and the sidecars. */
function recordFiles(project: MigrationProject, workspace: SourceWorkspace): string[] {
	const designText = workspace.read(project.designFile);
	if (designText === undefined) return [];
	const source = parseModule(project.designFile, designText);
	const imported = source.statements.flatMap((statement): string[] => {
		if (
			!typescript.isImportDeclaration(statement) ||
			statement.importClause?.isTypeOnly ||
			!statement.importClause?.name ||
			!typescript.isStringLiteral(statement.moduleSpecifier)
		)
			return [];
		return [join(project.directory, statement.moduleSpecifier.text)];
	});
	return [
		project.designFile,
		...imported,
		join(project.directory, 'design-decisions.ts'),
		join(project.directory, 'design-assumptions.ts'),
	].filter((file) => workspace.read(file) !== undefined);
}

function recordElements(file: string, text: string): RecordElement[] {
	const source = parseModule(file, text);
	return recordArray(file, source).elements.flatMap((element): RecordElement[] => {
		const id = stringProperty(helperArgument(element), 'id');
		return id ? [{ file, id, element }] : [];
	});
}

interface Incoming {
	readonly records: Map<string, string[]>;
	readonly entries: Map<string, { binding: string; moduleFile: string; dependsOn: string[] }[]>;
	readonly acceptances: Map<string, string[]>;
}

function pushTo<TValue>(map: Map<string, TValue[]>, key: string, value: TValue): void {
	map.set(key, [...(map.get(key) ?? []), value]);
}

/** Rewrite every authored TypeScript input the move touches. */
export function planSources(inputs: SourceInputs, workspace: SourceWorkspace): void {
	const { projects, resolution, maps } = inputs;
	const designBase = (project: string): string => {
		const found = projects.get(project);
		if (!found) throw new Error(`Unknown project ${project}.`);
		return found.designFile;
	};
	const incoming: Incoming = { records: new Map(), entries: new Map(), acceptances: new Map() };
	const withdrawn = new Set(resolution.withdrawn.map(({ address }) => address));
	const entriesByAddress = new Map(inputs.entries.map((entry) => [entry.deliverable.id, entry]));
	const finalDependsOn = (address: string): string[] => {
		const entry = entriesByAddress.get(address);
		const after = maps.deliverables.get(address) ?? address;
		const translated = (entry?.dependsOn ?? [])
			.filter((dependency) => !withdrawn.has(dependency))
			.map((dependency) => maps.deliverables.get(dependency) ?? dependency);
		const edit = resolution.dependsOnEdits.find(({ deliverable }) => deliverable === after);
		const list = edit
			? [...translated.filter((dependency) => !edit.remove.includes(dependency)), ...edit.add]
			: translated;
		return [...new Set(list)].map((dependency) =>
			dependency.startsWith(`${projectOf(after)}/`) ? localId(dependency) : dependency,
		);
	};
	const originalDependsOn = (address: string): string[] =>
		(entriesByAddress.get(address)?.dependsOn ?? []).map((dependency) =>
			dependency.startsWith(`${projectOf(address)}/`) ? localId(dependency) : dependency,
		);

	// Phase 1: in-place edits on what stays; collect what leaves.
	const removals = new Map<string, Set<string>>();
	for (const project of projects.values()) {
		if (project.created) continue;
		const designPath = project.designFile;
		for (const file of recordFiles(project, workspace)) {
			const text = workspace.read(file);
			if (text === undefined) continue;
			const source = parseModule(file, text);
			const edits: TextEdit[] = [];
			for (const { id, element } of recordElements(file, text)) {
				const address = `${project.name}/${id}`;
				const move = resolution.recordMoves.get(address);
				const argument = helperArgument(element);
				if (!move) {
					edits.push(
						...literalEdits(
							argument,
							source,
							recordRules,
							{
								owner: { before: project.name, after: project.name },
								linkBase: { before: designPath, after: designPath },
							},
							inputs,
						),
					);
					continue;
				}
				const target = projectOf(move.to);
				const recordEdits = literalEdits(
					argument,
					source,
					recordRules,
					{
						owner: { before: project.name, after: target },
						linkBase: { before: designPath, after: designBase(target) },
						newId: move.newId,
					},
					inputs,
				);
				const record = inputs.records.get(address);
				if (!record) throw new Error(`Unknown record ${address}.`);
				const callee =
					typescript.isCallExpression(element) && typescript.isIdentifier(element.expression)
						? element.expression
						: undefined;
				if (callee)
					recordEdits.push({
						start: callee.getStart(source),
						end: callee.end,
						text: helperToken(helperForKind[record.kind]),
					});
				const destinationFile =
					record.kind === 'choice' || record.kind === 'ruling'
						? 'design-decisions.ts'
						: record.kind === 'assumption'
							? 'design-assumptions.ts'
							: 'design.ts';
				pushTo(
					incoming.records,
					join(dirname(designBase(target)), destinationFile),
					nodeText(text, element, recordEdits, { withLeadingTrivia: true }),
				);
				const leaving = removals.get(file) ?? new Set<string>();
				leaving.add(id);
				removals.set(file, leaving);
			}
			const updated = applyTextEdits(text, edits);
			if (updated !== text) workspace.write(file, updated);
		}

		// design.ts: navigation targets, acceptances.
		const designText = workspace.read(designPath);
		if (designText !== undefined) {
			const source = parseModule(designPath, designText);
			const design = helperArgument(defaultExport(source));
			const acceptances = propertyValue(design, 'acceptances');
			const leavingAcceptance = (element: typescript.Expression): boolean => {
				const id = stringProperty(element, 'deliverableId');
				const address = `${project.name}/${id ?? ''}`;
				return (
					id !== undefined && (resolution.deliverableMoves.has(address) || withdrawn.has(address))
				);
			};
			const leavingIndexes = new Set<number>();
			if (acceptances && typescript.isArrayLiteralExpression(acceptances))
				acceptances.elements.forEach((element, index): void => {
					if (!leavingAcceptance(element)) return;
					leavingIndexes.add(index);
					const id = stringProperty(element, 'deliverableId') ?? '';
					const move = resolution.deliverableMoves.get(`${project.name}/${id}`);
					if (!move) return;
					const target = projectOf(move.to);
					const moved = literalEdits(
						element,
						source,
						acceptanceRules,
						{
							owner: { before: project.name, after: target },
							linkBase: { before: designPath, after: designBase(target) },
						},
						inputs,
					);
					pushTo(
						incoming.acceptances,
						designBase(target),
						nodeText(designText, element, moved, { withLeadingTrivia: true }),
					);
				});
			const edits = literalEdits(
				design,
				source,
				designRules,
				{
					owner: { before: project.name, after: project.name },
					linkBase: { before: designPath, after: designPath },
				},
				inputs,
				(path) =>
					recordOrPagePath(path) ||
					(path[0] === 'acceptances' && typeof path[1] === 'number' && leavingIndexes.has(path[1])),
			);
			const updated = applyTextEdits(designText, edits);
			if (updated !== designText) workspace.write(designPath, updated);
		}

		// Deliverable modules.
		const delivery = readDeliveryFile(project, workspace);
		if (!delivery) continue;
		for (const [id, module] of delivery.modules) {
			const address = `${project.name}/${id}`;
			const text = workspace.read(module.file);
			if (text === undefined) continue;
			const source = parseModule(module.file, text);
			const argument = helperArgument(defaultExport(source));
			if (withdrawn.has(address)) {
				workspace.remove(module.file);
				continue;
			}
			const move = resolution.deliverableMoves.get(address);
			if (!move) {
				const updated = applyTextEdits(
					text,
					literalEdits(
						argument,
						source,
						deliverableRules,
						{
							owner: { before: project.name, after: project.name },
							linkBase: { before: designPath, after: designPath },
						},
						inputs,
					),
				);
				if (updated !== text) workspace.write(module.file, updated);
				continue;
			}
			const target = projectOf(move.to);
			const targetProject = projects.get(target);
			if (!targetProject) throw new Error(`Unknown project ${target}.`);
			const destination = inputs.destinations.get(address);
			const updated = applyTextEdits(
				text,
				literalEdits(
					argument,
					source,
					deliverableRules,
					{
						owner: { before: project.name, after: target },
						linkBase: { before: designPath, after: targetProject.designFile },
						newId: move.newId,
						...(destination && {
							destinationPage: destination.page,
							destinationFragment: destination.fragment,
						}),
					},
					inputs,
				),
			);
			const name = basename(module.file).replace(/^del-\d+[a-z]?/iu, (prefix) =>
				move.oldId === move.newId ? prefix : slug(move.newId),
			);
			let file = join(targetProject.directory, 'deliverables', name);
			for (let counter = 2; workspace.read(file) !== undefined; counter += 1)
				file = join(
					targetProject.directory,
					'deliverables',
					name.replace(/\.ts$/u, `-${counter}.ts`),
				);
			workspace.write(file, updated);
			workspace.remove(module.file);
			pushTo(incoming.entries, target, {
				binding: module.binding,
				moduleFile: file,
				dependsOn: finalDependsOn(address),
			});
		}
	}

	// Added rows: later deliverables with their title and next free id.
	for (const row of resolution.added) {
		const project = projects.get(row.project);
		if (!project) throw new Error(`Unknown project ${row.project}.`);
		let file = join(project.directory, 'deliverables', `${slug(row.id)}-${slug(row.title)}.ts`);
		for (let counter = 2; workspace.read(file) !== undefined; counter += 1)
			file = file.replace(/(?:-\d+)?\.ts$/u, `-${counter}.ts`);
		workspace.write(file, laterDeliverableSource(row));
		pushTo(incoming.entries, row.project, {
			binding: camelBinding(row.title, new Set()),
			moduleFile: file,
			dependsOn: [],
		});
	}

	// Phase 2: structural edits, file by file.
	for (const [file, ids] of removals) {
		const text = workspace.read(file);
		if (text === undefined) continue;
		const source = parseModule(file, text);
		const array = recordArray(file, source);
		const edit = rebuildArray(
			text,
			array,
			(element) => !ids.has(stringProperty(helperArgument(element), 'id') ?? ''),
			[],
		);
		workspace.write(file, applyTextEdits(text, [edit]));
	}
	for (const [file, texts] of incoming.records) appendRecords(file, texts, workspace);
	for (const project of projects.values()) {
		updateDelivery(project, workspace, {
			incoming: incoming.entries.get(project.name) ?? [],
			finalDependsOn,
			originalDependsOn,
			leaves: (address) => resolution.deliverableMoves.has(address) || withdrawn.has(address),
		});
		updateDesign(project, inputs, workspace, incoming.acceptances.get(project.designFile) ?? []);
	}
	for (const file of inputs.sharedFiles) rewriteSharedRecords(file, inputs, workspace);
}

function appendRecords(file: string, texts: readonly string[], workspace: SourceWorkspace): void {
	let text = workspace.read(file) ?? recordsTemplate;
	const helpers = new Set<string>();
	for (const element of texts)
		for (const match of element.matchAll(helperTokenPattern)) if (match[1]) helpers.add(match[1]);
	const sdk = join(dirname(file), sdkFilename);
	const bindings = new Map<string, string>();
	for (const helper of helpers) {
		const ensured = ensureNamedImport(file, text, sdk, `./${sdkFilename}`, helper);
		text = ensured.text;
		bindings.set(helper, ensured.binding);
	}
	const resolved = texts.map((element) =>
		element.replace(helperTokenPattern, (_token, helper: string) => bindings.get(helper) ?? helper),
	);
	const source = parseModule(file, text);
	const edit = rebuildArray(text, recordArray(file, source), () => true, resolved);
	workspace.write(file, applyTextEdits(text, [edit]));
}

function updateDesign(
	project: MigrationProject,
	inputs: SourceInputs,
	workspace: SourceWorkspace,
	acceptances: readonly string[],
): void {
	const declarations = inputs.declaredPages.get(project.name) ?? [];
	const withdrawn = new Set(inputs.resolution.withdrawn.map(({ address }) => address));
	let text = workspace.read(project.designFile);
	if (text === undefined) return;
	let source = parseModule(project.designFile, text);
	let design = helperArgument(defaultExport(source));
	const current = propertyValue(design, 'acceptances');
	const leaving = (element: typescript.Expression): boolean => {
		const id = stringProperty(element, 'deliverableId');
		const address = `${project.name}/${id ?? ''}`;
		return (
			id !== undefined &&
			(inputs.resolution.deliverableMoves.has(address) || withdrawn.has(address))
		);
	};
	if (current && typescript.isArrayLiteralExpression(current)) {
		if (current.elements.some(leaving) || acceptances.length)
			text = applyTextEdits(text, [
				rebuildArray(text, current, (element) => !leaving(element), acceptances),
			]);
	} else if (acceptances.length && typescript.isObjectLiteralExpression(design)) {
		text = applyTextEdits(text, [
			insertFirstProperty(
				text,
				design,
				`acceptances: [${acceptances.map((entry) => entry.trim()).join(', ')}]`,
			),
		]);
	}
	if (declarations.length) {
		source = parseModule(project.designFile, text);
		design = helperArgument(defaultExport(source));
		const pages = propertyValue(design, 'pages');
		if (pages && typescript.isArrayLiteralExpression(pages))
			text = applyTextEdits(text, [
				rebuildArray(
					text,
					pages,
					() => true,
					declarations.map(
						(page) =>
							`{ id: ${JSON.stringify(page.id)}, title: ${JSON.stringify(page.title)}, href: ${JSON.stringify(page.href)}, icon: ${JSON.stringify(page.icon)} }`,
					),
				),
			]);
	}
	if (text !== workspace.read(project.designFile)) workspace.write(project.designFile, text);
}

function rewriteSharedRecords(
	file: string,
	inputs: SourceInputs,
	workspace: SourceWorkspace,
): void {
	const text = workspace.read(file);
	if (text === undefined) return;
	const source = parseModule(file, text);
	const array = defaultExport(source);
	if (!typescript.isArrayLiteralExpression(array)) return;
	const edits = array.elements.flatMap((element) =>
		literalEdits(
			helperArgument(element),
			source,
			recordRules,
			{
				owner: { before: 'initiative', after: 'initiative' },
				linkBase: { before: file, after: file },
			},
			inputs,
		),
	);
	const updated = applyTextEdits(text, edits);
	if (updated !== text) workspace.write(file, updated);
}
