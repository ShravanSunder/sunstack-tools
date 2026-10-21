---
name: demistify-explainer
description: "Use when writing, editing, or checking Demistify markup: component syntax, attributes, nesting, layouts, Markdown/Mermaid islands, and data references."
---

# Demistify explainer

Write a small HTML document. You supply content and layout; the shared library supplies styling, icons, rendering, navigation and inspection. Content belongs to islands; a presentation orders stops through existing content. Use library styles rather than authoring colors, borders, fonts or shadows. Layout CSS (grid, flex, gaps) is fine.

Choose matching assets and file delivery, compose the content, add navigation/presentations when useful, then check the markup and inspect the result through its intended opening method.

## Small HTML, no local server

Use the supplied library base; the public default is `https://demistify.getagentstudio.dev/v1/`. Before writing page markup, MUST load `release.json` and `authoring/SKILL.md` relative to that base and return the base, recorded version/source commit, and confirmation that chosen markup is documented there and both imports use that base. Reading this exact matching guide already satisfies its guide read; do not recursively reload it. Experimental v1 may change vocabulary. A source preview must use its matching local build rather than mixing new markup with an older public runtime; when its release metadata is unavailable, name that gap and do not claim a published binding.

```html
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Session architecture</title>
<link rel="stylesheet" href="https://demistify.getagentstudio.dev/v1/theme.css">
<script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script>
</head>
<body>
<ex-document>
<h1>Session architecture</h1>
<ex-card title="Session lifetime" tone="decision">The registry owns the session.</ex-card>
</ex-document>
</body>
</html>
```

**The reader can double-click the HTML. No Python, Node, Vite or local web server is required.** The browser loads JavaScript, CSS, fonts, renderer chunks and worker from Cloudflare. Keep those assets external; do not inline their bytes or base64 images into each page. Internet access is required for these HTTPS imports. No-server viewing does not mean offline viewing.

The same files can optionally be hosted on **any ordinary static HTTP server**. No Demistify-specific routes, backend, framework or per-page application build are needed. A server is useful for live editing and sibling-file fetches, not a reader prerequisite.

| Content | Double-click delivery |
| --- | --- |
| Inline Markdown/HTML/Mermaid | Already in the document |
| Local HTML pages, fragments and images | Ordinary relative links/paths; keep their folder structure |
| Sibling `.md` fetched by `src` | Embed exact source once while authoring, as below |
| HTTPS Markdown/JSON | May stay live when that origin allows cross-origin reads |
| Sibling explorer JSON | Supply inline JSON or an allowed HTTPS URL |

Complete delivery preparation by inspecting the HTML imports, retained image/page paths and source mode. Follow the browser tool's URL/access policy; a blocked file URL is a named proof gap, not a reason to start another browser instance.

## Markdown and exact source

Write prose as bare Markdown inside `ex-document`, tab sections, cards and callouts. Bare Markdown must not contain `<`; preserve literal text with a script-backed island:

```html
<ex-markdown id="notes"><script type="text/markdown">
## Recovery

The literal `<registry>` remains Markdown source.
</script></ex-markdown>
```

An existing Markdown file stays a real source reference when served over HTTP:

```html
<ex-markdown id="requirements" label="Requirements" src="./requirements.md"></ex-markdown>
```

`src` resolves relative to the HTML; Markdown images and links resolve relative to that Markdown file. Full Markdown includes tables, fenced code, nested/task lists, links, images and Mermaid fences. **Show Markdown source** opens the exact text beside the rendered content. Failed reads show an error.

For double-click viewing, the matching source CLI embeds local Markdown while retaining the shared library links:

```sh
node packages/demistify/bin/demistify.ts embed input.html output.html
node packages/demistify/bin/demistify.ts check output.html
```

Run from a dependency-installed matching sunstack-tools checkout with Node 26. Keep the output beside the input or preserve its relative page/image paths. Rerun after `.md` edits. This is an author-time source transform; the reader runs no command. It safely embeds exact source, including text that would otherwise terminate an HTML script. It does not bundle the runtime, fonts or images. Choose live `src` or an embedded snapshot deliberately: only the live source picks up later file edits.

