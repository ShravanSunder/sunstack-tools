import { readdirSync, statSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';

/**
 * What a planned project's folder is before the run: absent (the run creates it),
 * present without design.ts (the run adopts it and leaves its files alone), or
 * present with design files the run would overwrite (refused).
 */
export type PlannedFolder =
	| { readonly kind: 'new' }
	| { readonly kind: 'adopt'; readonly leftAlone: readonly string[] }
	| { readonly kind: 'refuse'; readonly reason: string };

/** The page the run writes in an adopted folder; the folder's own pages stay unregistered. */
export const adoptedPageName = 'moved-records.html';

const typedFiles = ['design-decisions.ts', 'design-assumptions.ts', 'design-delivery.ts'];

function isFile(path: string): boolean {
	try {
		return statSync(path).isFile();
	} catch {
		return false;
	}
}

function isDirectory(path: string): boolean {
	try {
		return statSync(path).isDirectory();
	} catch {
		return false;
	}
}

function filesUnder(directory: string): string[] {
	return readdirSync(directory, { withFileTypes: true }).flatMap((entry): string[] => {
		const path = join(directory, entry.name);
		if (entry.isDirectory()) return filesUnder(path);
		return entry.isFile() ? [path] : [];
	});
}

export function plannedFolder(directory: string, label: string): PlannedFolder {
	if (!isDirectory(directory)) {
		if (isFile(directory)) return { kind: 'refuse', reason: `${label} is a file.` };
		return { kind: 'new' };
	}
	if (isFile(join(directory, 'design.ts')))
		return { kind: 'refuse', reason: `${label} already exists and is not a registered project.` };
	const clash = [...typedFiles, adoptedPageName].find((name) => isFile(join(directory, name)));
	if (clash)
		return {
			kind: 'refuse',
			reason: `${label} has no design.ts but already has ${clash}, which adopting it would overwrite.`,
		};
	return { kind: 'adopt', leftAlone: filesUnder(directory) };
}

/** The folder new projects go in: the one parent all registered projects share, or the root. */
export function commonParent(directories: readonly string[], fallback: string): string {
	const parents = [...new Set(directories.map((directory) => dirname(directory)))];
	return parents.length === 1 && parents[0] ? parents[0] : fallback;
}

/** Files of every adoptable planned folder; the run must not change any of them. */
export function adoptedFiles(
	parent: string,
	folders: readonly string[],
	root: string,
): ReadonlySet<string> {
	const files = new Set<string>();
	for (const folder of folders) {
		const state = plannedFolder(join(parent, folder), relative(root, join(parent, folder)));
		if (state.kind === 'adopt') for (const file of state.leftAlone) files.add(file);
	}
	return files;
}
