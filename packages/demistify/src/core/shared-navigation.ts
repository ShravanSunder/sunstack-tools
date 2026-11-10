import { z } from 'zod';

import { searchDestinationSchema } from './document-search.ts';
import { subjectIcons } from './subject-icons.ts';

const navigationId = z.string().regex(/^[A-Za-z][A-Za-z0-9-]*$/);
const navigationIcon = z.enum(subjectIcons);
const navigationTargetSchema = z.strictObject({
	page: navigationId,
	fragment: z.string().min(1).optional(),
});

const sharedMapNodeSchema = z.strictObject({
	title: z.string().min(1),
	description: z.string().min(1),
	icon: navigationIcon,
	target: navigationTargetSchema.optional(),
	get children(): z.ZodDefault<z.ZodArray<typeof sharedMapNodeSchema>> {
		return z.array(sharedMapNodeSchema).default([]);
	},
});

/** One source owns collection menus, semantic relationships and ordered explanatory paths. */
export const sharedNavigationSchema = z
	.strictObject({
		version: z.literal(1),
		pages: z
			.array(
				z.strictObject({
					id: navigationId,
					title: z.string().min(1),
					href: searchDestinationSchema.shape.href,
					icon: navigationIcon,
				}),
			)
			.min(1),
		map: z.array(sharedMapNodeSchema).default([]),
		walkthroughs: z
			.array(
				z.strictObject({
					id: navigationId,
					title: z.string().min(1),
					description: z.string().min(1),
					steps: z
						.array(
							z.strictObject({
								target: navigationTargetSchema,
								narration: z.string().min(1),
								icon: navigationIcon.optional(),
							}),
						)
						.min(1),
				}),
			)
			.default([]),
	})
	.superRefine((navigation, context): void => {
		const pageIds = new Set<string>();
		for (const page of navigation.pages) {
			if (pageIds.has(page.id))
				context.addIssue({ code: 'custom', message: `Duplicate page id: ${page.id}` });
			pageIds.add(page.id);
		}
		const checkTarget = (target: z.infer<typeof navigationTargetSchema>): void => {
			if (!pageIds.has(target.page))
				context.addIssue({ code: 'custom', message: `Unknown page id: ${target.page}` });
		};
		const checkNode = (node: z.infer<typeof sharedMapNodeSchema>): void => {
			if (node.target) checkTarget(node.target);
			for (const child of node.children) checkNode(child);
		};
		for (const node of navigation.map) checkNode(node);
		const routeIds = new Set<string>();
		for (const route of navigation.walkthroughs) {
			if (routeIds.has(route.id))
				context.addIssue({ code: 'custom', message: `Duplicate walkthrough id: ${route.id}` });
			routeIds.add(route.id);
			for (const step of route.steps) checkTarget(step.target);
		}
	});

export type SharedNavigation = z.infer<typeof sharedNavigationSchema>;
export type SharedMapNode = z.infer<typeof sharedMapNodeSchema>;
export type NavigationTarget = z.infer<typeof navigationTargetSchema>;

/** Private row references connect collection search to the live, filtered Map. */
export function sharedMapReference(path: readonly number[]): string {
	return `ex-shared-map-${path.join('-')}`;
}

/** Page paths are resolved beside the shared source, never beside whichever peer is open. */
export function navigationTargetHref(
	navigation: SharedNavigation,
	target: NavigationTarget,
	sourceUrl: string,
): string {
	const page = navigation.pages.find((entry): boolean => entry.id === target.page);
	if (!page) throw new Error(`Unknown page id: ${target.page}`);
	const address = new URL(page.href, sourceUrl);
	if (target.fragment) address.hash = encodeURIComponent(target.fragment);
	return address.href;
}
