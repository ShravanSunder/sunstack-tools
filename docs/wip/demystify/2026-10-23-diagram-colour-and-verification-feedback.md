# Demistify: diagram colour, sizing and verification feedback

Updated 2026-10-23. Consumer feedback from building a real explainer on the public v1 binding
(1.12.0, `c49d2f2`). Every finding below was reproduced against that commit; file references are to
`packages/demistify` at `c49d2f2` unless stated.

Consumer page (relay-coordination-platform, uncommitted worktree):
`/Users/shravan.sunder/.cursor/worktrees/dsl-explainer-7c3e91a4/relay-coordination-platform.waypoint-workflows-dc226a789850/docs/specs/2026-10-08-agent-workflow-dsl/agent-workflow-dsl-explainer.html`
and its `.standalone.html` sibling. It uses five Mermaid islands (flowchart TD and LR, sequence),
three of them coloured with raw `classDef`.

## Agent brief

> Read `docs/wip/demystify/2026-10-14-demistify-handoff.md`, then this file. Work in an isolated
> worktree. Implement proposals P1 to P4 in order; P5 to P7 only if time allows. Keep reusable
> rendering, styles and checks in `packages/demistify` and examples in `apps/demistify-poc`.
> For each proposal: reproduce the finding with the fixture in "Reproduction fixtures", fix it,
> update the authoring guide and vocabulary, and prove it in a real browser in light and dark
> themes at 1500 px and 390 px. Report implementation, exercised behaviour and remaining limits
> separately, as in `2026-10-16-fixes-proof.md`. No release or deployment is implied.

## What already works

| Fact                                                                                                                                                     | Evidence                                                                                                        |
| -------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Diagrams inherit the page theme through beautiful-mermaid's two-colour model plus enrichment colours (`bg`, `fg`, `muted`, `line`, `surface`, `border`)  | `src/render/diagram.ts:8-26`                                                                                    |
| A `classDef` value is passed straight to the SVG attribute, including CSS variables: `classDef code fill:var(--chart-1)` renders `fill="var(--chart-1)"` | beautiful-mermaid 1.1.3 `resolveNodeStyle`; reproduced in Node                                                  |
| Three theme-aware categorical colours exist, with light and dark values: `--chart-1` blue, `--chart-2` amber, `--chart-3` teal                           | `src/styles/theme.css:17-19,105-107,149-151`; used by `ex-breakdown` in `src/styles/small-parts.css:165-171`    |
| beautiful-mermaid already gives xychart series a theme-driven palette through CSS custom properties (`--xychart-color-N`)                                | beautiful-mermaid README, XY chart styling                                                                      |
| Each Mermaid island dispatches a bubbling `island-ready` event after it renders                                                                          | `src/islands/mermaid.ts:125`                                                                                    |
| The standalone bundle renders all diagrams fully offline: with every hostname blocked, all five islands drew SVG with correct fonts                      | `removeSvgFontImports` in `src/render/svg-fonts.ts`, applied in `src/render/diagram.ts:11`; observed 2026-10-23 |

## Findings

### F1. No supported way to colour diagram nodes by meaning

The guide says "library styling only". The one documented route is node-ID matching: a node whose ID
matches a page element takes that element's tone and becomes a link to it (`src/islands/mermaid.ts:58-79`).
That couples colour to clickability, needs a page element per node, and borrows tones (`done`,
`risk`, `decision`) whose meanings differ from a categorical key such as code step, agent step, block.
The consumer page therefore used raw Mermaid `classDef`, which the guide never mentions.

### F2. `classDef` fails silently in three ways

| Input                                                        | Result                                             |
| ------------------------------------------------------------ | -------------------------------------------------- |
| `classDef ex-slot-1 fill:#ff000033` with `class a ex-slot-1` | Ignored. Class names must match `\w+`; no warning. |
| `classDef code fill:rgba(255,0,0,0.2)`                       | Dropped. Properties are split on commas.           |
| `fill:color-mix(in srgb, var(--chart-1) 20%, transparent)`   | Impossible for the same reason.                    |

So an author cannot express a translucent, theme-aware fill. The consumer page used fixed
8-digit hex fills (`#3b82f633`) so text keeps the theme colour. Those fixed colours do not follow
the theme, and the page has been observed only in dark mode; light mode is unverified.

### F3. Diagram width is unpredictable from the source

`sizeDiagram` (`src/islands/mermaid.ts:13-30`) scales the whole SVG so its smallest text reaches
16 px: `readableWidth = width * 16 / smallestFontSize`. Edge labels are 11 px and node labels 13 px,
so any diagram with an edge label is scaled by 16/11, about 1.45×. A flowchart whose intrinsic width
is 844 px became 1228 px and scrolled inside a 1165 px Canvas. Nothing in the source, checker or
guide reveals this; the consumer found it by rendering in Node and multiplying.

### F4. Layout surprises with no guidance

- A cycle (`inngest --> worker` and `worker --> inngest`) inverted the ranking of a top-down
  map: the hub moved to the bottom and edges ran upward.
