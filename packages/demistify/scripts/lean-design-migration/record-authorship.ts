import { dirname, join } from 'node:path';

import { parse, type DefaultTreeAdapterMap } from 'parse5';
import typescript from 'typescript';

import { decisionActorField, type recordActor } from '../../src/core/decision-record.ts';
import { declaredDesignPagePath } from '../../src/node/design-authoring/design-location.ts';
import { readDesign } from '../../src/node/design-authoring/read-design.ts';
import { readProjectManifest } from '../../src/node/design-authoring/read-project.ts';
import { containsPath } from '../../src/node/local-site.ts';
import type { SourceReader } from './legacy-records.ts';

export type RecordActor = ReturnType<typeof recordActor.parse>;

interface SourceChange {
	readonly start: number;
	readonly end: number;
	readonly replacement: string;
}

function applyChanges(source: string, changes: readonly SourceChange[]): string {
	let result = source;
	for (const { start, end, replacement } of changes.toSorted(
		(left, right) => right.start - left.start,
	))
		result = result.slice(0, start) + replacement + result.slice(end);
	return result;
}

function propertyInitializer(
	record: typescript.ObjectLiteralExpression,
	key: string,
): typescript.Expression | undefined {
	for (const property of record.properties) {
		if (!typescript.isPropertyAssignment(property)) continue;
		if (
			(typescript.isIdentifier(property.name) || typescript.isStringLiteral(property.name)) &&
			property.name.text === key
		)
			return property.initializer;
	}
	return undefined;
}

function migrateRecordModule(
	file: string,
	source: string,
	{
		namespace,
		actors,
	}: { readonly namespace: string; readonly actors: ReadonlyMap<string, RecordActor> },
): string {
	const parsed = typescript.createSourceFile(file, source, typescript.ScriptTarget.ES2022, true);
	const assumptions = new Set<string>();
	const choices = new Set<string>();
	const changes: SourceChange[] = [];
	function replace(node: typescript.Node, replacement: string): void {
		changes.push({ start: node.getStart(parsed), end: node.getEnd(), replacement });
	}
	for (const statement of parsed.statements) {
		if (
			!typescript.isImportDeclaration(statement) ||
			!typescript.isStringLiteral(statement.moduleSpecifier) ||
			!statement.moduleSpecifier.text.endsWith('/design-sdk.generated.ts')
		)
			continue;
		const named = statement.importClause?.namedBindings;
		if (!named || !typescript.isNamedImports(named)) continue;
		for (const entry of named.elements) {
			const imported = entry.propertyName ?? entry.name;
			if (imported.text === 'defineAssumption') assumptions.add(entry.name.text);
			if (imported.text !== 'defineDecision') continue;
			replace(imported, 'defineChoice');
			if (!entry.propertyName) choices.add(entry.name.text);
		}
	}

	function visit(node: typescript.Node): void {
		if (
			typescript.isCallExpression(node) &&
			typescript.isIdentifier(node.expression) &&
			choices.has(node.expression.text)
		)
			replace(node.expression, 'defineChoice');
		if (typescript.isObjectLiteralExpression(node)) {
			const kind = propertyInitializer(node, 'kind');
			const parent = node.parent;
			const isAssumptionCall =
				typescript.isCallExpression(parent) &&
				typescript.isIdentifier(parent.expression) &&
				assumptions.has(parent.expression.text);
			if (
				isAssumptionCall ||
				(kind && typescript.isStringLiteral(kind) && kind.text === 'assumption')
			) {
				const author = propertyInitializer(node, decisionActorField);
				if (author && typescript.isStringLiteral(author)) {
					const id = propertyInitializer(node, 'id');
					const reference =
						id && typescript.isStringLiteral(id) ? `${namespace}/${id.text}` : undefined;
					const formerAlias = author.text;
					const actor = reference ? actors.get(reference) : undefined;
					if (!actor || actor.alias !== formerAlias)
						throw new Error(
							`${file}: supply the existing alias and an explicit role in --record-input for ${reference ?? 'this assumption'}.`,
						);
					replace(author, JSON.stringify(actor));
				}
			}
		}
		typescript.forEachChild(node, visit);
	}
	visit(parsed);
	return applyChanges(source, changes);
}

