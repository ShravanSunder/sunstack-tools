import { readIcon } from '../core/head.ts';
import { createPhosphorIcon } from '../core/icons.ts';
import { createToneIcon, readTone } from '../core/tones.ts';
import { installIslandInspector } from './inspector.ts';

/* Earlier lab vocabulary, kept for the comparison pages and not taught in the authoring guide. */
export class DecisionIsland extends HTMLElement {
	connectedCallback(): void {
		this.classList.add('decision');
		installIslandInspector(this);
	}
}
export class TimelineIsland extends HTMLElement {
	connectedCallback(): void {
		this.classList.add('ex-timeline');
		this.setAttribute('role', 'list');
		for (const event of Array.from(this.children)) {
			if (!(event instanceof HTMLElement) || event.localName !== 'section') continue;
			event.setAttribute('role', 'listitem');
			if (event.querySelector(':scope > .ex-timeline-marker')) continue;
			const marker = document.createElement('span');
			marker.className = 'ex-timeline-marker';
			marker.setAttribute('aria-hidden', 'true');
			const icon = readIcon(event);
			const tone = readTone(event, undefined);
			if (icon) marker.append(createPhosphorIcon(icon));
			else if (tone) marker.append(createToneIcon(tone));
			else marker.classList.add('ex-timeline-dot');
			event.prepend(marker);
		}
		installIslandInspector(this);
	}
}
export class LayerIsland extends HTMLElement {
	connectedCallback(): void {
		this.classList.add('band');
		installIslandInspector(this);
	}
}
