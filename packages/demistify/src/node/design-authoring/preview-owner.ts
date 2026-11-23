import { readFile, readdir, realpath } from 'node:fs/promises';
import { basename, dirname, isAbsolute, join, resolve } from 'node:path';

import { containsPath } from '../local-site.ts';
import { readDesignPages } from './read-design.ts';
import { readProjectManifest } from './read-project.ts';

interface DesignPreviewOwner {
	readonly kind: 'project' | 'design';
	readonly directory: string;
}

async function declarationsInSite(root: string): Promise<string[]> {
	const entries = await readdir(root, { withFileTypes: true });
	const files = entries.flatMap((entry): string[] =>
		entry.isFile() && (entry.name === 'project.ts' || entry.name === 'design.ts')
			? [join(root, entry.name)]
			: [],
	);
	for (const entry of entries) {
		if (!entry.isDirectory() || entry.name.startsWith('.') || entry.name === 'node_modules')
			continue;
		// oxlint-disable-next-line no-await-in-loop -- bound filesystem concurrency to the current directory.
		files.push(...(await declarationsInSite(join(root, entry.name))));
	}
	return files;
}

function ancestorDeclarations(directory: string): string[] {
	const candidates: string[] = [];
	for (let current = directory; ; current = dirname(current)) {
		candidates.push(join(current, 'project.ts'), join(current, 'design.ts'));
		if (current === dirname(current)) return candidates;
	}
}

async function readDeclaration(file: string): Promise<string | undefined> {
	try {
		const physical = await realpath(file);
		if (!containsPath(dirname(file), physical)) return undefined;
		return await readFile(physical, 'utf8');
	} catch (error) {
		if (
			error instanceof Error &&
			'code' in error &&
			['ENOENT', 'ENOTDIR'].includes(String(error.code))
		)
			return undefined;
		throw error;
	}
}

async function declaresPage(file: string, page: string): Promise<boolean> {
	const text = await readDeclaration(file);
	if (text === undefined) return false;
	try {
		return readDesignPages(file, text).some(({ href }): boolean => {
			if (/[?#\\]/.test(href) || /^(?:[a-z][a-z0-9+.-]*:|\/\/)/i.test(href)) return false;
			const path = decodeURIComponent(href);
			return !isAbsolute(path) && !path.includes('\\') && resolve(dirname(file), path) === page;
		});
	} catch {
		// Only an authored declaration can establish preview ownership.
		return false;
	}
}

async function projectDesignFiles(file: string): Promise<string[] | undefined> {
	const text = await readDeclaration(file);
	if (text === undefined) return undefined;
	try {
		return Object.values(readProjectManifest(file, text).designs).map((design): string =>
			join(dirname(design), 'design.ts'),
		);
	} catch {
		// An unrelated project.ts may be ordinary application code.
		return undefined;
	}
}

export async function findDesignPreviewOwner(
	page: string,
	siteRoot: string | undefined,
): Promise<DesignPreviewOwner | undefined> {
	const files = siteRoot ? await declarationsInSite(siteRoot) : ancestorDeclarations(dirname(page));
	const declarations = new Map<string, Promise<boolean>>();
	function ownsPage(file: string): Promise<boolean> {
		const existing = declarations.get(file);
		if (existing) return existing;
		const reading = declaresPage(file, page);
		declarations.set(file, reading);
		return reading;
	}

	const projects = await Promise.all(
		files
			.filter((file): boolean => basename(file) === 'project.ts')
			.map(async (file) => ({ directory: dirname(file), designs: await projectDesignFiles(file) })),
	);
	const registered = new Set(projects.flatMap(({ designs }) => designs ?? []));
	const owners = (
		await Promise.all(
			projects.map(async ({ directory, designs }): Promise<DesignPreviewOwner[]> => {
				if (!designs) return [];
				const declared = await Promise.all(designs.map(ownsPage));
				return join(directory, 'index.html') === page || declared.some(Boolean)
					? [{ kind: 'project', directory }]
					: [];
			}),
		)
	).flat();
	const standalone = files.filter(
		(file): boolean => basename(file) === 'design.ts' && !registered.has(file),
	);
	owners.push(
		...(
			await Promise.all(
				standalone.map(async (file): Promise<DesignPreviewOwner[]> =>
					(await ownsPage(file)) ? [{ kind: 'design', directory: dirname(file) }] : [],
				),
			)
		).flat(),
	);
	if (owners.length > 1)
		throw new Error(
			`Multiple design owners declare ${page}: ${owners
				.map(({ directory }) => directory)
				.toSorted()
				.join(', ')}.`,
		);
	return owners[0];
}
