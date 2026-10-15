import { z } from 'zod';

export const explanationSchema = z.strictObject({
	title: z.string(),
	subtitle: z.string(),
	summary: z.string(),
	decision: z.string(),
	risk: z.string(),
	alternatives: z.array(z.strictObject({ name: z.string(), gain: z.string(), cost: z.string() })),
	phases: z.array(z.strictObject({ title: z.string(), body: z.string() })),
});
export type ExplanationFixture = z.infer<typeof explanationSchema>;
