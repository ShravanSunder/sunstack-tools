import { readdirSync, statSync } from 'node:fs';
import { basename, dirname, join, resolve } from 'node:path';

import { parse, type DefaultTreeAdapterMap } from 'parse5';
import typescript from 'typescript';

import { escapeHtml } from '../../src/core/html-text.ts';
import type { CutPlan } from './cut-plan.ts';
import { initiativeSource, type InitiativeProjectEntry } from './initiative-source.ts';
import type { SourceWorkspace } from './migration-workspace.ts';
import { adoptedFiles, commonParent } from './project-folders.ts';
import {
	applyTextEdits,
	defaultExport,
	parseModule,
	quotedLike,
	type TextEdit,
} from './source-edits.ts';

/**
 * The 1.33 layout: project.ts with `designs`, project-decisions.ts, and shared
 * records named `project/D1`. The move converts it to the initiative layout in
 * the same run; the library's own reader knows only initiative.ts.
 */
export interface LegacyConversion {
	readonly renamed: readonly { readonly from: string; readonly to: string }[];
	readonly references: number;
	readonly files: number;
	/** Projects the plan does not list, with the number they were given. */
	readonly unlisted: readonly { readonly key: string; readonly number: number }[];
}

const legacyAddress = /^project\/([A-Za-z][A-Za-z0-9-]*)$/u;
const legacyFragment = /#project(\/|%2F)/gu;

export function isLegacyLayout(root: string): boolean {
	const present = (name: string): boolean => {
		try {
			return statSync(join(root, name)).isFile();
		} catch {
			return false;
		}
	};
	return present('project.ts') && !present('initiative.ts');
}

function authoredFiles(directory: string): string[] {
	const files: string[] = [];
	for (const entry of readdirSync(directory, { withFileTypes: true })) {
		if (entry.name.startsWith('.') || entry.name === 'node_modules') continue;
		const path = join(directory, entry.name);
		if (entry.isDirectory()) files.push(...authoredFiles(path));
		else if (
			entry.isFile() &&
			!entry.name.includes('.generated.') &&
			!entry.name.endsWith('.d.ts') &&
			/\.(?:ts|html?)$/iu.test(entry.name)
		)
			files.push(path);
	}
	return files;
}

function convertedString(value: string): string {
	const address = legacyAddress.exec(value);
	if (address) return `initiative/${address[1] ?? ''}`;
	return value.replace(legacyFragment, '#initiative$1');
}

function convertTypeScript(file: string, text: string): { text: string; count: number } {
	const source = parseModule(file, text);
	const edits: TextEdit[] = [];
	const visit = (node: typescript.Node): void => {
		if (
			(typescript.isStringLiteral(node) || typescript.isNoSubstitutionTemplateLiteral(node)) &&
			!typescript.isImportDeclaration(node.parent)
		) {
			const value = convertedString(node.text);
			if (value !== node.text) {
				const start = node.getStart(source);
				const quote = text[start] === "'" ? "'" : '"';
				edits.push({ start, end: node.end, text: quotedLike(quote, value, 'value') });
			}
		}
		typescript.forEachChild(node, visit);
	};
	visit(source);
	return { text: applyTextEdits(text, edits), count: edits.length };
}

type PageNode = DefaultTreeAdapterMap['node'];
const recordAttributes = new Set(['data-design-record', 'data-design-status', 'data-design-need']);

function convertHtml(text: string): { text: string; count: number } {
	const edits: TextEdit[] = [];
	const visit = (node: PageNode): void => {
		if ('tagName' in node) {
			const location = node.sourceCodeLocation;
			const overview = node.tagName === 'ex-project-overview';
			if (overview && location?.startTag) {
				const start = location.startTag.startOffset + 1;
				edits.push({ start, end: start + node.tagName.length, text: 'ex-initiative-overview' });
				if (location.endTag) {
					const end = location.endTag.startOffset + 2;
					edits.push({
						start: end,
						end: end + node.tagName.length,
						text: 'ex-initiative-overview',
					});
				}
			}
			for (const { name, value } of node.attrs) {
				const span = location?.attrs?.[name];
				if (!span) continue;
				let converted = value;
				if (recordAttributes.has(name) || (name === 'record' && node.tagName === 'ex-section-item'))
					converted = convertedString(value.trim());
				else if (name === 'href' || name === 'src') {
					converted = value.replace(legacyFragment, '#initiative$1');
					if (overview && basename(converted) === 'project.generated.json')
						converted = `${converted.slice(0, -'project.generated.json'.length)}initiative.generated.json`;
				}
				if (converted !== value)
					edits.push({
						start: span.startOffset,
						end: span.endOffset,
						text: `${name}="${escapeHtml(converted)}"`,
					});
			}
			if (node.tagName === 'template' && 'content' in node) visit(node.content);
		}
		if ('childNodes' in node) node.childNodes.forEach(visit);
	};
	visit(parse(text, { sourceCodeLocationInfo: true }));
	return { text: applyTextEdits(text, edits), count: edits.length };
}

