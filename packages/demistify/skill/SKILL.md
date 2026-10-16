---
name: demistify-explainer
description: Use when explaining engineering work (architecture, a plan, a change, a review, a domain model, a status report) as a visual HTML page. Write content and HTML layout; the Demistify library renders Markdown, Mermaid, cards, trees, code notes, arrows, maps, guided routes, navigation, inspection and light/dark themes.
---

# Demistify explainer

Write one HTML file. You supply meaning, content and layout; the library supplies styling, icons, rendering, outline, theme toggle and route player. Never write colours, borders, fonts or shadows; layout CSS (grid, flex, gaps) is fine.

```html
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Session architecture</title>
<link rel="stylesheet" href="demistify/theme.css">
<script type="module" src="demistify/runtime.js"></script>
</head>
<body>
<ex-document>
…
</ex-document>
</body>
</html>
```

Use the library path you are given (default `demistify/`). A page with inline/embedded content and public HTTPS library links needs no app server or per-page framework. A CDN-bound file still needs network access; a fully offline file needs the standalone bundle. Follow your browser tool's supported URL/access policy rather than opening more browser instances to work around a denial.

## Markdown first

Write prose as bare Markdown directly inside `ex-document`, tab sections, cards, callouts and their `<details>`. Use section headings (`##` in Markdown or `<h2>` in HTML) to build useful outline navigation. Bare Markdown must not contain `<`; for that, use `<ex-markdown><script type="text/markdown">…</script></ex-markdown>`.

## Existing Markdown files and source

For a page served over HTTP, reference the real file instead of copying or rewriting its prose:

```html
<ex-markdown id="requirements" label="Requirements" src="./requirements.md"></ex-markdown>
```

The URL resolves relative to the HTML page. Markdown links/images resolve relative to the Markdown file. Mermaid fences in that file render diagrams through the same pipeline as inline Markdown. Every Markdown island has a small **Show Markdown source** control: it opens the exact source beside the rendered content. A failed read shows an error, not an empty island.

Browsers generally block `file://` pages fetching sibling files. For double-click/no-server viewing, embed the referenced file once with the supplied source CLI:

```sh
node packages/demistify/bin/demistify.ts embed input.html output.html
```

Run this from a dependency-installed matching sunstack-tools checkout with Node 26. This reads local Markdown files and embeds exact source safely; it retains the shared library links. Rerun after the source `.md` changes. No custom Python generator or application build is needed per page. Use the library's separate standalone bundle when runtime/fonts/data must also work offline. A live URL reference and an embedded snapshot have different update behavior; name which you produced.

Give an island a stable `id`. A Markdown heading `## Recovery` inside `id="requirements"` has fragment `#requirements--recovery`; repeated headings get suffixes. A link to a Mermaid component uses its island id, for example `#architecture-diagram`. Direct opening and clicking reveal the containing presentation/tab/detail before scrolling to the target. Plain headings outside named Markdown retain generated `heading-…` fragments; use explicit HTML ids for durable authored anchors.

## Presentation menu, pages and headings

A presentation is a complete named explainer with a short description. Several may share one HTML file: selecting one shows only that presentation. The **Presentations** button opens a side menu. Do not replace a presentation with an ordinary section heading.

```html
<ex-nav for="content">
  <nav slot="pages" aria-label="Pages">
    <a href="index.html">Overview</a>
    <a href="details.html">Implementation details</a>
  </nav>
  <nav slot="presentations" aria-label="Other presentations">
    <a href="review.html"><strong>Code review</strong><span>Findings and proposed repairs.</span></a>
  </nav>
</ex-nav>
<ex-document id="content">
  <nav class="ex-breadcrumbs" aria-label="Breadcrumb">
    <a href="index.html">Explainers</a><span aria-current="page">Session design</span>
  </nav>
  <ex-presentation id="architecture" title="Architecture" description="How sessions survive a connection loss.">
    <ex-markdown id="requirements" src="requirements.md"></ex-markdown>
    <ex-mermaid id="architecture-diagram"><script type="text/plain">flowchart LR
      Client --> Registry --> Session
    </script></ex-mermaid>
  </ex-presentation>
  <ex-presentation id="plan" title="Implementation plan" description="Delivery phases and their dependencies.">
    <ex-markdown><script type="text/markdown"># Implementation plan

1. Separate session ownership.
2. Add reconnection.
    </script></ex-markdown>
  </ex-presentation>
</ex-document>
```

