import { dirname, resolve } from 'node:path';

import typescript from 'typescript';

import { unwrapExpression } from '../read-design.ts';

/**
 * Surgical edits to authored TypeScript literals. The re-cut changes strings, removes
 * and appends array elements and adds imports; everything else, comments included,
 * keeps the author's text.
 */
export interface TextEdit {
	readonly start: number;
	readonly end: number;
	readonly text: string;
}

export function applyTextEdits(text: string, edits: readonly TextEdit[]): string {
	const ordered = edits.toSorted((left, right) => left.start - right.start || left.end - right.end);
	let output = '';
	let cursor = 0;
	for (const edit of ordered) {
		if (edit.start < cursor) throw new Error('Overlapping source edits.');
		output += text.slice(cursor, edit.start) + edit.text;
		cursor = edit.end;
	}
	return output + text.slice(cursor);
}

export function parseModule(file: string, text: string): typescript.SourceFile {
	return typescript.createSourceFile(file, text, typescript.ScriptTarget.ES2022, true);
}

export function defaultExport(source: typescript.SourceFile): typescript.Expression {
	const exported = source.statements.find(typescript.isExportAssignment);
	if (!exported) throw new Error(`${source.fileName}: expected a default export.`);
	return unwrapExpression(exported.expression);
}

/** The single literal argument of a helper call, or the literal itself. */
export function helperArgument(expression: typescript.Expression): typescript.Expression {
	const value = unwrapExpression(expression);
	if (!typescript.isCallExpression(value)) return value;
	const [argument] = value.arguments;
	if (!argument) throw new Error('Expected a helper call with one argument.');
	return unwrapExpression(argument);
}

export function propertyValue(
	object: typescript.Expression,
	name: string,
): typescript.Expression | undefined {
	if (!typescript.isObjectLiteralExpression(object)) return undefined;
	for (const property of object.properties) {
		if (
			typescript.isPropertyAssignment(property) &&
			(typescript.isIdentifier(property.name) || typescript.isStringLiteral(property.name)) &&
			property.name.text === name
		)
			return unwrapExpression(property.initializer);
	}
	return undefined;
}

export function stringProperty(object: typescript.Expression, name: string): string | undefined {
	const value = propertyValue(object, name);
	return value && typescript.isStringLiteral(value) ? value.text : undefined;
}

export type LiteralPath = readonly (string | number)[];

export interface StringSite {
	readonly path: LiteralPath;
	readonly value: string;
	readonly start: number;
	readonly end: number;
	/** A key is an object property name; a value is a string literal. */
	readonly role: 'key' | 'value';
	readonly quote: '"' | "'" | 'bare';
}

/** Every string value and property name inside a literal, with its path from the root. */
export function stringSites(
	root: typescript.Expression,
	source: typescript.SourceFile,
): StringSite[] {
	const sites: StringSite[] = [];
	const text = source.text;
	const quoteOf = (node: typescript.Node): StringSite['quote'] => {
		const first = text[node.getStart(source)];
		return first === "'" ? "'" : first === '"' ? '"' : 'bare';
	};
	function visit(input: typescript.Expression, path: LiteralPath): void {
		const node = unwrapExpression(input);
		if (typescript.isStringLiteral(node) || typescript.isNoSubstitutionTemplateLiteral(node)) {
			sites.push({
				path,
				value: node.text,
				start: node.getStart(source),
				end: node.end,
				role: 'value',
				quote: quoteOf(node),
			});
			return;
		}
		if (typescript.isArrayLiteralExpression(node)) {
			node.elements.forEach((element, index): void => visit(element, [...path, index]));
			return;
		}
		if (typescript.isObjectLiteralExpression(node)) {
			for (const property of node.properties) {
				if (!typescript.isPropertyAssignment(property)) continue;
				const { name } = property;
				if (!typescript.isIdentifier(name) && !typescript.isStringLiteral(name)) continue;
				sites.push({
					path: [...path, name.text],
					value: name.text,
					start: name.getStart(source),
					end: name.end,
					role: 'key',
					quote: quoteOf(name),
				});
				visit(property.initializer, [...path, name.text]);
			}
		}
	}
	visit(root, []);
	return sites;
}

