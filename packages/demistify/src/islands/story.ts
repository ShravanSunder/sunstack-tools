import { createHead, readIcon } from '../core/head.ts';
import { createPhosphorIcon } from '../core/icons.ts';
import { reportIssue } from '../core/issues.ts';
import {
	prepareStoryDisclosure,
	storyContentProblems,
	storyRoot,
	storySlot,
} from '../core/story-content.ts';
import { readTone } from '../core/tones.ts';
import { installIslandInspector } from './inspector.ts';

const storyRows = [
	{ slot: 'happened', label: 'Happened', icon: 'clock' },
	{ slot: 'for-you', label: 'For you', icon: 'user' },
	{ slot: 'next', label: 'Next', icon: 'circle-dashed' },
] as const;

/** A compact account over authored live HTML, with native details owning all disclosure state. */
export class StoryIsland extends HTMLElement {
	connectedCallback(): void {
		if (this.closest('[data-inspector-content], [data-inspector-copy]')) return;
		if (!this.hasAttribute('prepared')) {
			const problems = storyContentProblems(this);
			for (const problem of problems) reportIssue(this, problem);
			if (problems.length) return;
			const root = storyRoot(this);
			if (!root) return;
			const head = createHead('div', '', {
				title: undefined,
				icon: readIcon(this),
				tone: readTone(this, undefined),
			});
			const metadata = [this.getAttribute('subject'), this.getAttribute('time')]
				.filter(Boolean)
				.join(' · ');
			prepareStoryDisclosure(root, head.firstChild ?? undefined, metadata || undefined);
			const summary = root.querySelector(':scope > summary');
			const body = document.createElement('div');
			body.className = 'ex-story-body';
			body.append(...Array.from(root.childNodes).filter((node): boolean => node !== summary));
			root.append(body);
			storySlot(this, 'headline')?.classList.add('ex-story-headline');
			for (const row of storyRows) {
				const content = storySlot(this, row.slot);
				if (!content) continue;
				const wrapper = document.createElement('div');
				wrapper.className = 'ex-story-row';
				content.before(wrapper);
				const label = document.createElement('span');
				label.className = 'ex-story-row-label';
				label.textContent = `${row.label} `;
				content.prepend(label);
				wrapper.append(createPhosphorIcon(row.icon), content);
			}
			for (const name of ['more', 'record']) {
				const slot = storySlot(this, name);
				if (!(slot instanceof HTMLDetailsElement)) continue;
				prepareStoryDisclosure(
					slot,
					createPhosphorIcon(name === 'record' ? 'stack' : 'info'),
					undefined,
				);
				if (name === 'record')
					for (const entry of slot.children)
						if (entry instanceof HTMLDetailsElement) {
							prepareStoryDisclosure(entry, undefined, undefined);
							const entrySummary = entry.querySelector(':scope > summary');
							const output = document.createElement('div');
							output.className = 'ex-story-entry-body';
							output.append(
								...Array.from(entry.childNodes).filter((node): boolean => node !== entrySummary),
							);
							entry.append(output);
						}
			}
			this.setAttribute('prepared', '');
		}
		installIslandInspector(this);
		this.dispatchEvent(new Event('island-ready', { bubbles: true }));
	}
}
