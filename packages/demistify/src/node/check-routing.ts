import { readFile, stat } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';

/** A typed tree that `demistify check` runs its own check on. */
export interface TypedCheckTarget {
	readonly kind: 'initiative' | 'design';
	readonly directory: string;
	/** The paths given on the command line that led to this tree. */
	readonly inputs: readonly string[];
}

/** Where each `demistify check` argument goes: the page check, or a typed tree's check. */
export interface CheckRoutes {
	readonly pages: readonly string[];
	readonly trees: readonly TypedCheckTarget[];
}

/** Markup that only a typed check can verify: generated sources and record bindings. */
const typedPageMarkup =
	/(?:navigation|deliverables|initiative|design)\.generated\.json|records\.generated\.js|name="demistify-records"|data-design-(?:record|status|need)=|<ex-section-item\b[^>]*\brecord=/u;

async function isFile(path: string): Promise<boolean> {
	return stat(path).then(
		(entry) => entry.isFile(),
		() => false,
	);
}

/** The tree a typed page belongs to: the enclosing initiative, else the nearest design set. */
async function enclosingTree(file: string): Promise<Omit<TypedCheckTarget, 'inputs'> | undefined> {
	const ancestors: string[] = [];
	for (let directory = dirname(resolve(file)); ; directory = dirname(directory)) {
		ancestors.push(directory);
		if (dirname(directory) === directory) break;
	}
	const markers = await Promise.all(
		ancestors.map(async (directory) => ({
			directory,
			initiative: await isFile(join(directory, 'initiative.ts')),
			design: await isFile(join(directory, 'design.ts')),
		})),
	);
	const initiative = markers.find((marker) => marker.initiative);
	if (initiative) return { kind: 'initiative', directory: initiative.directory };
	const design = markers.find((marker) => marker.design);
	return design ? { kind: 'design', directory: design.directory } : undefined;
}

async function directoryTree(path: string): Promise<Omit<TypedCheckTarget, 'inputs'>> {
	const directory = resolve(path);
	if (await isFile(join(directory, 'initiative.ts'))) return { kind: 'initiative', directory };
	if (await isFile(join(directory, 'design.ts'))) return { kind: 'design', directory };
	throw new Error(`${path} has no initiative.ts or design.ts to check.`);
}

/**
 * Decide what each argument needs. A directory is checked as the initiative or design set
 * it holds. A page that binds generated design data is checked through the tree that owns
 * it, once per tree. Every other page gets the page check.
 */
export async function routeCheckTargets(paths: readonly string[]): Promise<CheckRoutes> {
	const pages: string[] = [];
	const trees = new Map<
		string,
		{ kind: TypedCheckTarget['kind']; directory: string; inputs: string[] }
	>();
	const routed = await Promise.all(
		paths.map(async (path) => {
			const entry = await stat(path).catch(() => undefined);
			if (!entry) throw new Error(`${path}: no such page or folder.`);
			if (entry.isDirectory()) return { path, tree: await directoryTree(path) };
			if (!typedPageMarkup.test(await readFile(path, 'utf8'))) return { path, tree: undefined };
			const tree = await enclosingTree(path);
			if (!tree)
				throw new Error(
					`${path} binds generated design data, but no initiative.ts or design.ts encloses it.`,
				);
			return { path, tree };
		}),
	);
	for (const { path, tree } of routed) {
		if (!tree) {
			pages.push(path);
			continue;
		}
		const known = trees.get(tree.directory) ?? { ...tree, inputs: [] };
		known.inputs.push(path);
		trees.set(tree.directory, known);
	}
	return { pages, trees: [...trees.values()] };
}
