import { expect } from 'vitest';
import { page } from 'vitest/browser';

export async function openFixture(
	name: 'navigation' | 'layout' | 'embedded' | 'gallery' | 'ui' | 'search' | 'review',
	fragment = '',
	file = 'index.html',
): Promise<{ readonly document: Document; readonly frame: ReturnType<typeof page.frameLocator> }> {
	const iframe = document.createElement('iframe');
	iframe.dataset['testid'] = 'explainer';
	iframe.title = 'Demistify fixture';
	iframe.style.cssText = 'display:block;width:100%;height:100vh;border:0';
	document.body.style.margin = '0';
	iframe.src = `/__demistify/${name}/${file}${fragment}`;
	document.body.append(iframe);
	await expect
		.poll((): boolean => !!iframe.contentDocument?.querySelector('ex-document'))
		.toBe(true);
	const fixtureDocument = iframe.contentDocument;
	if (!fixtureDocument) throw new Error('Same-origin fixture did not load.');
	await expect
		.poll(
			(): boolean =>
				!!fixtureDocument.querySelector('.ex-document-toolbar button') &&
				!!fixtureDocument.querySelector<HTMLLinkElement>('link[rel="stylesheet"]')?.sheet,
		)
		.toBe(true);
	await fixtureDocument.fonts.ready;
	return { document: fixtureDocument, frame: page.frameLocator(page.getByTestId('explainer')) };
}

export function requiredElement(root: ParentNode, selector: string): Element {
	const element = root.querySelector(selector);
	if (!element) throw new Error(`Missing fixture element: ${selector}`);
	return element;
}

/** Scope native actions to the fixture iframe rather than the Vitest runner's document. */
export function fixtureLocator(
	frame: ReturnType<typeof page.frameLocator>,
	element: Element,
): ReturnType<typeof page.getByTestId> {
	const testId = crypto.randomUUID();
	element.setAttribute('data-testid', testId);
	return frame.getByTestId(testId);
}

export function firstTextRect(element: Element): DOMRect {
	const walker = element.ownerDocument.createTreeWalker(element, NodeFilter.SHOW_TEXT);
	let text = walker.nextNode();
	while (text && !(text.textContent ?? '').trim()) text = walker.nextNode();
	if (!text) throw new Error('Expected rendered label text.');
	const range = element.ownerDocument.createRange();
	range.setStart(text, 0);
	range.setEnd(text, Math.min(3, text.textContent?.length ?? 0));
	return range.getBoundingClientRect();
}

export function insideViewport(element: Element): boolean {
	const bounds = element.getBoundingClientRect();
	const view = element.ownerDocument.defaultView;
	return (
		!!view &&
		bounds.width > 0 &&
		bounds.height > 0 &&
		bounds.left >= -1 &&
		bounds.top >= -1 &&
		bounds.right <= view.innerWidth + 1 &&
		bounds.bottom <= view.innerHeight + 1
	);
}
