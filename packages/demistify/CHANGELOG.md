# @sunstack/demistify

## 1.35.0

### Minor Changes

- 591ba3c: `initiative.ts` takes `pages`: initiative-level HTML pages, relative to it, that may place `initiative/` records and any project's records by qualified ID. `initiative check` warns about an HTML page under the initiative that binds records while no `design.ts` or `initiative.ts` registers it, and `open` finds the initiative as the owner of a registered page.

### Patch Changes

- 7a46401: The records-source error now names a `../<project>/` folder instead of an old milestone folder, and the library builds with npm 12, whose `npm pack --json` prints an object keyed by package name.

## 1.34.2

### Patch Changes

- 3aab3a3: A page that places design records but has no record source now shows a visible notice instead of empty sections with a zero count, and names `ex-project-overview` (replaced by `ex-initiative-overview` in 1.34) when a page still uses it. The standalone `check` accepts a typed choice inside a record section item without authored titles, as `initiative check` does.

## 1.34.1

### Patch Changes

- 6881cfd: The initiative migration script (internal, not in the CLI) adopts an existing project folder without design.ts: it adds the typed files and a moved-records.html page beside the folder's own pages and leaves every existing file untouched.
- 90188b9: `initiative status` and the initiative view list ready work in project-number order and leave out core deliverables with no implementation steps; those appear in a new "needs a plan" list.

## 1.34.0

### Minor Changes

- 47a7642: Organize typed design as an initiative of numbered projects. `initiative.ts` replaces `project.ts`: each project names its design, a number (the same number runs side by side; 2 comes after 1), what it achieves and what the owner checks. Deliverables may depend on deliverables in any project, and the check warns when one depends on a higher-numbered project. `demistify initiative check` replaces `project check`; `demistify initiative status` prints the plan, and the generated view lists what is ready now, what blocks the most and what is built but not accepted. The initiative overview opens with the projects by number, and every state, level and id prefix carries a plain explanation.

  This experimental v1 update renames the levels. Rename `project.ts` to `initiative.ts` with `projects: { name: { design, number } }`, `project-decisions.ts` and `project-assumptions.ts` to `initiative-decisions.ts` and `initiative-assumptions.ts`, `project/` references to `initiative/`, and `ex-project-overview` with `project.generated.json` to `ex-initiative-overview` with `initiative.generated.json`. Regenerate the design SDK and views with the matching CLI.

### Patch Changes

- 1d88f32: Remove `demistify initiative migrate` from the CLI. The one-time conversion and re-cut of `project.ts` trees runs from a source checkout as `node packages/demistify/scripts/initiative-migration/run.ts`, documented beside the script, and is not part of the published CLI or guide.

## 1.33.0

### Minor Changes

- a9e9a34: Render choices, rulings and assumptions as section items with their own evidence, state and links to dependent work. Keep option summaries and Details available after a choice is answered.

  This experimental v1 update changes typed decision and assumption authoring. Use the new record helpers and authority fields, move page content out of section declarations into section items, and put record option Details in templates. Regenerate the design SDK and record views with the matching CLI after migrating the source.

  Existing v1 pages keep their published Choice answers, Comments and Story disclosure controls while their owners migrate. Label chips use a plain outline and supported status tones carry a dot. Code blocks use the dedicated code surface.

## 1.32.1

### Patch Changes

- 7e19790: Load specialized authoring references only when a page needs them. Use Trace for new source-derived call paths and keep the prepared call-stack reader working for existing pages.
- 891b326: Preserve quoted evidence inside HTML containers during lean-record migration while keeping existing fragment links pointed at their records.

## 1.32.0

### Minor Changes

- 616dec9: Author lean typed design records and place their statements in HTML by ID. Map, Tree,
  Walkthrough, Choice, and Deliverables share the checked record projection. The CLI
  opens fresh project checkouts, prints scoped deliverable briefs, and checks core
  delivery against its current build and evidence. Existing typed designs must adopt
  the new record, placement, and delivery shapes.

## 1.31.0

### Minor Changes

