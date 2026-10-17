import { createPhosphorIcon } from './icons.ts';
import { reportIssue } from './issues.ts';
import { resolveSubjectIcon, subjectIcons } from './subject-icons.ts';
import { createToneIcon, type Tone } from './tones.ts';

/**
 * The one head every titled island uses: an icon (the subject icon, else the tone's icon), the
 * title, a tag chip and an optional "chosen" mark. Sharing it keeps cards, callouts, regions and
 * trees consistent.
 */
export interface HeadParts {
	readonly tone: Tone | undefined;
	readonly icon: string | undefined;
	readonly title: string | undefined;
	readonly tag?: string | null;
	readonly pick?: string | null;
}

/** Reads a curated subject icon, mapping common synonyms; reports anything else. */
export function readIcon(owner: HTMLElement): string | undefined {
	const requested = owner.getAttribute('icon')?.trim();
	if (!requested) return undefined;
	const meant = resolveSubjectIcon(requested);
	if (meant) return meant;
	reportIssue(owner, `Unknown icon "${requested}". Use one of: ${subjectIcons.join(', ')}.`);
	return undefined;
}

export function createHead(
	tagName: 'div' | 'h3',
	className: string,
	parts: HeadParts,
): HTMLElement {
	const head = document.createElement(tagName);
	head.className = `ex-head ${className}`;
	if (parts.icon) {
		const icon = createPhosphorIcon(parts.icon);
		icon.classList.add('ex-tone-icon');
		head.append(icon);
	} else if (parts.tone) head.append(createToneIcon(parts.tone));
	if (parts.title) {
		const title = document.createElement('strong');
		title.textContent = parts.title;
		head.append(title);
	}
	if (parts.tag) {
		const chip = document.createElement('span');
		chip.className = 'ex-head-tag';
		chip.textContent = parts.tag;
		head.append(chip);
	}
	if (parts.pick !== undefined && parts.pick !== null) {
		const chosen = document.createElement('span');
		chosen.className = 'ex-card-pick';
		chosen.textContent = parts.pick.trim() || 'Chosen';
		head.append(chosen);
	}
	return head;
}
