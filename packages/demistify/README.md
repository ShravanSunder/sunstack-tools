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

The agent-facing guide is [`skill/SKILL.md`](skill/SKILL.md), in skill format: a short core (about 1.2k tokens) plus reference files in `skill/references/` that an agent reads only when a page needs them. [`src/vocabulary.ts`](src/vocabulary.ts) is the complete, checked list of elements and attributes.

| Element                                                                | Job                                                                                                                                                                        |
| ---------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ex-document`                                                          | Document scope; adds outline navigation, theme toggle and layer switches; bare Markdown between islands renders as Markdown                                                |
| `ex-markdown`, `ex-mermaid`                                            | Markdown and diagram islands (flowchart, sequence, state, class, ER, `xychart-beta` charts)                                                                                |
| `ex-card`, `ex-callout`, `ex-region`                                   | Titled surfaces sharing one head: `tone` sets status icon and colour, `icon` names what the thing is (32 curated Phosphor icons), `tag`, `pick`                            |
| `ex-stat`                                                              | One number with label and toned note                                                                                                                                       |
| `ex-tree`                                                              | A Markdown list as a tree: status reports, checklists, breakdowns; tone words, links that jump to islands, collapsing; `layout="map"` mind map                             |
| `ex-code`, `ex-note`                                                   | Line-numbered code or unified diffs (one or many files) with margin notes                                                                                                  |
| `ex-compare`                                                           | Before and after, side by side or overlaid with a `slider`                                                                                                                 |
| `ex-grid`, `ex-flow`                                                   | Width-aware side-by-side layout (`cols`, child `span`, `sticky`, `swipe`); `ex-flow` routes its arrows through the gaps                                                    |
| `ex-tabs`                                                              | Alternative views; `columns` shows them side by side when they fit                                                                                                         |
| `ex-board`, `ex-region`                                                | Whiteboard: Read by default, `mode="map"` for the pan/zoom camera, fit, drill-down revealing `detail` islands; without regions, a free canvas over the author's own layout |
| `ex-arrow`                                                             | A labelled connection; outside a flow it routes orthogonally around every box, in lanes, with labels kept clear                                                            |
| `ex-route`, `ex-step`                                                  | Guided routes with narration; steps on islands, regions, diagram nodes (step-through) or other pages (tours)                                                               |
| `ex-layers`                                                            | Switches for `layer="…"` groups readers show or hide                                                                                                                       |
| `ex-file-tree`                                                         | Plain-text file trees                                                                                                                                                      |
| `ex-zoom`, `ex-decision`, `ex-diagram-diff`, `ex-timeline`, `ex-layer` | Earlier vocabulary kept for the lab pages; not in the guide                                                                                                                |

Inspection follows the hierarchy of the page: containers never get controls; diagrams, code, trees, comparisons and wide content get sidebar and expand buttons; an island whose depth is hidden (`slot="details"`) gets a "More ›" link; prose, plain cards and stats get none; `zoom` forces buttons and `zoom="none"` removes them. Routes and diagram links open any island in the drawer. Inspectors move and restore the real DOM content and leave a placeholder behind. The sidebar view is a docked, resizable drawer that the page reflows beside.

Hovering or focusing a box lights the arrows drawn to and from it; a diagram node lights its edges and neighbours. A Mermaid node whose id matches an element on the page takes that element's tone and opens it.

Light DOM keeps ordinary CSS layout and inheritance available; the library does not rearrange islands. Problems are reported twice: statically by `demistify check` (see below) and at runtime in the authoring-issues panel, which also sees layout (unroutable or tiny arrows, failed diagrams).

## Navigation and icons

`<ex-nav></ex-nav>` reads the first `ex-document` or `main`, or the body as fallback. `for="content-id"` selects a scope. It enumerates actual `h1`–`h6`, including asynchronous Markdown; hidden tab/detail headings are excluded. Missing anchors receive text-derived IDs, while authored IDs remain authoritative. Set explicit IDs for durable cross-document links when headings repeat or change.

At viewport widths of 1440px and above it keeps a quiet left sidebar visible; smaller views use a compact rail and native popover. Hovering/focusing a marker previews that corresponding outline row. The current reading section has a separate border indicator. A list icon opens the outline for keyboard/touch users. Native anchors handle section jumps. Relative links authored as children appear under Other pages. `.ex-breadcrumbs` styles ordinary accessible breadcrumb markup. The breakpoint is an experimental design choice, not a measured universal threshold.

Phosphor regular is the icon family, subset at build time to the icons the library uses: tones, the 32 subject icons, inspection, navigation and a few lab icons (about 6.6 KB). Authors name tones and curated subjects, never raw icon names outside that list. Text fonts are separate WOFF2 files referenced from the theme, so only faces a page uses download, after first paint. Upstream Mermaid font imports are removed so the document uses local theme fonts.

## Design scales

Every spacing, type size, radius, shadow and motion value in the library CSS comes from tokens on `:root` in `styles/theme.css`, so pages stay consistent and the look can be tuned in one place without editing explainers:

- Spacing `--space-1` … `--space-18` (n × 4px: 4, 8, 12, 16, 20, 24, 32, 40, 48, 64, 72). Hairlines (1–3px) and layout widths stay literal.
- Type `--text-2xs` 12, `xs` 13, `sm` 14, `md` 15, `base` 17 (prose), `lg` 18, `xl` 21, `2xl` 24, `3xl` 30, `4xl` 44.
- Radius `--radius-sm` 4, `md` 8, `lg` 12, `pill`.
- Motion `--motion-fast` 120ms, `base` 200ms, `slow` 450ms, `--ease-out`; shadows `--shadow-float`, `--shadow-edge`, `--ring`.

Authors normally write no CSS. Hand-placed layouts may use the spacing tokens in inline styles (`style="gap: var(--space-6)"`).

## Hosting

Serve the whole `dist/` folder together: `runtime.js`, `diagram-worker.js`, `chunks/`, `theme.css`, `fonts/` and `Phosphor.woff2`. Diagrams render in a module worker (`diagram-worker.js`, resolved beside `runtime.js`) so laying out a diagram never freezes the page; the worker is started from a same-origin Blob that imports it, which works when the library comes from another origin. Where workers are unavailable (a strict `worker-src`, `file://` pages), diagrams render on the main thread with identical output. Cross-origin hosting needs `Access-Control-Allow-Origin: *` on these files (module scripts and fonts are CORS-checked).

## Checking pages

`src/vocabulary.ts` is the one description of the authoring vocabulary: every element, its attributes, allowed values, id references and nesting. Everything that checks or documents the vocabulary derives from it.

```sh
node packages/demistify/bin/demistify.ts check page.html          # static checks, about 0.1 s per page
node packages/demistify/bin/demistify.ts check --fix page.html    # also applies mechanical repairs
node packages/demistify/bin/demistify.ts check --render page.html # adds layout checks in headless Chromium (needs Playwright)
```

Output is one line per problem (`page.html:16:31 error attribute: <ex-card> has no attribute "type". Use tone. (fixable)`); a problem repeated across lines prints once with every line number. Static checks cover unknown elements and attributes (with the likely intended name), values (tones, columns, spans, sides), required attributes, duplicate and missing ids, nesting, arrows in a flow, island sources the HTML parser broke, diagrams the engine rejects or draws empty, styling beyond layout CSS, cards and regions without a tone, bare Markdown an HTML formatter re-wrapped (a heading joined to its paragraph, a list on one line), and the library links. Keep explainer pages out of HTML formatters such as Prettier or oxfmt.

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
