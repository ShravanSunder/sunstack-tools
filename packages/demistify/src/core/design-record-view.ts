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