- c91a8bb: Split a milestone’s design records by subject. Import local TypeScript files that default-export literal record arrays, then spread them into `design.ts` alongside inline records. Run `demistify design types` to refresh the generated `DesignRecord` type.

### Patch Changes

- ca05702: Keep verification and acceptance current when adding a neighboring heading only changes the blank space after a design passage. Continue to require a recheck when the passage or its governing data changes.

## 1.30.1

### Patch Changes

- 963f346: Keep generated revisions out of routine CLI output, delivery views, and copied review text. Show delivery status and recheck guidance in plain language while retaining evidence matching in generated data.

## 1.30.0

### Minor Changes

- f69ccb9: Show decision authority, status, replacement links and affected records in the project overview. Link Map and Walkthrough targets to checked design record IDs, and carry the decision ID with answers from an HTML Choice bound to an open owner question.

## 1.29.0

### Minor Changes

- b9f42d8: Retire the old Zoom, Decision, Diagram Diff, and Layer elements. Use supported inspection controls, decisions, comparisons, and layer switches in authored pages. Timeline is now an alias for Trace. Use a native details disclosure and summary for each step.

## 1.28.1

### Patch Changes

- f38191f: Open Details beside card choices by default, with sidebar placement available on request. Add a built-in Close button to authored drawers and align desktop section fold controls on the component edge. Open Tree Details fullscreen from a compact trailing button so deep Maps retain their width.

## 1.28.0

### Minor Changes

- caa4546: Add `demistify open <page.html>` to preview authored HTML with its local Markdown, JSON, and assets. Use `--site-root` for a collection and `--no-browser` to print a URL for someone else to open.

## 1.27.0

### Minor Changes

- 9833e65: Check a Demistify project without writing generated files by default. Add `--write` to existing project check automation that refreshes project and milestone views. After updating the CLI, run `demistify design types <milestone>` for each registered milestone to refresh the generated SDK and types.

  Projects can link to declared reviewed passages within the site root. The project overview shows open owner questions, milestones, delivery state and builder assignments. The installed CLI can also check rendered pages at desktop and phone widths using a local Playwright installation.

## 1.26.0

### Minor Changes

- 533fd20: Compose a delivery graph from typed deliverable modules, track progress against a build and design scope, and render the generated delivery view in HTML.

  Record dated Lead and Owner decisions and link assumptions to the work that relies on them. Delivery state exposes open governing questions and unconfirmed or refuted assumptions.

  Keep decisions, assumptions, and delivery in named files beside each design. Check explicitly registered project sets together and resolve shared records, Lead aliases, and cross-set prerequisites.

- 83be660: Add typed design authoring and checking, including cross-set links within an explicit site root. Build an opt-in npm candidate containing the CLI, browser assets, and matching guide.

## 1.25.3

### Patch Changes

- 43c18fe: Keep saved comment counts inside their circular buttons without shifting the buttons off their target edges.

## 1.25.2

### Patch Changes

- a7d0431: Fit entity–relationship diagrams to the reading view and offer a one-click label zoom in Fullscreen.
- 4a51007: Center desktop comment controls at the upper corner of their blocks and align diagram Fullscreen below them on the same edge. Hide idle comment markers at phone width and on touch screens, with tap-to-comment and a marker toggle in the Comments panel.

## 1.25.1

### Patch Changes

- c3e2693: Keep Choice cards at reading width until beside-card Details opens. Place card comment buttons outside the border and reveal them on hover or focus. Keep saved comment markers visible.

## 1.25.0

### Minor Changes

- 38cab84: Align Details buttons at the right edge of Choice cards. Add a beside-card Details mode with an animated entrance, a Close button, keyboard dismissal, and reduced-motion support.

## 1.24.0

### Minor Changes

- 3cc0776: Draw a page or collection mind map from the same navigation JSON as the reader Map. Shared Trees have open branches, linked titles, descriptions and optional same-page Details.

## 1.23.0

### Minor Changes

- c62583f: Remove the experimental requirements, stage and related-document browser UI. Preserve the call-stack reader and prepared collection validation; call-to-need links now resolve to visible authored owner-page anchors.

