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

For a collection, repeat one native page-link list on every page. The first link is Home. The matching page URL provides its name; the library generates the compact sticky Pages dropdown and breadcrumbs. No separate manifest or handwritten breadcrumb markup is needed.

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

## Walkthroughs reference content

Several named/described presentations may share a page and the same islands. Each is an ordered path with Next/Previous; one stop is valid. Place the content normally and put only stops in `ex-presentation`:

```html
<ex-presentation id="session-review" title="Session review"
  description="Recovery, the Registry node and the decision.">
  <ex-step to="requirements--recovery">Read the required behavior.</ex-step>
  <ex-step to="architecture--Registry">The Registry owns lookup.</ex-step>
  <ex-step to="lifetime" open="sidebar">Inspect the shared decision.</ex-step>
</ex-presentation>
```

The menu is called **Walkthroughs**. Compact rows show a picture icon for local picture/gallery stops and a path icon for ordinary or cross-page explanations; no separate type column. Keep the title short and the description to one concise sentence. The underlying authoring tags remain `ex-presentation` and `ex-step`.

The shared Walkthroughs menu discovers `id`, `title` and `description`. A presentation does not own, copy or hide other content. Use the current `ex-presentation`/`ex-step` form; the earlier `ex-route` and wrapper-selection form are removed.

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
| `<ex-pre>` | Exact monospace/TUI drawing in a plain-text script |
| `<ex-gallery id label>` | Native figures, metadata filters and review/grid views within the shared document system |
| `<ex-grid cols="3">` | Related content side by side; child `span="2"` or `span="all"`; `sticky` side column; `swipe` row on phones |
| `<ex-tabs>` | Child `<section label icon>` content views; `columns` for side-by-side reading |

Tones: `note` `tldr` `decision` `risk` `question` `done` `planned` `changed` `added` `removed`. Tone supplies meaning; `pick` marks a chosen option without recoloring that meaning. `tag` adds timing/location/tradeoff. Review severity: blocking `risk`, nit `question`, nice `note`.

`icon` names the subject: database hard-drives cloud globe cpu stack plug code terminal git-branch package gear bug test-tube user users robot lock key shield-check file folder book-open clock envelope chat-circle bell shopping-cart credit-card truck chart-line rocket.

Mermaid supports flowchart, sequenceDiagram, stateDiagram-v2, classDiagram, erDiagram and xychart-beta. A diagram node whose ID matches a card ID takes its tone and opens that card; keep node labels short. Use qualified island/node IDs for links when diagrams repeat names.

A normal document uses a centered 1140px reading canvas. Put `class="wide-reading"` on `ex-document` for a centered 1600px visual canvas. Components and structural list rows use their container width; ordinary paragraphs keep their reading measure. Put `class="wide-reading"` on a short proof paragraph or a section whose direct paragraphs should use its width. This is an author choice: do not expose a width setting to the reader or apply paragraph caps to every descendant list item. A nested code block, Tree, comparison or diagram must be allowed its own width. Existing authored max-width rules still win where their specificity is higher; remove obsolete local clamps when adopting the shared layout.

Choose relationships before layout. Native flow, grid/flex and positioned boards are valid; do not force every explanation into a card grid. Keep related controls/content together using the shared spacing and styles. Sidebar inspection preserves the main content; fullscreen, Escape/backdrop dismissal and Cmd/Ctrl+wheel/drag use shared controls.

Use short labels for content tabs and keep the full heading/explanation inside each section. An optional curated `icon` names its subject; otherwise the library supplies a neutral file icon. Tabs use Arrow/Home/End keys and scroll locally when numerous. Exactly-one viewer modes use radio segments; independent visibility uses switches; Sources expands a disclosure. Do not recreate these as rows of large selected buttons.

## Load depth when needed

- IF the page includes screenshot review or comparison, load `references/pictures.md` and return native figures/captions and one shared-system gallery.
- IF the page includes a tree or literal TUI drawing, load `references/trees.md` and return independent disclosure/navigation or faithful `ex-pre` text.
- IF the page needs a pannable board, routed arrows or optional layers, load `references/maps.md` and return the chosen reading/map layout with valid endpoints.
- IF the page includes a multi-file diff or line notes, load `references/code.md` and return notes attached to the displayed file/row.
- IF the page uses a supplied explorer graph document, load `references/explorers.md` and return a page referencing that data without recreating it or its application code.

## Check and open

The checker ships in source, not as an installed standalone CLI. From the matching dependency-installed checkout:

```sh
node packages/demistify/bin/demistify.ts check /absolute/path/to/page.html
```

Fix every error/warning; `check --fix` repairs mechanical findings. Open the intended artifact and exercise its authored links, controls and loaded content. Check the exact source, external imports and image paths after embedding. Report HTML bytes separately from shared assets and images.

Completion means the requested content and interactions work through the intended delivery path. Name unavailable checker, mismatched runtime or blocked browser access explicitly. A checker pass is not rendered proof; HTTP preview is not direct-file proof; local success is not Cloudflare release proof.
