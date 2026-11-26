import { dirname, isAbsolute, join, relative, resolve, sep } from 'node:path';

import typescript from 'typescript';
import { z } from 'zod';

import { deliverySdkSource } from './delivery-sdk-source.ts';
import { designTypesFor } from './design-type-source.ts';
import {
	checkAuthoringTypes,
	designDeclarationFilename,
	deliverySdkFilename,
	readLiteral,
	unwrapExpression,
} from './read-design.ts';

export { deliverySdkFilename } from './read-design.ts';
export const deliveryEntryFilename = 'design-delivery.ts';

type HelperName = 'defineDelivery' | 'defineDeliverable';
type SourceReader = (file: string) => Promise<string | undefined>;
const deliveryOptions = z.strictObject({ lead: z.string().min(1) });

export interface AuthoredDelivery {
	readonly entries: unknown;
	readonly checkTypes: (knownRecordIds: readonly string[]) => void;
	readonly lead?: string;
}

interface AuthoringModule {
	source: typescript.SourceFile;
	imports: readonly typescript.ImportDeclaration[];
	exported: typescript.Expression;
}

function readModule(file: string, text: string): AuthoringModule {
	const source = typescript.createSourceFile(file, text, typescript.ScriptTarget.ES2022, true);
	const entries = source.statements.filter(typescript.isExportAssignment);
	const [entry] = entries;
	if (
		entries.length !== 1 ||
		!entry ||
		entry.isExportEquals ||
		source.statements.some(
			(statement): boolean => statement !== entry && !typescript.isImportDeclaration(statement),
		)
	)
		throw new Error(`${file}: expected imports and one default export.`);
	return {
		source,
		imports: source.statements.filter(typescript.isImportDeclaration),
		exported: unwrapExpression(entry.expression),
	};
}

function relativeImport(statement: typescript.ImportDeclaration, importer: string): string {
	const reference = statement.moduleSpecifier;
	if (
		!typescript.isStringLiteral(reference) ||
		(!reference.text.startsWith('./') && !reference.text.startsWith('../')) ||
		!reference.text.endsWith('.ts') ||
		reference.text.includes('\\')
	)
		throw new Error(`${importer}: expected a relative TypeScript import.`);
	return resolve(dirname(importer), reference.text);
}

function helperBinding(statement: typescript.ImportDeclaration, helper: HelperName): string {
	const clause = statement.importClause;
	const bindings = clause?.namedBindings;
	const [binding] = bindings && typescript.isNamedImports(bindings) ? bindings.elements : [];
	if (
		clause?.name ||
		!bindings ||
		!typescript.isNamedImports(bindings) ||
		bindings.elements.length !== 1 ||
		!binding ||
		binding.isTypeOnly ||
		(binding.propertyName ?? binding.name).text !== helper
	)
		throw new Error(`Import ${helper} from ${deliverySdkFilename}.`);
	return binding.name.text;
}

function calledHelper(
	module: AuthoringModule,
	binding: string | undefined,
	helper: HelperName,
	argumentCounts: readonly number[],
): readonly typescript.Expression[] {
	const expression = module.exported;
	if (
		!binding ||
		!typescript.isCallExpression(expression) ||
		!typescript.isIdentifier(expression.expression) ||
		expression.expression.text !== binding ||
		!argumentCounts.includes(expression.arguments.length)
	)
		throw new Error(`${module.source.fileName}: default export must call ${helper}.`);
	return expression.arguments;
}

/** Read authored imports and SDK calls as data, with no execution of project modules. */
export async function readDeliveryEntries(
	designFile: string,
	designText: string,
	readSource: SourceReader,
	designSources: ReadonlyMap<string, string>,
): Promise<AuthoredDelivery> {
	const directory = dirname(designFile);
	if ((await readSource(join(directory, 'delivery.ts'))) !== undefined)
		throw new Error(`Rename delivery.ts to ${deliveryEntryFilename} before checking this design.`);
	const entryFile = join(directory, deliveryEntryFilename);
	const sources = new Map([...designSources, [designFile, designText]]);
	const sdkFile = join(directory, deliverySdkFilename);
	sources.set(sdkFile, deliverySdkSource);
	const entryText = await readSource(entryFile);
	function checkTypes(knownRecordIds: readonly string[]): void {
		sources.set(join(directory, designDeclarationFilename), designTypesFor(knownRecordIds));
		checkAuthoringTypes(
			[designFile, ...designSources.keys(), ...(entryText === undefined ? [] : [entryFile])],
			sources,
		);
	}
	if (entryText === undefined) return { entries: [], checkTypes };
	sources.set(entryFile, entryText);

	async function readDeliverable(file: string): Promise<unknown> {
		const text = await readSource(file);
		if (text === undefined) throw new Error(`Missing deliverable file: ${file}`);
		sources.set(file, text);
		const module = readModule(file, text);
		let binding: string | undefined;
		for (const statement of module.imports) {
			if (statement.importClause?.isTypeOnly) continue;
			if (binding || relativeImport(statement, file) !== sdkFile)
				throw new Error(`${file}: deliverable value imports are limited to the design SDK.`);
			binding = helperBinding(statement, 'defineDeliverable');
		}
		const [data] = calledHelper(module, binding, 'defineDeliverable', [1]);
		if (!data) throw new Error(`${file}: expected deliverable data.`);
		return readLiteral(data, module.source);
	}

	const entry = readModule(entryFile, entryText);
	const deliverablesDirectory = join(directory, 'deliverables');
	const bindings = new Map<string, unknown>();
	let helper: string | undefined;
	let designBinding: string | undefined;
	const imports: { name: string; file: string }[] = [];
	const importedFiles = new Set<string>();
	for (const statement of entry.imports) {
		if (statement.importClause?.isTypeOnly) continue;
		const file = relativeImport(statement, entryFile);
		if (importedFiles.has(file)) throw new Error(`Duplicate delivery import: ${file}`);
		importedFiles.add(file);
		if (file === sdkFile) {
			helper = helperBinding(statement, 'defineDelivery');
			continue;
		}
		const clause = statement.importClause;
		if (!clause?.name || clause.namedBindings)
			throw new Error(`${entryFile}: import design and deliverables by their default exports.`);
		if (file === designFile) {
			designBinding = clause.name.text;
			continue;
		}
		const child = relative(deliverablesDirectory, file);
		if (
			!child ||
			isAbsolute(child) ||
			child === '..' ||
			child.startsWith(`..${sep}`) ||
			child.endsWith('.d.ts')
		)
			throw new Error(`Delivery import must name a TypeScript file in deliverables/: ${file}`);
		imports.push({ name: clause.name.text, file });
	}
	const [designArgument, declaredEntries, declaredOptions] = calledHelper(
		entry,
		helper,
		'defineDelivery',
		[2, 3],
	);
	if (
		!designBinding ||
		!designArgument ||
		!typescript.isIdentifier(designArgument) ||
		designArgument.text !== designBinding ||
		!declaredEntries
	)
		throw new Error('Pass the imported design.ts and a literal array to defineDelivery.');
	await Promise.all(
		imports.map(async ({ name, file }) => {
			bindings.set(name, await readDeliverable(file));
		}),
	);
	return {
		entries: readLiteral(declaredEntries, entry.source, bindings),
		checkTypes,
		...(declaredOptions === undefined
			? {}
			: deliveryOptions.parse(readLiteral(declaredOptions, entry.source))),
	};
}
