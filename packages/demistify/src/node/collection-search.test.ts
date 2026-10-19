import { expect, it } from 'vitest';

import { indexCollectionSearch, type SearchPageSource } from './collection-search.ts';

const pages: readonly SearchPageSource[] = [
	{
		filePath: '/collection/index.html',
		markdownFiles: {},
		html: '<title>Architecture</title><ex-document><h1>Architecture</h1><h2>Ownership</h2><h3>Summary</h3><h2>Expiry</h2><h3>Summary</h3></ex-document>',
	},
	{
		filePath: '/collection/verification.html',
		markdownFiles: {
			'proof.md': '# Planned proof\n\n## Summary\n\n```text\n## Not a heading\n```',
		},
		html: '<title>Verification</title><ex-document><h1>Verification</h1><ex-markdown id="proof" src="proof.md"></ex-markdown><figure><figcaption><h2>Caption only</h2></figcaption></figure></ex-document>',
	},
];

it('derives page and parent-heading context with real native and Markdown fragment destinations', (): void => {
	const result = indexCollectionSearch(pages);
	expect(
		result[0]?.destinations.map((entry): readonly [string, readonly string[], string] => [
			entry.title,
			entry.ancestors,
			entry.href,
		]),
	).toEqual([
		['Architecture', [], 'index.html#heading-architecture'],
		['Ownership', [], 'index.html#heading-ownership'],
		['Summary', ['Ownership'], 'index.html#heading-summary'],
		['Expiry', [], 'index.html#heading-expiry'],
		['Summary', ['Expiry'], 'index.html#heading-summary-2'],
		['Verification', [], 'verification.html#heading-verification'],
		['Planned proof', [], 'verification.html#proof--planned-proof'],
		['Summary', ['Planned proof'], 'verification.html#proof--summary'],
	]);
	expect(result[0]?.html).toContain('id="heading-summary-2"');
	expect(result[0]?.html).toContain('data-ex-search-index');
});

it('regenerates the same index instead of accumulating stale carriers or changing stable links', (): void => {
	const first = indexCollectionSearch(pages);
	const second = indexCollectionSearch(
		first.map((page, index): SearchPageSource => ({
			filePath: page.filePath,
			html: page.html,
			markdownFiles: pages[index]?.markdownFiles ?? {},
		})),
	);
	expect(second.map((page): string => page.html)).toEqual(first.map((page): string => page.html));
});

it('rejects missing source or unstable and ambiguous heading targets before returning a collection', (): void => {
	expect((): unknown =>
		indexCollectionSearch([
			{
				filePath: '/one.html',
				html: '<title>One</title><ex-markdown id="source" src="missing.md"></ex-markdown>',
				markdownFiles: {},
			},
		]),
	).toThrow('Missing Markdown source');
	expect((): unknown =>
		indexCollectionSearch([
			{
				filePath: '/one.html',
				html: '<title>One</title><ex-markdown><script type="text/markdown">## Recovery</script></ex-markdown>',
				markdownFiles: {},
			},
		]),
	).toThrow('stable id');
	expect((): unknown =>
		indexCollectionSearch([
			{
				filePath: '/one.html',
				html: '<title>One</title><h2 id="same">First</h2><h2 id="same">Second</h2>',
				markdownFiles: {},
			},
		]),
	).toThrow('Ambiguous heading');
});

it('retains walkthrough and gallery journey explanations in the cross-page index', (): void => {
	const result = indexCollectionSearch([
		...pages,
		{
			filePath: '/collection/captures.html',
			markdownFiles: {},
			html: '<title>Captures</title><figure id="one"></figure><ex-presentation id="pictures" title="Inspect captures" description="Review the archived views."><ex-step to="one">Look at this view.</ex-step></ex-presentation><ex-presentation id="guide" title="Review evidence" description="Follow the source into the proof plan."><ex-step to="elsewhere.html#proof">Read the proof plan.</ex-step></ex-presentation>',
		},
	]);
	expect(
		result[0]?.destinations.filter(
			(entry): boolean => entry.kind === 'walkthrough' || entry.kind === 'journey',
		),
	).toEqual([
		{
			kind: 'journey',
			title: 'Inspect captures',
			pageTitle: 'Captures',
			ancestors: [],
			href: 'captures.html#pictures',
			description: 'Review the archived views.',
		},
		{
			kind: 'walkthrough',
			title: 'Review evidence',
			pageTitle: 'Captures',
			ancestors: [],
			href: 'captures.html#guide',
			description: 'Follow the source into the proof plan.',
		},
	]);
});
