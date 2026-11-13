import type { z } from 'zod';

import type { frameSchema } from './collection.ts';
import { readCollection, type PreparedCollection, type PreparedRecord } from './collection.ts';
export interface NeedExample {
	readonly id: string;
	readonly owner: string;
}

export type FrameExample = z.infer<typeof frameSchema>;

export interface CallStackExample {
	readonly id: string;
	readonly title: string;
	readonly description: string;
	readonly primaryNeed: string;
	readonly frames: readonly FrameExample[];
}

const summary = (records: readonly PreparedRecord[]): string =>
	records
		.map(
			(item) =>
				`${item.record.id} · ${'title' in item.record ? item.record.title : ''} · ${item.summary}`,
		)
		.join(' ');

/** Existing consumers are views of the prepared records, never a second authored corpus. */
export function deriveDocumentModel(collection: PreparedCollection): {
	needs: readonly NeedExample[];
	callStacks: readonly CallStackExample[];
	entry: {
		readonly calls:
			| { readonly kind: 'empty' }
			| { readonly kind: 'available'; readonly stackId: string; readonly frameId: string };
	};
} {
	const needs = collection.records.flatMap((item): NeedExample[] => {
		const record = item.record;
		if (record.kind !== 'need') return [];
		const specificationIds = new Set(
			collection.records.flatMap(({ record: spec }) =>
				spec.kind === 'specification' && spec.needs.includes(record.id) ? [spec.id] : [],
			),
		);
		return [
			{
				id: record.id,
				owner: summary(
					collection.records.filter(
						({ record: design }) =>
							design.kind === 'program-design' &&
							design.specifications.some((id) => specificationIds.has(id)),
					),
				),
			},
		];
	});
	const callStacks = collection.records.flatMap(({ record }): CallStackExample[] => {
		if (record.kind !== 'stack') return [];
		const frames = record.frames.map((id) => {
			const frame = collection.records.find((item) => item.record.id === id)?.record;
			if (frame?.kind !== 'frame') throw new Error('Local data missing frame ' + id);
			return frame;
		});
		return [
			{
				id: record.id,
				title: record.title,
				description: record.description,
				primaryNeed: record.primaryNeed,
				frames,
			},
		];
	});
	const need = needs[0];
	const stack = callStacks[0];
	const frame = stack?.frames[0];
	if (!need) throw new Error('A reader collection needs at least one user need.');
	return {
		needs,
		callStacks,
		entry: {
			calls:
				stack && frame
					? { kind: 'available', stackId: stack.id, frameId: frame.id }
					: { kind: 'empty' },
		},
	};
}
export function loadDocumentModel(): ReturnType<typeof deriveDocumentModel> {
	return deriveDocumentModel(readCollection());
}
