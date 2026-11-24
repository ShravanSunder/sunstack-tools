import { basename, dirname, isAbsolute, join, relative, resolve, sep } from 'node:path';

import typescript from 'typescript';
import { z } from 'zod';

import { assumptionRecord, decisionRecord } from '../../core/decision-record.ts';
import { recordItemValidationError } from './record-item-diagnostics.ts';
import { authoredDesignSchema } from './schema.ts';
import type { AuthoredDesign, DesignCore, DesignRecord } from './schema.ts';

export const designDeclarationFilename = 'design-schema.d.ts';
export const deliverySdkFilename = 'design-sdk.generated.ts';

const recordHelperKinds: ReadonlyMap<string, DesignRecord['kind']> = new Map([
	['defineNeed', 'need'],
	['defineRule', 'rule'],
	['defineRealization', 'realization'],
	['defineCheck', 'planned-check'],
	['defineChoice', 'choice'],
	['defineRuling', 'ruling'],
	['defineAssumption', 'assumption'],
]);

export function unwrapExpression(input: typescript.Expression): typescript.Expression {
	let expression = input;
	while (
		typescript.isAsExpression(expression) ||
		typescript.isSatisfiesExpression(expression) ||
		typescript.isParenthesizedExpression(expression)
	)
		expression = expression.expression;
	return expression;
}

export function readLiteral(
	input: typescript.Expression,
	source: typescript.SourceFile,
	bindings?: ReadonlyMap<string, unknown>,
): unknown {
	const expression = unwrapExpression(input);
	if (typescript.isIdentifier(expression) && bindings?.has(expression.text))
		return bindings.get(expression.text);
	if (typescript.isStringLiteral(expression)) return expression.text;
	if (typescript.isNumericLiteral(expression)) return Number(expression.text);
	if (expression.kind === typescript.SyntaxKind.TrueKeyword) return true;
	if (expression.kind === typescript.SyntaxKind.FalseKeyword) return false;
	if (expression.kind === typescript.SyntaxKind.NullKeyword) return null;
	if (typescript.isArrayLiteralExpression(expression))
		return expression.elements.map((element): unknown => readLiteral(element, source, bindings));
	if (typescript.isObjectLiteralExpression(expression))
		return readObjectLiteral(expression, source, {
			bindings,
			readProperty: (_name, value): unknown => readLiteral(value, source, bindings),
		});
	const { line } = source.getLineAndCharacterOfPosition(expression.getStart(source));
	throw new Error(
		`${source.fileName}:${line + 1}: expected a data literal or an imported deliverable.`,
	);
}

interface ObjectLiteralReader {
	readonly bindings?: ReadonlyMap<string, unknown> | undefined;
	readonly readProperty: (name: string, value: typescript.Expression) => unknown;
}

function readObjectLiteral(
	expression: typescript.ObjectLiteralExpression,
	source: typescript.SourceFile,
	{ bindings, readProperty }: ObjectLiteralReader,
): Record<string, unknown> {
	const fields = new Map<string, unknown>();
	for (const property of expression.properties) {
		if (
			typescript.isShorthandPropertyAssignment(property) &&
			!property.objectAssignmentInitializer
		) {
			const name = property.name.text;
			if (!bindings?.has(name)) throw new Error(`Unknown authoring binding: ${name}.`);
			if (fields.has(name)) throw new Error(`Duplicate authoring object key: ${name}.`);
			fields.set(name, bindings.get(name));
			continue;
		}
		if (!typescript.isPropertyAssignment(property))
			throw new Error(`${source.fileName} object properties must be simple assignments.`);
		const { name } = property;
		if (!typescript.isStringLiteral(name) && !typescript.isIdentifier(name))
			throw new Error(`${source.fileName} object keys must be identifiers or quoted strings.`);
		if (fields.has(name.text))
			throw new Error(`${source.fileName}: duplicate object key ${name.text}.`);
		fields.set(name.text, readProperty(name.text, property.initializer));
	}
	return Object.fromEntries(fields);
}

function authoredModule(
	file: string,
	text: string,
): {
	source: typescript.SourceFile;
	imports: readonly typescript.ImportDeclaration[];
	expression: typescript.Expression;
} {
	const source = typescript.createSourceFile(file, text, typescript.ScriptTarget.ES2022, true);
	const exports = source.statements.filter(typescript.isExportAssignment);
	const [entry] = exports;
	if (
		exports.length !== 1 ||
		!entry ||
		entry.isExportEquals ||
		source.statements.some(
			(statement): boolean => statement !== entry && !typescript.isImportDeclaration(statement),
		)
	)
		throw new Error(`${file}: expected one default export and optional imports.`);
	return {
		source,
		imports: source.statements.filter(typescript.isImportDeclaration),
		expression: unwrapExpression(entry.expression),
	};
}

