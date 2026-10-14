import { adoptBareMarkdown } from '../core/bare-markdown.ts';
import { createHead, readIcon } from '../core/head.ts';
import { readTone } from '../core/tones.ts';
import { installIslandInspector } from './inspector.ts';

function takeTitle(owner: HTMLElement): string | undefined {
	const title = owner.getAttribute('title') ?? owner.dataset['title'];
	if (title === undefined) return undefined;
	owner.dataset['title'] = title;
	owner.removeAttribute('title');
	owner.setAttribute('label', owner.getAttribute('label') ?? title);
	return title;
}

/** Card and callout bodies, and their details, may be written as bare Markdown. */
function adoptBodyMarkdown(owner: HTMLElement): void {
	adoptBareMarkdown(owner);
	for (const details of owner.querySelectorAll(':scope > details')) adoptBareMarkdown(details);
}

/** A titled surface for one thing: a component, step, option or finding. */
export class CardIsland extends HTMLElement {
	connectedCallback(): void {
		if (this.hasAttribute('data-inspector-content')) return;
		if (!this.hasAttribute('prepared')) {
			adoptBodyMarkdown(this);
			const header = createHead('div', 'ex-card-head', {
				tone: readTone(this, undefined),
				icon: readIcon(this),
				title: takeTitle(this),
				tag: this.getAttribute('tag'),
				pick: this.getAttribute('pick'),
			});
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
			adoptBodyMarkdown(this);
			this.prepend(
				createHead('div', 'ex-callout-head', {
					tone: readTone(this, 'note'),
					icon: readIcon(this),
					title: takeTitle(this),
				}),
			);
			this.setAttribute('prepared', '');
		}
		installIslandInspector(this);
	}
}
