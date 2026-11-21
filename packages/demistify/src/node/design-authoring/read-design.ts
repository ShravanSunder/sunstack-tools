import { basename, dirname, resolve } from 'node:path';

import typescript from 'typescript';
import { z } from 'zod';

import { assumptionRecord, authoredDesignSchema, choiceRecord } from './schema.ts';
import type { AuthoredDesign, DesignRecord } from './schema.ts';

export const designDeclarationFilename = 'design-schema.d.ts';

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
	if (typescript.isObjectLiteralExpression(expression)) {
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
			fields.set(name.text, readLiteral(property.initializer, source, bindings));
		}
		return Object.fromEntries(fields);
	}
	const { line } = source.getLineAndCharacterOfPosition(expression.getStart(source));
	throw new Error(
		`${source.fileName}:${line + 1}: expected a data literal or an imported deliverable.`,
	);
}

function readDefaultLiteral(file: string, text: string): unknown {
	const source = typescript.createSourceFile(file, text, typescript.ScriptTarget.ES2022, true);
	const exportAssignment = source.statements.filter(typescript.isExportAssignment);
	const [entry] = exportAssignment;
	if (
		exportAssignment.length !== 1 ||
		!entry ||
		entry.isExportEquals ||
		source.statements.some(
			(statement): boolean =>
				statement !== entry &&
				!(typescript.isImportDeclaration(statement) && statement.importClause?.isTypeOnly),
		)
	)
		throw new Error(`${file}: expected one default export and optional type-only imports.`);
	return readLiteral(entry.expression, source);
}

/** Read the authored design without evaluating its TypeScript. */
export function readDesign(file: string, text: string): AuthoredDesign {
	return authoredDesignSchema.parse(readDefaultLiteral(file, text));
}

/** Read one literal decision or assumption file. */
export function readDesignItems(
	file: string,
	text: string,
	kind: 'choice' | 'assumption',
): DesignRecord[] {
	const input = readDefaultLiteral(file, text);
	return kind === 'choice'
		? z.array(choiceRecord).parse(input)
		: z.array(assumptionRecord).parse(input);
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
