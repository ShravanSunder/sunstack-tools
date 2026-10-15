# Brief: how a Demistify explainer page loads and renders

Audience: an engineer deciding whether to adopt Demistify for agent-written explainers, who cares about load speed and how the pieces fit. Facts measured on 2026-10-14 in Chromium over local HTTP with a simulated 10 Mbps / 40 ms link.

Opening summary: An explainer is a small HTML file. It loads one shared stylesheet and one small runtime; everything heavy loads later, only when the page uses it. Text paints before fonts arrive.

## Parts

- `html` — **Explainer HTML** — authored by the agent — about 5–25 KB. Holds content and layout only.
- `theme` — **theme.css** — shared, render-blocking — 27 KB raw, 6 KB gzip. Colours, typography, components, light and dark themes. Previously 302 KB because eight fonts were embedded in it as base64.
- `runtime` — **runtime.js** — shared, module script — 24 KB raw, 7 KB gzip. Registers the elements, adds outline navigation and the theme toggle, reports authoring issues.
- `fonts` — **Text fonts** — six separate WOFF2 files, 15–22 KB each. Only the faces a page actually uses download, after first paint (font-display: swap).
- `icons` — **Icon font** — Phosphor regular, subset to 24 icons — 2 KB. Previously 147 KB for 1,530 icons.
- `markdown` — **Markdown chunk** — lazy — 57 KB raw, 15 KB gzip. Loads when the page has a Markdown island.
- `mermaid` — **Diagram engine chunk** — lazy — 2.2 MB raw, 500 KB gzip (beautiful-mermaid with the elkjs layout engine). Loads only when a diagram scrolls into view.
- `inspector` — **Inspector** — part of the runtime. Opens any island in a modal or sidebar using the native dialog element, moving the real content and restoring it on close.
- `nav` — **Outline navigation** — part of the runtime. Built from the page's real headings, including headings rendered from Markdown.
- `issues` — **Authoring issues panel** — part of the runtime. Lists unknown elements or tones, broken or too-short arrows, duplicate ids and failed diagrams.

## Flow 1: first paint

1. Browser parses the explainer HTML.
2. It fetches theme.css (blocking) and runtime.js (module, deferred) in parallel.
3. First paint happens with fallback fonts once theme.css arrives.
4. Text fonts download and swap in.
5. The runtime registers elements and renders islands.

Relationships: Explainer HTML → theme.css; Explainer HTML → runtime.js; theme.css → Text fonts (after paint); theme.css → Icon font; runtime.js → Markdown chunk (lazy); runtime.js → Diagram engine chunk (lazy, on visibility); runtime.js → Inspector; runtime.js → Outline navigation; runtime.js → Authoring issues panel.

## Flow 2: rendering one diagram island

1. The diagram island waits until it is near the viewport.
2. It imports the diagram engine chunk once, shared by every diagram on the page.
3. It renders an SVG themed with the page's colour tokens.
4. Nodes whose id matches a card on the page take that card's tone colour and open the card when clicked.
5. The island becomes inspectable (expand button).

## Measurements: before and after this iteration

| Page                                        | Before  | After   |
| ------------------------------------------- | ------- | ------- |
| Main lab page without diagrams, transferred | 443 KiB | 143 KiB |
| Main lab page, first contentful paint       | 324 ms  | 152 ms  |
| Page with a diagram, transferred            | 949 KiB | 650 KiB |
| Single-file offline bundle                  | 570 KiB | 241 KiB |

## Options considered for arrows between islands

- **No crossings by layout** (chosen): a flow container keeps connected islands in rows or columns and routes arrows through the gaps. Light and predictable. Free-form whiteboard arrows stay simple and the issues panel flags crossings.
- **Routed anywhere**: any arrow finds a path around boxes in any layout. Most flexible, but needs a lazy routing engine of a few hundred KB and gives less predictable results.
- **Diagrams only**: connected structure belongs in Mermaid; arrows between islands may cross boxes. Lightest.

## Risks and open questions

- The diagram engine is still 500 KB gzip; a page with one small diagram pays all of it. Open question: can a lighter layout path serve simple flowcharts?
- Browser file:// opening and embedded WebViews are not verified.
- The single-file bundle embeds every font face, including ones the page never uses.
