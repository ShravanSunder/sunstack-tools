import type { z } from 'zod';

import type { frameSchema } from './collection-data.ts';
import { readCollection, type PreparedCollection, type PreparedRecord } from './collection-data.ts';
import { recordsForNeed } from './requirement-stages.ts';
export interface RequirementExample {
	readonly id: string;
	readonly capability: string;
	readonly title: string;
	readonly need: string;
	readonly owner: string;
}

export interface OwnerExample {
	readonly id: string;
	readonly name: string;
	readonly category: string;
	readonly role: 'boundary' | 'store' | 'state';
	readonly icon: string;
	readonly description: string;
}
export const owners: readonly OwnerExample[] = [
	{
		id: 'gateway',
		name: 'Gateway',
		category: 'Boundary',
		role: 'boundary',
		icon: 'globe',
		description: 'Routes reconnect requests.',
	},
	{
		id: 'registry',
		name: 'Registry',
		category: 'Store',
		role: 'store',
		icon: 'database',
		description: 'Owns session lookup.',
	},
	{
		id: 'session',
		name: 'Session',
		category: 'State',
		role: 'state',
		icon: 'cpu',
		description: 'Retains running state.',
	},
];

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
export function deriveStudyModel(collection: PreparedCollection): {
	requirements: readonly RequirementExample[];
	callStacks: readonly CallStackExample[];
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
	return { requirements, callStacks };
}
export function loadStudyModel(): ReturnType<typeof deriveStudyModel> {
	return deriveStudyModel(readCollection());
}