function sdkBindings(
	imports: readonly typescript.ImportDeclaration[],
	file: string,
	sdkFile: string,
): ReadonlyMap<string, string> {
	const bindings = new Map<string, string>();
	for (const statement of imports) {
		const { importClause, moduleSpecifier, attributes } = statement;
		if (importClause?.isTypeOnly) continue;
		const named = importClause?.namedBindings;
		if (
			!typescript.isStringLiteral(moduleSpecifier) ||
			!moduleSpecifier.text.startsWith('.') ||
			resolve(dirname(file), moduleSpecifier.text) !== sdkFile ||
			importClause?.name ||
			!named ||
			!typescript.isNamedImports(named) ||
			attributes
		)
			throw new Error(`${file}: expected named imports from the generated design SDK.`);
		for (const binding of named.elements) {
			if (binding.isTypeOnly) continue;
			const name = (binding.propertyName ?? binding.name).text;
			if (
				(name !== 'defineDesign' && !recordHelperKinds.has(name)) ||
				bindings.has(binding.name.text)
			)
				throw new Error(`${file}: unsupported record helper ${name}.`);
			bindings.set(binding.name.text, name);
		}
	}
	return bindings;
}

function readRecord(
	input: typescript.Expression,
	source: typescript.SourceFile,
	bindings: ReadonlyMap<string, string>,
): unknown {
	const expression = unwrapExpression(input);
	if (!typescript.isCallExpression(expression)) return readLiteral(expression, source);
	const helper =
		typescript.isIdentifier(expression.expression) && bindings.get(expression.expression.text);
	const kind = helper && recordHelperKinds.get(helper);
	const [argument] = expression.arguments;
	if (
		!kind ||
		!argument ||
		expression.arguments.length !== 1 ||
		!typescript.isObjectLiteralExpression(unwrapExpression(argument))
	)
		throw new Error(`${source.fileName}: use an imported record helper with one literal object.`);
	const value = unwrapExpression(argument);
	if (!typescript.isObjectLiteralExpression(value)) throw new Error('Expected a literal record.');
	const fields = readObjectLiteral(value, source, {
		readProperty: (_name, term): unknown => readLiteral(term, source),
	});
	if (Object.hasOwn(fields, 'kind'))
		throw new Error(`${source.fileName}: a record helper supplies its kind.`);
	return { ...fields, kind };
}

function readDefaultRecords(file: string, text: string, sdkFile: string): unknown {
	const { source, imports, expression } = authoredModule(file, text);
	const bindings = sdkBindings(imports, file, sdkFile);
	if (!typescript.isArrayLiteralExpression(expression))
		throw new Error(`${file}: export a literal array of records.`);
	return expression.elements.map((entry): unknown => readRecord(entry, source, bindings));
}

function recordValidationError(
	file: string,
	rawRecords: unknown,
	error: z.ZodError,
	embedded: boolean,
): Error {
	const records: readonly unknown[] = Array.isArray(rawRecords) ? rawRecords : [];
	const issues = error.issues.map(({ path, message }): string => {
		const index = embedded ? (path[0] === 'records' ? path[1] : undefined) : path[0];
		const record = typeof index === 'number' ? records[index] : undefined;
		const label =
			typeof record === 'object' && record && 'id' in record && typeof record.id === 'string'
				? ` record ${record.id}`
				: '';
		return `${file}${label} ${path.join('.') || 'input'}: ${message}`;
	});
	return new Error(issues.join('\n'));
}

interface AuthoredRecordImport {
	readonly binding: string;
	readonly file: string;
}

function resolveRecordImport(
	statement: typescript.ImportDeclaration,
	designFile: string,
): AuthoredRecordImport {
	const { importClause, moduleSpecifier, attributes } = statement;
	if (
		!importClause?.name ||
		importClause.namedBindings ||
		attributes ||
		!typescript.isStringLiteral(moduleSpecifier) ||
		!/^\.\.?\//.test(moduleSpecifier.text) ||
		!moduleSpecifier.text.endsWith('.ts') ||
		moduleSpecifier.text.endsWith('.d.ts') ||
		moduleSpecifier.text.includes('\\')
	)
		throw new Error(`${designFile}: record imports must be relative default imports of .ts files.`);
	const directory = dirname(designFile);
	const file = resolve(directory, moduleSpecifier.text);
	const child = relative(directory, file);
	if (child === '..' || child.startsWith(`..${sep}`) || isAbsolute(child))
		throw new Error(
			`${designFile}: record import escapes the design set: ${moduleSpecifier.text}.`,
		);
	return { binding: importClause.name.text, file };
}

