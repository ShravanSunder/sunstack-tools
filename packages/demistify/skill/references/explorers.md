# Existing graph documents

For a small ownership picture, prefer Board/Flow or Mermaid. Explorer earns its richer browser when supplied graph facts, evidence and related tables need coordinated selection and the data fits the version-1 graph document below. A graph whose kinds or fields fall outside that format keeps its facts in Board, Flow or Mermaid; never rename supplied values to force a fit. Explorer is not a request to regenerate those facts or create another application shell.

For direct-file delivery use supplied inline JSON or an allowed HTTPS source; embedding Markdown does not enable sibling JSON fetches. Set `selected` to a real supplied node ID and `view` explicitly when the initial relationship mode matters. An invalid graph or unknown selection is an error to correct, not an invitation to replace the source with a success-looking fallback.

Use an explorer when the reader needs searchable selection, a connected graph, incoming consumers and source-backed tables together. It is an island, so ordinary HTML can place other content around it.

```html
<ex-explorer id="architecture" src="architecture.json"
  selected="gateway" view="proposed">
  <ex-markdown><script type="text/markdown">
## Reading guide
Select a node, compare Current and Proposed, then inspect its evidence.
  </script></ex-markdown>
</ex-explorer>
```

`src` is a relative JSON file in the explorer graph-document format. Reference the supplied file unchanged; do not recreate its data, application JavaScript, CSS or renderer. `selected` is a real node ID, not its displayed label. `view` is `current` or `proposed`. Inline data may instead use a child `<script type="application/json">`: IF the graph is inline, load `content.md` and return its inline JSON carrier, which keeps every value exact and the carrier closed.

## The graph document, version 1

The library's parser fixes this format. Every field is required unless marked optional; text fields may be empty, and arrays may be empty except `nodes`:

- top level: `version` (`1`), `sourceBase` (an HTTP or HTTPS URL that source links resolve against), `title`, `description`, `note`, and the arrays `groups`, `nodes`, `edges` and `tables`;
- a group: `id`, `label` (the visible category name) and `kind`, one of `phase`, `role`, `method` or `principle`, which picks a category tint below and does not name the subject;
- a node: `id`, `label`, `description`, `group` (a group's `id`), `issue`, `status` (`current` or `proposed`) and `sources`;
- an edge: `id`, `source` and `target` (node IDs), `expansion` (`composition` or `related`), `status`, `label`, `why`, `when`, `use` and `sources`;
- a table: `id`, `title`, optional `label` and `icon`, `description`, `placement` (`overview` or `details`), `filter` (`all` or `selection`), `columns`, and `rows`; a row has `id`, `nodes` and one cell per column, and a cell has `text`, `nodes` and `sources`;
- a source: `label`, `href` (resolved against `sourceBase`; HTTP or HTTPS only) and `text`, the quoted passage.

IDs are non-empty and unique among groups, nodes, edges, tables and each table's rows. A node can't use the reserved ID `__issues`, and a details table can't use `connections` or `used-by`. Every referenced node and group exists, and every row has as many cells as its table has columns.

```html
<ex-explorer id="release" selected="build">
  <script type="application/json">{
    "version": 1, "sourceBase": "https://example.com/docs/",
    "title": "Release pipeline", "description": "How a change reaches production.", "note": "",
    "groups": [{ "id": "stages", "label": "Stages", "kind": "phase" }],
    "nodes": [
      { "id": "build", "label": "Build", "description": "Compiles and tests the change.", "group": "stages", "issue": "", "status": "current",
        "sources": [{ "label": "Pipeline guide", "href": "pipeline.html#build", "text": "Every change is built once." }] },
      { "id": "deploy", "label": "Deploy", "description": "Ships the tested artifact.", "group": "stages", "issue": "", "status": "current", "sources": [] }
    ],
    "edges": [{ "id": "build-deploy", "source": "build", "target": "deploy", "expansion": "composition", "status": "current",
      "label": "hands off", "why": "Deploy needs a tested artifact.", "when": "After the build passes.", "use": "Release", "sources": [] }],
    "tables": [{ "id": "owners", "title": "Stage owners", "label": "Owners", "icon": "users", "description": "Who answers for each stage.",
      "placement": "overview", "filter": "all", "columns": ["Stage", "Owner"],
      "rows": [{ "id": "build-owner", "nodes": ["build"], "cells": [
        { "text": "Build", "nodes": ["build"], "sources": [] }, { "text": "Platform team", "nodes": [], "sources": [] }] }] }]
  }</script>
</ex-explorer>
```

## What the reader gets

The component provides grouped search, selection history, current/proposed relationships, hover tracing, graph sidebar/fullscreen controls, source evidence and related tables. Click an arrow to select its Connections row. Sources are quotations and links, not proof that the relationship executes at runtime. Proposal status and target kind are separate meanings.

Declared kinds keep the same category tint and edge in the graph and compact list: `phase` uses chart slot 1, `role` slot 2, `method` slot 3 and `principle` slot 4. Kind labels remain visible, and selection uses an outline without changing the category. This mapping uses existing `groups[].kind`; it adds no graph fields and infers nothing from node names.

Current/With proposals is an exactly-one mode group. Sources is a disclosure; evidence views are content tabs. When authoring the graph data, a detail table may supply a short `label` and curated `icon` alongside its full `title`, for example `"label":"Reuse","icon":"book-open"`. The panel retains the complete title. Existing data without these fields still works with its title and a neutral file icon; unknown icons produce a visible validation error. Do not rewrite supplied graph facts to shorten controls.

Search settles after200ms and remains above the independently scrolling node list. Dashed connections are proposed; the legend explains this separately from hover/group color. Hover emphasizes the line while the arrowhead keeps its size.

For a primary explorer page, plain `<body>` is enough: it supplies its own browser and title. Child HTML/islands become introductory content after that title, so put a reading guide there rather than placing an h2 before the page h1. Use `ex-document` for a prose-led page, with real h2 headings for useful outline navigation; avoid duplicating the explorer's title and browse UI. Set an explicit `id` for stable selection links; selection uses an `ex.<id>` URL query parameter and leaves heading anchors available. Keep layout in HTML. IF the page places the explorer's neighbours in a grid, load `layouts.md` and return each row's column count and spans.

Run the normal page checker. It reads and validates the referenced JSON, real node IDs, table references and initial selection. Keep the supplied graph facts intact; routine authoring needs no browser check. The library is reusable; the complete referenced data still belongs in the content/token ledger.

At narrow component widths, Explorer starts with the node list. Select a node to open its existing details; Back to nodes preserves the filter, group expansion and list position, then restores focus to that node. An explicit valid selection URL opens details directly. Desktop retains both panes. Node-row carets indicate selection; group disclosure arrows still expand groups.