### Patch Changes

- 144aa4c: Keep Option shortcut hints beside reader icons, order the shared toolbar as Pages then Search before the other controls, use Compass for Walkthroughs, and return keyboard-opened search to the reader's prior focus when cancelled.

## 1.22.0

### Minor Changes

- 7dfb134: Load Pages, a fully expanded semantic Map, and complete Walkthroughs from one shared JSON source on every document page. Reuse the existing outline, drawer and player, with subject icons, Map search, cross-page routes, and validation of shared sources and local destinations.

## 1.21.1

### Patch Changes

- c3e2e15: Publish the authoring CLI at a content-hashed archive URL and bind the matching published guide commands to it, so package-manager caches select the current tool. Retain earlier hashed archives through the existing release history.

## 1.21.0

### Minor Changes

- e454cc8: Add public Requirements and call-stack readers, the collection `prepare` command, and matching authoring guidance for linked design records in ordinary HTML.

## 1.20.0

### Minor Changes

- 2f8db1d: Add reusable non-modal bottom drawers that preserve document and Comments interaction, share drawer motion and focus handling, and support nested content sheets. Align shared table and Trace presentation with the document workspace examples.
- 29d6322: Add native HTML stories and sections with scoped disclosure controls, derived section counts, and a shared non-modal Map that supports the M shortcut and works with Comments. Add folded Tree branches and distinct local-link glyphs, with matching authoring guidance and validation.

### Patch Changes

- 52eee5b: Teach the authoring guide's content reference to keep the meaning of rich content carried onto a page along with its text: supported attributes and states such as status, icon and a disclosure's initial `open` state. A carried form or state changes only when the request or the reader's task calls for it, keeping its meaning with a brief adjacent note; an unsupported value stays as visible text with the same note. Structural or literal whitespace is kept. Each supplied ID stays on the element the source gave it to, with no copied or moved IDs and every ID unique, including `template` content. Name each component once in its own title while surrounding summaries and headings name the larger question.
- 0d27bde: Repair the authoring guide after its holistic audit:

  - Bottom workspaces are routed to `inspection.md`, which documents the drawer's markup, lifecycle and its application-embedding trigger boundary. Numeric charts are reachable from the Mermaid row.
  - Explorer documents the version-1 graph format from its parser, with a valid example.
  - Inline JSON carriers encode `<` as `\u003c`. Sensitive content binds to a trusted complete distribution. The guide now says a standalone bundle counts as checked only after the binding's checker passes it; on the exercised source, the final bundle failed that check (its inlined theme and runtime were reported as missing), and this change doesn't repair that route. Carried legacy markup is kept unless a change is requested.
  - Code `lang` behavior is described as implemented. File-tree and image-slider examples are added. Routine authoring adds no interaction journey, while requested verification, defect diagnosis and caller-required checks remain. Stops are added to traces, call paths, layouts and comparisons.
  - Lab-era and measurement-only text is removed.
  - Tab section `label` and `icon` are now checked, so an unknown tab icon is reported.

## 1.19.0

### Minor Changes

- 89a3be4: Float shared inspection controls, use expand-only enlargement controls on narrow screens, and add optional transparent card/region bodies. Explorer now offers narrow list-to-detail navigation with Back to nodes and visible node-row carets.

## 1.18.1

### Patch Changes

- 6ef376d: Correct card and region surfaces to the selected category-filled heading, neutral body and optional neutral footer. Preserve one body inset, paragraph flow, inspection clearance and review anchors.

## 1.18.0

### Minor Changes

- 8abfcdc: Adopt rounded content surfaces, darker card header/footer bands and optional native footers. Add explicit card/region category series 1–6, and consistent Explorer kind tints from the shared chart palette. Round native tables with CSS while preserving comment target ancestry.

## 1.17.0

### Minor Changes

- c52c903: Keep paragraph highlights local when browser ranges include surrounding whitespace, and give ordinary table cells, rows and tables distinct comment targets. Add persistent comment deletion and icon-labelled Add comment, Save changes, Cancel, Edit and Delete actions.