type AuthoredRecord = AuthoredDesign['records'][number];

function readRecordArray(file: string, text: string, sdkFile: string): AuthoredRecord[] {
	const input = readDefaultRecords(file, text, sdkFile);
	const validated = authoredDesignSchema.shape.records.safeParse(input);
	if (validated.success) return validated.data;
	throw recordValidationError(file, input, validated.error, false);
}

function composeRecordArray(
	value: typescript.ArrayLiteralExpression,
	source: typescript.SourceFile,
	imports: ReadonlyMap<string, AuthoredRecord[]>,
	used: Set<string>,
	helpers: ReadonlyMap<string, string>,
): unknown[] {
	return value.elements.flatMap((element): unknown[] => {
		if (!typescript.isSpreadElement(element)) return [readRecord(element, source, helpers)];
		const binding = typescript.isIdentifier(element.expression) && element.expression.text;
		const records = binding ? imports.get(binding) : undefined;
		if (!binding || !records)
			throw new Error(
				`${source.fileName}: spread only an imported record array directly in records.`,
			);
		used.add(binding);
		return records;
	});
}

interface ReadDesignResult {
	readonly design: AuthoredDesign;
	readonly recordSources: ReadonlyMap<string, string>;
}

interface DesignDeclaration {
	readonly source: typescript.SourceFile;
	readonly designLiteral: typescript.ObjectLiteralExpression;
	readonly helpers: ReadonlyMap<string, string>;
	readonly sdkFile: string;
	readonly recordImports: readonly typescript.ImportDeclaration[];
}

function readDesignDeclaration(file: string, text: string): DesignDeclaration {
	const { source, imports, expression } = authoredModule(file, text);
	const sdkFile = join(dirname(file), deliverySdkFilename);
	const sdkImports = imports.filter(
		({ moduleSpecifier }) =>
			typescript.isStringLiteral(moduleSpecifier) &&
			resolve(dirname(file), moduleSpecifier.text) === sdkFile,
	);
	const helpers = sdkBindings(sdkImports, file, sdkFile);
	const [argument] = typescript.isCallExpression(expression) ? expression.arguments : [];
	const designLiteral =
		typescript.isCallExpression(expression) &&
		typescript.isIdentifier(expression.expression) &&
		helpers.get(expression.expression.text) === 'defineDesign' &&
		expression.arguments.length === 1 &&
		argument
			? unwrapExpression(argument)
			: expression;
	if (!typescript.isObjectLiteralExpression(designLiteral))
		throw new Error(`${file}: expected a literal design object or defineDesign call.`);

	const recordImports = imports.filter(
		(statement): boolean => !statement.importClause?.isTypeOnly && !sdkImports.includes(statement),
	);
	return { source, designLiteral, helpers, sdkFile, recordImports };
}

export function readDesignPages(file: string, text: string): AuthoredDesign['pages'] {
	const { source, designLiteral } = readDesignDeclaration(file, text);
	const fields = readObjectLiteral(designLiteral, source, {
		readProperty: (name, value): unknown =>
			name === 'pages' ? readLiteral(value, source) : undefined,
	});
	return authoredDesignSchema.shape.pages.parse(fields['pages']);
}

/** Compose explicit record arrays before validating the design, without executing author modules. */
export async function readDesign(
	file: string,
	text: string,
	readOwnedInput: (file: string) => Promise<string>,
): Promise<ReadDesignResult> {
	const { source, designLiteral, helpers, sdkFile, recordImports } = readDesignDeclaration(
		file,
		text,
	);

	const recordModules = await Promise.all(
		recordImports.map(async (statement) => {
			const { binding, file: recordPath } = resolveRecordImport(statement, file);
			return { binding, file: recordPath, text: await readOwnedInput(recordPath) };
		}),
	);
	const arraysByBinding = new Map(
		recordModules.map((record) => [
			record.binding,
			readRecordArray(record.file, record.text, sdkFile),
		]),
	);
	const used = new Set<string>();
	const input = readObjectLiteral(designLiteral, source, {
		readProperty(name, value): unknown {
			const literal = unwrapExpression(value);
			return name === 'records' && typescript.isArrayLiteralExpression(literal)
				? composeRecordArray(literal, source, arraysByBinding, used, helpers)
				: readLiteral(value, source);
		},
	});
	for (const binding of arraysByBinding.keys()) {
		if (!used.has(binding))
			throw new Error(`${file}: spread imported record array ${binding} directly in records.`);
	}
	const validated = authoredDesignSchema.safeParse(input);
	if (validated.success)
		return {
			design: validated.data,
			recordSources: new Map(recordModules.map((record) => [record.file, record.text])),
		};
	throw recordValidationError(file, input['records'], validated.error, true);
}

