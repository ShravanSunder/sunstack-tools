import { readdir } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';

import { parse, type DefaultTreeAdapterMap } from 'parse5';

import { escapeHtml } from '../../src/core/html-text.ts';
import { declaredDesignPagePath } from '../../src/node/design-authoring/design-location.ts';
import { readDesign } from '../../src/node/design-authoring/read-design.ts';
import { readProjectManifest } from '../../src/node/design-authoring/read-project.ts';
import { qualifyRecord } from '../../src/node/design-authoring/schema.ts';
import { containsPath } from '../../src/node/local-site.ts';
import type { SourceReader } from './legacy-records.ts';
import { migrateReleasedChoices } from './released-choices.ts';
import { migrateReleasedSidecar, type ReleasedRecordInput } from './released-records.ts';

function migrateReleasedSections(
	file: string,
	html: string,
): {
	readonly output: string;
	readonly sections: readonly string[];
} {
	const edits: { start: number; end: number; value: string }[] = [];
	const sections: string[] = [];
	function visit(node: DefaultTreeAdapterMap['node'], withinSection = false): void {
		const isSection = 'tagName' in node && node.tagName === 'ex-section';
		if (isSection && withinSection)
			throw new Error(`${file}: nested sections need manual migration.`);
		if (
			isSection &&
			node.childNodes.some(
				(child) => 'tagName' in child || ('value' in child && child.value.trim()),
			)
		) {
			const id = node.attrs.find((attribute) => attribute.name === 'id')?.value;
			const { startTag, endTag } = node.sourceCodeLocation ?? {};
			if (
				!id?.trim() ||
				id !== id.trim() ||
				!startTag ||
				!endTag ||
				node.attrs.some((attribute) => attribute.name === 'holds')
			)
				throw new Error(
					`${file}: a nonempty section needs explicit tags, a stable ID and no holds value.`,
				);
			if (
				node.childNodes.some((child) => 'tagName' in child && child.tagName === 'ex-section-item')
			)
				throw new Error(`${file}: move existing nested section items out of ${id} manually.`);
			const originalContent = html.slice(startTag.endOffset, endTag.startOffset);
			const originalClose = html.slice(endTag.startOffset, endTag.endOffset);
			edits.push({
				start: startTag.endOffset,
				end: endTag.endOffset,
				value: `${originalClose}<ex-section-item section="${escapeHtml(id)}">${originalContent}</ex-section-item>`,
			});
			sections.push(id);
		}
		if ('childNodes' in node)
			for (const child of node.childNodes) visit(child, withinSection || isSection);
	}
	visit(parse(html, { sourceCodeLocationInfo: true }));
	let output = html;
	for (const edit of edits.toReversed())
		output = output.slice(0, edit.start) + edit.value + output.slice(edit.end);
	return { output, sections };
}

async function authoredHtml(directory: string): Promise<string[]> {
	const entries = await readdir(directory, { withFileTypes: true });
	return (
		await Promise.all(
			entries.map(async (entry): Promise<string[]> => {
				const file = join(directory, entry.name);
				if (entry.name.startsWith('.')) return [];
				if (entry.isDirectory()) return authoredHtml(file);
				return entry.isFile() && /\.html?$/iu.test(entry.name) ? [file] : [];
			}),
		)
	).flat();
}

