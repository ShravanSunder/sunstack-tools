import { join, relative, sep } from 'node:path';

import typescript from 'typescript';

import { deliveryTemplate } from './project-scaffold.ts';
import type { RecutProject, SourceWorkspace } from './recut-workspace.ts';
import {
	addImportLine,
	applyTextEdits,
	defaultExport,
	helperArgument,
	parseModule,
	propertyValue,
	rebuildArray,
	removeDefaultImports,
	stringProperty,
} from './source-edits.ts';

/** Delivery file of a project: its entries by deliverable id and module files. */
export interface DeliveryFile {
	readonly file: string;
	readonly source: typescript.SourceFile;
	readonly array: typescript.ArrayLiteralExpression;
	readonly bindingFiles: ReadonlyMap<string, string>;
	readonly modules: ReadonlyMap<string, { readonly binding: string; readonly file: string }>;
	readonly entries: ReadonlyMap<string, typescript.ObjectLiteralExpression>;
}

export function readDeliveryFile(
	project: RecutProject,
	workspace: SourceWorkspace,
): DeliveryFile | undefined {
	const file = join(project.directory, 'design-delivery.ts');
	const text = workspace.read(file);
	if (text === undefined) return undefined;
	const source = parseModule(file, text);
	const exported = defaultExport(source);
	const array = typescript.isCallExpression(exported) ? exported.arguments[1] : undefined;
	if (!array || !typescript.isArrayLiteralExpression(array))
		throw new Error(`${file}: expected defineDelivery(design, [...]).`);
	const bindingFiles = new Map<string, string>();
	for (const statement of source.statements) {
		if (
			typescript.isImportDeclaration(statement) &&
			statement.importClause?.name &&
			typescript.isStringLiteral(statement.moduleSpecifier) &&
			statement.moduleSpecifier.text.includes('deliverables/')
		)
			bindingFiles.set(
				statement.importClause.name.text,
				join(project.directory, statement.moduleSpecifier.text),
			);
	}
	const modules = new Map<string, { binding: string; file: string }>();
	const entries = new Map<string, typescript.ObjectLiteralExpression>();
	for (const element of array.elements) {
		if (!typescript.isObjectLiteralExpression(element)) continue;
		const value = propertyValue(element, 'deliverable');
		if (!value || !typescript.isIdentifier(value)) continue;
		const moduleFile = bindingFiles.get(value.text);
		const moduleText = moduleFile ? workspace.read(moduleFile) : undefined;
		if (!moduleFile || moduleText === undefined) continue;
		const id = stringProperty(
			helperArgument(defaultExport(parseModule(moduleFile, moduleText))),
			'id',
		);
		if (!id) continue;
		modules.set(id, { binding: value.text, file: moduleFile });
		entries.set(id, element);
	}
	return { file, source, array, bindingFiles, modules, entries };
}

function quoteStyle(text: string): '"' | "'" {
	const single = (text.match(/'[^'\n]*'/gu) ?? []).length;
	const double = (text.match(/"[^"\n]*"/gu) ?? []).length;
	return single > double ? "'" : '"';
}

function quoted(value: string, quote: '"' | "'"): string {
	return quote === "'"
		? `'${value.replace(/\\/gu, '\\\\').replace(/'/gu, "\\'")}'`
		: JSON.stringify(value);
}

export interface DeliveryUpdate {
	/** Whether an existing deliverable of this project leaves it (moved or withdrawn). */
	readonly leaves: (address: string) => boolean;
	readonly incoming: readonly { binding: string; moduleFile: string; dependsOn: string[] }[];
	readonly finalDependsOn: (address: string) => string[];
	readonly originalDependsOn: (address: string) => string[];
}

function entryText(binding: string, dependsOn: readonly string[], quote: '"' | "'"): string {
	return dependsOn.length
		? `{ deliverable: ${binding}, dependsOn: [${dependsOn.map((value) => quoted(value, quote)).join(', ')}] }`
		: `{ deliverable: ${binding} }`;
}

/** Remove leaving entries and imports, rewrite changed dependsOn, append arrivals. */
export function updateDelivery(
	project: RecutProject,
	workspace: SourceWorkspace,
	update: DeliveryUpdate,
): void {
	const file = join(project.directory, 'design-delivery.ts');
	if (workspace.read(file) === undefined) {
		if (!update.incoming.length) return;
		workspace.write(file, deliveryTemplate(undefined));
	}
	const delivery = readDeliveryFile(project, workspace);
	if (!delivery) return;
	const text = delivery.source.text;
	const quote = quoteStyle(text);
	const leaving = new Set<string>();
	for (const [binding, moduleFile] of delivery.bindingFiles)
		if (workspace.read(moduleFile) === undefined) leaving.add(binding);
	const replaced = new Map<typescript.Expression, string>();
	for (const [id, entry] of delivery.entries) {
		const address = `${project.name}/${id}`;
		const module = delivery.modules.get(id);
		if (!module) continue;
		if (update.leaves(address)) {
			leaving.add(module.binding);
			continue;
		}
		const final = update.finalDependsOn(address);
		if (JSON.stringify(final) !== JSON.stringify(update.originalDependsOn(address)))
			replaced.set(entry, entryText(module.binding, final, quote));
	}
	const takenBindings = new Set(
		delivery.source.statements.flatMap((statement): string[] =>
			typescript.isImportDeclaration(statement) && statement.importClause?.name
				? [statement.importClause.name.text]
				: [],
		),
	);
	for (const binding of leaving) takenBindings.delete(binding);
	const imports: string[] = [];
	const appended: string[] = [];
	for (const arrival of update.incoming) {
		let binding = arrival.binding;
		for (let counter = 2; takenBindings.has(binding); counter += 1)
			binding = `${arrival.binding}${counter}`;
		takenBindings.add(binding);
		const specifier = `./${relative(project.directory, arrival.moduleFile).split(sep).join('/')}`;
		imports.push(`import ${binding} from ${quoted(specifier, quote)};`);
		appended.push(entryText(binding, arrival.dependsOn, quote));
	}
	if (!leaving.size && !replaced.size && !imports.length) return;
	const arrayText = rebuildArrayWithReplacements(text, delivery, leaving, replaced, appended);
	let updated = applyTextEdits(text, [
		...removeDefaultImports(delivery.source, leaving),
		{ start: delivery.array.getStart(delivery.source), end: delivery.array.end, text: arrayText },
	]);
	for (const line of imports) updated = addImportLine(parseModule(file, updated), updated, line);
	workspace.write(file, updated);
}

function rebuildArrayWithReplacements(
	text: string,
	delivery: DeliveryFile,
	leaving: ReadonlySet<string>,
	replaced: ReadonlyMap<typescript.Expression, string>,
	appended: readonly string[],
): string {
	const replacedText = applyTextEdits(
		text,
		[...replaced].map(([element, replacement]) => ({
			start: element.getStart(delivery.source),
			end: element.end,
			text: replacement,
		})),
	);
	const source = parseModule(delivery.file, replacedText);
	const exported = defaultExport(source);
	const array = typescript.isCallExpression(exported) ? exported.arguments[1] : undefined;
	if (!array || !typescript.isArrayLiteralExpression(array))
		throw new Error(`${delivery.file}: expected defineDelivery(design, [...]).`);
	const edit = rebuildArray(
		replacedText,
		array,
		(element) => {
			const value = propertyValue(element, 'deliverable');
			return !(value && typescript.isIdentifier(value) && leaving.has(value.text));
		},
		appended,
	);
	return edit.text;
}