Check a representative table, literal code, task list, image and Mermaid diagram in the actual output; compare the source sidebar with the original. A plain code block where a Mermaid SVG was expected, a broken relative image or a duplicate task bullet is a failed result.

## Pages, breadcrumbs and headings

For a collection, repeat one native page-link list on every page. The first link is Home. The matching page URL provides its name; the library generates the compact sticky Pages trigger and breadcrumbs. Pages stays a dropdown of native links at the top. A separate search button beside the bottom theme control opens the command bar; no handwritten breadcrumb markup is needed.

```html
<ex-nav for="content">
  <nav slot="pages" aria-label="Pages">
    <a href="index.html">Session guide</a>
    <a href="details.html">Implementation details</a>
  </nav>
</ex-nav>
<ex-document id="content">
  <h1>Implementation details</h1>
  <!-- Content islands and section headings. -->
</ex-document>
```

Real h2–h6 sections supply the page-named outline; it scrolls independently when long. Captions stay out of the outline. Keep useful reader names in navigation and stable IDs on content. Follow a page link and a section link to confirm the generated chrome names the right page and leaves controls reachable.

Home receives the built-in house icon. A clicked lower heading remains current even when the page cannot scroll it to the reading line; deliberate page scrolling resumes reading-position tracking. Preserve explicit IDs instead of relying on coincidental heading slugs.

### Search and shortcuts

Cmd/Ctrl-K or the bottom search button opens one command bar for pages and headings. The button shows ⌘K on Mac and Ctrl-K elsewhere. P opens the top Pages dropdown; W opens the command bar’s Walkthroughs scope. These shortcuts leave inputs, editable content, open dialogs/popovers and modified browser commands alone. In the Pages dropdown or an empty Walkthroughs search, 1–9 immediately opens that entry; use arrows and ↵ for later entries. Escape closes and restores the trigger. Holding Option reveals P/W hints beside the controls; browser/OS ownership still applies outside document focus.

Heading search results show the page and ancestor headings above the matching title. Walkthrough rows show their source page as an eyebrow, the title, a concise authored description underneath, and a path or images icon. The current page’s live headings and authored Pages links work without an index. To include other pages’ headings, generate a small embedded collection index from explicit local inputs:

```sh
demistify index index.html specification.html verification.html
```

Regenerate after changing headings, paths or walkthrough descriptions; an older embedded index cannot supply newly authored cross-page explanations. The command reads local Markdown sources and prepares every page before writing outputs; it never fetches sibling HTML at runtime. Give each searchable Markdown island a stable `id`; wrap bare Markdown headings in that island before indexing. The generator preserves authored heading IDs and assigns stable native heading IDs. Existing documents without the generated data do not automatically gain cross-page heading search. Search uses titles and heading ancestry, not whole-page text or fuzzy matching. Same titles remain distinct by page and fragment.

## Walkthroughs reference content

Several named/described presentations may share a page and the same islands. Each is an ordered path with Next/Previous; one stop is valid. Place the content normally and put only stops in `ex-presentation`:

Include `<ex-nav></ex-nav>` on every page that offers a Walkthrough or Gallery Journey, including a single-page, single-path example. It creates the visible Walkthroughs launcher; `ex-presentation` defines the path but is itself hidden. A collection adds its Pages links inside that same `ex-nav`. Confirm a reader can start the path from the page before checking its stops.

```html
<ex-presentation id="session-review" title="Session review"
  description="Recovery, the Registry node and the decision.">
  <ex-step to="requirements--recovery">Read the required behavior.</ex-step>
  <ex-step to="architecture--Registry">The Registry owns lookup.</ex-step>
  <ex-step to="lifetime" open="sidebar">Inspect the shared decision.</ex-step>
</ex-presentation>
```

The command bar’s guided-path scope is called **Walkthroughs**. A Walkthrough may span pages and mix text, diagrams and gallery pictures. **Gallery Journey** describes a pictured sequence over the same player. Compact rows show a picture icon for local picture-only journeys and a path icon for mixed, ordinary or cross-page explanations; no separate type column. Keep the title short and the description to one concise sentence. The underlying authoring tags remain `ex-presentation` and `ex-step`.

