import { createDisclosureControl, type ScopeControl } from '../core/disclosure-scope.ts';
import { createHead, readIcon } from '../core/head.ts';
import { reportIssue } from '../core/issues.ts';
import { readSectionMembership } from '../core/section-membership.ts';
import { prepareStoryDisclosure } from '../core/story-content.ts';
import { readTone } from '../core/tones.ts';

export class SectionIsland extends HTMLElement {
	private control: ScopeControl | undefined;
	private observer: MutationObserver | undefined;
	private count: HTMLElement | undefined;
	private headerObserver: ResizeObserver | undefined;

	connectedCallback(): void {
		if (this.closest('[data-inspector-content], [data-inspector-copy]')) return;
		if (!this.hasAttribute('prepared')) {
			const authoredChildren = Array.from(this.children);
			const smallerGroups = authoredChildren.filter(
				(child): boolean => child.getAttribute('slot') === 'smaller',
			);
			if (
				smallerGroups.length > 1 ||
				(smallerGroups[0] &&
					(!(smallerGroups[0] instanceof HTMLDetailsElement) ||
						authoredChildren.at(-1) !== smallerGroups[0]))
			) {
				reportIssue(this, 'A section has at most one native smaller-stories fold, at its end.');
				return;
			}
			const title = this.getAttribute('title') ?? '';
			if (!this.id || !title.trim()) {
				reportIssue(this, 'A section needs its stable id and title.');
				return;
			}
			this.dataset['title'] = title;
			this.removeAttribute('title');
			const header = createHead('h3', 'ex-section-head', {
				title,
				icon: readIcon(this),
				tone: readTone(this, undefined),
			});
			this.count = document.createElement('span');
			this.count.className = 'ex-section-count';
			const titleText = header.querySelector('strong');
			const heading = document.createElement('span');
			heading.className = 'ex-section-heading';
			if (titleText) {
				titleText.before(heading);
				heading.append(titleText);
			} else header.append(heading);
			heading.append(this.count);
			this.prepend(header);
			const smaller = this.querySelector(':scope > details[slot="smaller"]');
			if (smaller instanceof HTMLDetailsElement)
				prepareStoryDisclosure(smaller, undefined, undefined);
			this.setAttribute('prepared', '');
		}
		const header = this.querySelector(':scope > .ex-section-head');
		if (!header) return;
		this.control ??= createDisclosureControl(this, this.dataset['title'] ?? 'section');
		header.append(this.control.button);
		const refresh = (): void => {
			const membership = readSectionMembership(this);
			const text = ` · ${membership.primary}${membership.smaller ? ` + ${membership.smaller} smaller` : ''}`;
			if (this.count && this.count.textContent !== text) this.count.textContent = text;
		};
		this.observer?.disconnect();
		this.observer = new MutationObserver(refresh);
		this.observer.observe(this, { childList: true, subtree: true });
		refresh();
		const measureHeader = (): void => {
			this.style.setProperty(
				'--ex-section-header-height',
				`${header.getBoundingClientRect().height}px`,
			);
		};
		this.headerObserver?.disconnect();
		this.headerObserver = new ResizeObserver(measureHeader);
		this.headerObserver.observe(header);
		measureHeader();
	}

	disconnectedCallback(): void {
		this.control?.disconnect();
		this.control?.button.remove();
		this.control = undefined;
		this.observer?.disconnect();
		this.headerObserver?.disconnect();
	}
}

export class FoldToggleIsland extends HTMLElement {
	private control: ScopeControl | undefined;

	connectedCallback(): void {
		if (this.control || this.closest('[data-inspector-content], [data-inspector-copy]')) return;
		const root = document.getElementById(this.getAttribute('for') ?? '');
		if (!root || root === this) {
			reportIssue(this, 'A fold toggle needs an existing scope id.');
			return;
		}
		this.control = createDisclosureControl(
			root,
			root.localName === 'ex-document'
				? 'all stories'
				: (root.dataset['title'] ?? root.getAttribute('title') ?? 'stories'),
		);
		this.append(this.control.button);
	}

	disconnectedCallback(): void {
		this.control?.disconnect();
		this.control = undefined;
		this.replaceChildren();
	}
}
