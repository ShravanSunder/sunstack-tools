import { reportStructuralIssues } from './core/issues.ts';
import { CalloutIsland, CardIsland } from './islands/card.ts';
import { CodeIsland, CodeNoteIsland } from './islands/code.ts';
import { FileTreeIsland } from './islands/file-tree.ts';
import { ZoomIsland } from './islands/inspector.ts';
import {
	CompareIsland,
	DecisionIsland,
	DiagramDiffIsland,
	LayerIsland,
	TimelineIsland,
} from './islands/legacy.ts';
import { MarkdownIsland } from './islands/markdown.ts';
import { MermaidIsland } from './islands/mermaid.ts';
import { StatIsland } from './islands/stat.ts';
import { TabsIsland } from './islands/tabs.ts';
import { ArrowIsland } from './layout/arrows.ts';
import { BoardIsland, RegionIsland } from './layout/board.ts';
import { DocumentIsland, GridIsland, installDocumentBehavior } from './layout/document.ts';
import { FlowIsland } from './layout/flow.ts';
import { HeadingNavigation } from './layout/navigation.ts';
import { installRoutes, RouteElement, StepElement } from './layout/route.ts';

// A board is defined first so it arranges its children before they upgrade.
for (const [tagName, constructor] of [
	['ex-board', BoardIsland],
	['ex-region', RegionIsland],
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
	['ex-code', CodeIsland],
	['ex-note', CodeNoteIsland],
	['ex-stat', StatIsland],
	['ex-route', RouteElement],
	['ex-step', StepElement],
	['ex-document', DocumentIsland],
	['ex-nav', HeadingNavigation],
] satisfies readonly (readonly [string, CustomElementConstructor])[]) {
	if (!customElements.get(tagName)) customElements.define(tagName, constructor);
}

reportStructuralIssues();
installDocumentBehavior();
installRoutes();
