import { randomUUID } from 'node:crypto';
import { lstat, readFile, realpath, rename, unlink, writeFile } from 'node:fs/promises';
import { dirname, extname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { isDeepStrictEqual } from 'node:util';

import { parse, type DefaultTreeAdapterMap } from 'parse5';

import {
	type NavigationTarget,
	type SharedMapNode,
	sharedNavigationSchema,
} from '../../core/shared-navigation.ts';
import { checkHtml, formatFindings } from '../check.ts';
import { readDesign } from './read-design.ts';
import type { DesignRecord } from './schema.ts';

function containsPath(root: string, file: string): boolean {
	const child = relative(root, file);
	return !isAbsolute(child) && child !== '..' && !child.startsWith(`..${sep}`);
}

function isHtml(file: string): boolean {
	return /^\.html?$/i.test(extname(file));
}

function missing(error: unknown): boolean {
	return (
		error instanceof Error && 'code' in error && ['ENOENT', 'ENOTDIR'].includes(String(error.code))
	);
}

interface PageMarkup {
	ids: Set<string>;
	links: string[];
	navigationSources: string[];
}

function readPageMarkup(html: string, file: string): PageMarkup {
	const result: PageMarkup = { ids: new Set(), links: [], navigationSources: [] };
	function visit(node: DefaultTreeAdapterMap['node']): void {
		if ('attrs' in node) {
			const get = (name: string): string | undefined =>
				node.attrs.find((attribute): boolean => attribute.name === name)?.value;
			const id = get('id');
			const href = get('href');
			if (id) {
				if (result.ids.has(id)) throw new Error(`Duplicate HTML id "${id}" in ${file}.`);
				result.ids.add(id);
			}
			if (href) result.links.push(href);
			if (node.tagName === 'ex-nav') result.navigationSources.push(get('src') ?? '');
		}
		if ('childNodes' in node) node.childNodes.forEach(visit);
	}
	visit(parse(html));
	return result;
}

const relatedKind = {
	serves: 'need',
	'governed-by': 'choice',
	realizes: 'rule',
	checks: 'rule',
} as const;

function checkRecordRelationships(records: readonly DesignRecord[]): void {
	const recordsById = new Map(records.map((record) => [record.id, record]));
	if (recordsById.size !== records.length) throw new Error('Duplicate design record ID.');
	for (const record of records) {
		if (!('relationships' in record)) continue;
		for (const link of record.relationships) {
			const kind = relatedKind[link.relation];
			if (recordsById.get(link.targetId)?.kind !== kind)
				throw new Error(
					`${record.id} ${link.relation} must resolve to a ${kind}: ${link.targetId}`,
				);
		}
	}
}

/** Stage each derived file next to its destination so readers see complete JSON. */
async function writeGeneratedJson(
	file: string,
	value: unknown,
	sameValue: (current: unknown, next: unknown) => boolean = isDeepStrictEqual,
): Promise<void> {
	const existing = await lstat(file).catch((error: unknown) => {
		if (!missing(error)) throw error;
		return undefined;
	});
	if (existing) {
		if (!existing.isFile()) throw new Error(`Expected a generated file at ${file}.`);
		const current = await readFile(file, 'utf8');
		try {
			if (sameValue(JSON.parse(current), value)) return;
		} catch {
			// A malformed previous output can be regenerated from validated inputs.
		}
	}
	const temporary = `${file}.${randomUUID()}.tmp`;
	await writeFile(temporary, `${JSON.stringify(value, null, 2)}\n`, { flag: 'wx' });
	try {
		await rename(temporary, file);
	} finally {
		await unlink(temporary).catch((error: unknown): void => {
			if (!missing(error)) throw error;
		});
	}
}

export async function checkDesignSet(directory: string, siteRoot?: string): Promise<string> {
	const setDirectory = await realpath(resolve(directory));
	const siteDirectory = siteRoot ? await realpath(resolve(siteRoot)) : setDirectory;
	if (!containsPath(siteDirectory, setDirectory))
		throw new Error('The site root must contain the design set.');
	const label = (file: string): string => relative(siteDirectory, file);
	const inputFiles = new Map<string, Promise<string>>();
	function readSiteFile(file: string): Promise<string> {
		if (!containsPath(siteDirectory, file))
			throw new Error(`Path escapes the site root: ${label(file)}`);
		const present = inputFiles.get(file);
		if (present) return present;
		const reading = realpath(file).then(async (physical) => {
			if (!containsPath(siteDirectory, physical))
				throw new Error(`Local target resolves outside the site root: ${label(file)}`);
			return readFile(physical, 'utf8');
		});
		inputFiles.set(file, reading);
		return reading;
	}

	function resolveLink(href: string, from: string): { file: string; fragment: string } | undefined {
		if (/^[a-z][a-z0-9+.-]*:|^\/\//i.test(href)) return undefined;
		const hash = href.indexOf('#');
		const address = hash < 0 ? href : href.slice(0, hash);
		const pathname = decodeURIComponent(address.split('?')[0] ?? '');
		const fragment = hash < 0 ? '' : decodeURIComponent(href.slice(hash + 1));
		const file = pathname ? resolve(dirname(from), pathname) : from;
		if (isAbsolute(pathname) || pathname.includes('\\') || !containsPath(siteDirectory, file))
			throw new Error(`Relative link escapes the site root: ${href} from ${label(from)}`);
		return { file, fragment };
	}

	const pageMarkup = new Map<string, Promise<PageMarkup>>();
	function readPage(file: string): Promise<PageMarkup> {
		const existing = pageMarkup.get(file);
		if (existing) return existing;
		const reading = readSiteFile(file).then((html): PageMarkup =>
			readPageMarkup(html, label(file)),
		);
		pageMarkup.set(file, reading);
		return reading;
	}

	async function checkDestination(file: string, fragment: string): Promise<void> {
		try {
			if (isHtml(file)) {
				if (fragment && !(await readPage(file)).ids.has(fragment))
					throw new Error(`Missing HTML target: ${label(file)}#${fragment}`);
				await readPage(file);
			} else {
				await readSiteFile(file);
			}
		} catch (error) {
			if (missing(error)) throw new Error(`Missing local link: ${label(file)}`, { cause: error });
			throw error;
		}
	}

	const designFile = join(setDirectory, 'design.ts');
	const navigationFile = join(setDirectory, 'navigation.json');
	const design = readDesign(designFile, await readSiteFile(designFile));
	const navigation = {
		version: 1,
		pages: design.pages,
		map: design.mapGroups,
		...(design.walkthroughs === undefined ? {} : { walkthroughs: design.walkthroughs }),
	};
	const validatedNavigation = sharedNavigationSchema.parse(navigation);
	checkRecordRelationships(design.records);
	const pages = await Promise.all(
		design.pages.map(async (page) => {
			const target = resolveLink(page.href, designFile);
			if (!target || /[?#]/.test(page.href) || !isHtml(target.file))
				throw new Error(`A declared page must name a relative HTML file: ${page.href}`);
			await checkDestination(target.file, '');
			return [page.id, target.file] as const;
		}),
	);
	const filesByPage = new Map(pages);
	const ownedPages = new Set(
		[...filesByPage.values()].filter((file): boolean => containsPath(setDirectory, file)),
	);
	if (!ownedPages.size) throw new Error('Declare at least one HTML page owned by this design set.');
	const checkNavigationTarget = async (target: NavigationTarget): Promise<void> => {
		const file = filesByPage.get(target.page);
		if (file && target.fragment) await checkDestination(file, target.fragment);
	};
	const checkMapNode = async (node: SharedMapNode): Promise<void> => {
		if (node.target) await checkNavigationTarget(node.target);
		await Promise.all(node.children.map(checkMapNode));
	};
	await Promise.all([
		...validatedNavigation.map.map(checkMapNode),
		...validatedNavigation.walkthroughs.flatMap((route) =>
			route.steps.map((step) => checkNavigationTarget(step.target)),
		),
	]);
	await Promise.all(
		design.records.map(async (record) => {
			const destination = resolveLink(record.destination.page, designFile);
			if (!destination || !ownedPages.has(destination.file))
				throw new Error(`Record ${record.id} must point to a page owned by this design set.`);
			await checkDestination(destination.file, record.destination.fragment);
			await Promise.all(
				record.sources.map(async (source) => {
					const target = resolveLink(source.href, designFile);
					if (!target) throw new Error(`Source reference must be local: ${source.href}`);
					await checkDestination(target.file, target.fragment);
				}),
			);
		}),
	);

	const navigationText = JSON.stringify(navigation);
	const results = await Promise.all(
		[...ownedPages].map(async (file) => {
			const markup = await readPage(file);
			const [binding] = markup.navigationSources;
			if (
				markup.navigationSources.length !== 1 ||
				!binding ||
				/[?#]/.test(binding) ||
				resolveLink(binding, file)?.file !== navigationFile
			) {
				throw new Error(
					`The ex-nav src in ${label(file)} must resolve to the design set's navigation.json.`,
				);
			}
			await Promise.all(
				markup.links.map(async (href) => {
					const target = resolveLink(href, file);
					if (target) await checkDestination(target.file, target.fragment);
				}),
			);
			const findings = await checkHtml(await readSiteFile(file), {
				readSource: async (path) => {
					const local = resolve(dirname(file), path);
					if (local === navigationFile) return navigationText;
					return readSiteFile(local).catch((error: unknown): undefined => {
						if (!missing(error)) throw error;
						return undefined;
					});
				},
			});
			return { file: label(file), findings };
		}),
	);
	const details = results
		.filter(({ findings }): boolean => findings.length > 0)
		.map(({ file, findings }): string => formatFindings(file, findings))
		.join('\n');
	if (
		results.some(({ findings }): boolean =>
			findings.some((finding): boolean => finding.severity === 'error'),
		)
	)
		throw new Error(details);

	const recordDestinations = Object.fromEntries(
		design.records.map(({ id, destination }) => [
			id,
			`${destination.page}#${destination.fragment}`,
		]),
	);
	// Check successful input reads again before publishing either derived file.
	await Promise.all(
		[...inputFiles].map(async ([path, original]) => {
			const content = await original.catch((error: unknown): undefined => {
				if (!missing(error)) throw error;
				return undefined;
			});
			if (content !== undefined && (await readFile(path, 'utf8')) !== content)
				throw new Error(`Input changed during design check: ${label(path)}`);
		}),
	);
	await writeGeneratedJson(join(setDirectory, 'design.json'), {
		version: 1,
		pages: design.pages,
		records: design.records,
		recordDestinations,
	});
	await writeGeneratedJson(navigationFile, navigation, (current): boolean => {
		const parsed = sharedNavigationSchema.safeParse(current);
		return parsed.success && isDeepStrictEqual(parsed.data, validatedNavigation);
	});
	return [
		details,
		`Checked ${ownedPages.size} authored page(s), ${pages.length} navigation page(s), and ${design.records.length} design record(s).`,
	]
		.filter(Boolean)
		.join('\n');
}
