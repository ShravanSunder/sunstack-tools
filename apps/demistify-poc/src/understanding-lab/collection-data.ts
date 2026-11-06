import { z } from 'zod';

const identity = z.string().regex(/^[A-Za-z][A-Za-z0-9-]*$/);
const body = { bodyRef: identity, summaryRef: identity };
const text = z.string().min(1);
export const outcomeSchema = z.enum(['Pass', 'Fail', 'Partial', 'Not run']);
export type ScenarioOutcome = z.infer<typeof outcomeSchema>;
export const frameSchema = z.strictObject({
	kind: z.literal('frame'),
	id: identity,
	name: text,
	source: text,
	responsibility: text,
	statement: text,
	requirement: identity.optional(),
	category: z.enum(['Input', 'Resolution', 'Action', 'Plumbing']),
	supporting: z.boolean().optional(),
	branch: text.optional(),
});
export const recordSchema = z.discriminatedUnion('kind', [
	z.strictObject({ kind: z.literal('need'), id: identity, title: text, capability: text, ...body }),
	z.strictObject({
		kind: z.literal('specification'),
		id: identity,
		title: text,
		needs: z.array(identity).min(1),
		...body,
	}),
	z.strictObject({
		kind: z.literal('program-design'),
		id: identity,
		title: text,
		specifications: z.array(identity).min(1),
		...body,
	}),
	z.strictObject({
		kind: z.literal('scenario'),
		id: identity,
		title: text,
		specifications: z.array(identity).min(1),
		...body,
	}),
	z.strictObject({
		kind: z.literal('observation'),
		id: identity,
		scenario: identity,
		outcome: outcomeSchema.exclude(['Not run']),
		...body,
	}),
	frameSchema,
	z.strictObject({
		kind: z.literal('stack'),
		id: identity,
		title: text,
		description: text,
		primaryNeed: identity,
		frames: z.array(identity).min(1),
	}),
]);
export type AuthoredRecord = z.infer<typeof recordSchema>;
export const preparedRecordSchema = z.strictObject({
	record: recordSchema,
	page: text,
	summary: z.string(),
	bodyHtml: z.string(),
});
export const collectionSchema = z.strictObject({
	identity: text,
	records: z.array(preparedRecordSchema),
});
export type PreparedCollection = z.infer<typeof collectionSchema>;
export type PreparedRecord = z.infer<typeof preparedRecordSchema>;

/** Only the build-prepared, validated local snapshot supplies browser facts. */
export function readCollection(): PreparedCollection {
	const source = document.querySelector('script[data-lab-collection]');
	if (!source?.textContent)
		throw new Error(
			'Local data unavailable. Prepare the understanding lab before opening this page.',
		);
	const raw: unknown = JSON.parse(source.textContent);
	return collectionSchema.parse(raw);
}
