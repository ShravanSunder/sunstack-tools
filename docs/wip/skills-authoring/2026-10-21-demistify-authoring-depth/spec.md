# Demistify component-authoring guide refresh

Proposed update to `demistify-explainer` in `packages/demistify/skill/`. The guide owns actual component syntax and behavior; `presentation-html` owns the document-level authoring workflow. This draft specifies the teaching change before implementation.

## Why this needs a change

The current main file covers the components but combines selection, syntax, widths and controls in dense paragraphs. Several basic families have no detailed reference, while code and map references are too short to teach a complete authored example. Agents need enough detail to choose and compose each construct without copying another document's layout or inventing attributes.

The success condition is a fresh, realistic task: the agent chooses the supported construct, loads and applies its reference, preserves the supplied facts, and produces an independently usable artifact with its actual proof limits. Repeating the guide's words is not a success measure.

The current baseline is the guide and nine references published with Demistify 1.10.0 at `f5cd2f2d247830beb4f02719cdeceeac13ec0435`. The 1.10.0 main guide already describes the shared left edge and author-check mode; the broader component teaching proposed here is not implemented by that release. This is user-directed teaching work, not a measured claim that a prompt caused the product defects repaired in #60.

The guide should let an agent choose a suitable component, open the right reference, and author a correct page without reconstructing the API from examples in another project.

Its name and invocation stay the same. It remains the library-owned guide for writing, editing and checking Demistify markup. The runtime, theme, guide and checker must describe the same binding.

## What stays in the main file?

The main file must show the whole route in one scan:

1. Establish the matching library binding and the intended opening method.
2. Choose components and layout from the reader's task; load the relevant references.
3. Write the content once, provide useful navigation, and use supported composition.
4. Run the matching checker, open the intended artifact and exercise its authored behavior.
5. Return the artifact with its verified result and any exact limitations.

Keep the mental model near the start: the document owns the explanation; components provide reusable views; Pages move between documents; local disclosures reveal optional detail; Walkthroughs guide stops through existing content; Gallery Journeys guide picture-only stops.

