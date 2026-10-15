import { createPhosphorIcon } from './icons.ts';
import { reportIssue } from './issues.ts';
import { toneAliases, type ToneName } from './tone-names.ts';

export const toneIcons = {
	note: 'info',
	tldr: 'lightning',
	decision: 'signpost',
	risk: 'warning',
	question: 'question',
	done: 'check-circle',
	planned: 'circle-dashed',
	changed: 'pencil-simple',
	added: 'plus-circle',
	removed: 'minus-circle',
} as const satisfies Record<ToneName, string>;

export type Tone = keyof typeof toneIcons;

export function isTone(value: string): value is Tone {
	return Object.hasOwn(toneIcons, value);
}

export function readTone(owner: HTMLElement, fallback: Tone | undefined): Tone | undefined {
	const requested = (owner.getAttribute('tone') ?? owner.getAttribute('type') ?? '').trim();
	if (!requested) {
		if (fallback) owner.setAttribute('tone', fallback);
		return fallback;
	}
	if (isTone(requested)) {
		owner.setAttribute('tone', requested);
		return requested;
	}
	const meant = toneAliases[requested];
	if (meant) {
		owner.setAttribute('tone', meant);
		return meant;
	}
	// The legacy free-form `type` attribute stays valid; only `tone` is checked.
	if (owner.hasAttribute('tone'))
		reportIssue(
			owner,
			`Unknown tone "${requested}". Use one of: ${Object.keys(toneIcons).join(', ')}.`,
		);
	if (fallback) owner.setAttribute('tone', fallback);
	return fallback;
}

export function createToneIcon(tone: Tone): HTMLElement {
	const icon = createPhosphorIcon(toneIcons[tone]);
	icon.classList.add('ex-tone-icon');
	return icon;
}