The shared Walkthroughs scope discovers `id`, `title` and `description`. A presentation does not own, copy or hide other content. Use the current `ex-presentation`/`ex-step` form; the earlier `ex-route` and wrapper-selection form are removed.

Give every destination a stable ID. In Markdown island `requirements`, `## Recovery` becomes `#requirements--recovery` (repeated headings gain suffixes). A whole diagram uses `#architecture`; its Registry node uses `#architecture--Registry`. This scopes repeated node names to their diagram. Explicit HTML IDs remain valid. Fragments and stops reveal containing tabs/details/layers and await content readiness. A missing stop produces a visible error. Cross-page stops use `details.html#target`.

IF the page includes a guided presentation or cross-page stops, load `references/tours.md` and return named/described paths whose stops resolve to their intended scoped destinations.

## Islands and layout

| Element | Use |
| --- | --- |
| `<ex-card id title tone icon tag pick>` | One component, option, finding or decision; Markdown body; details for depth; separate Chosen badge with `pick` |
| `<ex-callout tone title>` | Summary, risk, decision or open question |
| `<ex-stat value label note tone>` | One number with its meaning and comparison |
| `<ex-mermaid id>` | Mermaid source in `<script type="text/plain">` |
| `<ex-code title lang start>` | Literal code/diff script, followed by `<ex-note line="42" tone="risk">…</ex-note>` |
| `<ex-tree title>` | Hierarchical Markdown, status lists and checklists; separate disclosure and go-to actions |
| `<ex-trace title>` | Ordered native disclosures with status and arbitrary rich evidence attachments |
| `<ex-compare label view choose selected>` | Aligned topic rows across arbitrary alternatives, or one detailed content tab at a time |
| `<ex-pre>` | Exact monospace/TUI drawing in a plain-text script |
| `<ex-gallery id label>` | Dedicated page: one direct child of `ex-document`, optionally after one h1; figures, filters and review/grid views |
| `<ex-grid cols="3">` | Related content side by side; child `span="2"` or `span="all"`; `sticky` side column; `swipe` row on phones |
| `<ex-tabs>` | Child `<section label icon>` content views; `columns` for side-by-side reading |

Tones: `note` `tldr` `decision` `risk` `question` `done` `planned` `changed` `added` `removed`. Tone supplies meaning; `pick` marks a chosen option without recoloring that meaning. `tag` adds timing/location/tradeoff. Review severity: blocking `risk`, nit `question`, nice `note`.

`icon` names the subject: database hard-drives cloud globe cpu stack plug code terminal git-branch package gear bug test-tube user users robot lock key shield-check file folder book-open clock envelope chat-circle bell shopping-cart credit-card truck chart-line rocket.

Mermaid supports flowchart, sequenceDiagram, stateDiagram-v2, classDiagram, erDiagram and xychart-beta. A diagram node whose ID matches a card ID takes its tone and opens that card; keep node labels short. Use qualified island/node IDs for links when diagrams repeat names.

Typography uses regular 18px Source Sans 3 prose, Source Sans 3 controls, Archivo headings, and IBM Plex Mono for code. Tree roots use 20px/600, branches 17px/500 and leaves 17px/400.

Two shared width standards sit in the space remaining beside the outline or inspector and share one left edge. **Reading** is 760px for prose, Markdown, lists, quotes, simple text cards and callouts; their text and headings stay left-aligned. **Canvas** fills the document up to 1600px for tables, images, galleries, code/diffs, diagrams, grids, comparisons, Explorer and rich trees. Ordinary prose/headings use Reading automatically; visual islands keep their own layout. Document headings, prose and dividers always share the Reading column, which starts at Canvas's left edge, so text, tables, code, diagrams and grids line up on one edge. A visual section can contain Reading text and Canvas islands; a lone `ex-stat` keeps the Reading measure. Apply `data-width="reading"` or `data-width="canvas"` to the whole component or native layout wrapper. A tab set includes its tab bar and panels in that width; the same convention applies to code, comparisons, galleries and other islands. The agent chooses the width. A Canvas wrapper leaves document headings and prose on Reading while its visual islands can expand. Component-internal titles and table cells retain their local layout. These are author choices, not reader settings. Nested grid/card/island content follows its component, and narrow viewports fit both standards to the available space. Tables use quiet alternating rows and one separator below the column header, with no rules between body rows. Section dividers always retain the Reading measure and its left edge, including above Canvas sections. Wide document tables keep readable columns and scroll sideways. Comparison topic headings render as small labels. Dedicated galleries retain their viewport surface. Remove obsolete per-page width overrides when adopting the standard.

