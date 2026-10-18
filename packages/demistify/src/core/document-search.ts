import { z } from 'zod';

function navigableHref(value: string): boolean {
	try {
		return ['http:', 'https:', 'file:'].includes(
			new URL(value, 'https://document.invalid/').protocol,
		);
	} catch {
		return false;
	}
}

/** The authoring index and browser share one destination shape; document content owns its labels. */
export const searchDestinationSchema = z.strictObject({
	kind: z.enum(['page', 'heading', 'walkthrough', 'journey']),
	title: z.string().min(1),
	pageTitle: z.string().min(1),
	ancestors: z.array(z.string().min(1)).readonly(),
	href: z.string().min(1).refine(navigableHref, 'Search destinations must be document links.'),
	description: z.string().optional(),
});
export const documentSearchIndexSchema = z.array(searchDestinationSchema).readonly();
export type SearchDestination = z.infer<typeof searchDestinationSchema>;
export type SearchScope = 'all' | 'pages' | 'walkthroughs';

function searchWords(value: string): string {
	return value
		.normalize('NFKD')
		.replace(/\p{M}/gu, '')
		.toLocaleLowerCase()
		.replace(/[^\p{L}\p{N}]+/gu, ' ')
		.trim();
}

/** Exact and title matches precede ancestry matches; an empty query preserves document order. */
export function searchDocumentDestinations(
	entries: readonly SearchDestination[],
	query: string,
	scope: SearchScope,
): readonly SearchDestination[] {
	const normalized = searchWords(query);
	const terms = normalized.split(' ').filter(Boolean);
	return entries
		.flatMap((entry, index): { entry: SearchDestination; rank: number; index: number }[] => {
			if (scope === 'all' && entry.kind !== 'page' && entry.kind !== 'heading') return [];
			if (scope === 'pages' && entry.kind !== 'page') return [];
			if (scope === 'walkthroughs' && entry.kind !== 'walkthrough' && entry.kind !== 'journey')
				return [];
			const title = searchWords(entry.title);
			const context = searchWords([entry.pageTitle, ...entry.ancestors].join(' '));
			if (!terms.every((term): boolean => `${title} ${context}`.includes(term))) return [];
			const rank = !normalized
				? 0
				: title === normalized
					? 100
					: title.startsWith(normalized)
						? 80
						: terms.every((term): boolean => title.includes(term))
							? 60
							: 20;
			return [{ entry, rank, index }];
		})
		.toSorted((first, second): number => second.rank - first.rank || first.index - second.index)
		.map((match): SearchDestination => match.entry);
}