interface LegacyManifest {
	readonly designs: ReadonlyMap<string, string>;
	readonly decisions: string | undefined;
	readonly assumptions: string | undefined;
	readonly identities: string | undefined;
}

function readLegacyManifest(file: string, text: string): LegacyManifest {
	const source = parseModule(file, text);
	const imports = new Map<string, string>();
	for (const statement of source.statements)
		if (
			typescript.isImportDeclaration(statement) &&
			statement.importClause?.name &&
			typescript.isStringLiteral(statement.moduleSpecifier)
		)
			imports.set(
				statement.importClause.name.text,
				resolve(dirname(file), statement.moduleSpecifier.text),
			);
	const exported = defaultExport(source);
	if (!typescript.isObjectLiteralExpression(exported))
		throw new Error(`${file}: expected a literal project object.`);
	const bound = (initializer: typescript.Node | undefined): string | undefined =>
		initializer && typescript.isIdentifier(initializer) ? imports.get(initializer.text) : undefined;
	const designs = new Map<string, string>();
	const fields = new Map<string, string | undefined>();
	for (const property of exported.properties) {
		const name =
			property.name &&
			(typescript.isIdentifier(property.name) || typescript.isStringLiteral(property.name))
				? property.name.text
				: undefined;
		if (!name) continue;
		if (typescript.isShorthandPropertyAssignment(property)) {
			fields.set(name, imports.get(name));
			continue;
		}
		if (!typescript.isPropertyAssignment(property)) continue;
		if (name !== 'designs') {
			fields.set(name, bound(property.initializer));
			continue;
		}
		if (!typescript.isObjectLiteralExpression(property.initializer))
			throw new Error(`${file}: expected designs to be a literal object.`);
		for (const design of property.initializer.properties) {
			const key =
				design.name &&
				(typescript.isIdentifier(design.name) || typescript.isStringLiteral(design.name))
					? design.name.text
					: undefined;
			const path = typescript.isShorthandPropertyAssignment(design)
				? key && imports.get(key)
				: typescript.isPropertyAssignment(design)
					? bound(design.initializer)
					: undefined;
			if (!key || !path) throw new Error(`${file}: each design must name an imported design.ts.`);
			designs.set(key, path);
		}
	}
	return {
		designs,
		decisions: fields.get('decisions'),
		assumptions: fields.get('assumptions'),
		identities: fields.get('identities'),
	};
}

/** Convert a project.ts tree in the workspace; the caller writes it to the working copy. */
export function convertLegacyLayout(
	root: string,
	plan: CutPlan,
	workspace: SourceWorkspace,
): LegacyConversion {
	const manifestFile = join(root, 'project.ts');
	const manifestText = workspace.read(manifestFile);
	if (manifestText === undefined) throw new Error(`${manifestFile} is missing.`);
	const manifest = readLegacyManifest(manifestFile, manifestText);
	const renamed: { from: string; to: string }[] = [];
	const renameShared = (
		file: string | undefined,
		legacyName: string,
		name: string,
	): string | undefined => {
		if (!file || basename(file) !== legacyName) return file;
		const target = join(dirname(file), name);
		renamed.push({ from: file, to: target });
		return target;
	};
	const decisions = renameShared(
		manifest.decisions,
		'project-decisions.ts',
		'initiative-decisions.ts',
	);
	const assumptions = renameShared(
		manifest.assumptions,
		'project-assumptions.ts',
		'initiative-assumptions.ts',
	);

	// Hand-made pages in a folder the move will adopt stay exactly as they are.
	const leftAlone = adoptedFiles(
		commonParent(
			[...manifest.designs.values()].map((design) => dirname(design)),
			root,
		),
		plan.projects.filter(({ key }) => !manifest.designs.has(key)).map(({ folder }) => folder),
		root,
	);
	let references = 0;
	let files = 0;
	for (const file of authoredFiles(root)) {
		if (file === manifestFile || leftAlone.has(file)) continue;
		const text = workspace.read(file);
		if (text === undefined) continue;
		const converted = file.endsWith('.ts') ? convertTypeScript(file, text) : convertHtml(text);
		const destination = renamed.find(({ from }) => from === file)?.to ?? file;
		references += converted.count;
		if (converted.count || destination !== file) files += 1;
		if (destination !== file) workspace.remove(file);
		workspace.write(destination, converted.text);
	}

	const highest = Math.max(...plan.projects.map(({ number }) => number));
	const unlisted: { key: string; number: number }[] = [];
	const projects = [...manifest.designs].map(([key, design]): InitiativeProjectEntry => {
		const planned = plan.projects.find((project) => project.key === key);
		if (!planned) unlisted.push({ key, number: highest });
		return {
			key,
			design,
			number: planned?.number ?? highest,
			achieves: planned?.achieves,
			acceptance: planned?.acceptance,
		};
	});
	workspace.remove(manifestFile);
	workspace.write(
		join(root, 'initiative.ts'),
		initiativeSource({ root, projects, decisions, assumptions, identities: manifest.identities }),
	);
	renamed.push({ from: manifestFile, to: join(root, 'initiative.ts') });
	return { renamed, references, files, unlisted };
}
