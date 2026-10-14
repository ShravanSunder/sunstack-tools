import { HeadingNavigation } from './heading-navigation.ts';
import { ArrowIsland } from './island-arrows.ts';
import { installIslandInspector, ZoomIsland } from './island-inspector.ts';
import { MarkdownIsland, MermaidIsland } from './markdown-diagram-islands.ts';
import { normalizeIslandText } from './source-text.ts';
import { TabsIsland } from './tabs-island.ts';

class StructuralIsland extends HTMLElement {
	connectedCallback(): void {
		installIslandInspector(this);
	}
}
class DecisionIsland extends HTMLElement {
	connectedCallback(): void {
		this.classList.add('decision');
		installIslandInspector(this);
	}
}
class CalloutIsland extends HTMLElement {
	connectedCallback(): void {
		this.classList.add('why');
		installIslandInspector(this);
	}
}
class CompareIsland extends HTMLElement {
	connectedCallback(): void {
		this.classList.add('ex-compare');
		installIslandInspector(this);
	}
}
class TimelineIsland extends HTMLElement {
	connectedCallback(): void {
		this.classList.add('ex-timeline');
		installIslandInspector(this);
	}
}
class LayerIsland extends HTMLElement {
	connectedCallback(): void {
		this.classList.add('band');
		installIslandInspector(this);
	}
}
class FileTreeIsland extends HTMLElement {
	connectedCallback(): void {
		if (this.hasAttribute('prepared')) {
			installIslandInspector(this);
			return;
		}
		this.classList.add('ex-file-tree');
		this.textContent = normalizeIslandText(
			this.querySelector('script[type="text/plain"]')?.textContent ?? this.textContent ?? '',
		);
		this.setAttribute('prepared', '');
		installIslandInspector(this);
	}
}
class DiagramDiffIsland extends CompareIsland {}

for (const [tagName, constructor] of [
	['ex-markdown', MarkdownIsland],
	['ex-mermaid', MermaidIsland],
	['ex-arrow', ArrowIsland],
	['ex-tabs', TabsIsland],
	['ex-zoom', ZoomIsland],
	['ex-decision', DecisionIsland],
	['ex-callout', CalloutIsland],
	['ex-compare', CompareIsland],
	['ex-diagram-diff', DiagramDiffIsland],
	['ex-timeline', TimelineIsland],
	['ex-layer', LayerIsland],
	['ex-file-tree', FileTreeIsland],
	['ex-document', StructuralIsland],
	['ex-nav', HeadingNavigation],
] satisfies readonly (readonly [string, CustomElementConstructor])[]) {
	if (!customElements.get(tagName)) customElements.define(tagName, constructor);
}

document.addEventListener('click', (event: MouseEvent): void => {
	if (!(event.target instanceof Element)) return;
	if (event.target.closest('[data-ex-theme]')) {
		document.documentElement.dataset['theme'] =
			document.documentElement.dataset['theme'] === 'dark' ? 'light' : 'dark';
		return;
	}
	const targetLink = event.target.closest('a');
	const href = targetLink?.getAttribute('href');
	if (!href?.startsWith('#')) return;
	const targetElement = document.getElementById(href.slice(1));
	if (!targetElement) return;
	for (let ancestor = targetElement.parentElement; ancestor; ancestor = ancestor.parentElement) {
		if (ancestor.getAttribute('role') === 'tabpanel') {
			const ownerTabs = ancestor.parentElement;
			ownerTabs
				?.querySelector<HTMLButtonElement>(`button[aria-controls="${CSS.escape(ancestor.id)}"]`)
				?.click();
		}
		if (ancestor instanceof HTMLDetailsElement) ancestor.open = true;
	}
});
