// Rolling v1 still renders the direct smaller fold on previously published pages.
export const smallerStoriesSelector =
	'ex-section > ex-section-item > details[slot="smaller"], ex-section > details[slot="smaller"]';

/** The heading and the Map count the same placed content. */
export function readSectionMembership(section: HTMLElement): {
	readonly primary: number;
	readonly smaller: number;
	readonly total: number;
} {
	const items = section.querySelectorAll(':scope > ex-section-item');
	if (!items.length) {
		const stories = [...section.querySelectorAll('ex-story')].filter(
			(story) => story.closest('ex-section') === section,
		);
		const smaller = stories.filter(
			(story) => story.closest('details[slot="smaller"]')?.parentElement === section,
		).length;
		return { primary: stories.length - smaller, smaller, total: stories.length };
	}
	let primary = 0;
	let smaller = 0;
	for (const item of items) {
		const group = item.querySelector(':scope > details[slot="smaller"]');
		const stories = item.querySelectorAll('ex-story').length;
		const grouped = group?.querySelectorAll('ex-story').length ?? 0;
		primary += stories ? stories - grouped : 1;
		smaller += grouped;
	}
	return { primary, smaller, total: primary + smaller };
}
