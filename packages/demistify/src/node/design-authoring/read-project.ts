import { basename, dirname, isAbsolute, relative, resolve, sep } from 'node:path';

import typescript from 'typescript';
import { z } from 'zod';

import { designDeclarationFilename, readLiteral, unwrapExpression } from './read-design.ts';

const alias = z.string().regex(/^[A-Za-z][A-Za-z0-9-]*$/);
const filePath = z.string().min(1);
const manifestSchema = z.strictObject({
	designs: z.record(alias, filePath).default({}),
	decisions: filePath.optional(),
	assumptions: filePath.optional(),
	identities: filePath.optional(),
});
const identitiesSchema = z.strictObject({
	owner: alias,
	agents: z.record(alias, z.strictObject({ sessionId: z.string().trim().min(1) })),
});

type ProjectManifest = z.infer<typeof manifestSchema>;
type ProjectIdentities = z.infer<typeof identitiesSchema>;

function importFile(statement: typescript.ImportDeclaration, file: string, root: string): string {
	const reference = statement.moduleSpecifier;
	if (
		!typescript.isStringLiteral(reference) ||
		!reference.text.startsWith('.') ||
		!reference.text.endsWith('.ts') ||
		reference.text.includes('\\')
	)
		throw new Error(`${file}: expected a relative TypeScript import.`);
	const target = resolve(dirname(file), reference.text);
	const child = relative(root, target);
	if (!child || isAbsolute(child) || child === '..' || child.startsWith(`..${sep}`))
		throw new Error(`${file}: import escapes the project root.`);
	return target;
}

function moduleBody(
	file: string,
	text: string,
): {
	source: typescript.SourceFile;
	body: readonly typescript.Statement[];
	value: typescript.Expression;
} {
	const source = typescript.createSourceFile(file, text, typescript.ScriptTarget.ES2022, true);
	const exports = source.statements.filter(typescript.isExportAssignment);
	const [entry] = exports;
	if (exports.length !== 1 || !entry || entry.isExportEquals)
		throw new Error(`${file}: expected one default export.`);
	return {
		source,
		body: source.statements.filter((statement): boolean => statement !== entry),
		value: entry.expression,
	};
}

export function readProjectManifest(file: string, text: string): ProjectManifest {
	const { source, body, value } = moduleBody(file, text);
	const imports = new Map<string, string>();
	for (const statement of body) {
		if (!typescript.isImportDeclaration(statement))
			throw new Error(`${file}: project configuration accepts imports and a literal export.`);
		const clause = statement.importClause;
		if (clause?.isTypeOnly) continue;
		if (!clause?.name || clause.namedBindings)
			throw new Error(`${file}: import each project input by its default export.`);
		if (imports.has(clause.name.text))
			throw new Error(`Duplicate project import: ${clause.name.text}`);
		imports.set(clause.name.text, importFile(statement, file, dirname(file)));
	}
	const manifest = manifestSchema.parse(readLiteral(value, source, imports));
	const importedFiles = new Set(imports.values());
	const check = (path: string | undefined, names: readonly string[]): void => {
		if (path !== undefined && (!importedFiles.has(path) || !names.includes(basename(path))))
			throw new Error(`${file}: reference an imported ${names.join(' or ')}.`);
	};
	for (const [name, path] of Object.entries(manifest.designs)) {
		if (name === 'project')
			throw new Error('The project namespace is reserved for shared records.');
		check(path, ['design.ts', 'design-delivery.ts']);
	}
	check(manifest.decisions, ['project-decisions.ts']);
	check(manifest.assumptions, ['project-assumptions.ts']);
	check(manifest.identities, ['agents-identity.ts']);
	return manifest;
}

export function projectRecordTypeImports(file: string, text: string, root: string): string[] {
	const { body } = moduleBody(file, text);
	return [
		...new Set(
			body.filter(typescript.isImportDeclaration).map((statement): string => {
				const path = importFile(statement, file, root);
				if (!statement.importClause?.isTypeOnly || basename(path) !== designDeclarationFilename)
					throw new Error(`${file}: use type-only imports from ${designDeclarationFilename}.`);
				return path;
			}),
		),
	];
}

export async function readProjectIdentities(
	file: string,
	root: string,
	read: (file: string) => Promise<string>,
): Promise<ProjectIdentities> {
	const { source, body, value } = moduleBody(file, await read(file));
	const agents = new Map<string, { sessionId: string }>();
	let helper: string | undefined;
	for (const statement of body) {
		if (typescript.isImportDeclaration(statement)) {
			if (statement.importClause?.isTypeOnly) continue;
			const bindings = statement.importClause?.namedBindings;
			const [binding] = bindings && typescript.isNamedImports(bindings) ? bindings.elements : [];
			const path = importFile(statement, file, root);
			if (
				helper ||
				statement.importClause?.name ||
				!bindings ||
				!typescript.isNamedImports(bindings) ||
				bindings.elements.length !== 1 ||
				!binding ||
				binding.isTypeOnly ||
				(binding.propertyName ?? binding.name).text !== 'agentId' ||
				basename(path) !== 'design-sdk.generated.ts'
			)
				throw new Error(`${file}: import agentId from the matching generated design SDK.`);

			helper = binding.name.text;
			continue;
		}
		if (
			!typescript.isVariableStatement(statement) ||
			!(statement.declarationList.flags & typescript.NodeFlags.Const)
		)
			throw new Error(`${file}: declare agent identities as constants.`);
		for (const declaration of statement.declarationList.declarations) {
			const expression = declaration.initializer && unwrapExpression(declaration.initializer);
			const [session] =
				expression && typescript.isCallExpression(expression) ? expression.arguments : [];
			if (
				!helper ||
				!typescript.isIdentifier(declaration.name) ||
				agents.has(declaration.name.text) ||
				!expression ||
				!typescript.isCallExpression(expression) ||
				!typescript.isIdentifier(expression.expression) ||
				expression.expression.text !== helper ||
				expression.arguments.length !== 1 ||
				!session ||
				!typescript.isStringLiteral(session)
			)
				throw new Error(`${file}: use const name = agentId("session id").`);
			agents.set(declaration.name.text, { sessionId: session.text });
		}
	}
	return identitiesSchema.parse(readLiteral(value, source, agents));
}
