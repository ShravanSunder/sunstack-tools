# Demistify

Demistify is a lightweight visual-document vocabulary implemented with native Web Components. Agents write content and ordinary HTML layout. The library owns Markdown/Mermaid rendering, consistent styles, icons, navigation and inspection. A document does not need a React app, backend, application router or per-document build setup.

Build the library once from the monorepo root:

```sh
pnpm --filter @sunstack/demistify build
```

Copy the complete `packages/demistify/dist/` directory to a local/static asset location. Keep the lazy `chunks/` directory and `Phosphor.woff2` beside the runtime/theme. Then a plain HTML document can load it:

```html
<link rel="stylesheet" href="./demistify/theme.css" />
<script type="module" src="./demistify/runtime.js"></script>

<ex-nav><a href="./implementation.html">Implementation</a></ex-nav>
<nav class="ex-breadcrumbs" aria-label="Breadcrumb">
	<a href="./index.html">Explainers</a>
	<span aria-current="page">Session architecture</span>
</nav>
<ex-document>
	<ex-markdown label="Proposal">
		<script type="text/markdown">
			# Session architecture

			Sessions survive transport disconnections.
		</script>
	</ex-markdown>
	<ex-mermaid label="Session topology">
		<script type="text/plain">
			flowchart LR
			  Client --> Socket --> Registry --> Session
		</script>
	</ex-mermaid>
</ex-document>
```

Serve that HTML and its assets with any static host. The PoC is a consumer, not a runtime requirement. For an offline single HTML artifact, the Node packaging adapter can pre-render content and embed the theme, fonts and core; see the PoC's packaging example. Browser `file://` and embedded WebView operation have not been verified.

## Composition vocabulary

The agent-facing guide is [`skill/SKILL.md`](skill/SKILL.md), written in skill format and kept under ~1.5k tokens. It is the only document an authoring model needs.

| Element                                                                              | Job                                                                                                                                      |
| ------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `ex-document`                                                                        | Document scope; adds outline navigation and theme toggle; bare Markdown text between islands renders as Markdown                         |
| `ex-markdown`, `ex-mermaid`                                                          | Markdown and diagram islands (flowchart, sequence, state, class, ER, `xychart-beta` charts)                                              |
| `ex-card`, `ex-callout`                                                              | Titled surfaces whose `tone` chooses icon and colour; bodies may be bare Markdown; `pick` marks a chosen option                          |
| `ex-stat`                                                                            | One number with label and toned note                                                                                                     |
| `ex-code`, `ex-note`                                                                 | Line-numbered code or unified diffs with margin notes                                                                                    |
| `ex-grid`, `ex-flow`                                                                 | Width-aware side-by-side layout (`cols`, child `span`, `sticky`, `swipe`); `ex-flow` routes its `ex-arrow` children through the gaps     |
| `ex-tabs`                                                                            | Alternative views; `columns` shows them side by side when they fit                                                                       |
| `ex-board`, `ex-region`                                                              | Whiteboard map: pan/zoom camera, overview fit, drill-down revealing `detail` islands, Map/Read switch (Read on narrow screens)           |
| `ex-route`, `ex-step`                                                                | Guided routes: a player walks named paths through islands, regions or diagram nodes, with narration and optional sidebar/centred opening |
| `ex-arrow`, `ex-file-tree`                                                           | Relationships between islands, file proposals                                                                                            |
| `ex-zoom`, `ex-decision`, `ex-compare`, `ex-diagram-diff`, `ex-timeline`, `ex-layer` | Earlier vocabulary kept for the lab pages; not in the guide                                                                              |

Every content island offers sidebar and expand controls; `zoom="none"` removes them. A Mermaid node whose id matches an element on the page takes that element's tone and opens it. An authoring-issues panel reports unknown elements or tones, broken, invisible or box-crossing arrows, misplaced notes, duplicate ids and failed diagrams.

Light DOM keeps ordinary CSS layout and inheritance available. Use flow, flex/grid or absolute-positioned whiteboards. The library does not rearrange islands. Per-document layout declarations are usage; library code is not.

