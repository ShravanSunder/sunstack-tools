# Existing graph documents

For a small ownership picture, prefer Board/Flow or Mermaid. Explorer earns its richer browser when supplied graph facts, evidence and related tables need coordinated selection. It is not a request to regenerate those facts or create another application shell.

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

`src` is a relative JSON file in the explorer graph-document format. Reference the supplied file unchanged; do not recreate its data, application JavaScript, CSS or renderer. `selected` is a real node ID, not its displayed label. `view` is `current` or `proposed`. Inline data may instead use a child `<script type="application/json">`.

The component provides grouped search, selection history, current/proposed relationships, hover tracing, graph sidebar/fullscreen controls, source evidence and related tables. Click an arrow to select its Connections row. Sources are quotations and links, not proof that the relationship executes at runtime. Proposal status and target kind are separate meanings.

Declared kinds keep the same category tint and edge in the graph and compact list: `phase` uses chart slot 1, `role` slot 2, `method` slot 3 and `principle` slot 4. Kind labels remain visible, and selection uses an outline without changing the category. This mapping uses existing `groups[].kind`; it adds no graph fields and infers nothing from node names.

Current/With proposals is an exactly-one mode group. Sources is a disclosure; evidence views are content tabs. When authoring the graph data, a detail table may supply a short `label` and curated `icon` alongside its full `title`, for example `"label":"Reuse","icon":"book-open"`. The panel retains the complete title. Existing data without these fields still works with its title and a neutral file icon; unknown icons produce a visible validation error. Do not rewrite supplied graph facts to shorten controls.

Search settles after200ms and remains above the independently scrolling node list. Dashed connections are proposed; the legend explains this separately from hover/group color. Hover emphasizes the line while the arrowhead keeps its size.

For a primary explorer page, plain `<body>` is enough: it supplies its own browser and title. Child HTML/islands become introductory content after that title, so put a reading guide there rather than placing an h2 before the page h1. Use `ex-document` for a prose-led page, with real h2 headings for useful outline navigation; avoid duplicating the explorer's title and browse UI. Set an explicit `id` for stable selection links; selection uses an `ex.<id>` URL query parameter and leaves heading anchors available. Keep layout in HTML. A summary spanning two columns plus two single-column stats needs four columns or a nested stats column, not a three-column row.

Run the normal page checker. It reads and validates the referenced JSON, real node IDs, table references and initial selection. Keep the supplied graph facts intact; routine authoring needs no browser check. The library is reusable; the complete referenced data still belongs in the content/token ledger.

At narrow component widths, Explorer starts with the node list. Select a node to open its existing details; Back to nodes preserves the filter, group expansion and list position, then restores focus to that node. An explicit valid selection URL opens details directly. Desktop retains both panes. Node-row carets indicate selection; group disclosure arrows still expand groups.