/** `*` matches one segment, `**` any number of segments. */
export function matchesPath(path: LiteralPath, pattern: readonly string[]): boolean {
	const match = (pathIndex: number, patternIndex: number): boolean => {
		if (patternIndex === pattern.length) return pathIndex === path.length;
		const segment = pattern[patternIndex];
		if (segment === '**')
			return (
				match(pathIndex, patternIndex + 1) ||
				(pathIndex < path.length && match(pathIndex + 1, patternIndex))
			);
		if (pathIndex === path.length) return false;
		return (
			(segment === '*' || segment === String(path[pathIndex])) &&
			match(pathIndex + 1, patternIndex + 1)
		);
	};
	return match(0, 0);
}

const identifierPattern = /^[A-Za-z_$][A-Za-z0-9_$]*$/;

export function quotedLike(
	quote: StringSite['quote'],
	value: string,
	role: StringSite['role'],
): string {
	if (role === 'key' && quote === 'bare' && identifierPattern.test(value)) return value;
	if (quote === "'")
		return `'${value.replace(/\\/gu, '\\\\').replace(/'/gu, "\\'").replace(/\n/gu, '\\n')}'`;
	return JSON.stringify(value);
}

export function replaceSite(site: StringSite, value: string): TextEdit | undefined {
	if (site.value === value) return undefined;
	return { start: site.start, end: site.end, text: quotedLike(site.quote, value, site.role) };
}

/** A node's text, leading comments included, with edits inside it applied. */
export function nodeText(
	text: string,
	node: typescript.Node,
	edits: readonly TextEdit[],
	{ withLeadingTrivia }: { readonly withLeadingTrivia: boolean },
): string {
	const start = withLeadingTrivia ? node.getFullStart() : node.getStart();
	const inside = edits
		.filter((edit) => edit.start >= start && edit.end <= node.end)
		.map((edit) => ({ text: edit.text, start: edit.start - start, end: edit.end - start }));
	return applyTextEdits(text.slice(start, node.end), inside);
}

function lineIndent(text: string, position: number): string {
	const lineStart = text.lastIndexOf('\n', position - 1) + 1;
	return /^[ \t]*/.exec(text.slice(lineStart))?.[0] ?? '';
}

/** Element text with its comments, re-indented to start on its own line. */
function placedElement(elementText: string, indent: string): string {
	const trimmed = elementText.replace(/^\s+/u, '');
	return `\n${indent}${trimmed}`;
}

/**
 * Rewrite an array's elements: keep the chosen ones exactly as written, then append
 * new element texts on their own lines.
 */
export function rebuildArray(
	text: string,
	array: typescript.ArrayLiteralExpression,
	keep: (element: typescript.Expression) => boolean,
	appended: readonly string[],
): TextEdit {
	const elements = [...array.elements];
	const kept = elements.filter(keep);
	if (kept.length === elements.length && appended.length === 0)
		return {
			start: array.getStart(),
			end: array.end,
			text: text.slice(array.getStart(), array.end),
		};
	const arrayIndent = lineIndent(text, array.getStart());
	const [first] = elements;
	const indentUnit = /^\t/mu.test(text) ? '\t' : '  ';
	const indent = first ? lineIndent(text, first.getStart()) : `${arrayIndent}${indentUnit}`;
	const last = elements.at(-1);
	const keptTexts = kept.map((element) => text.slice(element.getFullStart(), element.end));
	const parts = [...keptTexts, ...appended.map((element) => placedElement(element, indent))];
	if (parts.length === 0) return { start: array.getStart(), end: array.end, text: '[]' };
	let closing = last ? text.slice(last.end, array.end - 1) : `,\n${arrayIndent}`;
	if (appended.length > 0 && !closing.includes('\n'))
		closing = `${closing.replace(/\s*$/u, '')}\n${arrayIndent}`;
	return { start: array.getStart(), end: array.end, text: `[${parts.join(',')}${closing}]` };
}

