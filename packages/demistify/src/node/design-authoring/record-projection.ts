import { dirname, relative, sep } from 'node:path';

import type { DesignRecordView } from '../../core/design-record-view.ts';
import { escapeHtml } from '../../core/html-text.ts';
import type { PageMarkup } from './design-page-markup.ts';
import { recordLabel, recordRelationships, recordStatement, type DesignRecord } from './schema.ts';

export interface RecordPlacement {
	readonly file: string;
	readonly fragment: string;
}

export interface RecordProjection {
	readonly records: ReadonlyMap<string, DesignRecord>;
	readonly placements: ReadonlyMap<string, RecordPlacement>;
}

export function relativePage(from: string, file: string): string {
	return relative(from, file).split(sep).map(encodeURIComponent).join('/');
}

export function qualifyRecord(namespace: string | undefined, reference: string): string {
	return namespace && !reference.includes('/') ? `${namespace}/${reference}` : reference;
}

export function projectRecords(
	records: readonly DesignRecord[],
	pages: readonly { file: string; namespace?: string | undefined; markup: PageMarkup }[],
): RecordProjection {
	const recordsById = new Map(records.map((record) => [record.id, record]));
	if (recordsById.size !== records.length) throw new Error('Duplicate design record ID.');
	const placements = new Map<string, RecordPlacement>();
	for (const { file, namespace, markup } of pages) {
		for (const { reference, tag } of markup.placements) {
			const id = qualifyRecord(namespace, reference);
			const record = recordsById.get(id);
			if (!record) throw new Error(`${file}: unknown design record ${id}.`);
			if (placements.has(id)) throw new Error(`${file}: duplicate placement for ${id}.`);
			if (
				tag === 'ex-choice' &&
				(record.kind !== 'choice' || record.authority.state !== 'owner-open')
			)
				throw new Error(`${file}: Choice ${id} must bind to an open owner decision.`);
			placements.set(id, { file, fragment: reference });
		}
	}
	return { records: recordsById, placements };
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

export function prepareRecordPage(options: {
	readonly html: string;
	readonly file: string;
	readonly namespace?: string | undefined;
	readonly markup: PageMarkup;
	readonly projection: RecordProjection;
}): { html: string; records: DesignRecordView['pages'][string] } {
	const { html, file, namespace, markup, projection } = options;
	const records: DesignRecordView['pages'][string] = { placements: [], trees: [] };
	const edits: { start: number; end: number; text: string }[] = [];
	for (const { reference, tag, startTag, endTag } of markup.placements) {
		const id = qualifyRecord(namespace, reference);
		const record = projection.records.get(id);
		if (!record) throw new Error(`${file}: unknown design record ${id}.`);
		const anchor = ` id="${escapeHtml(reference)}"`;
		const statement = recordStatement(record);
		const text = escapeHtml(statement);
		records.placements.push({ reference, statement, ...(tag === 'ex-choice' && { decision: id }) });
		if (tag === 'ex-choice') {
			edits.push({
				start: startTag.end - 1,
				end: startTag.end - 1,
				text: `${anchor} data-decision="${escapeHtml(id)}" title="${text}"`,
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
