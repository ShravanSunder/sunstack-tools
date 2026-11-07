# Deliver the approved show-me page with Demistify

The owner has approved the page's appearance and behavior. The delivery keeps the page's content as authored HTML/Markdown and moves its shared styling and interaction into Demistify. The agent selects stories, text, groups, IDs and supplied log entries; the library neither reads Router nor stores or summarizes stories.

Reference: the design-documents Lead's commission, `demistify-commission-show-me-page.md`, and frozen `p1b-approved-2026-11-07.html`, SHA-256 `ece32b0cdb2a6fe432f0db763ebdd7e55b885f43114e9a36cc555584d7f676c4`. Source baseline: sunstack-tools `26b316c29ed3797c54bd639573f361e54b65207a`. Isolated branch: `feat/demistify-show-me-page`.

This is the requested initial delivery outline and estimate, not an executable ready implementation plan. The visual reference settles the reader experience; the reusable component contracts still need their design and independent review. No product source has changed.

## Existing parts and actual gaps

| Need | Existing owner and current behavior | Work required |
| --- | --- | --- |
| Native story and record folds, rich attachments | `islands/evidence-trace.ts` wraps native details without flattening their body; it retains authored summary nodes. Its theme is larger and heavier than the approved page. | A reusable compact story/entry presentation, metadata and labelled rows that match the reference; preserve supported rich child content. |
| Section surface | `islands/card.ts` and shared `createHead` already own title, tone, icon, tag, category and surface. | Sticky header below the real toolbar, count and a declarative bulk action; preserve existing cards' default behavior. |
| Fold state | Native details is already the authoritative live state. | Bulk action derives its icon, action and accessibility label from current descendants, including mixed state and changes made by other controls; no second remembered Boolean. |
| More, record lists and smaller stories | Native details and existing grid/table/quote/diagram components cover the content. | Document the composition and supply the compact shared appearance. Counts come from authored membership; do not infer story importance. |
| Right panel and Comments exclusion | `islands/drawer.ts::openDrawer` owns the mutually exclusive right sidebar and the phone sheet. Comments and inspection already use it. | A declarative document-facing Map panel and trigger that reuse this owner, rather than another drawer implementation. Narrow geometry must cover below the toolbar. |
| Bottom workspace | `DrawerIsland` is a separate non-modal bottom workspace, deliberately allowed beside Comments. | Preserve that established behavior; a Map panel must not silently change it. |
| In-page reveal | `layout/document.ts` routes hash changes through `target-resolution.ts`. Ancestor details open, but `revealElement` skips the target itself; a story whose own details carries the ID can stay shut. | Route Map links through one target/reveal owner, open the destination story and ancestors, close the narrow Map panel before scrolling, and retain ordinary link semantics. Same-fragment repeat clicks need coverage. |
| Tree link icon | `islands/tree.ts` always emits `arrow-up-right`. | Detect a same-document fragment by resolved URL identity and use the selected in-page jump icon; other destinations keep the external/other-page arrow. Do not make every URL with a hash look local. |
| Initial tree fold | The renderer always sets nested branches expanded. The guide's leading-chevron wording is about the generated control, not a supported Markdown marker. | Choose and document one unambiguous marker that survives the actual Markdown parser; keep tone/checklist meaning and literal content intact. Verify marker order and unmarked defaults. |
| Reader control row | Theme/search/Comments/walkthrough controls use fixed positions in navigation CSS. | Integrate the Map control without another hardcoded coordinate collision, including narrow toolbar placement. |

## Order of delivery