/** Read one literal decision or assumption file. */
export function readDesignItems(
	file: string,
	text: string,
	kind: 'choice' | 'assumption',
	projectDirectory = dirname(file),
): DesignRecord[] {
	const input = readDefaultRecords(file, text, join(dirname(file), deliverySdkFilename));
	const validated =
		kind === 'choice'
			? z.array(decisionRecord).safeParse(input)
			: z.array(assumptionRecord).safeParse(input);
	if (validated.success) return validated.data;
	throw recordItemValidationError(relative(projectDirectory, file), input, validated.error);
}

export async function readDesignSources(
	directory: string,
	read: (file: string) => Promise<string>,
	readOptional: (file: string) => Promise<string | undefined>,
	projectDirectory = directory,
): Promise<{
	design: DesignCore;
	designFile: string;
	designText: string;
	sources: ReadonlyMap<string, string>;
}> {
	const designFile = join(directory, 'design.ts');
	const decisionFile = join(directory, 'design-decisions.ts');
	const assumptionFile = join(directory, 'design-assumptions.ts');
	const [designText, decisions, assumptions] = await Promise.all([
		read(designFile),
		readOptional(decisionFile),
		readOptional(assumptionFile),
	]);
	const { design: authored, recordSources } = await readDesign(designFile, designText, read);
	const sources = new Map(recordSources);
	if (decisions !== undefined) sources.set(decisionFile, decisions);
	if (assumptions !== undefined) sources.set(assumptionFile, assumptions);
	const design: DesignCore = {
		...authored,
		records: [
			...authored.records,
			...(decisions === undefined
				? []
				: readDesignItems(decisionFile, decisions, 'choice', projectDirectory)),
			...(assumptions === undefined
				? []
				: readDesignItems(assumptionFile, assumptions, 'assumption', projectDirectory)),
		],
	};
	return { design, designFile, designText, sources };
}

/** Type-check only the files the authoring loader has resolved and read. */
export function checkAuthoringTypes(
	entryPoints: readonly string[],
	sources: ReadonlyMap<string, string>,
): void {
	const compilerOptions: typescript.CompilerOptions = {
		noEmit: true,
		strict: true,
		skipLibCheck: true,
		allowImportingTsExtensions: true,
		libReplacement: false,
		target: typescript.ScriptTarget.ES2022,
		module: typescript.ModuleKind.ESNext,
		moduleResolution: typescript.ModuleResolutionKind.Bundler,
		types: [],
	};
	const host = typescript.createCompilerHost(compilerOptions);
	const libraryDirectory = dirname(typescript.getDefaultLibFilePath(compilerOptions));
	const getSourceFile = host.getSourceFile.bind(host);
	host.getSourceFile = (name, languageVersion, onError, shouldCreateNewSourceFile) => {
		const text = sources.get(name);
		if (text !== undefined) return typescript.createSourceFile(name, text, languageVersion, true);
		if (dirname(name) === libraryDirectory && /^lib\..+\.d\.ts$/.test(basename(name)))
			return getSourceFile(name, languageVersion, onError, shouldCreateNewSourceFile);
		return undefined;
	};
	host.resolveModuleNameLiterals = (imports, importer) =>
		imports.map((moduleName) => {
			const path = resolve(dirname(importer), moduleName.text);
			return {
				resolvedModule:
					moduleName.text.startsWith('.') && sources.has(path)
						? {
								resolvedFileName: path,
								extension: path.endsWith('.d.ts')
									? typescript.Extension.Dts
									: typescript.Extension.Ts,
							}
						: undefined,
			};
		});
	host.resolveTypeReferenceDirectiveReferences = (references) =>
		references.map(() => ({ resolvedTypeReferenceDirective: undefined }));
	const program = typescript.createProgram([...entryPoints], compilerOptions, host);
	const diagnostics = typescript.getPreEmitDiagnostics(program);
	if (diagnostics.length)
		throw new Error(
			typescript.formatDiagnosticsWithColorAndContext(diagnostics, {
				getCurrentDirectory: () => process.cwd(),
				getCanonicalFileName: (name) => name,
				getNewLine: () => '\n',
			}),
		);
}
