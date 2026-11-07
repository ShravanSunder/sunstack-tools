export interface SectionMembership {
	readonly primary: number;
	readonly smaller: number;
	readonly total: number;
}

/** Header splits and Map totals read the same authored membership, never title text. */
export function readSectionMembership(section: HTMLElement): SectionMembership {
	let primary = 0;
	let smaller = 0;
	for (const story of section.querySelectorAll('ex-story')) {
		if (story.closest('ex-section') !== section) continue;
		const group = story.closest('details[slot="smaller"]');
		if (group?.parentElement === section) smaller += 1;
		else primary += 1;
	}
	return { primary, smaller, total: primary + smaller };
}
