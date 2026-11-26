import { realpath, stat } from 'node:fs/promises';
import { dirname, join, relative, resolve } from 'node:path';

import { buildDesignSet } from './check-design.ts';
import { buildInitiative } from './check-initiative.ts';
import { readDesignSources } from './read-design.ts';
import { readInitiativeManifest } from './read-initiative.ts';
import { readOptionalText, readText } from './record-edits.ts';
import type { RecordPlacement } from './record-projection.ts';
import type { DesignRecord } from './schema.ts';

/** Which record file a new record of each kind goes in. */
export type RecordField = 'decisions' | 'assumptions';

/**
 * A typed tree whose records `answer` and `add` edit in place: an initiative, where records
 * are qualified by namespace (`initiative/D1`, `messages/A2`), or a standalone design set,
 * where they are local (`D2`).
 */
export interface EditableTree {
	readonly kind: 'initiative' | 'design';
	readonly root: string;
	readonly records: readonly DesignRecord[];
	/** Record ID, as the tree's check knows it, to the page and fragment that place it. */
	readonly placements: ReadonlyMap<string, RecordPlacement>;
	/** Split an ID into its namespace (none in a design set) and its local ID. */
	readonly split: (id: string) => { readonly namespace?: string; readonly localId: string };
	/** The ID a new record gets in a namespace (none in a design set). */
	readonly qualify: (namespace: string | undefined, localId: string) => string;
	/** The files that can hold a namespace's record literals, and the folder their links resolve from. */
	readonly recordFiles: (
		namespace: string | undefined,
	) => Promise<{ readonly base: string; readonly files: readonly string[] }>;
	/** The file a new record goes in, the folder its links resolve from, and the folder its page sits in. */
	readonly newRecordFile: (
		namespace: string | undefined,
		field: RecordField,
	) => Promise<{ readonly file: string; readonly base: string; readonly localDirectory?: string }>;
	/** Run the tree's own check with `--write`, returning its report. */
	readonly checkAndWrite: () => Promise<string>;
}

async function isFile(path: string): Promise<boolean> {
	return stat(path).then(
		(entry) => entry.isFile(),
		() => false,
	);
}

/** An initiative record lives in a namespace: `initiative` or a project name. */
function required(namespace: string | undefined): string {
	if (!namespace)
		throw new Error('An initiative record needs --namespace initiative, or a project name.');
	return namespace;
}

/** A standalone design set's records are local, so it takes no namespace. */
function local(namespace: string | undefined): void {
	if (namespace)
		throw new Error(
			`A standalone design set has no namespaces; leave out --namespace ${namespace}.`,
		);
}

async function existingRecordFile(root: string, file: string): Promise<string> {
	if ((await readOptionalText(file)) === undefined)
		throw new Error(
			`${relative(root, file)} does not exist. Create it with \`export default [];\`, or add the record by hand to another record file.`,
		);
	return file;
}

async function openInitiative(root: string, siteRoot: string | undefined): Promise<EditableTree> {
	const build = await buildInitiative(root, { siteRoot });
	const initiativeFile = join(root, 'initiative.ts');
	const manifest = async (): Promise<ReturnType<typeof readInitiativeManifest>> =>
		readInitiativeManifest(initiativeFile, await readText(initiativeFile));
	const projectDirectory = (namespace: string): string => {
		const directory = [...build.projects].find(([, name]) => name === namespace)?.[0];
		if (!directory) throw new Error(`No project named ${namespace} in this initiative.`);
		return directory;
	};
	return {
		kind: 'initiative',
		root,
		records: build.design.records,
		placements: build.placements,
		split: (id) => {
			const slash = id.indexOf('/');
			if (slash === -1)
				throw new Error(`${id}: expected a qualified record ID such as initiative/D1.`);
			return { namespace: id.slice(0, slash), localId: id.slice(slash + 1) };
		},
		qualify: (namespace, localId) => `${required(namespace)}/${localId}`,
		recordFiles: async (namespace) => {
			if (required(namespace) === 'initiative') {
				const { decisions } = await manifest();
				// Shared records resolve their links from the decisions file, as the check does.
				return decisions
					? { base: dirname(decisions), files: [decisions] }
					: { base: root, files: [] };
			}
			const directory = projectDirectory(required(namespace));
			const { designFile, sources } = await readDesignSources(
				directory,
				readText,
				readOptionalText,
			);
			return { base: directory, files: [designFile, ...sources.keys()] };
		},
		newRecordFile: async (namespace, field) => {
			if (required(namespace) === 'initiative') {
				const file = (await manifest())[field];
				if (!file) throw new Error(`initiative.ts registers no ${field} file; add one first.`);
				return { file, base: dirname(file) };
			}
			const directory = projectDirectory(required(namespace));
			const file = await existingRecordFile(root, join(directory, `design-${field}.ts`));
			return { file, base: directory, localDirectory: directory };
		},
		checkAndWrite: async () => (await buildInitiative(root, { siteRoot, write: true })).result,
	};
}

async function openDesignSet(root: string, siteRoot: string | undefined): Promise<EditableTree> {
	const build = await buildDesignSet(root, { siteRoot });
	return {
		kind: 'design',
		root,
		records: build.design.records,
		placements: build.placements,
		split: (id) => {
			if (id.includes('/'))
				throw new Error(`${id}: a standalone design set uses local record IDs such as D1.`);
			return { localId: id };
		},
		qualify: (namespace, localId) => {
			local(namespace);
			return localId;
		},
		recordFiles: async (namespace) => {
			local(namespace);
			const { designFile, sources } = await readDesignSources(root, readText, readOptionalText);
			return { base: root, files: [designFile, ...sources.keys()] };
		},
		newRecordFile: async (namespace, field) => {
			local(namespace);
			const file = await existingRecordFile(root, join(root, `design-${field}.ts`));
			return { file, base: root, localDirectory: root };
		},
		checkAndWrite: async () => (await buildDesignSet(root, { siteRoot, write: true })).result,
	};
}

/** Open the initiative or standalone design set at `directory` for editing its records. */
export async function openEditableTree(
	directory: string,
	siteRoot: string | undefined,
): Promise<EditableTree> {
	const root = await realpath(resolve(directory));
	if (await isFile(join(root, 'initiative.ts'))) return openInitiative(root, siteRoot);
	if (await isFile(join(root, 'design.ts'))) return openDesignSet(root, siteRoot);
	throw new Error(`${directory} has no initiative.ts or design.ts.`);
}
