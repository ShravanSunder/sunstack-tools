/** Independent R33/R36 checks: complete candidate identity, request shape and accounting. */
import { z } from 'zod';

import {
	arrayValue,
	equal,
	jsonObjectSchema,
	numberValue,
	objectValue,
	requireCondition,
	textValue,
	type JsonObject,
	type JsonValue,
} from './rulebook-fixture-support.ts';

export const selectionOutcomeSchema = z.discriminatedUnion('kind', [
	z.object({ kind: z.literal('selected'), yes: z.number() }),
	z.object({ kind: z.literal('not_selected'), yes: z.number() }),
	z.object({ kind: z.literal('uncertain'), yes: z.number() }),
	z.object({ kind: z.literal('incomplete'), reason: jsonObjectSchema }),
]);
const locationSchema = z
	.object({
		file: z.string(),
		first_line: z.number().int().positive(),
		last_line: z.number().int().positive(),
		span: z
			.object({
				byte_start: z.number().int().nonnegative(),
				byte_end: z.number().int().nonnegative(),
			})
			.passthrough(),
	})
	.passthrough();
export const selectionEntrySchema = z.object({
	kind: z.string(),
	where: locationSchema,
	first_line: z.string(),
	outcome: selectionOutcomeSchema,
});
export const selectionCountSchema = z.object({
	candidates: z.number().int().nonnegative(),
	asked: z.number().int().nonnegative(),
	selected: z.number().int().nonnegative(),
	not_selected: z.number().int().nonnegative(),
	uncertain: z.number().int().nonnegative(),
	incomplete: z.number().int().nonnegative(),
});
export const kindCountSchema = z.object({
	matches: z.number().int().nonnegative(),
	pieces: z.number().int().nonnegative(),
	picked: z.number().int().nonnegative(),
	limited: z.boolean(),
	selection: selectionCountSchema.nullable().optional(),
});
export const fileCountSchema = z.object({
	file: z.string(),
	kinds: z.record(z.string(), kindCountSchema),
});
export type SelectionEntry = z.infer<typeof selectionEntrySchema>;
export function selectionEntries(report: JsonObject): SelectionEntry[] {
	return z.array(selectionEntrySchema).parse(report['selections']);
}
export function requireSelectionCounts(options: {
	report: JsonObject;
	files: readonly string[];
	kinds: readonly string[];
	selectedKinds: readonly string[];
}): void {
	const counts = z.array(fileCountSchema).parse(options.report['file_counts']);
	equal(
		counts.map((row) => row.file),
		options.files.toSorted(),
		'File counts omit/reorder checked files, including zero-candidate files',
	);
	const entries = selectionEntries(options.report);
	equal(
		entries.map((entry) => [entry.where.file, entry.kind, entry.where.span.byte_start]),
		entries
			.map((entry) => [entry.where.file, entry.kind, entry.where.span.byte_start])
			.toSorted(
				(left, right) =>
					String(left[0]).localeCompare(String(right[0])) ||
					String(left[1]).localeCompare(String(right[1])) ||
					Number(left[2]) - Number(right[2]),
			),
		'Selection entries out of file/kind/range order',
	);
	const identities = entries.map((entry) =>
		JSON.stringify([
			entry.kind,
			entry.where.file,
			entry.where.span.byte_start,
			entry.where.span.byte_end,
		]),
	);
	equal(new Set(identities).size, entries.length, 'Repeated candidate identity');
	for (const file of counts) {
		equal(
			Object.keys(file.kinds).toSorted(),
			options.kinds.toSorted(),
			'File count lacks a configured kind',
		);
		for (const [kind, count] of Object.entries(file.kinds)) {
			requireCondition(
				count.matches >= count.pieces && count.picked <= count.pieces,
				'Impossible match/piece/picked counts',
			);
			const selected = options.selectedKinds.includes(kind);
			requireCondition(
				selected === Boolean(count.selection),
				'Selection counts must exist exactly for selected kinds',
			);
			if (!count.selection) continue;
			const selection = count.selection;
			equal(
				selection.selected + selection.not_selected + selection.uncertain + selection.incomplete,
				selection.asked,
				'Selection outcomes must add up to asked',
			);
			requireCondition(selection.candidates >= selection.asked, 'Asked exceeds candidates');
			equal(
				count.pieces,
				selection.selected,
				'Selected-kind pieces differ from selected candidates',
			);
			const rows = entries.filter((entry) => entry.kind === kind && entry.where.file === file.file);
			equal(rows.length, selection.asked, 'Report omits an asked candidate');
			for (const outcome of ['selected', 'not_selected', 'uncertain', 'incomplete'])
				equal(
					numberValue(objectValue(selection)[outcome]),
					rows.filter((entry) => entry.outcome.kind === outcome).length,
					'Per-file outcome count differs from candidate entries',
				);
		}
	}
}
function qualifyField(value: JsonValue, key: string, shared: readonly string[]): JsonValue {
	if (typeof value === 'string')
		return shared.includes(value) ? `shared.${value}` : `candidates.${key}.${value}`;
	if (Array.isArray(value)) return value.map((item) => qualifyField(item, key, shared));
	throw new Error('Instruction field must name one or more fields');
}
export function qualifiedSelectionQuestion(options: {
	question: JsonObject;
	key: string;
	shared: readonly string[];
}): JsonObject {
	const result = structuredClone(options.question);
	const instructions = objectValue(result['instructions']);
	for (const field of ['check', 'against', 'context']) {
		const value = instructions[field];
		if (value !== undefined) instructions[field] = qualifyField(value, options.key, options.shared);
	}
	return result;
}
export function requireSelectionRequest(options: {
	body: JsonObject;
	file: string;
	question: JsonObject;
	sharedNames: readonly string[];
	candidates: JsonObject;
	shared: JsonObject;
}): void {
	const keys = Object.keys(options.candidates);
	requireCondition(
		keys.length > 0 && keys.every((key) => /^c[1-9][0-9]*$/.test(key)),
		'Expected candidate keys',
	);
	const expectedQuestions = Object.fromEntries(
		keys.map((key) => [
			key,
			qualifiedSelectionQuestion({ question: options.question, key, shared: options.sharedNames }),
		]),
	);
	equal(
		options.body['questions'],
		expectedQuestions,
		'One unchanged qualified noul question required for every candidate',
	);
	equal(
		options.body['state'],
		{ where: { file: options.file }, candidates: options.candidates, shared: options.shared },
		'Selection state carries outside text, repeated shared fields or wrong candidate views',
	);
}
export function selectionState(body: JsonObject): {
	file: string;
	candidates: JsonObject;
	shared: JsonObject;
} {
	const state = objectValue(body['state']);
	return {
		file: textValue(objectValue(state['where'])['file']),
		candidates: objectValue(state['candidates']),
		shared: objectValue(state['shared']),
	};
}
const orderedKeys = (value: JsonObject): string[] =>
	Object.keys(value).toSorted((left, right) => Number(left.slice(1)) - Number(right.slice(1)));