## 1.16.1

### Patch Changes

- 8c45c11: Keep Code, Pre and Mermaid feedback on their own semantic targets, with separate flowchart-node comments. Editing a comment shows only its editor and restores the saved entry on cancel. Existing page comments and saved identities are preserved.

## 1.16.0

### Minor Changes

- 2f44f29: Ship a self-contained authoring CLI tarball with the CDN distribution for pnpm dlx and npx. Reduce routine document authoring to one static lint pass; remove browser, phone/theme and shared-control regression requirements from the component guide.

## 1.15.4

### Patch Changes

- 30a3680: Teach supported multiline flowchart labels and named numeric series in the component guide. Clarify logical capture groups, variations and filters with a screen-size Gallery Journey example. Keep routine phone/theme regressions with component development while preserving authored content, markup, binding, links and requested interactions.

## 1.15.3

### Patch Changes

- 3498162: Reopen and edit the same comment when a reader comments on an item or selects text inside it. Group selected answers with their comments in the review panel and copied handoff instead of repeating the option title. Label choice detail buttons with their icon and visible Details text.

## 1.15.2

### Patch Changes

- 2861b47: Continue gallery Up/Down navigation across screen-group boundaries in both the main viewer and fullscreen. Retain Left/Right navigation at the current variation, filtered result boundaries and existing walkthrough key ownership.

## 1.15.1

### Patch Changes

- 802c20b: Keep Mermaid fullscreen inspection on its expand icon only. Remove its sidebar button and direct drawing-click enlargement while preserving linked diagram-node actions and image click-to-fullscreen.

## 1.15.0

### Minor Changes

- 85be4e0: Use a shared six-color palette across supported Mermaid families, with soft fills and automatic identity distinctions. Add built-in flowchart category classes and keep authored styles and semantic tones authoritative. Fix entity and class fields inheriting the page's monospace font size instead of their measured SVG size, and preserve field-note tooltips.

### Patch Changes

- 39431e0: Open ordinary images and Mermaid drawings fullscreen when clicked. Preserve linked visuals, diagram node actions, Gallery controls, text selection and explicit sidebar inspection; restore the original visual on close.

## 1.14.0

### Minor Changes

- 7403724: Always provide Comments automatically on ordinary documents. Derive missing review identity from the page address and revision from original content and initial loaded sources; preserve explicit identity overrides. Add automatic semantic targets, retain navigation and inspection behavior, and teach automatic availability in the matching authoring guide.

## 1.13.0

### Minor Changes

- 36bb207: Add reusable native choices and local Comments with exact text anchors, identified review transfer and shared inspection lifecycle.

## 1.12.0

### Minor Changes

- 4e57fa1: Add reusable badge, exact progress, and phase breakdown parts with their authoring guide.

### Patch Changes

- e033344: Add shared identity styling, centered content-fitting progress dials, and badge-local status paint for combined evidence rows.

## 1.11.1

### Patch Changes

- f977f4a: Preserve Reading prose and Canvas visuals inside document disclosures. Enlarged code, diagrams, file trees and comparisons use the inspector's available width while keeping their original page sizing on close.
- 01063eb: Keep adjacent headings, callouts, cards and other views separated inside tab panels and Trace attachments, including inspection. Preserve nested grid and component layouts.

  Teach a worked Trace composition with prose, a consequential callout, native table cells with an existing chip, and linked code evidence.

## 1.11.0

### Minor Changes

- 2f25f61: Require a dedicated page for Gallery authoring. The checker rejects embedded or multiple galleries with an actionable repair, while inert template data does not count as displayed content.

### Patch Changes

- de6ecb9: Teach component selection and composition through a shorter main guide and detailed content, layout, navigation, inspection, diagram and evidence references. Align width descriptions with the shared left edge and explain the existing rounded table wrappers.

  Keep Markdown headings searchable after embedding a direct-file reader, including after source edits regenerate the collection index.

  Validate code-note rows from decoded template carriers as well as scripts, preserving literal source that contains closing-script markup.

## 1.10.0

### Minor Changes

