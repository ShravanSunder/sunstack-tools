import { z } from 'zod';

export const designRecordsFilename = 'records.generated.js';

const reference = z.string().min(1);

export const designRecordViewSchema = z.strictObject({
	version: z.literal(1),
	pages: z.record(
		z.string().min(1),
		z.strictObject({
			placements: z.array(
				z.strictObject({ reference, statement: z.string(), decision: reference.optional() }),
			),
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
	const segments = path.split('/');
	if (segments.at(-1) !== filename) return undefined;
	return segments.map(encodeURIComponent).join('/');
}
