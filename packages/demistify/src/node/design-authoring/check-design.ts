import { createHash } from 'node:crypto';
import { lstat, readFile, realpath } from 'node:fs/promises';
import { dirname, extname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { isDeepStrictEqual } from 'node:util';

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
import { checkDecisionChoices, compileDesignNavigation } from './design-navigation.ts';
import { commitDesignOutputs, planGeneratedJson, type GeneratedFile } from './design-output.ts';
import {
	checkDesignItemMarkup,
	normalizeExcerpt,
	passageText,
	readPageMarkup,
	type PageMarkup,
} from './design-page-markup.ts';
import { readDeliveryEntries } from './read-delivery.ts';
import { readDesign, readDesignItems } from './read-design.ts';
import type { DesignCore, DesignRecord } from './schema.ts';
import {
	deliveryRevisionInputs,
	readScopeBoundaries,
	recordScopeRevision,
	type RecordScopeRevision,
} from './scope-boundary.ts';

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
		if (!('relationships' in record)) continue;
		for (const link of record.relationships) {
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
	readonly firstPage: { readonly href: string; readonly title: string };
	readonly pages: readonly { file: string; owned: boolean }[];
	readonly recordRevisions: ReadonlyMap<string, RecordScopeRevision>;
	readonly plan: (
		delivery: DeliveryProjection,
		projectSource?: ProjectNavigationSource,
	) => Promise<{
		readonly result: string;
		readonly outputs: () => Promise<readonly (GeneratedFile | undefined)[]>;
	}>;
}

interface ProjectNavigationSource {
	readonly file: string;
	readonly text: string;
	readonly records: ReadonlyMap<string, DesignRecord>;
}

interface ChoiceCheckScope {
	readonly pages: ReadonlyMap<string, string | undefined>;
	readonly records: ReadonlyMap<string, DesignRecord>;
	readonly sourceFile: string;
	readonly project?: true;
}

interface DesignCheck {
	readonly read: (file: string, authoringRoot?: string) => Promise<string>;
	readonly checkEvidence: (href: string, sourceFile: string) => Promise<void>;
	readonly inspectRecords: (
		records: readonly DesignRecord[],
		sourceFile: string,
		options?: { pages?: ReadonlySet<string>; previousBoundaries?: ReadonlyMap<string, string> },
	) => Promise<Map<string, RecordScopeRevision>>;
	readonly readScopeBoundaries: (
		file: string,
		location: 'design' | 'project',
	) => Promise<ReadonlyMap<string, string>>;
	readonly checkChoices: (scope: ChoiceCheckScope) => Promise<void>;
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

	async function inspectRecords(
		records: readonly DesignRecord[],
		sourceFile: string,
		{
			pages,
			previousBoundaries,
		}: { pages?: ReadonlySet<string>; previousBoundaries?: ReadonlyMap<string, string> } = {},
	): Promise<Map<string, RecordScopeRevision>> {
		return new Map(
			await Promise.all(
				records.map(async (record) => {
					const destination = resolveLink(record.destination.page, sourceFile);
					if (!destination || !isHtml(destination.file) || (pages && !pages.has(destination.file)))
						throw new Error(`Record ${record.id} must point to a declared HTML page.`);
					await checkDestination(destination.file, record.destination.fragment, true);
					const sources =
						record.kind === 'assumption' && record.resolution
							? [...record.sources, record.resolution]
							: record.sources;
					await Promise.all(
						sources.map(async (source) => {
							const target = resolveLink(source.href, sourceFile);
							if (!target) throw new Error(`Source reference must be local: ${source.href}`);
							await checkDestination(target.file, target.fragment, source.verbatim === true);
							if (source.verbatim) {
								if (!isHtml(target.file))
									throw new Error(
										`${record.id}: verbatim excerpts require an HTML source: ${source.href}`,
									);
								const cited = target.fragment
									? (await readPage(target.file)).ids.get(target.fragment)?.source
									: await readSiteFile(target.file);
								if (!cited || !passageText(cited).includes(normalizeExcerpt(source.excerpt)))
									throw new Error(
										`${record.id}: verbatim excerpt does not appear in ${source.href}.`,
									);
							}
						}),
					);
					const passage = (await readPage(destination.file)).ids.get(record.destination.fragment);
					checkDesignItemMarkup(record, passage);
					const revision = recordScopeRevision({
						record,
						source: passage?.source ?? '',
						heading: passage?.heading === true,
						previousBoundary: previousBoundaries?.get(record.id),
					});
					return [record.id, revision] as const;
				}),
			),
		);
	}

	async function readPreviousBoundaries(
		file: string,
		location: 'design' | 'project',
	): Promise<ReadonlyMap<string, string>> {
		const existing = await lstat(file).catch((error: unknown) => {
			if (missing(error)) return undefined;
			throw error;
		});
		return existing?.isFile() ? readScopeBoundaries(await readSiteFile(file), location) : new Map();
	}

	async function checkChoices(scope: ChoiceCheckScope): Promise<void> {
		const { records, sourceFile, project } = scope;
		const externalFragments = new Map<string, Set<string>>();
		for (const record of records.values()) {
			if (!project && record.kind !== 'choice') continue;
			const target = resolveLink(record.destination.page, sourceFile);
			if (!target || scope.pages.has(target.file)) continue;
			const fragments = externalFragments.get(target.file) ?? new Set<string>();
			fragments.add(record.destination.fragment);
			externalFragments.set(target.file, fragments);
		}
		const pages = new Map(scope.pages);
		for (const file of externalFragments.keys()) pages.set(file, undefined);
		await Promise.all(
			[...pages].map(async ([file, namespace]) => {
				const { decisionChoices } = await readPage(file);
				const fragments = project ? undefined : externalFragments.get(file);
				checkDecisionChoices({
					choices: fragments
						? decisionChoices.filter(({ id }) => fragments.has(id))
						: decisionChoices,
					file,
					displayPath: label(file),
					resolveRecord(reference) {
						const qualified = reference.includes('/');
						if (qualified && !project)
							throw new Error(`Qualified reference ${reference} requires demistify project check.`);
						if (project && !namespace && !qualified)
							throw new Error(
								`${label(file)}: use a qualified decision ID on a shared page: ${reference}.`,
							);
						const id = namespace && !qualified ? `${namespace}/${reference}` : reference;
						const record = records.get(id);
						const destination = record && resolveLink(record.destination.page, sourceFile);
						if (!record || !destination)
							throw new Error(`${label(file)}: unknown design record ${id}.`);
						return { record, file: destination.file };
					},
				});
			}),
		);
	}

	async function prepare(directory: string, authoringRoot?: string): Promise<PreparedDesign> {
		const setDirectory = await resolveSiteFile(resolve(directory), authoringRoot);
		const designFile = join(setDirectory, 'design.ts');
		const declarationFile = join(setDirectory, 'design-schema.d.ts');
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
		const decisionFile = join(setDirectory, 'design-decisions.ts');
		const assumptionFile = join(setDirectory, 'design-assumptions.ts');
		const [designText, declaration, decisions, assumptions] = await Promise.all([
			readOwnedInput(designFile),
			readOptionalOwnedInput(declarationFile),
			readOptionalOwnedInput(decisionFile),
			readOptionalOwnedInput(assumptionFile),
		]);
		const { design: authored, recordSources } = await readDesign(
			designFile,
			designText,
			readOwnedInput,
		);
		const designSources = new Map(recordSources);
		if (decisions !== undefined) designSources.set(decisionFile, decisions);
		if (assumptions !== undefined) designSources.set(assumptionFile, assumptions);
		const design: DesignCore = {
			...authored,
			records: [
				...authored.records,
				...(decisions === undefined ? [] : readDesignItems(decisionFile, decisions, 'choice')),
				...(assumptions === undefined
					? []
					: readDesignItems(assumptionFile, assumptions, 'assumption')),
			],
		};
		const deliverySource = await readDeliveryEntries(
			designFile,
			designText,
			declaration,
			readOptionalOwnedInput,
			designSources,
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
		const previousBoundaries = await readPreviousBoundaries(generatedDesign, 'design');
		const recordRevisions = await inspectRecords(design.records, designFile, {
			pages: declaredPages,
			previousBoundaries,
		});
		const storedRevisions = Object.fromEntries(
			[...recordRevisions].map(([id, entry]) => [id, entry.current]),
		);
		const localRecords = new Map(design.records.map((record) => [record.id, record]));
		async function plan(
			delivery: DeliveryProjection,
			projectSource?: ProjectNavigationSource,
		): ReturnType<PreparedDesign['plan']> {
			function resolveRecord(reference: string): { record: DesignRecord; file: string } {
				const qualified = reference.includes('/');
				if (qualified && !projectSource)
					throw new Error(`Qualified reference ${reference} requires demistify project check.`);
				const record = qualified
					? projectSource?.records.get(reference)
					: localRecords.get(reference);
				if (!record) throw new Error(`${label(designFile)}: unknown design record ${reference}.`);
				const target = resolveLink(
					record.destination.page,
					qualified && projectSource ? projectSource.file : designFile,
				);
				if (!target) throw new Error(`Record ${reference} must point to a local HTML page.`);
				return { record, file: target.file };
			}
			const navigation = compileDesignNavigation(design, (reference): NavigationTarget => {
				const destination = resolveRecord(reference);
				const page = pageIdsByFile.get(destination.file);
				if (!page)
					throw new Error(
						`${label(designFile)}: declare ${label(destination.file)} in pages to navigate to ${reference}.`,
					);
				return { page, fragment: destination.record.destination.fragment };
			});
			const validatedNavigation = sharedNavigationSchema.parse(navigation);
			await Promise.all([
				...validatedNavigation.map.map(checkMapNode),
				...validatedNavigation.walkthroughs.flatMap((route) =>
					route.steps.map((step) => checkNavigationTarget(step.target)),
				),
			]);
			const snapshotRevision = createHash('sha256')
				.update(JSON.stringify([storedRevisions, navigation, delivery.deliverables]))
				.digest('hex');
			const deliverables = deliverablesViewSchema.parse({
				version: 1,
				snapshotRevision,
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
					const findings = await checkHtml(await readSiteFile(file), {
						readSource: async (path) => {
							const target = resolveLink(path, file);
							if (!target) return undefined;
							if (target.file === navigationFile) return navigationText;
							if (target.file === deliverablesFile) return deliverablesText;
							if (target.file === projectSource?.file) return projectSource.text;
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
			const outputs = (): Promise<(GeneratedFile | undefined)[]> =>
				Promise.all([
					planGeneratedJson(generatedDesign, {
						version: 1,
						snapshotRevision,
						pages: design.pages,
						records: design.records,
						recordDestinations,
						recordRevisions: storedRevisions,
						boundaryWhitespace: Object.fromEntries(
							[...recordRevisions].flatMap(([id, entry]) =>
								entry.boundaryWhitespace === undefined ? [] : [[id, entry.boundaryWhitespace]],
							),
						),
					}),
					planGeneratedJson(navigationFile, navigation, (current): boolean => {
						const parsed = sharedNavigationSchema.safeParse(current);
						return parsed.success && isDeepStrictEqual(parsed.data, validatedNavigation);
					}),
					planGeneratedJson(deliverablesFile, deliverables),
				]);
			const result = [
				details,
				`Checked ${ownedPages.size} authored page(s), ${pages.length} navigation page(s), and ${design.records.length} design record(s).`,
				...Object.values(delivery.deliverables).map(({ id, state, title, reasons }): string =>
					[`${id} · ${state} · ${title}`, ...(state === 'accepted' ? [] : reasons)].join('\n  '),
				),
			]
				.filter(Boolean)
				.join('\n');
			return { result, outputs };
		}
		return {
			directory: setDirectory,
			firstPage: { href: firstPage.href, title: firstPage.title },
			pages,
			design,
			entries,
			recordRevisions,
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
		checkChoices,
		inspectRecords,
		readScopeBoundaries: readPreviousBoundaries,
		prepare,
		verify,
	};
}

export async function checkDesignSet(directory: string, siteRoot?: string): Promise<string> {
	const setDirectory = await realpath(resolve(directory));
	const check = await openDesignCheck(siteRoot ?? setDirectory);
	const set = await check.prepare(setDirectory);
	const qualified =
		set.design.records
			.flatMap((record) => [
				...('relationships' in record ? record.relationships.map(({ targetId }) => targetId) : []),
				...(record.kind === 'choice' && record.supersedes ? [record.supersedes] : []),
			])
			.find((reference): boolean => reference.includes('/')) ??
		set.entries
			.flatMap(({ dependsOn, deliverable }) => [
				...dependsOn,
				...deliverable.scope.rules,
				...deliverable.scope.designItems,
				...deliverable.scope.verificationChecks,
			])
			.find((reference): boolean => reference.includes('/'));
	if (qualified)
		throw new Error(`Qualified reference ${qualified} requires demistify project check.`);
	checkRecordRelationships(set.design.records);
	await check.checkChoices({
		pages: new Map(set.pages.filter(({ owned }) => owned).map(({ file }) => [file, undefined])),
		records: new Map(set.design.records.map((record) => [record.id, record])),
		sourceFile: join(setDirectory, 'design.ts'),
	});
	const revisions = deliveryRevisionInputs(set.recordRevisions);
	const delivery = projectDelivery(set.design, set.entries, revisions.current, {
		legacyRecordRevisions: revisions.legacy,
	});
	const { result, outputs } = await set.plan(delivery);
	await commitDesignOutputs([set.directory], [{ verify: check.verify, plan: outputs }]);
	return result;
}
