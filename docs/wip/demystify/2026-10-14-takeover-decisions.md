# Demistify takeover — owner decisions and baseline evidence

Date: 2026-10-14. Branch: `claude/serene-pascal-llse28`, fast-forwarded from `feat/demistify-html-islands` at `a31c1da`. Toolchain: Node 26.12.0 (repo `engines` ≥26 unchanged), pnpm 12.11.2. This file records owner decisions from the takeover discussion with Shravan and the evidence that shaped them. It is the exchange point for other agents working on this branch; add a dated section rather than editing earlier ones.

## Goal (owner, confirmed)

Agents make new explainers cheaply and correctly. The pages load fast as plain web HTML with light and dark themes. Acceptance test: **a small model (Haiku-class subagent), given only the authoring guide and a topic, produces a correct explainer.** Token efficiency of authoring and of the guide itself both count.

## Decisions (owner, confirmed)

| ID  | Decision                                                                                                                                                                                                                                                                             |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| D1  | Delivery: an explainer is a small HTML file that loads one shared, cacheable library (folder or CDN URL) by default. An optional bundle step produces a single offline file.                                                                                                         |
| D2  | Look: keep the current typefaces and the Phosphor regular icon style. Ship only the font weights and icon weight the theme actually uses. Fonts must not block first paint.                                                                                                          |
| D3  | Lean details (weights, subsets, loading strategy) are the implementer's call, provided the visible look is preserved.                                                                                                                                                                |
| D4  | Less choice, more system: no giant lists for agents to pick from. Agents name a meaning; the library chooses icon, color and treatment. The authoring guide must not enumerate a large catalogue (e.g. 1,530 icons).                                                                 |
| D5  | Icons: Phosphor regular only, subset to a small semantic set (risk, decision, added, removed, step, file, …) that the library maps from meanings. Constraint without pedantry.                                                                                                       |
| D6  | Arrows: no crossings, guaranteed by layout. A flow container places connected islands in rows/columns and routes connectors through the gaps (A→C goes around B). Free-form whiteboard arrows stay simple; the library flags an arrow that crosses a box. No general routing engine. |

## Baseline evidence (observed this session)

Cold load over local HTTP in Chromium, simulated 10 Mbps / 40 ms:

| Page                                  | Transferred | Main costs                                                                                                           |
| ------------------------------------- | ----------- | -------------------------------------------------------------------------------------------------------------------- |
| `html-islands-live.html` (no diagram) | ~443 KiB    | `theme.css` 168 KiB gzip, of which ~200 KB raw is 8 base64 woff2 fonts (render-blocking); full Phosphor font 144 KiB |
| `html-islands-composition-live.html`  | ~949 KiB    | plus the lazy Mermaid chunk, 500 KiB gzip / 2.2 MB raw (beautiful-mermaid + elkjs)                                   |

Authoring: the existing report counts `planning.html` at 4,909 o200k tokens (Markdown-directive prototype: 2,391). Much of it is wrapper markup and formatter indentation inside script blocks.

Haiku baseline: given only `packages/demistify/README.md`, Haiku wrote a 4.6 KB merge-vs-rebase explainer. It rendered with no console errors, both Mermaid diagrams drew, and the Markdown dedented correctly. Failures that a reader would see:

- It hand-wrote ~25 lines of layout CSS because `ex-compare` has no documented child contract. Its borders ignore the theme.
- An `ex-arrow` between two `ex-zoom` boxes is present in the DOM but invisible: the boxes stretch, leaving a ~32 px gap.
- `ex-zoom` has no visible surface or inspect control in that layout, and `ex-decision` ignored the guessed `label`.
- No theme toggle, because the README does not show it.
- It guessed attribute names for `ex-arrow`, `ex-decision`, `ex-callout`, `ex-tabs` labels, and the timeline, layer and file-tree elements.

Reading: the runtime mostly works; the gaps are an undocumented or unforgiving vocabulary and silent failure modes.

## Open

- Haiku pass criteria (proposed, not yet confirmed by the owner).