export function requireCandidateKeys(options: {
	bodies: readonly JsonObject[];
	groups: readonly (readonly string[])[];
}): void {
	// Question maps may serialize lexically (c1,c10,c2); the batches must contain
	// consecutive numeric candidate identities, not a particular object-key order.

	equal(
		options.bodies.map((body) => orderedKeys(objectValue(body['questions']))),
		options.groups,
		'Request grouping differs from consecutive candidate batches',
	);
	for (const body of options.bodies)
		equal(
			orderedKeys(selectionState(body).candidates),
			orderedKeys(objectValue(body['questions'])),
			'Candidate state/question keys differ',
		);
	const groups = options.groups.flat();
	equal(groups.length, new Set(groups).size, 'Candidate sent more than once');
}
export function requireSameShared(bodies: readonly JsonObject[]): void {
	const first = bodies[0];
	requireCondition(first, 'Need at least one captured request');
	const shared = selectionState(first).shared;
	for (const body of bodies)
		equal(selectionState(body).shared, shared, 'One candidate changed the file-wide shared view');
}
export function selectionPieces(report: JsonObject, kind: string, file?: string): JsonObject[] {
	return arrayValue(report['pieces'])
		.map(objectValue)
		.filter(
			(piece) => piece['kind'] === kind && (!file || objectValue(piece['where'])['file'] === file),
		);
}
