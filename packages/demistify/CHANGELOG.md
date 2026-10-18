# @sunstack/demistify

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
