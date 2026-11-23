import { createDisclosureControl, type ScopeControl } from '../core/disclosure-scope.ts';
import { createHead, readIcon } from '../core/head.ts';
import { reportIssue } from '../core/issues.ts';
import { sectionItemCount } from '../core/section-membership.ts';
import { readTone } from '../core/tones.ts';

export class SectionIsland extends HTMLElement {
	private headerObserver: ResizeObserver | undefined;

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
			count.textContent = `· ${sectionItemCount(this)}`;
			header.append(count);
			this.dataset['title'] = title;
			this.removeAttribute('title');
			this.classList.add('ex-section-floating');
			this.prepend(header);
			this.setAttribute('prepared', '');
		}
		const header = this.querySelector(':scope > .ex-section-head');
		if (!header) return;
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
