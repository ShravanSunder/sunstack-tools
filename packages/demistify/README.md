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
	<ex-markdown zoom="sidebar" label="Proposal">
		<script type="text/markdown">
			# Session architecture

			Sessions survive transport disconnections.
		</script>
	</ex-markdown>
	<ex-mermaid zoom="modal" label="Session topology">
		<script type="text/plain">
			flowchart LR
			  Client --> Socket --> Registry --> Session
		</script>
	</ex-mermaid>
</ex-document>
```

Serve that HTML and its assets with any static host. The PoC is a consumer, not a runtime requirement. For an offline single HTML artifact, the Node packaging adapter can pre-render content and embed the theme, fonts and core; see the PoC's packaging example. Browser `file://` and embedded WebView operation have not been verified.

## Composition vocabulary

| Element                                   | Job                                                                         |
| ----------------------------------------- | --------------------------------------------------------------------------- |
| `ex-markdown`                             | Headings, prose, lists, tables and fenced code                              |
| `ex-mermaid`                              | Explicit diagram island; Mermaid fences in Markdown create this same island |
| `ex-zoom`                                 | Generic inspection wrapper around arbitrary HTML or nested islands          |
| `ex-arrow`                                | Connect `from` and `to` HTML IDs                                            |
| `ex-nav`                                  | Discover actual h1–h6 headings and provide document/page navigation         |
| `ex-tabs`                                 | Labeled child sections become tab panels                                    |
| `ex-compare`, `ex-diagram-diff`           | Composition containers; graph interpretation remains caller-owned           |
| `ex-decision`, `ex-callout`               | Reusable semantic emphasis and nesting                                      |
| `ex-timeline`, `ex-layer`, `ex-file-tree` | Implementation phases, architecture layers and file proposals               |
| `ex-document`                             | Optional document scope; it does not impose a mandatory layout              |

Light DOM keeps ordinary CSS layout and inheritance available. Use flow, flex/grid or absolute-positioned whiteboards. The library does not rearrange islands. Per-document layout declarations are usage; library code is not.

Use `zoom="modal"` or `zoom="sidebar"` on native islands. For an arbitrary composition use `<ex-zoom mode="sidebar" label="Details">…</ex-zoom>`. Optional direct children with `slot="details"` are hidden inline and revealed in inspection. Inspectors move and restore real DOM content, reserve the inline space, and offer icon controls for magnification/reset/close. There are no saved viewer settings or author-managed state objects.

Diagrams are centered and naturally sized. Code blocks, tables and the inspector body own their scrolling. A bounded diagram viewport is explicit author markup such as `style="max-height:420px;overflow:auto"`; the library does not silently crop diagrams.

`ex-arrow` supports `from-anchor`/`to-anchor` values left, right, top and bottom. Without them, it chooses facing edges. Redraw follows rendering, scroll and resize events and never moves content. It does not perform obstacle-avoiding graph routing.

## Navigation and icons

`<ex-nav></ex-nav>` reads the first `ex-document` or `main`, or the body as fallback. `for="content-id"` selects a scope. It enumerates actual `h1`–`h6`, including asynchronous Markdown; hidden tab/detail headings are excluded. Missing anchors receive text-derived IDs, while authored IDs remain authoritative. Set explicit IDs for durable cross-document links when headings repeat or change.

At viewport widths of 1440px and above it keeps a quiet left sidebar visible; smaller views use a compact rail and native popover. Hovering/focusing a marker previews that corresponding outline row. The current reading section has a separate border indicator. A list icon opens the outline for keyboard/touch users. Native anchors handle section jumps. Relative links authored as children appear under Other pages. `.ex-breadcrumbs` styles ordinary accessible breadcrumb markup. The breakpoint is an experimental design choice, not a measured universal threshold.

Phosphor is the standard icon family: the shared theme includes regular-weight `@phosphor-icons/web@2.1.2`. Write `<i class="ph ph-arrow-right" aria-hidden="true"></i>` anywhere. Put accessible labels on the parent button when an icon replaces text. Only regular weight is shipped; there is no icon JavaScript or CDN call. The shared font is 147,380 raw bytes and is reported as a library delivery cost. Upstream Mermaid font imports are removed so the document uses local theme fonts.

## Packaging boundary and limits

`@sunstack/demistify/packaging` exposes `prepareIslandContent(sourceHtml, diagramDictionary, standalone)`, `localFontStyles` and `localPhosphorStyles`. This experimental Node entry currently exports TypeScript source and targets the lab's Node 26 tooling; it is not a published npm release. Browser consumers use built JavaScript and CSS.

Mermaid `src` references are resolved by the packaging caller's explicit dictionary. Browser-time arbitrary repository fetching is not implemented. Markdown escapes raw HTML; the outer authored HTML remains trusted content. Mermaid rendering uses beautiful-mermaid's supported subset. D2 is excluded from this library. Semantic patches are implemented by the separate flat graph experiment, not by a generic Mermaid diff parser.

The detailed intent, experiments, evidence boundaries and next iteration questions are in [the monorepo handoff](../../docs/wip/demystify/2026-10-14-demistify-handoff.md).
