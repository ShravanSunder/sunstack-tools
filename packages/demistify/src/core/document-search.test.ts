import { expect, it } from 'vitest';

import {
	documentSearchIndexSchema,
	searchDocumentDestinations,
	type SearchDestination,
} from './document-search.ts';

const entries: readonly SearchDestination[] = [
	{
		kind: 'page',
		title: 'Summary reference',
		pageTitle: 'Summary reference',
		ancestors: [],
		href: 'reference.html',
	},
	{
		kind: 'heading',
		title: 'Summary',
		pageTitle: 'Architecture',
		ancestors: ['Lookup'],
		href: 'architecture.html#summary',
	},
	{
		kind: 'heading',
		title: 'Summary',
		pageTitle: 'Verification',
		ancestors: ['Expiry'],
		href: 'verification.html#summary',
	},
	{
		kind: 'heading',
		title: 'Résumé',
		pageTitle: 'Architecture',
		ancestors: [],
		href: 'architecture.html#resume',
	},
	{
		kind: 'walkthrough',
		title: 'Review ownership',
		pageTitle: 'Architecture',
		ancestors: [],
		href: 'architecture.html#review',
	},
	{
		kind: 'journey',
		title: 'Checkout',
		pageTitle: 'Screens',
		ancestors: [],
		href: 'screens.html#checkout',
	},
];

it('uses parent context to distinguish equal heading titles and ranks title matches first', (): void => {
	expect(
		searchDocumentDestinations(entries, 'expiry summary', 'all').map((entry): string => entry.href),
	).toEqual(['verification.html#summary']);
	expect(
		searchDocumentDestinations(entries, 'summary', 'all').map((entry): string => entry.href),
	).toEqual(['architecture.html#summary', 'verification.html#summary', 'reference.html']);
	expect(searchDocumentDestinations(entries, 'resume', 'all')[0]?.href).toBe(
		'architecture.html#resume',
	);
	expect(searchDocumentDestinations(entries, 'unrelated needle', 'all')).toEqual([]);
});

it('keeps page and walkthrough browsing separate, with stable source order', (): void => {
	expect(
		searchDocumentDestinations(entries, '', 'pages').map((entry): string => entry.href),
	).toEqual(['reference.html']);
	expect(
		searchDocumentDestinations(entries, '', 'walkthroughs').map((entry): string => entry.href),
	).toEqual(['architecture.html#review', 'screens.html#checkout']);
});

it('rejects malformed or executable destination data instead of enabling a search result', (): void => {
	expect(
		documentSearchIndexSchema.safeParse([{ ...entries[0], href: 'javascript:alert(1)' }]).success,
	).toBe(false);
	expect(documentSearchIndexSchema.safeParse([{ ...entries[0], title: '' }]).success).toBe(false);
	expect(documentSearchIndexSchema.parse(entries)).toHaveLength(6);
});