The library discovers in-page presentations from their `id`, `title` and `description`. `slot="presentations"` adds named/described links to other files or fragments. `slot="pages"` supplies the current collection's page list; a presentation can also contain its own `nav slot="pages"`. Real visible h1–h6 populate the separate page outline automatically. Keep the complete navigation on every page in the collection. Preserve relative page URLs, stable island/heading fragments and breadcrumbs on revision.

## Islands

| Element | For |
|---|---|
| `<ex-card id title tone icon tag pick>` | one thing (component, entity, step, option, finding); body in Markdown; depth in `<details><summary>…</summary>…</details>`; `pick` marks the chosen option |
| `<ex-callout tone title>` | TL;DR, decision, risk, gotchas, open questions |
| `<ex-stat value label note tone>` | one number: `value="143 KiB" label="First load" note="was 443 KiB" tone="done"` |
| `<ex-mermaid>` | Mermaid in `<script type="text/plain">` |
| `<ex-code title lang start>` | code or a diff in `<script type="text/plain">`, then `<ex-note line="42" tone="risk">…</ex-note>` |
| `<ex-tree title>` | a Markdown list as a tree: status reports, checklists (`- [x]`), breakdowns; `- risk: text` gives a node a tone |

Tones (only these): `note` `tldr` `decision` `risk` `question` `done` `planned` `changed` `added` `removed`. Every card and region takes a tone: its icon and colour. Title = the specific thing; tone = its status or kind; `tag` = timing, location or tradeoff (`tone="planned" tag="after implementation"`, not a bold status sentence). Review severity: blocking `risk`, nit `question`, nice `note`.

`icon` = what the thing is: database hard-drives cloud globe cpu stack plug code terminal git-branch package gear bug test-tube user users robot lock key shield-check file folder book-open clock envelope chat-circle bell shopping-cart credit-card truck chart-line rocket.

Mermaid: `flowchart`, `sequenceDiagram`, `stateDiagram-v2`, `classDiagram`, `erDiagram`, `xychart-beta` (`x-axis ["A", "B"]`, `bar "Before" [1, 2]`). A node whose id equals a card `id` takes its tone and opens it, so keep node labels short.

## Layout follows the explanation

| Element | Does |
|---|---|
| `<ex-grid cols="3">` | side by side; child `span="2"` or `span="all"`; `sticky` keeps a side column in view; `swipe` makes a swipe row on phones |
| `<ex-tabs>` | child `<section label>` views; `columns` shows them side by side when they fit |

Ordinary HTML flow, grid/flex and absolute-positioned boards are all valid. Choose relationships first; do not force every explainer into the same card grid. Patterns when useful: a four-column summary row (`<ex-grid cols="4">` with a `tldr` callout `span="2"` + two stats); a diagram `span="2"` beside a `sticky` column of cards; options as cards in `<ex-grid swipe>` with `pick`; a chart beside the callout that states its conclusion. Phones get one column automatically.

## Routes

```html
<ex-route label="60-second tour">
<ex-step to="verdict">Start here.</ex-step>
<ex-step to="f1" open="sidebar">The blocking bug.</ex-step>
</ex-route>
```

Steps target any island, region or diagram node id. Add a route when a guided reading sequence helps. Picture/Mermaid/visual inspection supports Cmd+scroll to zoom and Cmd+drag to pan (Ctrl also works); ordinary scrolling stays native. Zoom/reset buttons remain available.

## When you need more

Read the matching file in `references/` beside this guide only when the page needs it:

- `references/maps.md`: many things at once (a domain, a system) as a pannable board, arrows between boxes, hand-placed layouts, layers readers switch on and off.
- `references/code.md`: diffs across several files, line rules for notes.
- `references/trees.md`: trees that link to islands, mind maps.
- `references/pictures.md`: images, galleries, before and after.
- `references/tours.md`: tours across pages, walking a diagram step by step.

IF the page uses a supplied explorer graph document, load `references/explorers.md` and return a page that references it without recreating its data or application code.

## Check

Run the checker you are given (`demistify check page.html`) and fix every line until it reports 0 errors and 0 warnings; `--fix` repairs the mechanical ones.