function migrateOptionDetails(source: string): string {
	const changes: SourceChange[] = [];
	type PageNode = DefaultTreeAdapterMap['node'];
	function visit(node: PageNode): void {
		if (
			'tagName' in node &&
			node.tagName === 'ex-option' &&
			node.attrs.some((attribute) => attribute.name === 'data-option')
		) {
			for (const child of node.childNodes) {
				if (!('tagName' in child) || child.tagName === 'template') continue;
				const slot = child.attrs.find(
					(attribute) => attribute.name === 'slot' && attribute.value === 'details',
				);
				if (!slot) continue;
				const location = child.sourceCodeLocation;
				const slotLocation = location?.attrs?.[slot.name];
				if (!location?.startTag || !slotLocation || !location.endTag)
					throw new Error('Record option Details must have an explicit opening and closing tag.');
				const replacement =
					child.tagName === 'div' && child.attrs.length === 1
						? source.slice(location.startTag.endOffset, location.endTag.startOffset)
						: source.slice(location.startOffset, slotLocation.startOffset) +
							source.slice(slotLocation.endOffset, location.endOffset);
				changes.push({
					start: location.startOffset,
					end: location.endOffset,
					replacement: `<template slot="details">${replacement}</template>`,
				});
			}
		}
		if ('childNodes' in node) node.childNodes.forEach(visit);
	}
	visit(parse(source, { sourceCodeLocationInfo: true }));
	return applyChanges(source, changes);
}

export async function planRecordAuthorship({
	source,
	project,
	read,
	actors,
}: {
	readonly source: string;
	readonly project: string;
	readonly read: SourceReader;
	readonly actors: ReadonlyMap<string, RecordActor>;
}): Promise<{
	readonly edits: ReadonlyMap<string, string>;
	readonly report: Record<string, unknown>;
}> {
	const projectFile = join(project, 'project.ts');
	const manifestText = await read(projectFile);
	if (manifestText === undefined) throw new Error(`Missing migration input: ${projectFile}.`);
	const manifest = readProjectManifest(projectFile, manifestText);
	const edits = new Map<string, string>();
	const pages = new Set([join(project, 'index.html')]);
	async function upgrade(file: string, namespace: string): Promise<string | undefined> {
		const original = await read(file);
		if (original === undefined) return undefined;
		const next = migrateRecordModule(file, original, { namespace, actors });
		if (next !== original) edits.set(file, next);
		return next;
	}
	await Promise.all(
		Object.entries(manifest.designs).map(async ([namespace, registered]) => {
			const directory = dirname(registered);
			const designFile = join(directory, 'design.ts');
			const designSource = await upgrade(designFile, namespace);
			if (designSource === undefined) throw new Error(`Missing migration input: ${designFile}.`);
			const { design } = await readDesign(designFile, designSource, async (file) => {
				if (!containsPath(directory, file))
					throw new Error(`Record import leaves its design set: ${file}.`);
				const next = await upgrade(file, namespace);
				if (next === undefined) throw new Error(`Missing migration input: ${file}.`);
				return next;
			});
			for (const { href } of design.pages)
				pages.add(declaredDesignPagePath(href, designFile, source));
			await Promise.all(
				['design-decisions.ts', 'design-assumptions.ts'].map((name) =>
					upgrade(join(directory, name), namespace),
				),
			);
		}),
	);
	await Promise.all(
		[manifest.decisions, manifest.assumptions]
			.filter((file) => file !== undefined)
			.map((file) => upgrade(file, 'project')),
	);
	await Promise.all(
		[...pages].map(async (file) => {
			const original = await read(file);
			if (original === undefined) return;
			const next = migrateOptionDetails(original);
			if (next !== original) edits.set(file, next);
		}),
	);
	return { edits, report: { migration: 'record-authorship', inspectedPages: pages.size } };
}