function importSpecifierPath(statement: typescript.ImportDeclaration, file: string): string {
	return typescript.isStringLiteral(statement.moduleSpecifier)
		? resolve(dirname(file), statement.moduleSpecifier.text)
		: '';
}

/** The local name a module gives to a named export of another file, if imported. */
export function namedImportBinding(
	source: typescript.SourceFile,
	moduleFile: string,
	exportedName: string,
): string | undefined {
	for (const statement of source.statements) {
		if (!typescript.isImportDeclaration(statement) || statement.importClause?.isTypeOnly) continue;
		if (importSpecifierPath(statement, source.fileName) !== moduleFile) continue;
		const named = statement.importClause?.namedBindings;
		if (!named || !typescript.isNamedImports(named)) continue;
		for (const element of named.elements)
			if (!element.isTypeOnly && (element.propertyName ?? element.name).text === exportedName)
				return element.name.text;
	}
	return undefined;
}

/** Ensure `exportedName` is imported from `moduleFile`; returns the text and the local name. */
export function ensureNamedImport(
	file: string,
	text: string,
	moduleFile: string,
	specifier: string,
	exportedName: string,
): { readonly text: string; readonly binding: string } {
	const source = parseModule(file, text);
	const existing = namedImportBinding(source, moduleFile, exportedName);
	if (existing) return { text, binding: existing };
	for (const statement of source.statements) {
		if (!typescript.isImportDeclaration(statement) || statement.importClause?.isTypeOnly) continue;
		if (importSpecifierPath(statement, file) !== moduleFile) continue;
		const named = statement.importClause?.namedBindings;
		if (!named || !typescript.isNamedImports(named)) continue;
		const lastElement = named.elements.at(-1);
		const position = lastElement ? lastElement.end : named.getStart() + 1;
		const separator = lastElement ? ', ' : ' ';
		return {
			text: applyTextEdits(text, [
				{ start: position, end: position, text: `${separator}${exportedName}` },
			]),
			binding: exportedName,
		};
	}
	return {
		text: addImportLine(source, text, `import { ${exportedName} } from '${specifier}';`),
		binding: exportedName,
	};
}

export function addImportLine(source: typescript.SourceFile, text: string, line: string): string {
	const imports = source.statements.filter(typescript.isImportDeclaration);
	const last = imports.at(-1);
	if (last) return applyTextEdits(text, [{ start: last.end, end: last.end, text: `\n${line}` }]);
	return `${line}\n\n${text.replace(/^\s+/u, '')}`;
}

/** Remove default-import statements by their local binding. */
export function removeDefaultImports(
	source: typescript.SourceFile,
	bindings: ReadonlySet<string>,
): TextEdit[] {
	const text = source.text;
	return source.statements.flatMap((statement): TextEdit[] => {
		if (!typescript.isImportDeclaration(statement)) return [];
		const name = statement.importClause?.name?.text;
		if (!name || !bindings.has(name)) return [];
		const start = statement.getFullStart();
		const lineBreak = text.startsWith('\n', start) ? 1 : 0;
		return [
			{
				start: start + lineBreak,
				end: statement.end + (text.startsWith('\n', statement.end) ? 1 : 0),
				text: '',
			},
		];
	});
}

/** Insert `name: value` as the first property of an object literal. */
export function insertFirstProperty(
	text: string,
	object: typescript.ObjectLiteralExpression,
	propertyText: string,
): TextEdit {
	const [first] = object.properties;
	if (!first)
		return { start: object.getStart() + 1, end: object.getStart() + 1, text: ` ${propertyText} ` };
	const indent = lineIndent(text, first.getStart());
	return { start: first.getStart(), end: first.getStart(), text: `${propertyText},\n${indent}` };
}

export function isIdentifierName(value: string): boolean {
	return identifierPattern.test(value);
}
