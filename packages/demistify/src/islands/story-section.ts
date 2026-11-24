import { createDisclosureControl, type ScopeControl } from '../core/disclosure-scope.ts';
import { createHead, readIcon } from '../core/head.ts';
import { reportIssue } from '../core/issues.ts';
import { readSectionMembership, smallerStoriesSelector } from '../core/section-membership.ts';
import { prepareStoryDisclosure } from '../core/story-content.ts';
import { readTone } from '../core/tones.ts';

export class SectionIsland extends HTMLElement {
	private headerObserver: ResizeObserver | undefined;
	private membershipObserver: MutationObserver | undefined;
	private control: ScopeControl | undefined;

	connectedCallback(): void {
		if (this.closest('[data-inspector-content], [data-inspector-copy]')) return;
		if (!this.hasAttribute('prepared')) {
			const title = this.getAttribute('title')?.trim();
			if (!this.id || !title) {
				reportIssue(this, 'A section needs its stable id and title.');
				return;
			}
			const header = createHead('h3', 'ex-section-head', {
				title,
				icon: readIcon(this),
				tone: readTone(this, undefined),
			});
			const count = document.createElement('span');
			count.className = 'ex-section-count';
			header.append(count);
			this.dataset['title'] = title;
			this.removeAttribute('title');
			this.classList.add('ex-section-floating');
			this.prepend(header);
			this.setAttribute('prepared', '');
		}
		const header = this.querySelector(':scope > .ex-section-head');
		if (!header) return;
		const count = header.querySelector('.ex-section-count');
		const refreshMembership = (): void => {
			const { primary, smaller } = readSectionMembership(this);
			const label = `· ${primary}${smaller ? ` + ${smaller} smaller` : ''}`;
			if (count && count.textContent !== label) count.textContent = label;
			for (const group of this.querySelectorAll(smallerStoriesSelector))
				if (group instanceof HTMLDetailsElement)
					prepareStoryDisclosure(group, undefined, undefined);
			if (!this.control && this.querySelector('ex-story')) {
				this.control = createDisclosureControl(this, this.dataset['title'] ?? 'section');
				header.append(this.control.button);
			}
		};
		this.membershipObserver?.disconnect();
		this.membershipObserver = new MutationObserver(refreshMembership);
		this.membershipObserver.observe(this, { childList: true, subtree: true });
		refreshMembership();
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
		this.membershipObserver?.disconnect();
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
