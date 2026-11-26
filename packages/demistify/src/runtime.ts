import { reportStructuralIssues } from './core/issues.ts';
import { captureReviewDocument, installDefaultComments } from './core/review-document.ts';
import { CallStackIsland } from './islands/call-stack.ts';
import { CalloutIsland, CardIsland } from './islands/card.ts';
import { ChoiceIsland, OptionIsland } from './islands/choice.ts';
import { CodeIsland, CodeNoteIsland } from './islands/code.ts';
import { CommentsIsland } from './islands/comments.ts';
import { CompareIsland } from './islands/compare.ts';
import { DeliverablesIsland } from './islands/deliverables.ts';
import { DrawerIsland } from './islands/drawer.ts';
import { EvidenceTraceIsland } from './islands/evidence-trace.ts';
import { ExplorerIsland } from './islands/explorer.ts';
import { FileTreeIsland } from './islands/file-tree.ts';
import { GalleryIsland } from './islands/gallery.ts';
import { InitiativeOverviewIsland } from './islands/initiative-overview.ts';
import { MapIsland } from './islands/map.ts';
import { MarkdownIsland } from './islands/markdown.ts';
import { MermaidIsland } from './islands/mermaid.ts';
import { PreformattedIsland } from './islands/preformatted.ts';
import { BadgeIsland, BreakdownIsland, ProgressIsland } from './islands/small-parts.ts';
import { StatIsland } from './islands/stat.ts';
import { FoldToggleIsland, SectionIsland } from './islands/story-section.ts';
import { StoryIsland } from './islands/story.ts';
import { TabsIsland } from './islands/tabs.ts';
import { TreeIsland } from './islands/tree.ts';
import { ArrowIsland } from './layout/arrows.ts';
import { BoardIsland, RegionIsland } from './layout/board.ts';
import { hydrateDesignRecords } from './layout/design-records.ts';
import { DocumentIsland, GridIsland, installDocumentBehavior } from './layout/document.ts';
import { FlowIsland } from './layout/flow.ts';
import { LayersElement } from './layout/layers.ts';
import { HeadingNavigation } from './layout/navigation.ts';
import { installRoutes, RouteElement, StepElement } from './layout/route.ts';
import { placeSectionItems, SectionItemIsland } from './layout/section-items.ts';
import { installArrowTracing } from './layout/trace.ts';

// The custom-element registry requires a distinct constructor for each name.
class TimelineAlias extends EvidenceTraceIsland {}

// A board is defined first so it arranges its children before they upgrade.
const islands = [
	['ex-board', BoardIsland],
	['ex-region', RegionIsland],
	['ex-trace', EvidenceTraceIsland],
	['ex-timeline', TimelineAlias],
	['ex-story', StoryIsland],
	['ex-section', SectionIsland],
	['ex-section-item', SectionItemIsland],
	['ex-fold-toggle', FoldToggleIsland],
	['ex-markdown', MarkdownIsland],
	['ex-option', OptionIsland],
	['ex-choice', ChoiceIsland],
	['ex-comments', CommentsIsland],
	['ex-mermaid', MermaidIsland],
	['ex-arrow', ArrowIsland],
	['ex-tabs', TabsIsland],
	['ex-drawer', DrawerIsland],
	['ex-call-stack', CallStackIsland],
	['ex-callout', CalloutIsland],
	['ex-compare', CompareIsland],
	['ex-file-tree', FileTreeIsland],
	['ex-gallery', GalleryIsland],
	['ex-pre', PreformattedIsland],
	['ex-explorer', ExplorerIsland],
	['ex-deliverables', DeliverablesIsland],
	['ex-initiative-overview', InitiativeOverviewIsland],
	['ex-tree', TreeIsland],
	['ex-map', MapIsland],
	['ex-card', CardIsland],
	['ex-grid', GridIsland],
	['ex-flow', FlowIsland],
	['ex-code', CodeIsland],
	['ex-note', CodeNoteIsland],
	['ex-stat', StatIsland],
	['ex-badge', BadgeIsland],
	['ex-progress', ProgressIsland],
	['ex-breakdown', BreakdownIsland],
	['ex-presentation', RouteElement],
	['ex-step', StepElement],
	['ex-document', DocumentIsland],
	['ex-nav', HeadingNavigation],
	['ex-layers', LayersElement],
] satisfies readonly (readonly [string, CustomElementConstructor])[];

function startRuntime(): void {
	placeSectionItems();
	// Preserve authored source before any island upgrades replace it or add controls.
	for (const root of document.querySelectorAll<HTMLElement>('ex-document'))
		captureReviewDocument(root);

	for (const [tagName, constructor] of islands) {
		if (!customElements.get(tagName)) customElements.define(tagName, constructor);
	}

	reportStructuralIssues();
	installDefaultComments();
	installDocumentBehavior();
	if (!document.querySelector('ex-nav[src]')) installRoutes();
	installArrowTracing();
}

const recordsReady = hydrateDesignRecords();
if (recordsReady) void recordsReady.then(startRuntime);
else startRuntime();
