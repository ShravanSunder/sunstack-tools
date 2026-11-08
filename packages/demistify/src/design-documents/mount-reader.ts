import { deriveDocumentModel } from './collection-model.ts';
import { readCollection } from './collection.ts';
import { ExecutionReview } from './execution-review.ts';
import { HiddenCalls } from './hidden-calls.ts';
import { RequirementBrowser } from './requirement-browser.ts';
import { RequirementRelatedView } from './requirement-related-view.ts';
import { SearchPicker } from './search-picker.ts';

export type ReaderKind = 'requirements' | 'call-stack';
export interface DocumentReaderController {
	resume(): void;
	dispose(): void;
}

/** The validated collection belongs to this mount; internal controls carry no authoring contract. */
export function mountDocumentReader(
	owner: HTMLElement,
	kind: ReaderKind,
): { readonly controller: DocumentReaderController; readonly source: string } {
	const collection = readCollection(owner.ownerDocument);
	// Custom-element reactions report constructor errors outside the caller's promise.
	// Check the model here so invalid input resolves through the island's failed-source path.
	deriveDocumentModel(collection);
	for (const [tag, constructor] of [
		['ex-design-picker', SearchPicker],
		['ex-design-hidden-calls', HiddenCalls],
		['ex-design-related', RequirementRelatedView],
		['ex-design-requirements-view', RequirementBrowser],
		['ex-design-call-view', ExecutionReview],
	] satisfies readonly (readonly [string, CustomElementConstructor])[])
		if (!customElements.get(tag)) customElements.define(tag, constructor);
	const view = document.createElement(
		kind === 'requirements' ? 'ex-design-requirements-view' : 'ex-design-call-view',
	);
	owner.replaceChildren(view);
	return {
		source: JSON.stringify(collection),
		controller: {
			resume(): void {
				if (!view.isConnected) owner.replaceChildren(view);
			},
			dispose(): void {
				view.remove();
			},
		},
	};
}
