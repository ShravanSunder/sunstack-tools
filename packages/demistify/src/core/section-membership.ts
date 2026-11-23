/** The Map and the heading count the same placed items. */
export function sectionItemCount(section: HTMLElement): number {
	return section.querySelectorAll(':scope > ex-section-item').length;
}