- 3659492: Start the Reading column at Canvas's left edge so headings, prose, tables, code, diagrams and grids share one left edge. Use calmer section rhythm (40px title, 28px sections, less space above each section), keep a lone stat on the Reading measure, render comparison topic headings as labels, let wide document tables scroll with readable columns, and use a narrow tab width in code. On phones, theme, search and walkthrough controls join the top toolbar instead of covering text. Authoring issues stay agent-facing: each is a `[demistify]` console warning and counted in `data-ex-issues` on the root element; the on-page panel and outlines appear only with `?ex-check`. Gallery and Explorer load failures retain a quiet in-place message for readers.

## 1.9.4

### Patch Changes

- 69f5306: Reserve space for native Flow connectors, wrap complete captions on their arrows, and align facing card ports to avoid unnecessary bends. Keep hover arrowheads modest. Restore spacing between keyed topics in tabbed comparisons and keep inspector prose at the document's regular size.
- 69f5306: Give Markdown and wrapped HTML/Explorer tables a neutral gray header, a quiet outer border, and the shared 8px rounded corners. Retain the existing row stripes and horizontal scrolling in light and dark themes.

## 1.9.3

### Patch Changes

- 80b68d8: Show authored tags beside the selected gallery picture title. Animate thumbnail collapse with reduced-motion support. Gallery Journeys and Walkthroughs share the same centered narration width, elevation and focused Markdown renderer while retaining the measured gallery gutter.

## 1.9.2

### Patch Changes

- 304d10e: Remove gallery screenshot sidebar inspection. The explicit fullscreen action and picture clicks in Review/Grid open the shared large dialog, keeping the current browse state on return. Fullscreen arrows retain screen/variation navigation without reopening the dialog. The thumbnail hamburger sits to the left of search. Clarify gallery navigation, fullscreen and Gallery Journey authoring guidance.

## 1.9.1

### Patch Changes

- 8be94a2: Restore the top Pages dropdown and add a separate search button beside the bottom theme control, with a visible platform shortcut. Preserve page context, authored descriptions and path/image icons in local and indexed walkthrough results.

## 1.9.0

### Minor Changes

- 7f9e8c0: Let rich comparisons grow with their content instead of using a height-capped scroll surface. Add an accessible gallery thumbnail toggle that retains the selected image and browse state across views and inspection.

## 1.8.0

### Minor Changes

- e7eb984: Search pages and heading ancestry from one command bar. Use Cmd/Ctrl-K for document search, P/W for Pages/Walkthroughs, numbered scoped choices and Option-held hints. Generate a small embedded collection index with `demistify index` for cross-page heading search without runtime sibling-page fetches.

### Patch Changes

- a73490f: Explain that a single Walkthrough or Gallery Journey also needs document navigation to expose its visible launch control.

## 1.7.1

### Patch Changes

- ac9ad9d: Clarify the picture guide's two-image slider limit and link rich, multi-option comparisons to their canonical guide.
- 79ebaea: Give Reading/Canvas transitions more space on wide documents, and keep diagram labels at a readable size and contrast in both themes. Clarify pages, tabs, disclosure and walkthrough selection in the authoring guide.

## 1.7.0

### Minor Changes

- af47d07: Align arbitrary comparison topics with shared CSS Grid rows, authored columns or tabs, optional content-pair selectors, sticky headings and preserved live island state. Add rich native Trace disclosures and source-derived call-path composition guidance alongside Tree.

  Make walkthrough stops land without an animated page sweep and give the player title a clear hierarchy above narration.

## 1.6.0

### Minor Changes

- 8ad68dd: Use button groups for shared mode choices. Navigate gallery screens with Left/Right and same-screen variations with Up/Down from gallery or page-body focus. Filter by authored tags and simplify zoom to Fit, Fill, minus and plus without numeric percentages.

  Keep the open inspector and reserved document/header space within the current viewport after a window resize, and restore full header width when the inspector closes.

  Reuse the native walkthrough player for gallery journeys, with a floating full-gallery surface, reserved bottom gutter, optional variation buttons and Up/Down shortcuts, explicit missing captures, and restored browse/filter/zoom state on exit.

