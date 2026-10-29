# @sunstack/demistify

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
