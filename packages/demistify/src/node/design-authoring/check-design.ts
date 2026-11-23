import { readFile, realpath } from 'node:fs/promises';
import { dirname, extname, isAbsolute, join, relative, resolve, sep } from 'node:path';

import { deliverablesViewSchema } from '../../core/deliverables-view.ts';
import {
	type NavigationTarget,
	type SharedMapNode,
	sharedNavigationSchema,
} from '../../core/shared-navigation.ts';
import { checkHtml, documentFragmentIds, formatFindings } from '../check.ts';
import {
	parseDeliveryEntries,
	projectDelivery,
	type DeliveryEntry,
	type DeliveryProjection,
} from './delivery-model.ts';
import { compileDesignNavigation } from './design-navigation.ts';
import { commitDesignOutputs, planGeneratedJson } from './design-output.ts';
import {
	normalizeExcerpt,
	passageText,
	readPageMarkup,
	type PageMarkup,
} from './design-page-markup.ts';
import { readDeliveryEntries } from './read-delivery.ts';
import { readDesignSources } from './read-design.ts';
import {
	prepareRecordPage,
	projectedDestinations,
	projectRecords,
	type RecordProjection,
} from './record-projection.ts';
import { recordRelationships, type DesignCore, type DesignRecord } from './schema.ts';

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

const relatedKind = {
	serves: 'need',
	'governed-by': 'choice',
	'relies-on': 'assumption',
	realizes: 'rule',
	checks: 'rule',
} as const;

export function checkRecordRelationships(records: readonly DesignRecord[]): void {
	const recordsById = new Map(records.map((record) => [record.id, record]));
	if (recordsById.size !== records.length) throw new Error('Duplicate design record ID.');
	const replacements = new Map<string, string>();
	for (const record of records) {
		if (record.kind !== 'choice' || !record.supersedes) continue;
		const previous = recordsById.get(record.supersedes);
		if (previous?.kind !== 'choice')
			throw new Error(`${record.id} supersedes an unknown decision: ${record.supersedes}.`);
		if (record.authority.state === 'owner-open')
			throw new Error(
				`${record.id}: settle the owner question before superseding ${record.supersedes}.`,
			);
		if (previous.authority.state !== 'lead-decided' && record.authority.state !== 'owner-decided')
			throw new Error(`${record.id}: only an Owner decision can supersede ${record.supersedes}.`);
		if (replacements.has(record.supersedes))
			throw new Error(`More than one decision supersedes ${record.supersedes}.`);
		replacements.set(record.supersedes, record.id);
	}
	for (const initial of replacements.keys()) {
		const seen = new Set<string>();
		let current: string | undefined = initial;
		while (current) {
			if (seen.has(current))
				throw new Error(`Decision supersession cycle: ${[...seen, current].join(' -> ')}.`);
			seen.add(current);
			current = replacements.get(current);
		}
	}
	for (const record of records) {
		for (const link of recordRelationships(record)) {
			const kind = relatedKind[link.relation];
			const replacement = link.relation === 'governed-by' && replacements.get(link.targetId);
			if (replacement)
				throw new Error(
					`${record.id} uses superseded decision ${link.targetId}. Review ${replacement}.`,
				);
			if (recordsById.get(link.targetId)?.kind !== kind)
				throw new Error(
					`${record.id} ${link.relation} must resolve to a ${kind}: ${link.targetId}`,
				);
		}
	}
}

interface PreparedDesign {
	readonly directory: string;
	readonly lead?: string;
	readonly design: DesignCore;
	readonly entries: readonly DeliveryEntry[];
	readonly checkTypes: (knownRecordIds: readonly string[]) => void;
	readonly firstPage: { readonly href: string; readonly title: string };
	readonly pages: readonly { file: string; owned: boolean }[];
	readonly plan: (delivery: DeliveryProjection, scope: PlanDesignScope) => Promise<DesignPlan>;
}

interface PlanDesignScope {
	readonly projection: RecordProjection;
	readonly preparedPages: ReadonlyMap<string, string>;
	readonly namespace?: string;
	readonly projectSource?: { readonly file: string; readonly text: string };
}

interface DesignPlan {
	readonly result: string;
	readonly generated: ReadonlyMap<string, unknown>;
}

export interface CheckedDesignBuild extends DesignPlan {
	readonly preparedPages: ReadonlyMap<string, string>;
	readonly design: DesignCore;
	readonly entries: readonly DeliveryEntry[];
	readonly delivery: DeliveryProjection;
}