Choose relationships before layout. Native flow, grid/flex and positioned boards are valid; do not force every explanation into a card grid. Keep related controls/content together using the shared spacing and styles. Sidebar inspection preserves the main content; fullscreen, Escape/backdrop dismissal and Cmd/Ctrl+wheel/drag use shared controls.

Use short labels for content tabs and keep the full heading/explanation inside each section. An optional curated `icon` names its subject; otherwise the library supplies a neutral file icon. Tabs use Arrow/Home/End keys and scroll locally when numerous. Exactly-one viewer modes use compact button groups; independent visibility uses switches; Sources expands a disclosure. Use the shared controls rather than radio inputs or per-page selected-button styles.

Choose the reader's task before choosing a control:

| Reader needs | Use |
| --- | --- |
| Another document | A Pages link to its HTML file |
| A peer alternative in the same place | Tabs |
| Optional detail beneath the current explanation | Native `details`/`summary` disclosure; a chevron expands it locally |
| A guided sequence through selected existing targets | A Walkthrough with Previous/Next stops; it may cross pages |
| A pictured UI flow with captions | A Gallery Journey over the same walkthrough player |

Progressive disclosure reveals optional depth in place; it does not require a guided path. Keep ordinary page and heading navigation available without starting a walkthrough. Use a rich Trace when the ordered evidence itself helps explain the subject.

A card groups one option, component or finding. A callout emphasizes a consequential proposal, decision, risk, open question or unverified outcome. Give it a short explicit label and a relevant supported subject icon; loose status emoji alone do not explain the claim. A recommendation remains a recommendation until the owner selects it, and a supplied source change remains separate from runtime proof. Keep ordinary explanatory prose in document flow.

Where Reading meets Canvas, wide documents add a larger gap between those blocks; consecutive prose or wide blocks keep their ordinary spacing. Narrow layouts retain their compact rhythm. Diagram labels keep a 16px rendered text floor and use the shared readable text colours; a diagram that needs more room scrolls locally rather than shrinking its labels.

## Load depth when needed

- IF the page includes screenshot review or comparison, load `references/pictures.md` and return native figures/captions and one shared-system gallery.
- IF the page includes a tree or literal TUI drawing, load `references/trees.md` and return independent disclosure/navigation or faithful `ex-pre` text.
- IF the page compares alternatives on corresponding topics, load `references/comparisons.md` and return authored layout/width and consistent topic keys; readers choose content rather than layout.
- IF the page uses ordered steps with rich evidence, load `references/traces.md` and return native disclosures with precise source/observation status.
- IF the page explains call-path changes, load `references/call-paths.md` and return a source-derived composition with calls, branches and unknown runtime evidence kept distinct.
- IF the page needs a pannable board, routed arrows or optional layers, load `references/maps.md` and return the chosen reading/map layout with valid endpoints.
- IF the page includes a multi-file diff or line notes, load `references/code.md` and return notes attached to the displayed file/row.
- IF the page uses a supplied explorer graph document, load `references/explorers.md` and return a page referencing that data without recreating it or its application code.

## Check and open

The checker ships in source, not as an installed standalone CLI. From the matching dependency-installed checkout:

```sh
node packages/demistify/bin/demistify.ts check /absolute/path/to/page.html
```

Fix every error/warning; `check --fix` repairs mechanical findings. Runtime authoring issues are for agents and authors, not readers: each one is a `[demistify]` console warning and raises the count in `data-ex-issues` on the root element. Open the page with `?ex-check` to see them listed on the page. Open the intended artifact and exercise its authored links, controls and loaded content. Check the exact source, external imports and image paths after embedding. Report HTML bytes separately from shared assets and images.

Completion means the requested content and interactions work through the intended delivery path. Name unavailable checker, mismatched runtime or blocked browser access explicitly. A checker pass is not rendered proof; HTTP preview is not direct-file proof; local success is not Cloudflare release proof.
