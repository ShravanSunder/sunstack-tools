import { readFile, realpath } from 'node:fs/promises';
import { basename, dirname, isAbsolute, relative, resolve, sep } from 'node:path';

import typescript from 'typescript';
import { z } from 'zod';

import { containsPath } from '../local-site.ts';
import { findDesignInput } from './design-location.ts';
import { designDeclarationFilename, readLiteral, unwrapExpression } from './read-design.ts';

const alias = z.string().regex(/^[A-Za-z][A-Za-z0-9-]*$/);
const filePath = z.string().min(1);
const sentence = z.string().trim().min(1);
const initiativeProjectSchema = z.strictObject({
	design: filePath,
	/** The initiative's order: lower first; the same number means side by side. */
	number: z.number().int().positive(),
	/** What the project achieves, in one sentence. */
	achieves: sentence.optional(),
	/** What the owner checks to accept the project. */
	acceptance: sentence.optional(),
});
const manifestSchema = z.strictObject({
	projects: z.record(alias, initiativeProjectSchema).default({}),
	decisions: filePath.optional(),
	assumptions: filePath.optional(),
	identities: filePath.optional(),
});
const identitiesSchema = z.strictObject({
	owner: alias,
	agents: z.record(alias, z.strictObject({ sessionId: z.string().trim().min(1) })),
});

type InitiativeManifest = z.infer<typeof manifestSchema>;
export type InitiativeProject = z.infer<typeof initiativeProjectSchema>;
type InitiativeIdentities = z.infer<typeof identitiesSchema>;

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
		throw new Error(`${file}: import escapes the initiative root.`);
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

export function readInitiativeManifest(file: string, text: string): InitiativeManifest {
	const { source, body, value } = moduleBody(file, text);
	const imports = new Map<string, string>();
	for (const statement of body) {
		if (!typescript.isImportDeclaration(statement))
			throw new Error(`${file}: initiative configuration accepts imports and a literal export.`);
		const clause = statement.importClause;
		if (clause?.isTypeOnly) continue;
		if (!clause?.name || clause.namedBindings)
			throw new Error(`${file}: import each initiative input by its default export.`);
		if (imports.has(clause.name.text))
			throw new Error(`Duplicate initiative import: ${clause.name.text}`);
		imports.set(clause.name.text, importFile(statement, file, dirname(file)));
	}
	const manifest = manifestSchema.parse(readLiteral(value, source, imports));
	const importedFiles = new Set(imports.values());
	const check = (path: string | undefined, names: readonly string[]): void => {
		if (path !== undefined && (!importedFiles.has(path) || !names.includes(basename(path))))
			throw new Error(`${file}: reference an imported ${names.join(' or ')}.`);
	};
	for (const [name, project] of Object.entries(manifest.projects)) {
		if (name === 'initiative')
			throw new Error('The initiative namespace is reserved for shared records.');
		check(project.design, ['design.ts', 'design-delivery.ts']);
	}
	check(manifest.decisions, ['initiative-decisions.ts']);
	check(manifest.assumptions, ['initiative-assumptions.ts']);
	check(manifest.identities, ['agents-identity.ts']);
	return manifest;
}

interface RegisteredProject {
	readonly directory: string;
	readonly manifest: InitiativeManifest;
}

async function registeredManifest(
	root: string,
	directory: string,
): Promise<InitiativeManifest | undefined> {
	const file = resolve(root, 'initiative.ts');
	if (!containsPath(root, await realpath(file))) return undefined;
	const text = await readFile(file, 'utf8');
	let manifest: InitiativeManifest;
	try {
		manifest = readInitiativeManifest(file, text);
	} catch {
		// A neighboring initiative.ts may be application code.
		return undefined;
	}
	const designDirectories = Object.values(manifest.projects).map(({ design }) => dirname(design));
	if (designDirectories.includes(directory)) return manifest;
	const registered = await Promise.all(
		designDirectories.map((designDirectory) =>
			realpath(designDirectory).catch((error: unknown): undefined => {
				if (
					error instanceof Error &&
					'code' in error &&
					['ENOENT', 'ENOTDIR'].includes(String(error.code))
				)
					return undefined;
				throw error;
			}),
		),
	);
	return registered.includes(directory) ? manifest : undefined;
}

export async function findRegisteredInitiative(
	directory: string,
): Promise<RegisteredProject | undefined> {
	for (let start = directory; ;) {
		// oxlint-disable-next-line no-await-in-loop -- the nearest actual registration owns the design.
		const root = await findDesignInput(start, 'initiative.ts');
		if (!root) return undefined;
		// oxlint-disable-next-line no-await-in-loop -- an unrelated declaration may hide a registered ancestor.
		const manifest = await registeredManifest(root, directory);
		if (manifest) return { directory: root, manifest };
		start = dirname(root);
		if (start === root) return undefined;
	}
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

export async function readInitiativeIdentities(
	file: string,
	root: string,
	read: (file: string) => Promise<string>,
): Promise<InitiativeIdentities> {
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
