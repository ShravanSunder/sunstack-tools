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

## Design scales

Every spacing, type size, radius, shadow and motion value in the library CSS comes from tokens on `:root` in `styles/theme.css`, so pages stay consistent and the look can be tuned in one place without editing explainers:

- Spacing `--space-1` … `--space-18` (n × 4px: 4, 8, 12, 16, 20, 24, 32, 40, 48, 64, 72). Hairlines (1–3px) and layout widths stay literal.
- Type `--text-2xs` 12, `xs` 13, `sm` 14, `md` 15, `base` 17 (prose), `lg` 18, `xl` 21, `2xl` 24, `3xl` 30, `4xl` 44.
- Radius `--radius-sm` 4, `md` 8, `lg` 12, `pill`.
- Motion `--motion-fast` 120ms, `base` 200ms, `slow` 450ms, `--ease-out`; shadows `--shadow-float`, `--shadow-edge`, `--ring`.

Authors normally write no CSS. Hand-placed layouts may use the spacing tokens in inline styles (`style="gap: var(--space-6)"`).

## Checking pages

`src/vocabulary.ts` is the one description of the authoring vocabulary: every element, its attributes, allowed values, id references and nesting. Everything that checks or documents the vocabulary derives from it.

```sh
node packages/demistify/bin/demistify.ts check page.html          # static checks, about 0.1 s per page
node packages/demistify/bin/demistify.ts check --fix page.html    # also applies mechanical repairs
node packages/demistify/bin/demistify.ts check --render page.html # adds layout checks in headless Chromium (needs Playwright)
```

Output is one line per problem (`page.html:16:31 error attribute: <ex-card> has no attribute "type". Use tone. (fixable)`); a problem repeated across lines prints once with every line number. Static checks cover unknown elements and attributes (with the likely intended name), values (tones, columns, spans, sides), required attributes, duplicate and missing ids, nesting, arrows in a flow, island sources the HTML parser broke, diagrams the engine rejects or draws empty, styling beyond layout CSS, cards and regions without a tone, and the library links.

For Claude Code, a PostToolUse hook runs the check whenever an agent writes a page and returns problems to the agent (exit 2); clean or non-explainer files stay silent:

```json
{
	"hooks": {
		"PostToolUse": [
			{
				"matcher": "Write|Edit",
				"hooks": [
					{ "type": "command", "command": "node packages/demistify/bin/demistify.ts check --hook" }
				]
			}
		]
	}
}
```

The build also writes `dist/custom-elements.json` (Custom Elements Manifest) and `dist/html-custom-data.json` (VS Code `html.customData`) for completions and hover docs.

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