- Seven branches into one shared `end` node sprawled long edges across the diagram. One terminal
  node per branch fixed it.
- A two-way edge (`a <-->|label| b`) into a hexagon draws its upward arrowhead under the
  hexagon's border, so one direction is nearly invisible.

### F5. Verification needs a hand-built harness

Islands render only when within 300 px of the viewport (`src/islands/mermaid.ts:97-107`). A
headless `--screenshot` of a page, or of a fragment URL, shows blank diagrams below the fold. The
per-island `island-ready` event exists but is not in the guide, and there is no document-level
"all rendered" signal or way to force off-screen rendering. The consumer wrote a DevTools-protocol
script that scrolls in steps, waits, opens disclosures, clicks tabs and captures element boxes, and
misdiagnosed one blank island as a syntax problem before finding it was timing.

### F6. The checker rejects its own standalone output

`demistify check` on a `bundleStandaloneHtml` result reports 1 error and 121 warnings: the
`library` rule (`src/node/check.ts:766`) wants linked theme and runtime, and the `styling` rule
(`src/node/check.ts:378-387`) flags the inlined library theme. The bundle itself renders correctly.

### F7. No Gantt or timeline family

beautiful-mermaid 1.1.3 rejects `gantt` with `Invalid mermaid header: "gantt"`. Durations (a
one-hour quiet period beside seconds-long steps) had to be shown as a sequence diagram plus prose.

## Proposals, highest value first

### P1. Diagram colour slots

Extend the existing `--chart-N` tokens to six or eight slots, each with a precomputed soft fill
(for example `--chart-1-soft`) defined per theme, so no `color-mix` or commas are needed in
Mermaid source. Have `renderThemedDiagram` prepend predefined classes with word-only names, such
as `classDef slot1 fill:var(--chart-1-soft),stroke:var(--chart-1)`, so authors write
`class findJobTickets,resolveJobId slot1`. Keep tones for semantic states and slots for
categories.

Done when: slot classes render theme-correct in light and dark; text over each soft fill meets
contrast; the guide's `diagrams.md` documents slots, word-only class names and the comma rule.

### P2. A legend component

Add `<ex-legend for="island-id">` reusing the `ex-breakdown` child vocabulary:
`<data series="1">Code step</data>`. It draws swatches in the same slots, so colour never
carries meaning alone and authors stop writing colour keys into prose.

Done when: the legend renders beside or under its diagram at both widths and themes, and the
checker reports a legend slot that the diagram never uses, or a used slot missing from the legend.

### P3. Diagram rules in the checker

Warn on: class names that do not match `\w+`; `classDef` values containing commas; raw colour
literals when slots exist; `class` lines naming an undefined class. Report each diagram's
rendered width, computed with the same formula as `sizeDiagram`, against Reading (760), Canvas
(1600) and phone (390) widths, for example "scrolls below 1228 px".

Done when: each fixture below produces exactly one finding, and the consumer page reports the
same widths that a browser measures.

### P4. A rendered signal and a capture command

Document `island-ready`. Add a document-level signal when every island on the page has
rendered, for example a `data-ex-pending` count reaching zero plus an `ex-ready` event, and a
`?ex-render=all` mode that disables lazy rendering. Add
`demistify capture page.html --width 1500 --theme light|dark --section id --open-details`
beside `check`, `embed` and `index`.

Done when: a capture of the consumer page at 390 px and 1500 px in both themes shows every
diagram without scrolling tricks.

### P5. Checker support for standalone output

Mark bundles produced by `bundleStandaloneHtml` (for example `<meta name="demistify-bundle">`)
and have `check` skip authoring-only `library` and `styling` rules for them, or add
`check --delivered`.

### P6. Layout guidance, and the hidden arrowhead

Add to `diagrams.md`: cycles change ranking, so prefer one two-way edge; give branches their own
terminal nodes; long edge labels widen diagrams. Draw edge markers above node shapes, or offset
them, so a two-way edge keeps both arrowheads visible.

### P7. Timelines

Either contribute a Gantt or timeline family to the renderer, or document the closest native
alternative and its limits (`ex-breakdown` with `scale="elapsed"` holds at most three phases).

## Reproduction fixtures

```text
flowchart LR
  a[Code] --> b[Agent]
  classDef ex-slot-1 fill:#ff000033
  class a ex-slot-1
```

```text
flowchart LR
  a[Code] --> b[Agent]
  classDef code fill:rgba(255,0,0,0.2)
  class a code
```

```text
flowchart TD
  hub{{Hub}} <-->|both ways| worker[Worker]
```

```text
flowchart TD
  source[Source] --> hub{{Hub}}
  hub --> worker[Worker]
  worker --> hub
```

```text
gantt
  title Run
  dateFormat X
  section First run
  gates :0, 2
```

For width, render any flowchart with edge labels and compare its SVG `width` attribute with the
island's rendered width: the ratio is 16 divided by the smallest `font-size` in the SVG.
