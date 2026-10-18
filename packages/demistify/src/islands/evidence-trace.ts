import { createHead, readIcon } from '../core/head.ts';
import { createPhosphorIcon } from '../core/icons.ts';
import { reportIssue } from '../core/issues.ts';
import { readTone } from '../core/tones.ts';
import { installIslandInspector } from './inspector.ts';

/** Ordered explanation steps retain native disclosure and arbitrary live attachment content. */
export class EvidenceTraceIsland extends HTMLElement {
	connectedCallback(): void {
		if (this.hasAttribute('data-inspector-content')) return;
		if (!this.hasAttribute('prepared')) {
			const rows = Array.from(this.children).filter(
				(child): child is HTMLDetailsElement => child instanceof HTMLDetailsElement,
			);
			const list = document.createElement('ol');
			list.className = 'ex-trace-list';
			for (const details of rows) {
				const summary = details.querySelector(':scope > summary');
				if (!(summary instanceof HTMLElement)) {
					reportIssue(
						details,
						'A Trace step needs a native summary naming its evidence or action.',
					);
					continue;
				}
				const titleNodes = Array.from(summary.childNodes);
				const head = createHead('div', '', {
					title: summary.textContent?.trim() || 'Step',
					icon: readIcon(details),
					tone: readTone(details, undefined),
					tag: details.getAttribute('tag'),
				});
				const title = head.querySelector('strong');
				if (title) {
					title.classList.add('ex-trace-title');
					if (titleNodes.length) title.replaceChildren(...titleNodes);
				}
				if (!head.querySelector('.ex-tone-icon')) {
					const dot = document.createElement('span');
					dot.className = 'ex-trace-dot';
					dot.setAttribute('aria-hidden', 'true');
					head.prepend(dot);
				}
				const chevron = createPhosphorIcon('caret-right');
				chevron.classList.add('ex-trace-chevron');
				summary.classList.add('ex-head', 'ex-trace-summary');
				summary.replaceChildren(...head.childNodes, chevron);
				const body = document.createElement('div');
				body.className = 'ex-trace-body';
				body.append(...Array.from(details.childNodes).filter((node): boolean => node !== summary));
				details.append(body);
				const item = document.createElement('li');
				item.append(details);
				list.append(item);
			}
			const title = this.getAttribute('title');
			if (title) {
				this.dataset['title'] = title;
				this.prepend(
					createHead('div', 'ex-trace-head', {
						title,
						icon: readIcon(this),
						tone: readTone(this, undefined),
					}),
				);
			}
			this.append(list);
			this.setAttribute('prepared', '');
		}
		installIslandInspector(this);
		this.dispatchEvent(new Event('island-ready', { bubbles: true }));
	}
}
