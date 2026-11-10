import { expect, test } from 'vitest';

import { navigationTargetHref, sharedNavigationSchema } from './shared-navigation.ts';

const collection = {
	version: 1,
	pages: [{ id: 'work', title: 'Work', href: '../pages/work.html', icon: 'path' }],
	map: [
		{
			label: 'Delivery',
			icon: 'envelope',
			children: [
				{
					label: 'Preserve custody',
					icon: 'user',
					target: { page: 'work', fragment: 'saved%20state' },
				},
			],
		},
	],
	walkthroughs: [
		{
			id: 'follow-delivery',
			title: 'How does delivery survive?',
			description: 'Inspect the real state owner.',
			steps: [
				{ target: { page: 'work', fragment: 'saved%20state' }, narration: 'Find the durable row.' },
			],
		},
	],
};

test('shared targets resolve beside the JSON and encode literal fragment characters once', (): void => {
	const navigation = sharedNavigationSchema.parse(collection);
	const href = navigationTargetHref(
		navigation,
		{ page: 'work', fragment: 'saved%20state' },
		'https://example.test/design/navigation/catalog.json',
	);
	expect(href).toBe('https://example.test/design/pages/work.html#saved%2520state');
});

test('shared navigation rejects dangling targets in both Map and Walkthroughs', (): void => {
	const map = [{ label: 'Lost', icon: 'user', target: { page: 'missing' } }];
	expect(sharedNavigationSchema.safeParse({ ...collection, map }).success).toBe(false);
	const walkthroughs = [
		{ ...collection.walkthroughs[0], steps: [{ target: { page: 'missing' }, narration: 'Lost.' }] },
	];
	expect(sharedNavigationSchema.safeParse({ ...collection, walkthroughs }).success).toBe(false);
});

test('shared identities, icons and link protocols cannot silently become ambiguous', (): void => {
	expect(
		sharedNavigationSchema.safeParse({
			...collection,
			pages: [...collection.pages, ...collection.pages],
		}).success,
	).toBe(false);
	expect(
		sharedNavigationSchema.safeParse({
			...collection,
			walkthroughs: [...collection.walkthroughs, ...collection.walkthroughs],
		}).success,
	).toBe(false);
	expect(
		sharedNavigationSchema.safeParse({
			...collection,
			map: [{ label: 'Bad icon', icon: 'not-a-subject' }],
		}).success,
	).toBe(false);
	expect(
		sharedNavigationSchema.safeParse({
			...collection,
			pages: [{ ...collection.pages[0], href: 'javascript:alert(1)' }],
		}).success,
	).toBe(false);
});
