import { icon } from './document-markup.js';

/** A disclosure preserves omitted calls in place instead of implying a direct invocation. */
export class HiddenCalls extends HTMLElement {
	connectedCallback(): void {
		if (this.querySelector(':scope > details')) return;
		const calls = [...this.children];
		const count = calls.length;
		const details = document.createElement('details');
		details.open = this.hasAttribute('expanded');
		const summary = document.createElement('summary');
		const label = (): string =>
			`${icon('caret-right')}<strong>${count} ${details.open ? 'supporting calls' : 'calls hidden'}</strong><span>${details.open ? 'Hide' : 'Show'} plumbing</span><small class="ex-design-hidden-selection"></small>`;
		summary.innerHTML = label();
		details.addEventListener('toggle', (): void => {
			const title = summary.querySelector('strong');
			const action = summary.querySelector('span');
			if (title)
				title.textContent = `${count} ${details.open ? 'supporting calls' : 'calls hidden'}`;
			if (action) action.textContent = `${details.open ? 'Hide' : 'Show'} plumbing`;
		});
		details.append(summary);
		const content = document.createElement('div');
		content.className = 'ex-design-hidden-call-content';
		content.setAttribute('aria-label', 'Supporting calls in invocation order');
		content.append(...calls);
		details.append(content);
		this.replaceChildren(details);
	}
}