interface DesignCheck {
	readonly read: (file: string, authoringRoot?: string) => Promise<string>;
	readonly checkEvidence: (href: string, sourceFile: string) => Promise<void>;
	readonly checkRecordSources: (
		records: readonly DesignRecord[],
		sourceFile: string,
	) => Promise<void>;
	readonly projectPages: (
		records: readonly DesignRecord[],
		pages: ReadonlyMap<string, string | undefined>,
	) => Promise<{
		readonly projection: RecordProjection;
		readonly preparedPages: ReadonlyMap<string, string>;
	}>;
	readonly prepare: (directory: string, authoringRoot?: string) => Promise<PreparedDesign>;
	readonly verify: () => Promise<void>;
}

export async function openDesignCheck(siteRoot: string): Promise<DesignCheck> {
	const siteDirectory = await realpath(resolve(siteRoot));
	const label = (file: string): string => relative(siteDirectory, file);
	const inputFiles = new Map<string, Promise<string>>();
	const authoringRoots = new Map<string, string>();
	async function resolveSiteFile(file: string, authoringRoot?: string): Promise<string> {
		if (authoringRoot && !containsPath(authoringRoot, file))
			throw new Error(`Authoring input escapes the project root: ${label(file)}`);
		if (!containsPath(siteDirectory, file))
			throw new Error(`Path escapes the site root: ${label(file)}`);
		const physical = await realpath(file);
		if (!containsPath(siteDirectory, physical))
			throw new Error(`Local target resolves outside the site root: ${label(file)}`);
		if (authoringRoot && !containsPath(authoringRoot, physical))
			throw new Error(`Authoring input resolves outside the project root: ${label(file)}`);
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

	async function checkRecordSources(
		records: readonly DesignRecord[],
		sourceFile: string,
	): Promise<void> {
		await Promise.all(
			records.flatMap((record) => {
				const sources =
					record.kind === 'choice' || record.kind === 'assumption' ? (record.sources ?? []) : [];
				const resolution = record.kind === 'assumption' ? record.resolution : undefined;
				return [...sources, ...(resolution ? [resolution] : [])].map(async (source) => {
					const target = resolveLink(source.href, sourceFile);
					if (!target) throw new Error(`Source reference must be local: ${source.href}`);
					await checkDestination(target.file, target.fragment, source.verbatim === true);
					if (!source.verbatim) return;
					if (!isHtml(target.file))
						throw new Error(
							`${record.id}: verbatim excerpts require an HTML source: ${source.href}`,
						);
					const text = passageText(await readSiteFile(target.file), target.fragment || undefined);
					if (!text.includes(normalizeExcerpt(source.excerpt)))
						throw new Error(`${record.id}: verbatim excerpt does not appear in ${source.href}.`);
				});
			}),
		);
	}

	async function projectPages(
		records: readonly DesignRecord[],
		pages: ReadonlyMap<string, string | undefined>,
	): ReturnType<DesignCheck['projectPages']> {
		const authored = await Promise.all(
			[...pages].map(async ([file, namespace]) => ({
				file,
				namespace,
				markup: await readPage(file),
				html: await readSiteFile(file),
			})),
		);
		const projection = projectRecords(records, authored);
		return {
			projection,
			preparedPages: new Map(
				authored.map((page) => [page.file, prepareRecordPage({ ...page, projection })]),
			),
		};
	}

	async function prepare(directory: string, authoringRoot?: string): Promise<PreparedDesign> {
		const setDirectory = await resolveSiteFile(resolve(directory), authoringRoot);
		const navigationFile = join(setDirectory, 'navigation.generated.json');
		const deliverablesFile = join(setDirectory, 'deliverables.generated.json');
		async function readOwnedInput(file: string): Promise<string> {
			authoringRoots.set(file, setDirectory);
			if (!containsPath(setDirectory, await resolveSiteFile(file)))
				throw new Error(`${label(file)} must remain inside this design set.`);
			return readSiteFile(file);
		}
		const readOptionalOwnedInput = (file: string): Promise<string | undefined> =>
			readOwnedInput(file).catch((error: unknown): undefined => {
				if (missing(error)) return undefined;
				throw error;
			});
		const { design, designFile, designText, sources } = await readDesignSources(
			setDirectory,
			readOwnedInput,
			readOptionalOwnedInput,
		);
		const deliverySource = await readDeliveryEntries(
			designFile,
			designText,
			readOptionalOwnedInput,
			sources,
		);
		const entries = parseDeliveryEntries(deliverySource.entries);
		for (const {
			deliverable: { id },
		} of entries)
			if (id.includes('/')) throw new Error(`Deliverable ID must be local to its design: ${id}.`);
		const pages = await Promise.all(
			design.pages.map(async (page) => {
				const target = resolveLink(page.href, designFile);
				if (!target || /[?#]/.test(page.href) || !isHtml(target.file))
					throw new Error(`A declared page must name a relative HTML file: ${page.href}`);
				await checkDestination(target.file, '');
				const owned =
					containsPath(setDirectory, target.file) &&
					containsPath(setDirectory, await resolveSiteFile(target.file));
				return { id: page.id, href: page.href, title: page.title, file: target.file, owned };
			}),
		);
		const filesByPage = new Map(pages.map(({ id, file }) => [id, file]));
		const pageIdsByFile = new Map(pages.map(({ id, file }) => [file, id]));
		const ownedPages = new Set(pages.filter(({ owned }): boolean => owned).map(({ file }) => file));
		const declaredPages = new Set(pages.map(({ file }) => file));
		const [firstPage] = pages;
		if (!firstPage) throw new Error('Declare at least one HTML page.');
		const checkNavigationTarget = async (target: NavigationTarget): Promise<void> => {
			const file = filesByPage.get(target.page);
			if (file && target.fragment) await checkDestination(file, target.fragment);
		};
		const checkMapNode = async (node: SharedMapNode): Promise<void> => {
			if (node.target) await checkNavigationTarget(node.target);
			await Promise.all(node.children.map(checkMapNode));
		};
		const generatedDesign = join(setDirectory, 'design.generated.json');
		await checkRecordSources(design.records, designFile);
		async function plan(
			delivery: DeliveryProjection,
			{ projection, preparedPages, namespace, projectSource }: PlanDesignScope,
		): Promise<DesignPlan> {
			const navigation = compileDesignNavigation(design, (reference) => {
				const qualified = reference.includes('/');
				if (qualified && !namespace)
					throw new Error(`Qualified reference ${reference} requires demistify project check.`);
				const id = namespace && !qualified ? `${namespace}/${reference}` : reference;
				const record = projection.records.get(id);
				if (!record) throw new Error(`${label(designFile)}: unknown design record ${reference}.`);
				const destination = projection.placements.get(id);
				if (!destination) return { record };
				const page = pageIdsByFile.get(destination.file);
				if (!page)
					throw new Error(
						`${label(designFile)}: declare ${label(destination.file)} in pages to navigate to ${reference}.`,
					);
				return { record, target: { page, fragment: destination.fragment } };
			});
			const validatedNavigation = sharedNavigationSchema.parse(navigation);
			await Promise.all([
				...validatedNavigation.map.map(checkMapNode),
				...validatedNavigation.walkthroughs.flatMap((route) =>
					route.steps.flatMap((step) => (step.target ? [checkNavigationTarget(step.target)] : [])),
				),
			]);
			const deliverables = deliverablesViewSchema.parse({
				version: 1,
				deliverables: delivery.deliverables,
			});
			await Promise.all([
				...delivery.destinations.map(async ({ page, fragment }) => {
					const target = resolveLink(page, designFile);
					if (!target || !declaredPages.has(target.file))
						throw new Error(`Deliverable destination must be a declared HTML page: ${page}`);
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
						if (
							!source ||
							/[?#]/.test(source) ||
							resolveLink(source, file)?.file !== deliverablesFile
						)
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
					const findings = await checkHtml(preparedPages.get(file) ?? (await readSiteFile(file)), {
						readSource: async (path) => {
							const target = resolveLink(path, file);
							if (!target) return undefined;
							if (target.file === navigationFile) return navigationText;
							if (target.file === deliverablesFile) return deliverablesText;
							if (target.file === projectSource?.file) return projectSource.text;
							return (
								preparedPages.get(target.file) ??
								readSiteFile(target.file).catch((error: unknown): undefined => {
									if (!missing(error)) throw error;
									return undefined;
								})
							);
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
				[...projectedDestinations(projection, setDirectory)].flatMap(([id, destination]) => {
					if (namespace && !id.startsWith(`${namespace}/`)) return [];
					return [[namespace ? id.slice(namespace.length + 1) : id, destination]];
				}),
			);
			const generated = new Map<string, unknown>([
				[
					generatedDesign,
					{ version: 1, pages: design.pages, records: design.records, recordDestinations },
				],
				[navigationFile, navigation],
				[deliverablesFile, deliverables],
			]);
			const result = [
				details,
				`Checked ${ownedPages.size} authored page(s), ${pages.length} navigation page(s), and ${design.records.length} design record(s).`,
				...Object.values(delivery.deliverables).map((item): string =>
					[
						`${item.id} · ${item.state} · ${item.title}`,
						...(item.kind === 'core' && item.state !== 'accepted' ? item.reasons : []),
					].join('\n  '),
				),
			]
				.filter(Boolean)
				.join('\n');
			return { result, generated };
		}
		return {
			directory: setDirectory,
			firstPage: { href: firstPage.href, title: firstPage.title },
			pages,
			design,
			entries,
			checkTypes: deliverySource.checkTypes,
			plan,
			...(deliverySource.lead === undefined ? {} : { lead: deliverySource.lead }),
		};
	}

	async function verify(): Promise<void> {
		await Promise.all(
			[...inputFiles].map(async ([path, original]) => {
				const content = await original.catch((error: unknown): undefined => {
					if (!missing(error)) throw error;
					return undefined;
				});
				const current = await resolveSiteFile(path, authoringRoots.get(path))
					.then((file) => readFile(file, 'utf8'))
					.catch((error: unknown): undefined => {
						if (!missing(error)) throw error;
						return undefined;
					});
				if (current !== content)
					throw new Error(`Input changed during design check: ${label(path)}`);
			}),
		);
	}
	async function checkEvidence(href: string, sourceFile: string): Promise<void> {
		const target = resolveLink(href, sourceFile);
		if (!target) throw new Error(`Deliverable evidence must be local: ${href}`);
		await checkDestination(target.file, target.fragment);
	}
	return {
		read: async (file, authoringRoot): Promise<string> => {
			if (authoringRoot) {
				authoringRoots.set(file, authoringRoot);
				await resolveSiteFile(file, authoringRoot);
			}
			return readSiteFile(file);
		},
		checkEvidence,
		checkRecordSources,
		projectPages,
		prepare,
		verify,
	};
}

export async function buildDesignSet(
	directory: string,
	options: { readonly siteRoot?: string | undefined; readonly write?: boolean | undefined } = {},
): Promise<CheckedDesignBuild> {
	const setDirectory = await realpath(resolve(directory));
	const check = await openDesignCheck(options.siteRoot ?? setDirectory);
	const set = await check.prepare(setDirectory);
	const qualified =
		set.design.records
			.flatMap((record) => [
				...recordRelationships(record).map(({ targetId }) => targetId),
				...(record.kind === 'choice' && record.supersedes ? [record.supersedes] : []),
			])
			.find((reference): boolean => reference.includes('/')) ??
		set.entries
			.flatMap(({ dependsOn, deliverable }) => [
				...dependsOn,
				...(deliverable.kind === 'core'
					? [
							...deliverable.scope.rules,
							...deliverable.scope.designItems,
							...deliverable.scope.verificationChecks,
						]
					: []),
			])
			.find((reference): boolean => reference.includes('/'));
	if (qualified)
		throw new Error(`Qualified reference ${qualified} requires demistify project check.`);
	set.checkTypes(set.design.records.map(({ id }) => id));
	checkRecordRelationships(set.design.records);
	const { projection, preparedPages } = await check.projectPages(
		set.design.records,
		new Map(set.pages.map(({ file }) => [file, undefined])),
	);
	const delivery = projectDelivery(
		set.design,
		set.entries,
		projectedDestinations(projection, setDirectory),
	);
	const plan = await set.plan(delivery, { projection, preparedPages });
	if (options.write) await writeDesignBuild([set.directory], check.verify, plan.generated);
	else await check.verify();
	return { ...plan, design: set.design, entries: set.entries, preparedPages, delivery };
}

export async function writeDesignBuild(
	directories: readonly string[],
	verify: () => Promise<void>,
	generated: ReadonlyMap<string, unknown>,
): Promise<void> {
	await commitDesignOutputs(directories, [
		{
			verify,
			plan: () =>
				Promise.all([...generated].map(([file, value]) => planGeneratedJson(file, value))),
		},
	]);
}

export async function checkDesignSet(
	directory: string,
	siteRoot?: string,
	write = false,
): Promise<string> {
	return (await buildDesignSet(directory, { siteRoot, write })).result;
}
