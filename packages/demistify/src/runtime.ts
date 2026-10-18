import { reportStructuralIssues } from './core/issues.ts';
import { CalloutIsland, CardIsland } from './islands/card.ts';
import { CodeIsland, CodeNoteIsland } from './islands/code.ts';
import { CompareIsland, DiagramDiffIsland } from './islands/compare.ts';
import { EvidenceTraceIsland } from './islands/evidence-trace.ts';
import { ExplorerIsland } from './islands/explorer.ts';
import { FileTreeIsland } from './islands/file-tree.ts';
import { GalleryIsland } from './islands/gallery.ts';
import { ZoomIsland } from './islands/inspector.ts';
import { DecisionIsland, LayerIsland, TimelineIsland } from './islands/legacy.ts';
import { MarkdownIsland } from './islands/markdown.ts';
import { MermaidIsland } from './islands/mermaid.ts';
import { PreformattedIsland } from './islands/preformatted.ts';
import { StatIsland } from './islands/stat.ts';
import { TabsIsland } from './islands/tabs.ts';
import { TreeIsland } from './islands/tree.ts';
import { ArrowIsland } from './layout/arrows.ts';
import { BoardIsland, RegionIsland } from './layout/board.ts';
import { DocumentIsland, GridIsland, installDocumentBehavior } from './layout/document.ts';
import { FlowIsland } from './layout/flow.ts';
import { LayersElement } from './layout/layers.ts';
import { HeadingNavigation } from './layout/navigation.ts';
import { installRoutes, RouteElement, StepElement } from './layout/route.ts';
import { installArrowTracing } from './layout/trace.ts';

// A board is defined first so it arranges its children before they upgrade.
for (const [tagName, constructor] of [
	['ex-board', BoardIsland],
	['ex-region', RegionIsland],
	['ex-trace', EvidenceTraceIsland],
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
	['ex-gallery', GalleryIsland],
	['ex-pre', PreformattedIsland],
	['ex-explorer', ExplorerIsland],
	['ex-tree', TreeIsland],
	['ex-card', CardIsland],
	['ex-grid', GridIsland],
	['ex-flow', FlowIsland],
	['ex-code', CodeIsland],
	['ex-note', CodeNoteIsland],
	['ex-stat', StatIsland],
	['ex-presentation', RouteElement],
	['ex-step', StepElement],
	['ex-document', DocumentIsland],
	['ex-nav', HeadingNavigation],
	['ex-layers', LayersElement],
] satisfies readonly (readonly [string, CustomElementConstructor])[]) {
	if (!customElements.get(tagName)) customElements.define(tagName, constructor);
}

reportStructuralIssues();
installDocumentBehavior();
installRoutes();
installArrowTracing();
