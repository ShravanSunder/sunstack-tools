import { readFile, realpath } from 'node:fs/promises';
import { basename, dirname, extname, join, relative, resolve } from 'node:path';

import { isDecision } from '../../core/decision-record.ts';
import { deliverablesViewSchema } from '../../core/deliverables-view.ts';
import { designRecordsFilename, type DesignRecordView } from '../../core/design-record-view.ts';
import type { NavigationTarget, SharedMapNode } from '../../core/shared-navigation.ts';
import { checkHtml, documentFragmentIds, formatFindings } from '../check.ts';
import { containsPath } from '../local-site.ts';
import {
	designCheckNotes,
	parseDeliveryEntries,
	placeholderWarnings,
	projectDelivery,
	type DeliveryEntry,
	type DeliveryProjection,
	type DeliveryViews,
} from './delivery-model.ts';
import { declaredDesignPagePath, resolveDesignLink } from './design-location.ts';
import { compileDesignNavigation } from './design-navigation.ts';
import {
	commitDesignOutputs,
	generatedJsonIsCurrent,
	generatedTextIsCurrent,
	planGeneratedJson,
	planGeneratedText,
} from './design-output.ts';
import {
	normalizeExcerpt,
	passageText,
	readPageMarkup,
	type PageMarkup,
} from './design-page-markup.ts';
import {
	designIgnoreDirectory,
	generatedDesignPaths,
	planDesignIgnore,
} from './generated-ignore.ts';
import { readDeliveryEntries } from './read-delivery.ts';
import { readDesignSources } from './read-design.ts';
import { putPageRecords, recordOutputFile, recordRegistrationScript } from './record-output.ts';
import {
	prepareRecordPage,
	attachDecisionDeliverables,
	projectedDestinations,
	projectRecords,
	type RecordProjection,
} from './record-projection.ts';
import { recordRelationships, type DesignCore, type DesignRecord } from './schema.ts';

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

export function checkRecordRelationships(
	records: readonly DesignRecord[],
): ReadonlyMap<string, DesignRecord> {
	const recordsById = new Map(records.map((record) => [record.id, record]));
	if (recordsById.size !== records.length) throw new Error('Duplicate design record ID.');
	const replacements = new Map<string, string>();
	for (const record of records) {
		if (record.kind === 'choice') {
			const optionIds = new Set(record.options.map(({ id }) => id));
			if (optionIds.size !== record.options.length)
				throw new Error(`${record.id}: option IDs must be unique.`);
			if (record.recommended !== 'none' && !optionIds.has(record.recommended.option))
				throw new Error(`${record.id}: recommendation names an unknown option.`);
			if (record.authority.state !== 'owner-open' && !optionIds.has(record.authority.chosen))
				throw new Error(`${record.id}: chosen option is unknown.`);
		}
		if (!isDecision(record) || !record.supersedes) continue;
		const previous = recordsById.get(record.supersedes);
		if (!isDecision(previous))
			throw new Error(`${record.id} supersedes an unknown decision: ${record.supersedes}.`);
		if (record.authority.state === 'owner-open')
			throw new Error(
				`${record.id}: settle the owner question before superseding ${record.supersedes}.`,
			);
		if (previous.authority.state !== 'agent-decided' && record.authority.state !== 'owner-decided')
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
			const target = recordsById.get(link.targetId);
			if (link.relation === 'governed-by' ? !isDecision(target) : target?.kind !== kind)
				throw new Error(
					`${record.id} ${link.relation} must resolve to a ${kind}: ${link.targetId}`,
				);
		}
	}
	for (const record of records) {
		const weight =
			record.kind === 'assumption'
				? record.weight
				: isDecision(record) && record.authority.state === 'agent-decided'
					? record.authority.weight
					: 'design';
		if (weight === 'design') continue;
		const governsRequirement = records.some(
			(dependent) =>
				(dependent.kind === 'need' || dependent.kind === 'rule') &&
				recordRelationships(dependent).some(({ targetId }) => targetId === record.id),
		);
		if (governsRequirement)
			throw new Error(
				`${record.id}: a record that governs a need or rule must have design weight.`,
			);
	}
	return recordsById;
}

export interface LoadedDesign {
	readonly directory: string;
	readonly designFile: string;
	readonly lead?: string;
	readonly design: DesignCore;
	readonly entries: readonly DeliveryEntry[];
	readonly checkTypes: (knownRecordIds: readonly string[]) => void;
}