All content islands automatically show two small circular Phosphor controls on hover or keyboard focus: a right sidebar and a large centred view; only the innermost hovered island shows them, touch screens always do, and `zoom="none"` removes them. Document scopes, layout containers, navigation and arrows are not content inspectors. For an arbitrary composition use `<ex-zoom label="Details">…</ex-zoom>`. Native `<details>` inside an island are expanded in both views, and optional direct children with `slot="details"` are hidden inline and revealed in inspection. Inspectors move and restore real DOM content, reserve the inline space, and offer icon controls for magnification/reset/close. There are no saved viewer settings or author-managed state objects.

Diagrams are centered and naturally sized. Code blocks, tables and the inspector body own their scrolling. A bounded diagram viewport is explicit author markup such as `style="max-height:420px;overflow:auto"`; the library does not silently crop diagrams.

`ex-arrow` supports `from-anchor`/`to-anchor` values left, right, top and bottom. Without them, it chooses facing edges. Redraw follows rendering, scroll and resize events and never moves content. It does not perform obstacle-avoiding graph routing.

## Navigation and icons

`<ex-nav></ex-nav>` reads the first `ex-document` or `main`, or the body as fallback. `for="content-id"` selects a scope. It enumerates actual `h1`–`h6`, including asynchronous Markdown; hidden tab/detail headings are excluded. Missing anchors receive text-derived IDs, while authored IDs remain authoritative. Set explicit IDs for durable cross-document links when headings repeat or change.

At viewport widths of 1440px and above it keeps a quiet left sidebar visible; smaller views use a compact rail and native popover. Hovering/focusing a marker previews that corresponding outline row. The current reading section has a separate border indicator. A list icon opens the outline for keyboard/touch users. Native anchors handle section jumps. Relative links authored as children appear under Other pages. `.ex-breadcrumbs` styles ordinary accessible breadcrumb markup. The breakpoint is an experimental design choice, not a measured universal threshold.

Phosphor regular is the icon family, subset at build time to the icons the library uses (tones, inspection, navigation and a few lab icons; about 2 KB). Authors name tones rather than icons. Text fonts are separate WOFF2 files referenced from the theme, so only faces a page uses download, after first paint. Upstream Mermaid font imports are removed so the document uses local theme fonts.

## Source layout

```text
src/
  runtime.ts     Entry: registers every element, then runs page checks and document behaviour
  core/          Shared services: authoring issues, tones, Phosphor icon buttons, source/HTML text helpers
  islands/       One file per content element: markdown, mermaid, card (+ callout), code (+ note),
                 tabs, file-tree, inspector (+ ex-zoom), legacy (earlier lab vocabulary)
  layout/        document (chrome, bare Markdown, ex-grid), flow, arrows, navigation
  render/        Markdown and diagram rendering adapters, chart-source fixes, SVG font cleanup
  node/          packaging.ts: Node-only build/packaging adapter (fonts, icons, pre-rendering)
  styles/        theme, reference, components, navigation and vocabulary CSS, concatenated by the build
```

## Packaging boundary and limits

`@sunstack/demistify/packaging` exposes `prepareIslandContent(sourceHtml, diagramDictionary, standalone)`, `localFontStyles` and `localPhosphorStyles`. This experimental Node entry currently exports TypeScript source and targets the lab's Node 26 tooling; it is not a published npm release. Browser consumers use built JavaScript and CSS.

Mermaid `src` references are resolved by the packaging caller's explicit dictionary. Browser-time arbitrary repository fetching is not implemented. Markdown escapes raw HTML; the outer authored HTML remains trusted content. Mermaid rendering uses beautiful-mermaid's supported subset. D2 is excluded from this library. Semantic patches are implemented by the separate flat graph experiment, not by a generic Mermaid diff parser.

The detailed intent, experiments, evidence boundaries and next iteration questions are in [the monorepo handoff](../../docs/wip/demystify/2026-10-14-demistify-handoff.md).
