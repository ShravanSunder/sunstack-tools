import { CalloutIsland, CardIsland } from './card-island.ts';
import { HeadingNavigation } from './heading-navigation.ts';
import { createIconButton } from './icon-controls.ts';
import { ArrowIsland } from './island-arrows.ts';
import { FlowIsland } from './island-flow.ts';
import { installIslandInspector, ZoomIsland } from './island-inspector.ts';
import { reportIssue } from './issues.ts';
import { MarkdownIsland, MermaidIsland } from './markdown-diagram-islands.ts';
import { normalizeIslandText } from './source-text.ts';
import { TabsIsland } from './tabs-island.ts';

class StructuralIsland extends HTMLElement {
	connectedCallback(): void {
		installIslandInspector(this);
	}
}
class GridIsland extends HTMLElement {}
class DocumentIsland extends StructuralIsland {
	override connectedCallback(): void {
		super.connectedCallback();
		if (!document.querySelector('ex-nav')) {
			const navigation = document.createElement('ex-nav');
			this.id ||= 'ex-content';
			navigation.setAttribute('for', this.id);
			document.body.prepend(navigation);
		}
		if (!document.querySelector('[data-ex-theme]')) {
			const toggle = createIconButton('circle-half', 'Switch light or dark theme', (): void => {});
			toggle.dataset['exTheme'] = '';
			toggle.classList.add('ex-theme-toggle');
			document.body.append(toggle);
		}
	}
}
class DecisionIsland extends HTMLElement {
	connectedCallback(): void {
		this.classList.add('decision');
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
	['ex-card', CardIsland],
	['ex-grid', GridIsland],
	['ex-flow', FlowIsland],
	['ex-document', DocumentIsland],
	['ex-nav', HeadingNavigation],
] satisfies readonly (readonly [string, CustomElementConstructor])[]) {
	if (!customElements.get(tagName)) customElements.define(tagName, constructor);
}

const seenIds = new Set<string>();
for (const element of document.querySelectorAll('[id]')) {
	if (seenIds.has(element.id))
		reportIssue(element, `Duplicate id "${element.id}"; ids must be unique.`);
	seenIds.add(element.id);
}
for (const element of document.querySelectorAll('*')) {
	if (element.localName.startsWith('ex-') && !customElements.get(element.localName))
		reportIssue(element, `Unknown element <${element.localName}>.`);
}

document.addEventListener('click', (event: MouseEvent): void => {
	if (!(event.target instanceof Element)) return;
	if (event.target.closest('[data-ex-theme]')) {
		const current =
			document.documentElement.dataset['theme'] ??
			(matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');
		document.documentElement.dataset['theme'] = current === 'dark' ? 'light' : 'dark';
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
