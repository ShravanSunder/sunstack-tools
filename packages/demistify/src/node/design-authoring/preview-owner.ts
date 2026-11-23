import { readFile, readdir, realpath, stat } from 'node:fs/promises';
import { dirname, join } from 'node:path';

import { containsPath } from '../local-site.ts';
import { declaredDesignPagePath, resolveDesignLink } from './design-location.ts';
import { readDesignPages } from './read-design.ts';
import { readProjectManifest } from './read-project.ts';

interface PreviewRequest {
	readonly requestedPage: string;
	readonly physicalPage: string;
	readonly siteRoot: string | undefined;
	readonly sources: readonly string[];
}

interface PreviewClaim {
	readonly kind: 'project' | 'design';
	readonly directory: string;
	readonly pages: ReadonlySet<string>;
}

type DesignPreviewOwner = Omit<PreviewClaim, 'pages'> & { readonly page: string };

function filesystemError(error: unknown, codes: readonly string[]): boolean {
	return error instanceof Error && 'code' in error && codes.includes(String(error.code));
}

async function discoveryRead<TResult>(
	path: string,
	required: boolean,
	read: () => Promise<TResult>,
): Promise<TResult | undefined> {
	try {
		return await read();
	} catch (error) {
		if (filesystemError(error, ['ENOENT', 'ENOTDIR'])) return undefined;
		if (filesystemError(error, ['EACCES', 'EPERM'])) {
			if (!required) return undefined;
			throw new Error(`Cannot read preview ownership location: ${path}.`, { cause: error });
		}
		throw error;
	}
}

function candidateDirectories(page: string, root?: string): string[] {
	const directories: string[] = [];
	for (let directory = dirname(page); ; directory = dirname(directory)) {
		directories.push(directory);
		if (directory === root || directory === dirname(directory)) return directories;
	}
}

async function* fallbackDirectories(root: string, required = true): AsyncGenerator<string> {
	const entries = await discoveryRead(root, required, () => readdir(root, { withFileTypes: true }));
	if (!entries) return;
	if (entries.some(({ name }) => name === 'project.ts' || name === 'design.ts')) yield root;
	for (const entry of entries) {
		if (!entry.isDirectory() || entry.name.startsWith('.') || entry.name === 'node_modules')
			continue;
		yield* fallbackDirectories(join(root, entry.name), false);
	}
}

async function readDeclaration(file: string, required: boolean): Promise<string | undefined> {
	return discoveryRead(file, required, async () => {
		const physical = await realpath(file);
		if (!containsPath(dirname(file), physical) || !(await stat(physical)).isFile())
			return undefined;
		return readFile(physical, 'utf8');
	});
}

async function projectDesignFiles(file: string, required: boolean): Promise<string[] | undefined> {
	const text = await readDeclaration(file, required);
	if (text === undefined) return undefined;
	try {
		return Object.values(readProjectManifest(file, text).designs).map((design): string =>
			join(dirname(design), 'design.ts'),
		);
	} catch {
		// A project.ts may be unrelated application code.
		return undefined;
	}
}

function bindingDirectories(request: PreviewRequest, page: string): Set<string> {
	const { siteRoot, sources } = request;
	const directories = new Set<string>();
	if (!siteRoot) return directories;
	for (const source of sources) {
		try {
			const target = resolveDesignLink(source, page, siteRoot);
			if (target) {
				for (const directory of candidateDirectories(target.file, siteRoot))
					directories.add(directory);
			}
		} catch {
			// These sources are hints until the checker confirms their declared URL base.
		}
	}
	return directories;
}