- 8b7fe29: Render focused Markdown walkthrough narration with optional subject icons, regular 18px text and a measured floating player. Keep headings out of the narration hierarchy, escape authored HTML and reject executable link protocols. Carry narration and icons through cross-page walkthroughs; reserve space using the actual player height.

  Add the shared author-selected data-width="reading|canvas" convention to components and layout wrappers, with checker/editor metadata. The entire tab set, including its tab bar, follows the selected centered width.

## 1.5.1

### Patch Changes

- 4c34f44: Keep document headings and prose on the same centered Reading column while tables, images and other visual islands use Canvas. Remove automatic heading widening based on following siblings and the wide-section prose override. Component-internal titles retain their own layout.

  Use quiet alternating table rows and a single header separator instead of rules between every body row.

## 1.5.0

### Minor Changes

- 3a6a2ec: Standardize centered Reading (760px) and Canvas (1600px) sections. Keep text and headings left-aligned within their shared section, let visual islands use the canvas, and preserve nested component layouts and dedicated gallery sizing. Document section ownership instead of independent paragraph character caps.

  Adopt Source Sans 3 regular 18px prose and controls with Archivo headings. Give Tree roots, branches and leaves a consistent decreasing hierarchy. Section dividers always retain the centered Reading measure, even above Canvas sections.

## 1.4.0

### Minor Changes

- 65535c8: Center wide content canvases without constraining structural list rows. Let short proof text opt into the available width while narrative keeps its reading measure. Rename the presentation chooser to Walkthroughs and use compact rows with picture or path icons, without a separate type column. Use #191919 for the dark page background and #F1EFED for primary text and diagram foreground.

## 1.3.0

### Minor Changes

- 596018b: Center reading documents in the available canvas and let dedicated galleries use its full width. Refine heading hierarchy, section spacing, soft outline selection, bounded callouts, semantic accents, diff notes, icon controls, connectors and thin scrollbars while retaining existing HTML and interactions.

## 1.2.2

### Patch Changes

- 7f439c6: Settle diagram and explorer hover highlighting after 100ms on entry and exit, cancel stale pointer updates, and keep focus and presentation feedback immediate.
- ebc0978: Retain gallery keyboard focus when viewport navigation replaces a capture, anchor filters and keyboard help beside their controls, and reduce toolbar glyph size.

## 1.2.1

### Patch Changes

- 66dfe97: Keep the newly inspected item in view when replacing sidebar content, while returning focus to its trigger when the sidebar closes.

## 1.2.0

### Minor Changes

- ed5fadc: Align tree, timeline and layer content; simplify gallery and explorer controls; preserve lower-section navigation and literal source through formatting. Add concise icon labels for content views while retaining shared navigation, inspection and external assets.

## 1.1.0

### Minor Changes

- e371886: Add screenshot galleries with review/grid views, metadata filters, keyboard navigation and shared inspection. Standardize compact page navigation, ordered presentations through existing Markdown and Mermaid targets, tree links, literal text drawings and semantic styling. Update the matching authoring guide for small HTML documents importing shared Cloudflare assets, with no reader-local server.

## 1.0.2

### Patch Changes

- 93d88ad: Narrow the API guide description to writing, editing, and checking Demistify markup. Preserve component instructions and references; presentation authoring remains in the consuming workflow skill.

## 1.0.1

### Patch Changes

- 9920969: Clarify that checking and Markdown embedding use the matching dependency-installed source checkout, while full offline bundling is the source API bundleStandaloneHtml. Do not imply a published standalone CLI, bundle command or npm installation. Keep the public v1 asset bindings and implemented component APIs unchanged.

## 1.0.0

### Major Changes

- de40bc3: Initialize the experimental v1 HTML-island library with the graph explorer, retained sidebar inspection, referenced Markdown and source viewing, navigation and diagram gesture fixes. Build the runtime, theme, diagram worker, lazy chunks and authoring guide together. Experimental v1 may receive breaking changes until the owner declares the vocabulary stable.