1. **Pin the component contract and reference.** Materialize the approved reader outcomes, minimal public HTML/Markdown contracts, current/target owners and verification rows. Review those contracts before code consumes them. Preserve every original story and supplied entry without promoting historical claims to current facts.
2. **Build the smallest complete story section.** Shared compact story/entry/labelled-row appearance, one rich showpiece, optional closed More, closed record list and native entry folds; sticky section header and page/section bulk actions. Exercise mixed state, manual folding, nested records and long metadata before filling out the rest of the page.
3. **Complete tree links and initial folds.** Correct the local-link icon and one documented folded-branch marker through the existing tree renderer/parser. A fold action stays independent of following its link.
4. **Integrate Map with the existing sidebar owner.** Declarative trigger in the shared control row, one supplied tree, Comments exclusion, close/Escape/focus behavior, narrow viewport coverage and reveal-then-scroll. Preserve the separate bottom workspace and normal Comments behavior.
5. **Compose the full comparison page and guide.** Use only the new documented components plus existing HTML/Markdown islands. No page-owned style block or executable interaction script. Keep all four sections, routine-story folds and decision columns. Put reusable syntax in the component guide; keep real private story content outside public product source, with sanitized representative fixtures in the demo app.
6. **Review, prove and release.** Focused permanent regressions and repository quality gates; direct browser interaction and screenshots at 1280, 420 and the repository's 390 CSS-pixel width, in affected light/dark themes. Independent implementation review first. Then release the reviewed build/guide through the repository's existing release process, verify the actual hosted matching assets, and ask the existing Terra evaluator to compare a page bound only to those released parts with the approved reference. The owner sees it only after that check.

One coherent source PR is the default for this deliverable, with these internal slices; no separate source-repository skill rewrite is assumed. Release notes identify the guide sections and what shipped. A source merge, HTTP 200 or parser pass alone does not establish released usability or visual fidelity.

## Acceptance cases that decide the result

- Stories open on first load; More, the record list and individual entries start closed. A supplied explicit state is preserved when carrying existing content.
- The headline, Happened / For you / Next rows, subject/time/icon and optional showpiece retain their hierarchy and rich content at all required widths.
- A section header sticks below the toolbar with an accurate count and the icon-only double-down/double-up action at its right. Bulk actions stay synchronized after manual and other-scope changes; they do not change another section.
- A routine-story link opens the routine fold, story and required ancestors without opening unrelated record entries. Direct load, repeated same-fragment click and browser back/forward are considered.
- Map and Comments are mutually exclusive in both opening orders; close/Escape/focus return work; narrow Map closes before the target scroll. Outside interactions and existing inspector/bottom-workspace behavior retain their contracts.
- Fragment, same-page absolute/relative, other-page fragment and external links display the intended navigation icon and retain their destination. Malformed or absent targets fail visibly rather than silently selecting another story.
- The chosen tree marker produces a closed branch through real Markdown parsing; unmarked branches retain their default. Unicode chevrons remain literal unless explicitly made part of the chosen contract.
- The final composed page has no prototype CSS or custom interaction script and all controls come from the matching released Demistify version. Labels, counts, URLs and story/entry IDs are compared with the frozen content fixture.

## Estimate and current limits

Initial estimate: roughly 4–8 focused engineering hours for component implementation and local proof, then the independent review, release and Terra comparison. Allow about one working day overall if those checks are available; that is an estimate, not a release promise. The Map/Comments/control-row integration is the largest uncertainty, not the two tree fixes. Report actual progress after the first working section and after the Map flow.

Current source inventory is verified, not a visual comparison. The browser tool reports Chrome unavailable and explicitly refuses observation of the frozen `file:` URL under its URL policy. No HTTP copy, alternate surface, raw CDP or other workaround was attempted. An owner-supplied capture or another expressly permitted reference-observation route is needed for the reference's pixel comparison; independent source/contract work can continue. Candidate runtime proof later needs an allowed served test target and must not be reported as proof that the blocked reference was inspected.

No public component names or initial-fold syntax are declared shipped by this outline. Those choices belong in the reviewed reusable API design; the guide will carry concrete examples once implemented. No log reader, story persistence, story-ranking system, new framework or replacement Comments drawer is in scope.