export async function findDesignPreviewOwner(
	request: PreviewRequest,
): Promise<DesignPreviewOwner | undefined> {
	const { requestedPage, physicalPage, siteRoot } = request;
	const visited = new Map<string, 'required' | 'provisional'>();
	const projects = new Map<string, PreviewClaim>();
	const standalone = new Map<string, PreviewClaim>();
	const registered = new Set<string>();
	const declarations = new Map<string, string[]>();

	async function matchingPages(file: string, root: string, required: boolean): Promise<string[]> {
		const key = `${root}\0${file}`;
		const saved = declarations.get(key);
		if (saved) return saved;
		const text = await readDeclaration(file, required);
		if (text === undefined) return [];
		let authored: ReturnType<typeof readDesignPages>;
		try {
			authored = readDesignPages(file, text);
		} catch {
			return [];
		}
		const matches: string[] = [];
		for (const { href } of authored) {
			let page: string;
			try {
				page = declaredDesignPagePath(href, file, root);
			} catch {
				continue;
			}
			// oxlint-disable-next-line no-await-in-loop -- inspect one declared page identity at a time.
			if ((await discoveryRead(page, false, () => realpath(page))) === physicalPage)
				matches.push(page);
		}
		declarations.set(key, matches);
		return matches;
	}

	async function inspect(directory: string, required: boolean): Promise<void> {
		const canonical = await discoveryRead(directory, required, () => realpath(directory));
		if (!canonical || (siteRoot && !containsPath(siteRoot, canonical))) return;
		const checked = visited.get(canonical);
		if (checked === 'required' || (checked && !required)) return;
		visited.set(canonical, required ? 'required' : 'provisional');
		projects.delete(canonical);
		standalone.delete(canonical);
		const root = siteRoot ?? canonical;
		const project = await projectDesignFiles(join(canonical, 'project.ts'), required);
		if (project) {
			const index = join(canonical, 'index.html');
			const pages = new Set<string>();
			if ((await discoveryRead(index, false, () => realpath(index))) === physicalPage)
				pages.add(index);
			for (const file of project) {
				const milestoneDirectory = dirname(file);
				// oxlint-disable-next-line no-await-in-loop -- project candidates share the same sequential discovery.
				const milestone = await discoveryRead(milestoneDirectory, required, () =>
					realpath(milestoneDirectory),
				);
				if (!milestone || !containsPath(canonical, milestone)) continue;
				const declaration = join(milestone, 'design.ts');
				registered.add(milestone);
				// oxlint-disable-next-line no-await-in-loop -- read each registered declaration in turn.
				for (const page of await matchingPages(declaration, root, required)) pages.add(page);
			}
			if (pages.size) projects.set(canonical, { kind: 'project', directory: canonical, pages });
			if (projects.size > 1) throw competingOwners(physicalPage, [...projects.values()]);
		}
		if (registered.has(canonical)) return;
		const pages = await matchingPages(join(canonical, 'design.ts'), root, required);
		if (pages.length)
			standalone.set(canonical, { kind: 'design', directory: canonical, pages: new Set(pages) });
	}

	for (const directory of candidateDirectories(requestedPage, siteRoot)) {
		// oxlint-disable-next-line no-await-in-loop -- read adjacent declarations before exploring the site.
		await inspect(directory, true);
	}
	for (const directory of bindingDirectories(request, requestedPage)) {
		// oxlint-disable-next-line no-await-in-loop -- a physical path can give bindings the wrong URL base.
		await inspect(directory, false);
	}
	if (siteRoot) {
		for await (const directory of fallbackDirectories(siteRoot)) {
			await inspect(directory, false);
		}
	}
	function selectOwner(): DesignPreviewOwner | undefined {
		const owners = [
			...projects.values(),
			...[...standalone.values()].filter(({ directory }) => !registered.has(directory)),
		];
		if (owners.length > 1) throw competingOwners(physicalPage, owners);
		const [owner] = owners;
		if (!owner) return undefined;
		const { kind, directory, pages } = owner;
		if (pages.has(requestedPage)) return { kind, directory, page: requestedPage };
		const [page] = pages;
		if (!page || pages.size > 1)
			throw new Error(
				`Ambiguous declared preview paths for ${physicalPage}: ${[...pages].toSorted().join(', ')}.`,
			);
		return { kind, directory, page };
	}

	const owner = selectOwner();
	for (const directory of bindingDirectories(request, owner?.page ?? requestedPage)) {
		// oxlint-disable-next-line no-await-in-loop -- enforce access at the selected URL, or the request when no owner is known.
		await inspect(directory, true);
	}
	const checked = selectOwner();
	if (
		checked?.kind !== owner?.kind ||
		checked?.directory !== owner?.directory ||
		checked?.page !== owner?.page
	)
		throw new Error(`Design ownership changed while opening ${requestedPage}.`);
	return owner;
}

function competingOwners(page: string, claims: readonly PreviewClaim[]): Error {
	const owners = claims.map(({ directory }) => directory).toSorted();
	return new Error(`Multiple design owners declare ${page}: ${owners.join(', ')}.`);
}
