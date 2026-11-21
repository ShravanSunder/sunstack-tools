import { createHash, randomUUID } from 'node:crypto';
import { lstat, open, readFile, realpath, rename, unlink, writeFile } from 'node:fs/promises';
import { dirname, extname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { isDeepStrictEqual } from 'node:util';

import { parse, type DefaultTreeAdapterMap } from 'parse5';

import { deliverablesViewSchema } from '../../core/deliverables-view.ts';
import {
	type NavigationTarget,
	type SharedMapNode,
	sharedNavigationSchema,
} from '../../core/shared-navigation.ts';
import { checkHtml, documentFragmentIds, formatFindings } from '../check.ts';
import { projectDelivery } from './delivery-model.ts';
import { readDeliveryEntries } from './read-delivery.ts';
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

interface AuthoredFragment {
	readonly source: string;
	readonly designItem: string | undefined;
	readonly authority: string | undefined;
}

interface PageMarkup {
	ids: Map<string, AuthoredFragment>;
	links: string[];
	navigationSources: string[];
	deliverySources: string[];
	hasDocumentBase: boolean;
}

function readPageMarkup(html: string, file: string): PageMarkup {
	const result: PageMarkup = {
		ids: new Map(),
		links: [],
		navigationSources: [],
		deliverySources: [],
		hasDocumentBase: false,
	};
	function visit(node: DefaultTreeAdapterMap['node']): void {
		if ('attrs' in node) {
			const get = (name: string): string | undefined =>
				node.attrs.find((attribute): boolean => attribute.name === name)?.value;
			const id = get('id');
			const href = get('href');
			if (id) {
				if (result.ids.has(id)) throw new Error(`Duplicate HTML id "${id}" in ${file}.`);
				const source = node.sourceCodeLocation;
				result.ids.set(id, {
					source: source ? html.slice(source.startOffset, source.endOffset) : '',
					designItem: get('data-design-item'),
					authority: get('data-authority'),
				});
			}
			if (href) result.links.push(href);
			if (node.tagName === 'base' && href !== undefined) result.hasDocumentBase = true;
			if (node.tagName === 'ex-nav') result.navigationSources.push(get('src') ?? '');
			if (node.tagName === 'ex-deliverables') result.deliverySources.push(get('src') ?? '');
		}
		if ('childNodes' in node) node.childNodes.forEach(visit);
	}
	visit(parse(html, { sourceCodeLocationInfo: true }));
	return result;
}

function checkDesignItemMarkup(record: DesignRecord, fragment: AuthoredFragment | undefined): void {
	if (!fragment || (record.kind !== 'choice' && record.kind !== 'assumption')) return;
	const itemKind = record.kind === 'choice' ? 'decision' : 'assumption';
	if (fragment.designItem !== undefined && fragment.designItem !== itemKind)
		throw new Error(
			`${record.id}: expected data-design-item="${itemKind}" on its HTML destination.`,
		);

	if (record.kind !== 'choice' || fragment.authority === undefined) return;
	const { authority } = record;
	const expected = authority.state === 'owner-open' ? 'unknown' : authority.sourceKind;
	if (fragment.authority !== expected)
		throw new Error(`${record.id}: expected data-authority="${expected}" on its HTML destination.`);
}

const relatedKind = {
	serves: 'need',
	'governed-by': 'choice',
	'relies-on': 'assumption',
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

interface GeneratedFile {
	file: string;
	previous: string | undefined;
	text: string;
}

async function planGeneratedJson(
	file: string,
	value: unknown,
	sameValue: (current: unknown, next: unknown) => boolean = isDeepStrictEqual,
): Promise<GeneratedFile | undefined> {
	const existing = await lstat(file).catch((error: unknown) => {
		if (!missing(error)) throw error;
		return undefined;
	});
	if (existing && !existing.isFile()) throw new Error(`Expected a generated file at ${file}.`);
	const previous = existing ? await readFile(file, 'utf8') : undefined;
	if (previous !== undefined) {
		try {
			if (sameValue(JSON.parse(previous), value)) return undefined;
		} catch {
			// A malformed previous output can be regenerated from validated inputs.
		}
	}
	return { file, previous, text: `${JSON.stringify(value, null, 2)}\n` };
}

/** Preflight all outputs, then stage their replacements and rollback copies together. */
async function writeGeneratedFiles(outputs: GeneratedFile[]): Promise<void> {
	const writes = outputs.map((output) => ({
		...output,
		stage: `${output.file}.${randomUUID()}.tmp`,
		rollback: `${output.file}.${randomUUID()}.tmp`,
	}));
	const committed: typeof writes = [];
	try {
		for (const { stage, text, previous, rollback } of writes) {
			// oxlint-disable-next-line no-await-in-loop -- finish staging before renaming either output.
			await writeFile(stage, text, { flag: 'wx' });
			if (previous !== undefined) {
				// oxlint-disable-next-line no-await-in-loop -- stage restoration bytes before renaming.
				await writeFile(rollback, previous, { flag: 'wx' });
			}
		}
		for (const pending of writes) {
			// oxlint-disable-next-line no-await-in-loop -- record each completed rename for rollback.
			await rename(pending.stage, pending.file);
			committed.push(pending);
		}
	} catch (error) {
		for (const { file, previous, rollback } of committed.toReversed()) {
			// oxlint-disable-next-line no-await-in-loop -- undo each completed write before reporting failure.
			await (previous === undefined ? unlink(file) : rename(rollback, file));
		}
		throw error;
	} finally {
		await Promise.all(
			writes
				.flatMap(({ stage, rollback }) => [stage, rollback])
				.map(async (path) =>
					unlink(path).catch((error: unknown): void => {
						if (!missing(error)) throw error;
					}),
				),
		);
	}
}

export async function checkDesignSet(directory: string, siteRoot?: string): Promise<string> {
	const setDirectory = await realpath(resolve(directory));
	const siteDirectory = siteRoot ? await realpath(resolve(siteRoot)) : setDirectory;
	if (!containsPath(siteDirectory, setDirectory))
		throw new Error('The site root must contain the design set.');
	const label = (file: string): string => relative(siteDirectory, file);
	const inputFiles = new Map<string, Promise<string>>();
	async function resolveSiteFile(file: string): Promise<string> {
		if (!containsPath(siteDirectory, file))
			throw new Error(`Path escapes the site root: ${label(file)}`);
		const physical = await realpath(file);
		if (!containsPath(siteDirectory, physical))
			throw new Error(`Local target resolves outside the site root: ${label(file)}`);
		return physical;
	}
	function readSiteFile(file: string): Promise<string> {
		const present = inputFiles.get(file);
		if (present) return present;
		const reading = resolveSiteFile(file).then((physical) => readFile(physical, 'utf8'));
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

	const navigationIds = new Map<string, Promise<ReadonlySet<string>>>();
	function readNavigationIds(file: string): Promise<ReadonlySet<string>> {
		const existing = navigationIds.get(file);
		if (existing) return existing;
		const reading = readSiteFile(file).then((html) =>
			documentFragmentIds(html, async (source) => {
				const target = resolveLink(source, file);
				if (!target) return undefined;
				try {
					return await readSiteFile(target.file);
				} catch (error) {
					if (missing(error)) return undefined;
					throw error;
				}
			}),
		);
		navigationIds.set(file, reading);
		return reading;
	}

	async function checkDestination(
		file: string,
		fragment: string,
		requireAuthoredId = false,
	): Promise<void> {
		try {
			if (isHtml(file)) {
				const { ids } = await readPage(file);
				if (fragment && !ids.has(fragment)) {
					if (requireAuthoredId || !(await readNavigationIds(file)).has(fragment))
						throw new Error(`Missing HTML target: ${label(file)}#${fragment}`);
				}
			} else {
				await readSiteFile(file);
			}
		} catch (error) {
			if (missing(error)) throw new Error(`Missing local link: ${label(file)}`, { cause: error });
			throw error;
		}
	}

	const designFile = join(setDirectory, 'design.ts');
	const declarationFile = join(setDirectory, 'design-schema.d.ts');
	const navigationFile = join(setDirectory, 'navigation.generated.json');
	const deliverablesFile = join(setDirectory, 'deliverables.generated.json');
	async function readOwnedInput(file: string): Promise<string> {
		if (!containsPath(setDirectory, await resolveSiteFile(file)))
			throw new Error(`${label(file)} must remain inside this design set.`);
		return readSiteFile(file);
	}
	const readOptionalOwnedInput = (file: string): Promise<string | undefined> =>
		readOwnedInput(file).catch((error: unknown): undefined => {
			if (missing(error)) return undefined;
			throw error;
		});
	const [designText, declaration] = await Promise.all([
		readOwnedInput(designFile),
		readOptionalOwnedInput(declarationFile),
	]);
	const design = readDesign(designFile, designText);
	const entries = await readDeliveryEntries(
		designFile,
		designText,
		declaration,
		readOptionalOwnedInput,
	);
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
			const owned =
				containsPath(setDirectory, target.file) &&
				containsPath(setDirectory, await resolveSiteFile(target.file));
			return { id: page.id, file: target.file, owned };
		}),
	);
	const filesByPage = new Map(pages.map(({ id, file }) => [id, file]));
	const ownedPages = new Set(pages.filter(({ owned }): boolean => owned).map(({ file }) => file));
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
	const recordRevisions = new Map(
		await Promise.all(
			design.records.map(async (record) => {
				const destination = resolveLink(record.destination.page, designFile);
				if (!destination || !ownedPages.has(destination.file))
					throw new Error(`Record ${record.id} must point to a page owned by this design set.`);
				await checkDestination(destination.file, record.destination.fragment, true);
				const sources =
					record.kind === 'assumption' && record.resolution
						? [...record.sources, record.resolution]
						: record.sources;
				await Promise.all(
					sources.map(async (source) => {
						const target = resolveLink(source.href, designFile);
						if (!target) throw new Error(`Source reference must be local: ${source.href}`);
						await checkDestination(target.file, target.fragment);
					}),
				);
				const passage = (await readPage(destination.file)).ids.get(record.destination.fragment);
				checkDesignItemMarkup(record, passage);
				const governingRecord =
					'relationships' in record
						? {
								...record,
								relationships: record.relationships.toSorted(
									(left, right) =>
										left.relation.localeCompare(right.relation) ||
										left.targetId.localeCompare(right.targetId),
								),
							}
						: record;
				const revision = createHash('sha256')
					.update(JSON.stringify([governingRecord, passage?.source ?? '']))
					.digest('hex');
				return [record.id, revision] as const;
			}),
		),
	);
	const delivery = projectDelivery(design, entries, recordRevisions);
	const snapshotRevision = createHash('sha256')
		.update(
			JSON.stringify([Object.fromEntries(recordRevisions), navigation, delivery.deliverables]),
		)
		.digest('hex');
	const deliverables = deliverablesViewSchema.parse({
		version: 1,
		snapshotRevision,
		deliverables: delivery.deliverables,
	});
	await Promise.all([
		...delivery.destinations.map(async ({ page, fragment }) => {
			const target = resolveLink(page, designFile);
			if (!target || !ownedPages.has(target.file))
				throw new Error(`Deliverable destination must be a page owned by its design set: ${page}`);
			await checkDestination(target.file, fragment ?? '', true);
		}),
		...delivery.evidence.map(async (href) => {
			const target = resolveLink(href, designFile);
			if (!target) throw new Error(`Deliverable evidence must be local: ${href}`);
			await checkDestination(target.file, target.fragment);
		}),
	]);

	const navigationText = JSON.stringify(navigation);
	const deliverablesText = JSON.stringify(deliverables);
	const results = await Promise.all(
		[...ownedPages].map(async (file) => {
			const markup = await readPage(file);
			if (markup.hasDocumentBase)
				throw new Error(
					`${label(file)}: remove <base href> so design links resolve from the authored page.`,
				);
			for (const source of markup.deliverySources) {
				if (!source || /[?#]/.test(source) || resolveLink(source, file)?.file !== deliverablesFile)
					throw new Error(
						`The ex-deliverables src in ${label(file)} must resolve to the design set's deliverables.generated.json.`,
					);
			}
			const [binding] = markup.navigationSources;
			if (
				markup.navigationSources.length !== 1 ||
				!binding ||
				/[?#]/.test(binding) ||
				resolveLink(binding, file)?.file !== navigationFile
			) {
				throw new Error(
					`The ex-nav src in ${label(file)} must resolve to the design set's navigation.generated.json.`,
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
					const target = resolveLink(path, file);
					if (!target) return undefined;
					if (target.file === navigationFile) return navigationText;
					if (target.file === deliverablesFile) return deliverablesText;
					return readSiteFile(target.file).catch((error: unknown): undefined => {
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
	const lockPath = join(setDirectory, '.demistify-design.lock');
	const lock = await open(lockPath, 'wx').catch((error: unknown) => {
		if (error instanceof Error && 'code' in error && error.code === 'EEXIST')
			throw new Error(`Another design check has the output lock: ${label(lockPath)}`, {
				cause: error,
			});
		throw error;
	});
	try {
		// Check successful input reads again while holding the output lock.
		await Promise.all(
			[...inputFiles].map(async ([path, original]) => {
				const content = await original.catch((error: unknown): undefined => {
					if (!missing(error)) throw error;
					return undefined;
				});
				if (
					content !== undefined &&
					(await readFile(await resolveSiteFile(path), 'utf8')) !== content
				)
					throw new Error(`Input changed during design check: ${label(path)}`);
			}),
		);
		const outputs = await Promise.all([
			planGeneratedJson(join(setDirectory, 'design.generated.json'), {
				version: 1,
				snapshotRevision,
				pages: design.pages,
				records: design.records,
				recordDestinations,
				recordRevisions: Object.fromEntries(recordRevisions),
			}),
			planGeneratedJson(navigationFile, navigation, (current): boolean => {
				const parsed = sharedNavigationSchema.safeParse(current);
				return parsed.success && isDeepStrictEqual(parsed.data, validatedNavigation);
			}),
			planGeneratedJson(deliverablesFile, deliverables),
		]);
		await writeGeneratedFiles(
			outputs.filter((output): output is GeneratedFile => output !== undefined),
		);
	} finally {
		await lock.close();
		await unlink(lockPath);
	}
	return [
		details,
		`Last checked snapshot: ${snapshotRevision}`,
		`Checked ${ownedPages.size} authored page(s), ${pages.length} navigation page(s), and ${design.records.length} design record(s).`,
		...Object.values(delivery.deliverables).map(
			({ id, state, title, scopeRevision, reasons }): string =>
				[`${id} · ${state} · ${title}`, `Scope revision: ${scopeRevision}`, ...reasons].join(
					'\n  ',
				),
		),
	]
		.filter(Boolean)
		.join('\n');
}
