import { dirname, relative, resolve, sep } from 'node:path';

import { isDecision } from '../../core/decision-record.ts';
import { decisionDetailsId, type DesignRecordView } from '../../core/design-record-view.ts';
import { escapeHtml } from '../../core/html-text.ts';
import type { DeliveryViews } from './delivery-model.ts';
import type { PageMarkup } from './design-page-markup.ts';
import {
	qualifyRecord,
	recordLabel,
	recordRelationships,
	recordStatement,
	type DesignRecord,
} from './schema.ts';
import { projectSectionItems } from './section-layout.ts';

export interface RecordPlacement {
	readonly file: string;
	readonly fragment: string;
}

export interface RecordProjection {
	readonly records: ReadonlyMap<string, DesignRecord>;
	readonly placements: ReadonlyMap<string, RecordPlacement>;
	readonly sectionItems: ReadonlyMap<string, ReturnType<typeof projectSectionItems>>;
}

export function relativePage(from: string, file: string): string {
	return relative(from, file).split(sep).map(encodeURIComponent).join('/');
}

export { qualifyRecord } from './schema.ts';

export function projectRecords(
	records: readonly DesignRecord[],
	pages: readonly { file: string; namespace?: string | undefined; markup: PageMarkup }[],
): RecordProjection {
	const recordsById = new Map(records.map((record) => [record.id, record]));
	if (recordsById.size !== records.length) throw new Error('Duplicate design record ID.');
	const placements = new Map<string, RecordPlacement>();
	const sectionItems = new Map<string, ReturnType<typeof projectSectionItems>>();
	const itemIds = new Set<string>();
	for (const { file, namespace, markup } of pages) {
		sectionItems.set(
			file,
			projectSectionItems(markup.sectionLayout, recordsById, file, namespace, itemIds),
		);
		for (const { reference, tag, status } of markup.placements) {
			if (status) continue;
			const id = qualifyRecord(namespace, reference);
			const record = recordsById.get(id);
			if (!record) throw new Error(`${file}: unknown design record ${id}.`);
			if (placements.has(id)) throw new Error(`${file}: duplicate placement for ${id}.`);
			if (tag === 'ex-section-item' && record.kind === 'choice')
				throw new Error(`${file}: ${id} needs exactly one native ex-choice.`);
			if (tag === 'ex-choice' && record.kind !== 'choice')
				throw new Error(`${file}: ${id} is not a choice.`);
			placements.set(id, { file, fragment: reference });
		}
	}
	return { records: recordsById, placements, sectionItems };
}

export function projectedDestinations(
	projection: RecordProjection,
	directory: string,
): Map<string, { page: string; fragment: string }> {
	return new Map(
		[...projection.placements].map(([id, { file, fragment }]) => [
			id,
			{ page: relativePage(directory, file), fragment },
		]),
	);
}

