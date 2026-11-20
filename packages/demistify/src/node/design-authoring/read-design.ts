import { basename, dirname, join } from 'node:path';

import typescript from 'typescript';

import { designSchema, type DesignCore } from './schema.ts';

function readLiteral(expression: typescript.Expression, source: typescript.SourceFile): unknown {
	if (
		typescript.isAsExpression(expression) ||
		typescript.isSatisfiesExpression(expression) ||
		typescript.isParenthesizedExpression(expression)
	)
		return readLiteral(expression.expression, source);
	if (typescript.isStringLiteral(expression)) return expression.text;
	if (typescript.isNumericLiteral(expression)) return Number(expression.text);
	if (expression.kind === typescript.SyntaxKind.TrueKeyword) return true;
	if (expression.kind === typescript.SyntaxKind.FalseKeyword) return false;
	if (expression.kind === typescript.SyntaxKind.NullKeyword) return null;
	if (typescript.isArrayLiteralExpression(expression))
		return expression.elements.map((element): unknown => readLiteral(element, source));
	if (typescript.isObjectLiteralExpression(expression)) {
		const fields = new Map<string, unknown>();
		for (const property of expression.properties) {
			if (!typescript.isPropertyAssignment(property))
				throw new Error('design.ts object properties must be simple assignments.');
			const { name } = property;
			if (!typescript.isStringLiteral(name) && !typescript.isIdentifier(name))
				throw new Error('design.ts object keys must be identifiers or quoted strings.');
			if (fields.has(name.text)) throw new Error(`Duplicate design.ts object key: ${name.text}`);
			fields.set(name.text, readLiteral(property.initializer, source));
		}
		return Object.fromEntries(fields);
	}
	const { line } = source.getLineAndCharacterOfPosition(expression.getStart(source));
	throw new Error(`design.ts:${line + 1}: expected a data literal.`);
}

/** Read a TypeScript data object. The authoring file is never imported or evaluated. */
export function readDesign(
	file: string,
	text: string,
	declaration: string | undefined,
): DesignCore {
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
		throw new Error('Expected one default export and optional type-only imports in design.ts.');
	const data = readLiteral(entry.expression, source);
	const compilerOptions: typescript.CompilerOptions = {
		noEmit: true,
		strict: true,
		skipLibCheck: true,
		allowImportingTsExtensions: true,
		target: typescript.ScriptTarget.ES2022,
		module: typescript.ModuleKind.ESNext,
		moduleResolution: typescript.ModuleResolutionKind.Bundler,
		types: [],
	};
	const host = typescript.createCompilerHost(compilerOptions);
	const declarationName = 'design-schema.d.ts';
	const declarationFile = join(dirname(file), declarationName);
	const libraryDirectory = dirname(typescript.getDefaultLibFilePath(compilerOptions));
	const getSourceFile = host.getSourceFile.bind(host);
	host.getSourceFile = (name, languageVersion, onError, shouldCreateNewSourceFile) => {
		if (name === file) return source;
		if (name === declarationFile && declaration !== undefined)
			return typescript.createSourceFile(name, declaration, languageVersion, true);
		if (dirname(name) === libraryDirectory && /^lib\..+\.d\.ts$/.test(basename(name)))
			return getSourceFile(name, languageVersion, onError, shouldCreateNewSourceFile);
		return undefined;
	};
	host.resolveModuleNameLiterals = (imports) =>
		imports.map((moduleName) => ({
			resolvedModule:
				moduleName.text === `./${declarationName}` && declaration !== undefined
					? { resolvedFileName: declarationFile, extension: typescript.Extension.Dts }
					: undefined,
		}));
	host.resolveTypeReferenceDirectiveReferences = (references) =>
		references.map(() => ({ resolvedTypeReferenceDirective: undefined }));
	const program = typescript.createProgram([file], compilerOptions, host);
	const diagnostics = typescript.getPreEmitDiagnostics(program);
	if (diagnostics.length)
		throw new Error(
			typescript.formatDiagnostics(diagnostics, {
				getCurrentDirectory: () => process.cwd(),
				getCanonicalFileName: (name) => name,
				getNewLine: () => '\n',
			}),
		);
	return designSchema.parse(data);
}