export async function planReleasedProject({
	source,
	project,
	read,
	input,
}: {
	readonly source: string;
	readonly project: string;
	readonly read: SourceReader;
	readonly input: ReleasedRecordInput;
}): Promise<{
	readonly edits: ReadonlyMap<string, string>;
	readonly report: Record<string, unknown>;
	readonly blocked: boolean;
}> {
	const manifestFile = join(project, 'project.ts');
	const originalManifest = await read(manifestFile);
	if (originalManifest === undefined) throw new Error(`Missing project manifest: ${manifestFile}.`);
	const manifest = readProjectManifest(manifestFile, originalManifest);
	const sidecars: { file: string; namespace: string; type: 'decision' | 'assumption' }[] = [];
	if (manifest.decisions)
		sidecars.push({ file: manifest.decisions, namespace: 'project', type: 'decision' });
	if (manifest.assumptions)
		sidecars.push({ file: manifest.assumptions, namespace: 'project', type: 'assumption' });
	const declared = await Promise.all(
		Object.entries(manifest.designs).map(async ([namespace, file]) => {
			const designSource = await read(file);
			if (designSource === undefined) throw new Error(`Missing design: ${file}.`);
			const { design } = await readDesign(file, designSource, async (path) => {
				const imported = await read(path);
				if (imported === undefined) throw new Error(`Missing design input: ${path}.`);
				return imported;
			});
			return {
				namespace,
				directory: dirname(file),
				pages: design.pages.map(({ href }) => declaredDesignPagePath(href, file, source)),
			};
		}),
	);
	for (const { namespace, directory } of declared) {
		sidecars.push(
			{ file: join(directory, 'design-decisions.ts'), namespace, type: 'decision' },
			{ file: join(directory, 'design-assumptions.ts'), namespace, type: 'assumption' },
		);
	}
	const edits = new Map<string, string>();
	const inventory = [];
	const options = new Map<string, readonly { id: string; title: string }[]>();
	const pages = await authoredHtml(project);
	const pagePaths = [...new Set([...pages, ...declared.flatMap((set) => set.pages)])].toSorted();
	for (const file of pagePaths) {
		// oxlint-disable-next-line no-await-in-loop -- parse one source page at a time to bound retained ASTs.
		const text = await read(file);
		if (text === undefined) continue;
		const namespace =
			declared.find((candidate) => containsPath(candidate.directory, file))?.namespace ?? 'project';
		const page = migrateReleasedSections(file, text);
		const choices = migrateReleasedChoices(file, page.output);
		for (const choice of choices.choices) {
			const id = qualifyRecord(namespace, choice.reference);
			if (options.has(id))
				throw new Error(`${file}: duplicate released Choice placement for ${id}.`);
			options.set(id, choice.options);
		}
		if (choices.output === text) continue;
		edits.set(file, choices.output);
		inventory.push({
			file: relative(source, file),
			sections: page.sections,
			...(choices.choices.length && { choices: choices.choices.map((choice) => choice.reference) }),
		});
	}
	const recordModules = await Promise.all(
		sidecars.map(async (sidecar) => {
			const text = await read(sidecar.file);
			if (text === undefined) return undefined;
			return {
				file: sidecar.file,
				text,
				migration: migrateReleasedSidecar({ ...sidecar, source: text, input, options }),
			};
		}),
	);
	const missing = new Map<string, ReleasedRecordInput[string]>();
	const sourceCandidates = new Map<string, readonly unknown[]>();
	const conflicts = [];
	const records: { file: string; id: string; kind: string }[] = [];
	const seen = new Set<string>();
	for (const module of recordModules) {
		if (!module) continue;
		const { output, missing: missingRecords, records: converted } = module.migration;
		for (const [id, candidates] of module.migration.sourceCandidates)
			sourceCandidates.set(id, candidates);
		conflicts.push(
			...module.migration.conflicts.map((conflict) => ({
				file: relative(source, module.file),
				...conflict,
			})),
		);
		for (const id of [
			...missingRecords.map(([key]) => key),
			...converted.map((record) => record.id),
		]) {
			if (seen.has(id)) throw new Error(`Duplicate migration input ID: ${id}.`);
			seen.add(id);
		}
		for (const [id, fields] of missingRecords) missing.set(id, fields);
		for (const record of converted)
			records.push({ file: relative(source, module.file), ...record });
		if (output !== module.text) edits.set(module.file, output);
	}
	for (const id of options.keys())
		if (!seen.has(id)) throw new Error(`No released record matches bound Choice ${id}.`);
	for (const id of Object.keys(input))
		if (!seen.has(id)) throw new Error(`No released record matches --record-input entry ${id}.`);
	return {
		edits,
		blocked: missing.size > 0,
		report: {
			migration: 'released-lean',
			inspectedPages: pagePaths.length,
			records,
			pages: inventory,
			missingCount: missing.size,
			recordInput: Object.fromEntries(missing),
			...(sourceCandidates.size && { sourceCandidates: Object.fromEntries(sourceCandidates) }),
			...(conflicts.length && { conflicts }),
		},
	};
}