function treeLabel(
	record: DesignRecord,
	target: RecordPlacement | undefined,
	file: string,
): string {
	const label = `${record.id} · ${recordLabel(record)}`
		.replace(/\s+/gu, ' ')
		.replace(/[\\`*_{}[\]()#+.!<>|~&-]/g, '\\$&');
	if (!target) return label;
	const href = `${target.file === file ? '' : relativePage(dirname(file), target.file)}#${encodeURIComponent(target.fragment)}`;
	return `[${label}](<${href}>)`;
}

function dependentFragmentOnThisPage({
	id,
	placement,
	file,
	namespace,
	anchors,
}: {
	readonly id: string;
	readonly placement: RecordPlacement | undefined;
	readonly file: string;
	readonly namespace: string | undefined;
	readonly anchors: ReadonlySet<string>;
}): string | undefined {
	if (placement) return placement.file === file ? placement.fragment : undefined;
	const localId = id.split('/').at(-1);
	if (!localId || qualifyRecord(namespace, localId) !== id) return undefined;
	return anchors.has(localId) ? localId : undefined;
}

export function prepareRecordPage(options: {
	readonly html: string;
	readonly file: string;
	readonly namespace?: string | undefined;
	readonly markup: PageMarkup;
	readonly projection: RecordProjection;
}): { html: string; records: DesignRecordView['pages'][string] } {
	const { html, file, namespace, markup, projection } = options;
	const sectionItems = projection.sectionItems.get(file);
	const records: DesignRecordView['pages'][string] = {
		placements: [],
		trees: [],
		...(sectionItems?.length && { sectionItems }),
	};
	const edits: { start: number; end: number; text: string }[] = [];
	for (const {
		reference,
		tag,
		startTag,
		endTag,
		options: authoredOptions,
		status,
	} of markup.placements) {
		const id = qualifyRecord(namespace, reference);
		const record = projection.records.get(id);
		if (!record) throw new Error(`${file}: unknown design record ${id}.`);
		const anchor = ` id="${escapeHtml(reference)}"`;
		const boundRecord =
			isDecision(record) || record.kind === 'assumption'
				? { ...record, id: record.id.split('/').at(-1) ?? record.id }
				: undefined;
		const statement =
			record.kind === 'choice' && tag === 'ex-choice' ? record.question : recordStatement(record);
		const text = escapeHtml(statement);
		const target = projection.placements.get(id);
		if (status) {
			if (!boundRecord || !target)
				throw new Error(`${file}: status needs a placed decision or assumption: ${id}.`);
			const href = `${target.file === file ? '' : relativePage(dirname(file), target.file)}#${encodeURIComponent(target.fragment)}`;
			(records.statuses ??= []).push({ reference, href, record: boundRecord });
			continue;
		}
		const dependents = boundRecord
			? [...projection.records.values()].filter((candidate) =>
					recordRelationships(candidate).some(({ targetId }) => targetId === id),
				)
			: [];
		const ruledBy = (['need', 'rule', 'realization', 'planned-check'] as const).find((kind) =>
			dependents.some((item) => item.kind === kind),
		);
		const replacement =
			boundRecord &&
			[...projection.records.values()].find(
				(candidate) => isDecision(candidate) && candidate.supersedes === id,
			);
		records.placements.push({
			reference,
			statement,
			...(tag === 'ex-choice' && { decision: id }),
			...(boundRecord &&
				(tag === 'ex-choice' || tag === 'ex-section-item') && {
					face: {
						record: boundRecord,
						dependents: dependents.map((item) => {
							const fragment = dependentFragmentOnThisPage({
								id: item.id,
								placement: projection.placements.get(item.id),
								file,
								namespace,
								anchors: markup.ids,
							});
							const label = recordLabel(item);
							return fragment ? { id: item.id, label, fragment } : { id: item.id, label };
						}),
						...(ruledBy && { ruledBy }),
						...(replacement && { replacement: replacement.id }),
					},
				}),
		});
		if (tag === 'ex-choice' && record.kind === 'choice') {
			const unmatchedOptions = new Map(record.options.map((option) => [option.id, option]));
			for (const authored of authoredOptions ?? []) {
				const option = unmatchedOptions.get(authored.id);
				if (!option)
					throw new Error(
						`${file}: ${id} Details or option markup names unknown option ${authored.id}.`,
					);
				unmatchedOptions.delete(authored.id);
				edits.push({
					start: authored.start,
					end: authored.end,
					text: `<ex-option id="${escapeHtml(reference)}--${escapeHtml(option.id)}" title="${escapeHtml(option.title)}">`,
				});
			}
			if (unmatchedOptions.size)
				throw new Error(
					`${file}: ${id} needs markup for option ${[...unmatchedOptions.keys()].join(', ')}.`,
				);
			edits.push({
				start: startTag.end - 1,
				end: startTag.end - 1,
				text: `${anchor} data-decision="${escapeHtml(id)}" title="${text}"`,
			});
			const targets = [
				decisionDetailsId(reference),
				...record.options.map((option) => decisionDetailsId(reference, option.id)),
			];
			edits.push({
				start: endTag.end,
				end: endTag.end,
				text: targets.map((value) => `<span id="${escapeHtml(value)}" hidden></span>`).join(''),
			});
		} else if (tag === 'ex-section-item' && boundRecord) {
			edits.push({
				start: startTag.end - 1,
				end: startTag.end - 1,
				text: anchor,
			});
			edits.push({
				start: endTag.end,
				end: endTag.end,
				text: `<span id="${escapeHtml(decisionDetailsId(reference))}" hidden></span>`,
			});
		} else {
			edits.push({ start: startTag.end - 1, end: endTag.start, text: `${anchor}>${text}` });
		}
	}
	for (const { reference, startTag, endTag } of markup.trees) {
		const id = qualifyRecord(namespace, reference);
		const need = projection.records.get(id);
		if (need?.kind !== 'need') throw new Error(`${file}: Tree needs a known need: ${id}.`);
		const label = (record: DesignRecord): string =>
			treeLabel(record, projection.placements.get(record.id), file);
		const children = [...projection.records.values()].filter(
			(candidate): boolean =>
				candidate.kind === 'rule' &&
				recordRelationships(candidate).some(
					({ relation, targetId }) => relation === 'serves' && targetId === id,
				),
		);
		const markdown = [
			`- ${label(need)}`,
			...children.map((rule): string => `  - ${label(rule)}`),
		].join('\n');
		records.trees.push({ reference, markdown });
		edits.push({
			start: startTag.end,
			end: endTag.start,
			text: `<template>${escapeHtml(markdown)}</template>`,
		});
	}
	let prepared = html;
	for (const { start, end, text } of edits.toSorted(
		(left, right): number => right.start - left.start,
	))
		prepared = `${prepared.slice(0, start)}${text}${prepared.slice(end)}`;
	return { html: prepared, records };
}

export function attachDecisionDeliverables(
	outputs: ReadonlyMap<string, DesignRecordView>,
	delivery: DeliveryViews,
	deliveryRoot: string,
	projection: RecordProjection,
): void {
	const pageRecords = new Map<string, Map<string, string>>();
	for (const [id, { file, fragment }] of projection.placements) {
		const records = pageRecords.get(file) ?? new Map<string, string>();
		records.set(fragment, id);
		pageRecords.set(file, records);
	}
	for (const [outputFile, output] of outputs) {
		for (const [pagePath, page] of Object.entries(output.pages)) {
			const pageFile = resolve(dirname(outputFile), decodeURIComponent(pagePath));
			for (const { face, reference } of page.placements) {
				const id = pageRecords.get(pageFile)?.get(reference);
				if (!face || !id) continue;
				const related = Object.values(delivery.deliverables).filter(
					(item) =>
						item.kind === 'core' && item.links.governingItems.some((entry) => entry.id === id),
				);
				face.held = related.filter((item) => item.state === 'design-blocked').length;
				for (const item of related) {
					if (item.kind !== 'core') continue;
					const { page: target, fragment } = item.links.plan;
					face.dependents.push(
						fragment && resolve(deliveryRoot, decodeURIComponent(target)) === pageFile
							? { id: item.id, label: item.title, fragment }
							: { id: item.id, label: item.title },
					);
				}
			}
		}
	}
}
