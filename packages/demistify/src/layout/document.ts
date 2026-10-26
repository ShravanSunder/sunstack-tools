import { adoptBareMarkdown } from '../core/bare-markdown.ts';
import { createIconButton } from '../core/icons.ts';
import { captureReviewDocument, installDefaultComments } from '../core/review-document.ts';
import { installIslandInspector, openIsland } from '../islands/inspector.ts';
import { resolveTarget } from './target-resolution.ts';
export { revealElement } from './target-resolution.ts';
import { revealElement } from './target-resolution.ts';

export class StructuralIsland extends HTMLElement {
	connectedCallback(): void {
		installIslandInspector(this);
	}
}
export class GridIsland extends HTMLElement {}
export class DocumentIsland extends StructuralIsland {
	override connectedCallback(): void {
		captureReviewDocument(this);
		queueMicrotask(installDefaultComments);
		adoptBareMarkdown(this);
		const content = Array.from(this.children).filter(
			(child): boolean => !child.matches('script,template,style,[hidden],.ex-inspect-controls'),
		);
		const galleryOnly = content.length === 1 && content[0]?.localName === 'ex-gallery';
		const titledGallery =
			content.length === 2 &&
			content[0]?.localName === 'h1' &&
			content[1]?.localName === 'ex-gallery';
		this.classList.toggle('ex-gallery-document', galleryOnly || titledGallery);
		for (const panel of this.querySelectorAll('section, article')) {
			if (!panel.closest('ex-gallery, ex-presentation')) adoptBareMarkdown(panel);
		}
		super.connectedCallback();
		if (!document.querySelector('ex-nav')) {
			const navigation = document.createElement('ex-nav');
			this.id ||= 'ex-content';
			navigation.setAttribute('for', this.id);
			document.body.prepend(navigation);
		}
		// Layers get their toggles at the top of the page unless the author placed them.
		if (document.querySelector('[layer]') && !document.querySelector('ex-layers')) {
			const layers = document.createElement('ex-layers');
			const first = this.firstElementChild;
			if (first) first.after(layers);
			else this.prepend(layers);
		}
		if (!document.querySelector('[data-ex-theme]')) {
			const toggle = createIconButton('circle-half', 'Switch light or dark theme', (): void => {});
			toggle.dataset['exTheme'] = '';
			toggle.classList.add('ex-theme-toggle');
			document.body.append(toggle);
		}
	}
}

/** Theme toggling and in-page links that reveal their target inside tabs or details. */
export function installDocumentBehavior(): void {
	let pending: AbortController | undefined;
	const revealFragment = (): void => {
		pending?.abort();
		pending = new AbortController();
		let reference: string;
		try {
			reference = decodeURIComponent(location.hash.slice(1));
		} catch {
			return;
		}
		if (!reference || document.getElementById(reference)?.localName === 'ex-presentation') return;
		const signal = pending.signal;
		void resolveTarget(reference, signal).then((target): void => {
			if (target && !signal.aborted) target.scrollIntoView({ behavior: 'instant', block: 'start' });
		});
	};
	window.addEventListener('hashchange', revealFragment);
	// The resumed player owns the initial target; later ordinary fragment links still reveal theirs.
	const params = new URLSearchParams(location.search);
	if (!params.has('ex-tour') || !params.has('ex-route')) revealFragment();
	document.addEventListener('click', (event: MouseEvent): void => {
		if (!(event.target instanceof Element)) return;
		if (event.target.closest('[data-ex-theme]')) {
			const current =
				document.documentElement.dataset['theme'] ??
				(matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');
			document.documentElement.dataset['theme'] = current === 'dark' ? 'light' : 'dark';
			return;
		}
		// Direct visual clicks enlarge the original; explicit sidebar buttons keep their own action.
		// Figures travel with their captions, and linked diagram nodes remain links.
		const image = event.target.closest('img');
		const diagram = event.target.closest('ex-mermaid > svg')?.parentElement;
		const visual = image ?? diagram;
		if (
			visual?.closest('ex-document') &&
			!event.target.closest(
				'a,button,input,textarea,select,[role="button"],[contenteditable],.ex-drawer,dialog,ex-gallery,ex-compare[slider],[data-inspector-content]',
			) &&
			event.button === 0 &&
			!event.metaKey &&
			!event.ctrlKey &&
			!event.altKey &&
			!event.shiftKey &&
			document.getSelection()?.isCollapsed !== false
		) {
			const owner = diagram ?? image?.closest('figure') ?? image?.parentElement;
			if (owner instanceof HTMLElement && owner.getAttribute('zoom') !== 'none')
				openIsland(owner, 'modal');
			return;
		}
		const targetLink = event.target.closest('a');
		const href = targetLink?.getAttribute('href');
		if (!href?.startsWith('#')) return;
		const targetElement = document.getElementById(href.slice(1));
		if (targetElement) revealElement(targetElement);
	});
}
