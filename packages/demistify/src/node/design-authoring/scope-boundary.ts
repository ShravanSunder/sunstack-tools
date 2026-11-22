import { createHash } from 'node:crypto';

import { z } from 'zod';

import type { DesignRecord } from './schema.ts';

const separator = /^[\t\n\f\r ]*$/;
const designBoundaryData = z.object({
	boundaryWhitespace: z.record(z.string(), z.string().regex(separator)).optional(),
});
const projectBoundaryData = z.object({
	records: z.array(
		z.object({ id: z.string(), boundaryWhitespace: z.string().regex(separator).optional() }),
	),
});

export interface RecordScopeRevision {
	readonly current: string;
	readonly original: string;
	readonly previous: string;
	readonly boundaryWhitespace?: string;
}

export function readScopeBoundaries(
	source: string,
	location: 'design' | 'project',
): ReadonlyMap<string, string> {
	let input: unknown;
	try {
		input = JSON.parse(source);
	} catch {
		return new Map();
	}
	if (location === 'design') {
		const checked = designBoundaryData.safeParse(input);
		return new Map(checked.success ? Object.entries(checked.data.boundaryWhitespace ?? {}) : []);
	}
	const checked = projectBoundaryData.safeParse(input);
	return new Map(
		checked.success
			? checked.data.records.flatMap(({ id, boundaryWhitespace }): [string, string][] =>
					boundaryWhitespace === undefined ? [] : [[id, boundaryWhitespace]],
				)
			: [],
	);
}

/** Reconstruct legacy revisions from current content and the old separator only. */
export function recordScopeRevision(options: {
	readonly record: DesignRecord;
	readonly source: string;
	readonly heading: boolean;
	readonly previousBoundary: string | undefined;
}): RecordScopeRevision {
	const { record, source, heading, previousBoundary } = options;
	const orderedRecord =
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
	const hash = (passage: string): string =>
		createHash('sha256')
			.update(JSON.stringify([orderedRecord, passage]))
			.digest('hex');
	const original = hash(source);
	if (!heading) return { current: original, original, previous: original };

	const passage = source.replace(/[\t\n\f\r ]+$/, '');
	const boundaryWhitespace = previousBoundary ?? source.slice(passage.length);
	return {
		current: hash(passage),
		original,
		previous: hash(passage + boundaryWhitespace),
		boundaryWhitespace,
	};
}

export function deliveryRevisionInputs(records: ReadonlyMap<string, RecordScopeRevision>): {
	readonly current: Map<string, string>;
	readonly legacy: readonly [Map<string, string>, Map<string, string>];
} {
	const current = new Map<string, string>();
	const original = new Map<string, string>();
	const previous = new Map<string, string>();
	for (const [id, revision] of records) {
		current.set(id, revision.current);
		original.set(id, revision.original);
		previous.set(id, revision.previous);
	}
	return { current, legacy: [original, previous] };
}
