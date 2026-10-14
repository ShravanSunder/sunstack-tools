import { installIslandInspector } from './island-inspector.ts';
import { createToneIcon, readTone } from './tones.ts';
import type { Tone } from './tones.ts';

function takeTitle(owner: HTMLElement): string | undefined {
	const title = owner.getAttribute('title') ?? owner.dataset['title'];
	if (title === undefined) return undefined;
	owner.dataset['title'] = title;
	owner.removeAttribute('title');
	owner.setAttribute('label', owner.getAttribute('label') ?? title);
	return title;
}

function createHeader(
	className: string,
	tone: Tone | undefined,
	title: string | undefined,
): HTMLElement {
	const header = document.createElement('div');
	header.className = className;
	if (tone) header.append(createToneIcon(tone));
	if (title) {
		const heading = document.createElement('strong');
		heading.textContent = title;
		header.append(heading);
	}
	return header;
}

/** A titled surface for one thing: a component, step, option or finding. */
export class CardIsland extends HTMLElement {
	connectedCallback(): void {
		if (this.hasAttribute('data-inspector-content')) return;
		if (!this.hasAttribute('prepared')) {
			const tone = readTone(this, undefined);
			const header = createHeader('ex-card-head', tone, takeTitle(this));
			const tag = this.getAttribute('tag');
			if (tag) {
				const chip = document.createElement('span');
				chip.className = 'ex-card-tag';
				chip.textContent = tag;
				header.append(chip);
			}
			if (this.hasAttribute('pick')) {
				const chosen = document.createElement('span');
				chosen.className = 'ex-card-pick';
				const pickLabel = this.getAttribute('pick')?.trim();
				chosen.textContent = pickLabel === undefined || pickLabel === '' ? 'Chosen' : pickLabel;
				header.append(chosen);
			}
			this.prepend(header);
			this.setAttribute('prepared', '');
		}
		installIslandInspector(this);
	}
}

/** Emphasised aside whose meaning (tone) chooses its icon and colour. */
export class CalloutIsland extends HTMLElement {
	connectedCallback(): void {
		if (this.hasAttribute('data-inspector-content')) return;
		if (!this.hasAttribute('prepared')) {
			const tone = readTone(this, 'note');
			this.prepend(createHeader('ex-callout-head', tone, takeTitle(this)));
			this.setAttribute('prepared', '');
		}
		installIslandInspector(this);
	}
}
