import type { z } from 'zod';

import type { frameSchema } from './collection.ts';
import { readCollection, type PreparedCollection, type PreparedRecord } from './collection.ts';
import { recordsForNeed } from './requirement-stages.ts';
export interface RequirementExample {
	readonly id: string;
	readonly capability: string;
	readonly title: string;
	readonly need: string;
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
	requirements: readonly RequirementExample[];
	callStacks: readonly CallStackExample[];
	entry: {
		readonly needId: string;
		readonly calls:
			| { readonly kind: 'empty' }
			| { readonly kind: 'available'; readonly stackId: string; readonly frameId: string };
	};
} {
	const requirements = collection.records.flatMap((item): RequirementExample[] => {
		const record = item.record;
		if (record.kind !== 'need') return [];
		return [
			{
				id: record.id,
				title: record.title,
				capability: record.capability,
				need: item.summary,
				owner: summary(recordsForNeed(collection, record.id, 'program-design')),
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
	const need = requirements[0];
	const stack = callStacks[0];
	const frame = stack?.frames[0];
	if (!need) throw new Error('A reader collection needs at least one user need.');
	return {
		requirements,
		callStacks,
		entry: {
			needId: need.id,
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
