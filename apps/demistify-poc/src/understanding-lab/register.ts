import { ColorStudy } from './color-study.js';
import { CornerStudy } from './corner-study.js';
import { HiddenCalls } from './hidden-calls.js';
import { SearchPicker } from './search-picker.js';

customElements.define('lab-corner-study', CornerStudy);
customElements.define('lab-color-study', ColorStudy);
customElements.define('lab-search-picker', SearchPicker);
if (document.querySelector('lab-requirement-browser,lab-execution-review')) {
	try {
		const { readCollection } = await import('./collection-data.js');
		readCollection();
		const [{ RequirementBrowser }, { ExecutionReview }, { RequirementRelatedView }] =
			await Promise.all([
				import('./requirement-browser.js'),
				import('./execution-review.js'),
				import('./requirement-related-view.js'),
			]);
		customElements.define('lab-requirement-related', RequirementRelatedView);
		customElements.define('lab-requirement-browser', RequirementBrowser);
		customElements.define('lab-execution-review', ExecutionReview);
	} catch (error: unknown) {
		for (const owner of document.querySelectorAll('lab-requirement-browser,lab-execution-review')) {
			const message = document.createElement('p');
			message.setAttribute('role', 'alert');
			message.textContent =
				error instanceof Error ? error.message : 'Local data unavailable. Prepare this lab first.';
			owner.replaceChildren(message);
		}
	}
}
customElements.define('lab-hidden-calls', HiddenCalls);
