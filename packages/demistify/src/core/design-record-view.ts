import { z } from 'zod';

import { assumptionRecord, decisionRecord } from './decision-record.ts';
import { sectionHolds } from './section-routing.ts';

export const designRecordsFilename = 'records.generated.js';

const reference = z.string().min(1);

const decisionItem = z.union([decisionRecord, assumptionRecord]);
const recordFace = z.strictObject({
	record: decisionItem,
	held: z.number().int().min(0).optional(),
	dependents: z.array(z.strictObject({ id: reference, label: reference })),
	ruledBy: z.enum(['need', 'rule', 'realization', 'planned-check']).optional(),
	replacement: reference.optional(),
});
export type RecordFace = z.infer<typeof recordFace>;

export function decisionDetailsId(placement: string, option?: string): string {
	return option ? `${placement}--option--${option}--details` : `${placement}--details`;
}

export const designRecordViewSchema = z.strictObject({
	version: z.literal(1),
	pages: z.record(
		z.string().min(1),
		z.strictObject({
			placements: z.array(
				z.strictObject({
					reference,
					statement: z.string(),
					decision: reference.optional(),
					face: recordFace.optional(),
				}),
			),
			sectionItems: z.array(z.strictObject({ reference, holds: z.enum(sectionHolds) })).optional(),
			statuses: z
				.array(z.strictObject({ reference, href: reference, record: decisionItem }))
				.optional(),
			trees: z.array(z.strictObject({ reference, markdown: z.string() })),
		}),
	),
});

export type DesignRecordView = z.infer<typeof designRecordViewSchema>;

/** Resolve the same relative data hint in the checker and the browser. */
export function recordSourceHref(source: string, filename: string): string | undefined {
	const href = source.trim();
	if (/^(?:[a-z][a-z0-9+.-]*:|\/)/iu.test(href)) return undefined;
	const path = decodeURIComponent(href.split(/[?#]/u, 1)[0] ?? '');
	if (path.startsWith('/') || path.includes('\\')) return undefined;
	const segments = path.split('/').filter(Boolean);
	if (segments.at(-1) !== filename) return undefined;
	return segments.map(encodeURIComponent).join('/');
}
