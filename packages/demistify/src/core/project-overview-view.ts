import { z } from 'zod';

import { deliverablePlanSummarySchema, localDeliveryHref } from './deliverables-view.ts';

export const projectOverviewFilename = 'project-overview.generated.json';

const link = z.strictObject({ title: z.string().min(1), href: localDeliveryHref });

export const projectOverviewViewSchema = z.strictObject({
	version: z.literal(1),
	deliverables: z.array(deliverablePlanSummarySchema),
	documents: z.array(link),
	decisions: z.array(
		link.extend({
			status: z.enum(['Open', 'Needs review']),
			outcome: z.string().optional(),
		}),
	),
	workLogs: z.array(link.extend({ date: z.iso.date() })),
});

export type ProjectOverviewView = z.infer<typeof projectOverviewViewSchema>;

export function parseProjectOverviewView(source: string): ProjectOverviewView {
	return projectOverviewViewSchema.parse(JSON.parse(source));
}