Keep the two width standards visible in the main file. Reading holds explanatory text at 760px with left-aligned headings and paragraphs; from Demistify 1.10.0 ([#63](https://github.com/ShravanSunder/sunstack-tools/pull/63), owner-approved after a rendered before/after comparison) Reading and Canvas share one left edge instead of centering Reading inside Canvas. Teach whichever the bound release ships. Canvas gives visual material more room. The detailed reference owns the dimensions and examples; the main file keeps the rule that whole components include their controls, while document headings and dividers follow Reading.

Replace dense mixed-purpose paragraphs with a component chooser. Each choice leads to one named reference and says what the agent must bring back. The chooser covers every supported authoring component family, including the basic components whose current documentation is thin.

## What does a useful component reference contain?

Each reference teaches the work selected by the main file:

- **Choose it for:** the reader task it serves, with a nearby alternative when that changes the choice.
- **Start with:** a small, complete example using the documented API.
- **Configure it:** relevant attributes, accepted child structure and important defaults.
- **Use it:** the reader's controls, keyboard behavior and state changes.
- **Combine it:** width, nesting, navigation and inspection behavior that matter for this component.
- **Avoid the common mistake:** a concrete bad result and its repair.
- **Check the result:** observations that show this authored composition works.

These are teaching requirements, not mandatory headings or a form to fill with empty boilerplate. A simple component earns a short explanation. A Gallery earns substantially more detail.

## Reference homes

The following file allocation is proposed. Existing references retain their responsibilities; new homes collect detail currently scattered through the main file.

| Reference                                     | Components and detail it owns                                                                                                             | Example it must make possible                                                                             |
| --------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| `references/content.md` — new                 | Document content, bare Markdown, `ex-markdown`, exact-source carriers, live versus embedded sources, native tables and local disclosure   | An explanation with a table, literal markup, optional detail and preserved source                         |
| `references/layouts.md` — new                 | Reading/Canvas application, heading/divider alignment, width transitions, `ex-grid`, `ex-tabs`, labels/icons, columns and narrow behavior | Reading text around a Canvas view, plus a text-only tab set whose bar and panels share Reading width      |
| `references/cards-and-callouts.md` — new      | `ex-card`, `ex-callout`, `ex-stat`; subject icons, tones, tags and chosen state                                                           | A clear proposal, its consequence and an unresolved decision, without turning every paragraph into a card |
| `references/navigation.md` — new              | `ex-nav`, Pages, outline, breadcrumbs, command search, collection indexing and stable heading/island links                                | A directly openable collection with Pages and contextual heading search                                   |
| `references/inspection.md` — new              | Shared sidebar/fullscreen inspection, inspection opt-in/out, zoom/pan, closing and content restoration                                    | Inspect a rich island beside its explanation, then fullscreen and return                                  |
| `references/diagrams.md` — new                | `ex-mermaid`, supported families, source carriers, readable labels and documented node/card interactions                                  | A readable relationship diagram with a useful stable target                                               |
| `references/code.md` — expand                 | `ex-code`, `ex-note`, literal source, unified diffs, file/line addressing                                                                 | Single-file code and a multi-file diff with notes attached to the displayed lines                         |
| `references/maps.md` — expand                 | `ex-board`, `ex-region`, `ex-flow`, `ex-arrow`, `ex-layers`; reading versus map view, endpoints and optional detail                       | A small ownership board and a flow whose arrows reference real boxes                                      |
| `references/pictures.md` — update             | Ordinary figures, dedicated Gallery pages, metadata, filters, tags, thumbnails, fullscreen and browse keys                                | An explanation linking to a Gallery with actual screen/variation evidence                                 |
| `references/tours.md` — update                | `ex-presentation`, `ex-step`, narration, cross-page continuation, Gallery Journeys and optional variations                                | A guided path through existing text and diagram targets, with a separate pictured journey                 |
| `references/comparisons.md` — update          | Corresponding rows, columns/tabs, pair selection, sticky headers, content height and nested supported views                               | Long alternatives aligned by topic without a fixed-height comparison scroller                             |
| `references/trees.md` — retain and refine     | `ex-tree`, `ex-file-tree`, `ex-pre`; hierarchy, status, independent disclosure/navigation and literal text                                | A readable work tree whose folding and go-to actions are independent                                      |
| `references/traces.md` — update               | Ordered native disclosures, rich attachments and precise source/observation labels                                                        | An evidence sequence with code, an ordinary picture and a link to a Gallery                               |
| `references/call-paths.md` — refine           | Source-derived calls, branches, outcomes and unknown execution edges, composed through Compare and Trace                                  | A before/after path that does not turn a branch into a call frame                                         |
| `references/explorers.md` — retain and refine | Supplied graph documents, real selection IDs, relationship views, evidence and data delivery                                              | A supplied graph opened with its original facts and usable navigation                                     |

Keep authoring syntax in these component homes. Generated API metadata remains the authoritative vocabulary; examples and option descriptions must agree with it. Legacy accepted tags are not new teaching choices.

## How the main file calls the references

Each component choice uses an observable condition and a concrete result. For example:

> IF the page needs screenshot browsing, load `references/pictures.md` and return a dedicated Gallery page using the supplied captures, authored metadata and supported controls.

> IF the page contains tabs or a grid, load `references/layouts.md` and return the component's width, child structure, labels and narrow-layout behavior.

> IF the page offers a guided path, load `references/tours.md` and return its named path, narration and resolving destinations.

The same form applies to code, cards/callouts, diagrams, maps, trees, Trace, comparisons and Explorer when those constructs occur. General document/source delivery remains an obligation in the main route; `content.md` supplies its coherent detailed procedure. Navigation depth is loaded when a page offers a collection, heading navigation or command-search indexing. Inspection depth is loaded when an authored view uses sidebar/fullscreen inspection.

References return a usable decision or authored result. A reference that merely lists names does not finish the component's teaching.

## Changes that need consistent examples

**Gallery pages.** Teach the dedicated-screen direction throughout Pictures, Tours, Compare, Trace, the main chooser and the package README. An inline picture uses an ordinary figure; an explanation links to the Gallery. The prepared `gallery-page` checker enforces its exact accepted layout. Its final guidance and diagnostic must agree.

**Gallery controls.** The hamburger beside search controls thumbnails. Pages and the heading outline remain separate document navigation. Explain title followed by quiet authored tags, the short collapse animation, reduced-motion handling, fullscreen arrows and how closing restores browse state. Keyboard instructions distinguish ordinary browsing from an active guided journey and preserve editing/overlay ownership.

**Narration.** General Walkthroughs and Gallery Journeys use the same centered player, title hierarchy, elevation and focused Markdown. Explain supported narration with one structured example; large document views belong at the target. Narration size reserves the space it uses.

**Widths and typography.** Show the shared rules in examples rather than adding per-page fonts, colors, width caps or shadows. Text remains left aligned inside Reading. A table or visual can occupy Canvas while its document heading stays with the text. Tree hierarchy uses the existing shared styling.

**Surface shape and tables.** Use the existing shared corner-radius scale consistently. The owner's preference is soft rounded corners in most enclosed surfaces; the medium token is the starting point for an ordinary table, card or control. Retain compact and larger tokens where the component's established scale warrants them. The new table direction uses a neutral gray header fill and border, existing quiet body stripes and rounded outer corners. The shared-theme treatment shipped in Demistify 1.9.4. Teach it as a shared default for that release rather than having each authored page recreate it.

**Proposals and evidence.** Use supported subject icons and clear short labels. A proposal is visibly proposed; a recommendation does not become an owner's chosen option. Supplied code is separate from observed execution. The icon list must include the implemented `path` name used by the existing narration example.

**Chips and status labels.** Explain the existing shared `.chip` styling separately from an island's `tag` and `pick` header labels. The current chip uses monospace text, a leading dot and a pill shape; there is no dedicated `ex-chip` or general inline status-badge element in the current vocabulary. The owner's screenshot raises a useful product gap for statuses such as published, pass and measured counts in tables. A regular-text badge with semantic tone and optional icon is only a proposal. Teach supported existing forms without inventing a new element, and revisit the example if that product capability is accepted and released.

**Comparison height.** Preserve natural document height, corresponding topic alignment and sticky column headers. The page owns vertical scrolling; a nested viewer retains its own controls. Use ordinary pictures or Gallery links in the dedicated-page standard.

**Card composition.** Teach one card as one named component, option or finding. Its body can contain Markdown, ordinary HTML, lists, short code, pictures and optional detail. Rich child islands and nested cards are technically possible, but they need a reason beyond enclosing the whole section. A tabbed comparison already groups its alternatives: prefer an explanation and Flow directly in the option, with cards for the Flow's named objects, rather than another large card around those cards. Keep long explanations in document flow and screenshot browsing on a dedicated Gallery page. Show the framed and unframed versions so the choice is visible.

**Optional depth and connectors.** A short disclosure has one concise summary and ordinary body text; it does not need another document-sized heading and section gap immediately inside it. A rich wide disclosure may use the selected shared surface treatment once that proposal is chosen. Flow connectors need room for their complete captions, multiline wrapping where needed, and alignment appropriate to the facing cards. Moving a label away from its arrow is not a substitute for sufficient connector space. The released 1.10.0 runtime can still omit a caption when no collision-free placement is found. Fallback placement is an open product requirement; teach the behavior of the bound release rather than describing that fallback as shipped.

The package README keeps its quickstart and examples consistent with these homes and links readers to their detail. It does not become another full manual. Source-only checker commands remain the documented route until a portable checker is actually released; the proposal does not turn the measured portable-checker experiment into a published capability.

## Evidence status of this proposal

Each part of this spec carries one label. **measured**: observed in the 2026-10-20 authoring pyramid (13 fresh agents, each on a different subject, guide at 1.9.3; `docs/wip/demystify/2026-10-20-agent-authoring-pyramid.md`, [#63](https://github.com/ShravanSunder/sunstack-tools/pull/63)). **owner**: an owner decision. **release**: product behavior already shipped or pending. **intent**: not yet observed in any run.

| Part                                                                                                                                         | Label                     | Evidence                                                                                                                       |
| -------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| Main file too dense; selection, syntax, widths and controls mixed in paragraphs                                                              | measured                  | Every Sonnet/Opus agent called the route heavy; five agents skipped MUST loads to stay in budget                               |
| Code and map references too short to author from                                                                                             | measured                  | Agents probed `ex-note` diff numbering (H3), `ex-code` languages (H4), Flow label placement (H2, O2)                           |
| Comparison topic headings flood the outline; `comparisons.md` example uses `<h3>`                                                            | measured (L1)             | S2 audit 6 duplicates; PoC pages; O1, O2, H4 worked around it                                                                  |
| Callout `icon`, `path` icon, `<ex-mermaid id>` disagree between guide, checker data and skill                                                | measured (friction 1)     | custom data vs guide island table                                                                                              |
| Undocumented facts agents had to probe: slug rule, cross-page scoped stop IDs, `index` edits in place, script-assigned IDs, tone optionality | measured (friction 6)     | O1, S1, S3                                                                                                                     |
| Walkthrough `ex-nav` launcher rule stated in three places                                                                                    | measured                  | presentation-html step 3, guide, `tours.md`                                                                                    |
| Literal `<` in bare Markdown corrupts layout; Markdown in a plain `div` renders literally; `## Heading {#id}` renders literally              | measured (L2, L3, L11)    | repro page R1, R3, R7                                                                                                          |
| `xychart-beta` legend "Bar 1 / Line 1" and smoothed lines                                                                                    | measured (L7)             | repro R10                                                                                                                      |
| Table neutral gray header and rounded corners                                                                                                | release 1.9.4             | shipped                                                                                                                        |
| Shared left edge; author-only issue panel (`?ex-check`, `data-ex-issues`); stat at Reading width; phone toolbar                              | release 1.10.0 (#63, #64) | owner-approved layout; browser tests                                                                                           |
| Flow labels fall back and report to the author only                                                                                          | owner                     | 20 October decision                                                                                                            |
| New reference homes (`content`, `layouts`, `cards-and-callouts`, `navigation`, `inspection`, `diagrams`)                                     | intent                    | allocation not yet tried by an agent                                                                                           |
| Gallery dedicated-page direction, narration player, chips vs tags, card composition, comparison height                                       | intent                    | owner direction; no fresh authoring run yet                                                                                    |
| Fresh authoring tasks as proof                                                                                                               | intent                    | planned; the 2026-10-21 presentation-html Haiku experiments exercise this guide at 1.9.4 and their results will be linked here |

## How we will know this teaching is useful

Use fresh authoring tasks that require selecting and applying the detail: a mixed-width explanation, a screenshot collection with variations, a multi-page guided path, and a rich comparison or Trace. Inspect the resulting artifacts and the recorded reference reads. Judge the output against the request, not whether it repeats this proposal's words.

Run the existing matching markup/data checker on the actual examples. Exercise authored links, keys, fullscreen and narrow layouts through the intended delivery method where access permits. Include a missing target or invalid Gallery composition in the established checker harness to show that the relevant check rejects the wrong build.

A syntax pass does not establish good component choice or readable content. A browser-access failure does not establish a skill failure. Until changed teaching has been used in a fresh task, describe it as authored from the requested intent, with behavior not yet evaluated.

## Scope and source boundaries

Use the existing public component vocabulary and shared theme at the artifact's binding. Examples must use capabilities that release actually supports. Keep static structure checks, rendered behavior, source truth and measured agent usage as separate claims.

Do not introduce a navigation engine, server requirement, renderer or new component API in this teaching change. The proposed general inline status badge remains a separate product question; existing chips and header tags are the current forms. The canceled curved-sidebar experiment is excluded.

Runtime fixes and the neutral rounded table treatment belong to Demistify's product release. The authoring guide may explain them once they are part of the bound release; authored examples should not recreate them with page-specific paint. Preserve the current radius scale and purposeful exceptions rather than assigning a new radius per page.

The implementation keeps each rule in one owner and updates its active callers together. Preserve existing complete material where the inventory identifies it as working. Keep ordinary reader examples meaningful; state-retention instruments belong in explicit fixtures.

The companion workflow proposal is [presentation-html #155](https://github.com/ShravanSunder/sunstack/pull/155). It retrieves this component guide and preserves this ownership boundary.
