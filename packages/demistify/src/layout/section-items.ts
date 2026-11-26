import { reportIssue } from '../core/issues.ts';

export class SectionItemIsland extends HTMLElement {}

export function placeSectionItems(): void {
	const sections = [...document.querySelectorAll('ex-section')];
	for (const item of document.querySelectorAll<HTMLElement>('ex-section-item')) {
		const named = item.getAttribute('section');
		const requiredHold = item.dataset['sectionHold'];
		const destination = named
			? sections.find((section) => section.id === named && !section.hasAttribute('holds'))
			: sections.find((section) => requiredHold && section.getAttribute('holds') === requiredHold);
		if (destination) destination.append(item);
		else {
			item.hidden = true;
			reportIssue(
				item,
				'Section item has no checked destination. Run the design or initiative checker.',
			);
		}
	}
}