interface PreparedDesign {
	readonly firstPage: { readonly href: string; readonly title: string };
	readonly pages: readonly { file: string; owned: boolean }[];
	readonly plan: (delivery: DeliveryViews, scope: PlanDesignScope) => Promise<DesignPlan>;
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
	/** Advice the check found, also printed in `result` as `Warning:` lines. */
	readonly warnings: readonly string[];
	readonly preparedPages: ReadonlyMap<string, string>;
	readonly design: DesignCore;
	readonly entries: readonly DeliveryEntry[];
	readonly delivery: DeliveryProjection;
	/** Record ID to the page and fragment that place it. */
	readonly placements: RecordProjection['placements'];
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
		outputDirectories: readonly string[],
	) => Promise<{
		readonly projection: RecordProjection;
		readonly preparedPages: ReadonlyMap<string, string>;
		readonly recordOutputs: ReadonlyMap<string, DesignRecordView>;
	}>;
	readonly load: (directory: string, authoringRoot?: string) => Promise<LoadedDesign>;
	readonly prepare: (set: LoadedDesign) => Promise<PreparedDesign>;
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

	function resolveLink(href: string, from: string): ReturnType<typeof resolveDesignLink> {
		return resolveDesignLink(href, from, siteDirectory);
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
					isDecision(record) || record.kind === 'assumption' ? (record.sources ?? []) : [];
				const resolution =
					record.kind === 'assumption' && record.status !== 'open' ? record.resolution : undefined;
				const basis =
					isDecision(record) && 'source' in record.authority ? record.authority.source : undefined;
				return [...sources, ...(resolution ? [resolution] : []), ...(basis ? [basis] : [])].map(
					async (source) => {
						const target = resolveLink(source.href, sourceFile);
						if (!target) throw new Error(`Source reference must be local: ${source.href}`);
						// With `shown`, the record owns the owner's words; `href` only says where.
						const pageVerbatim = source.verbatim === true && source.shown === undefined;
						await checkDestination(target.file, target.fragment, pageVerbatim);
						if (!pageVerbatim) return;
						if (!isHtml(target.file))
							throw new Error(
								`${record.id}: verbatim excerpts require an HTML source: ${source.href}`,
							);
						const text = passageText(await readSiteFile(target.file), target.fragment || undefined);
						if (!text.includes(normalizeExcerpt(source.excerpt)))
							throw new Error(`${record.id}: verbatim excerpt does not appear in ${source.href}.`);
					},
				);
			}),
		);
	}

	async function projectPages(
		records: readonly DesignRecord[],
		pages: ReadonlyMap<string, string | undefined>,
		outputDirectories: readonly string[],
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
		const preparedPages = new Map<string, string>();
		const recordOutputs = new Map<string, DesignRecordView>(
			outputDirectories.map((directory) => [
				join(directory, designRecordsFilename),
				{ version: 1, pages: {} },
			]),
		);
		for (const page of authored) {
			const prepared = prepareRecordPage({ ...page, projection });
			preparedPages.set(page.file, prepared.html);
			if (
				prepared.records.placements.length === 0 &&
				prepared.records.trees.length === 0 &&
				!prepared.records.statuses?.length &&
				!prepared.records.sectionItems?.length
			)
				continue;
			const output = recordOutputFile(page.markup, page.file, siteDirectory, recordOutputs);
			const view = recordOutputs.get(output);
			if (!view) throw new Error('No registered record output at ' + output);
			putPageRecords(view, output, page.file, prepared.records);
		}
		return { projection, preparedPages, recordOutputs };
	}

	async function load(directory: string, authoringRoot?: string): Promise<LoadedDesign> {
		const setDirectory = await resolveSiteFile(resolve(directory), authoringRoot);
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
			authoringRoot ?? setDirectory,
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
		return {
			directory: setDirectory,
			designFile,
			design,
			entries,
			checkTypes: deliverySource.checkTypes,
			...(deliverySource.lead === undefined ? {} : { lead: deliverySource.lead }),
		};
	}

	async function prepare({
		directory: setDirectory,
		design,
		designFile,
	}: LoadedDesign): Promise<PreparedDesign> {
		const navigationFile = join(setDirectory, 'navigation.generated.json');
		const deliverablesFile = join(setDirectory, 'deliverables.generated.json');
		const pages = await Promise.all(
			design.pages.map(async (page) => {
				const file = declaredDesignPagePath(page.href, designFile, siteDirectory);
				await checkDestination(file, '');
				const owned =
					containsPath(setDirectory, file) &&
					containsPath(setDirectory, await resolveSiteFile(file));
				return { id: page.id, href: page.href, title: page.title, file, owned };
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
			delivery: DeliveryViews,
			{ projection, preparedPages, namespace, projectSource }: PlanDesignScope,
		): Promise<DesignPlan> {
			const navigation = compileDesignNavigation(
				design,
				{ source: label(designFile), records: projection.records, namespace },
				(id) => {
					const destination = projection.placements.get(id);
					if (!destination) return undefined;
					const page = pageIdsByFile.get(destination.file);
					if (!page)
						throw new Error(
							`${label(designFile)}: declare ${label(destination.file)} in pages to navigate to ${id}.`,
						);
					return { page, fragment: destination.fragment };
				},
			);
			await Promise.all([
				...navigation.map.map(checkMapNode),
				...navigation.walkthroughs.flatMap((route) =>
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
			firstPage: { href: firstPage.href, title: firstPage.title },
			pages,
			plan,
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
		load,
		prepare,
		verify,
	};
}

export async function loadDesignSet(
	directory: string,
	siteRoot?: string,
): Promise<LoadedDesign & { readonly check: DesignCheck }> {
	const setDirectory = await realpath(resolve(directory));
	const check = await openDesignCheck(siteRoot ?? setDirectory);
	const set = await check.load(setDirectory);
	const qualified =
		set.design.records
			.flatMap((record) => [
				...recordRelationships(record).map(({ targetId }) => targetId),
				...(isDecision(record) && record.supersedes ? [record.supersedes] : []),
			])
			.find((reference): boolean => reference.includes('/')) ??
		set.entries
			.flatMap(({ dependsOn, deliverable }) => [
				...dependsOn,
				...(deliverable.kind === 'core'
					? [
							...deliverable.scope.needs,
							...deliverable.scope.rules,
							...deliverable.scope.designItems,
							...deliverable.scope.verificationChecks,
						]
					: []),
			])
			.find((reference): boolean => reference.includes('/'));
	if (qualified)
		throw new Error(`Qualified reference ${qualified} requires demistify initiative check.`);
	set.checkTypes(set.design.records.map(({ id }) => id));
	const records = checkRecordRelationships(set.design.records);
	compileDesignNavigation(set.design, { source: set.designFile, records });
	return { ...set, check };
}

export async function buildDesignSet(
	directory: string,
	options: { readonly siteRoot?: string | undefined; readonly write?: boolean | undefined } = {},
): Promise<CheckedDesignBuild> {
	const set = await loadDesignSet(directory, options.siteRoot);
	const { check, design, entries } = set;
	const reader = await check.prepare(set);
	const { projection, preparedPages, recordOutputs } = await check.projectPages(
		design.records,
		new Map(reader.pages.map(({ file }) => [file, undefined])),
		[set.directory],
	);
	const delivery = projectDelivery(
		design,
		entries,
		projectedDestinations(projection, set.directory),
	);
	attachDecisionDeliverables(recordOutputs, delivery, set.directory, projection);
	const plan = await reader.plan(delivery, { projection, preparedPages });
	if (options.write)
		await writeDesignBuild(
			[set.directory],
			check.verify,
			plan.generated,
			recordOutputs,
			await designIgnoreDirectory(set.directory),
		);
	else await check.verify();
	const warnings = placeholderWarnings(design.records);
	return {
		...plan,
		result: [
			plan.result,
			...warnings.map((warning) => `Warning: ${warning}`),
			...(options.write ? [] : await generatedViewNotes('design', plan.generated, recordOutputs)),
			...designCheckNotes(design.records, delivery),
		].join('\n'),
		warnings,
		design,
		entries,
		preparedPages,
		delivery,
		placements: projection.placements,
	};
}

export async function generatedViewNotes(
	kind: 'design' | 'initiative',
	generated: ReadonlyMap<string, unknown>,
	recordOutputs: ReadonlyMap<string, DesignRecordView>,
): Promise<string[]> {
	const current = await Promise.all([
		...[...generated].map(([file, value]) => generatedJsonIsCurrent(file, value)),
		...[...recordOutputs].map(([file, view]) =>
			generatedTextIsCurrent(
				file,
				recordRegistrationScript(view),
				Object.keys(view.pages).length === 0,
			),
		),
	]);
	return current.every(Boolean)
		? []
		: [
				`Generated views are missing or stale. Run demistify ${kind} check --write to refresh them.`,
			];
}

export async function writeDesignBuild(
	directories: readonly string[],
	verify: () => Promise<void>,
	generated: ReadonlyMap<string, unknown>,
	recordOutputs: ReadonlyMap<string, DesignRecordView>,
	ignoreDirectory: string,
): Promise<void> {
	const generatedPaths = generatedDesignPaths(
		[...generated.keys()]
			.filter((file) => basename(file) === 'design.generated.json')
			.map((file) => dirname(file)),
		[...generated.keys(), ...recordOutputs.keys()],
	);
	await commitDesignOutputs(
		[...directories, ignoreDirectory],
		[
			{
				verify,
				plan: () =>
					Promise.all([
						planDesignIgnore(ignoreDirectory, generatedPaths),
						...[...generated].map(([file, value]) => planGeneratedJson(file, value)),
						...[...recordOutputs].map(([file, view]) =>
							planGeneratedText(file, recordRegistrationScript(view), {
								omitIfAbsent: Object.keys(view.pages).length === 0,
							}),
						),
					]),
			},
		],
	);
}

export async function checkDesignSet(
	directory: string,
	siteRoot?: string,
	write = false,
): Promise<string> {
	return (await buildDesignSet(directory, { siteRoot, write })).result;
}
