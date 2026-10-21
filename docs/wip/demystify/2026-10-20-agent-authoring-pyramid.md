# Agent authoring pyramid: where Demistify and its skills fail

Date: 20 October 2026. Stage: **research, not accepted**. Target: Demistify 1.9.3 (`c191e88`, local build) and the `presentation-html` skill at sunstack `e34ad3f` (sunstack-dev 2.92.0). A second build of `fix/demistify-flow-labels-and-tab-spacing` (`2a871f4`) was used only as an A/B comparison for Flow labels.

Evidence: the agent-written pages, harness scripts, audit JSON and screenshots stayed in the run's scratch space and are not committed, so they do not land in the app or package folders. The rendered before/after comparison was shared with the owner as a private review page. The layout and issue changes from this analysis ship in the same PR as this document.

## Method

Thirteen independent agents each wrote one page about a different real subject. Every agent followed `presentation-html`, which loads the Demistify guide (`packages/demistify/skill`), and returned a fixed friction log.

| Tier   | Model (effort)     | Experiments                                                                                                                                                              |
| ------ | ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Base   | Haiku (high) ×8    | H1 Stop hook flowchart, H2 OTel board, H3 code review, H4 manifest comparison, H5 tree + trace, H6 gallery + Gallery Journey, H7 sequence/state tabs, H8 stats + xychart |
| Middle | Sonnet (medium) ×3 | S1 three-page collection + index + cross-page Walkthrough, S2 call-path change, S3 revise an existing design-set page + embed/compare                                    |
| Top    | Opus (medium) ×2   | O1 four-document design-set entry page, O2 rich explainer + owner decision brief                                                                                         |

The hosted CDN was blocked by the sandbox network policy. A byte-identical local build was served by request interception, with pages keeping the public URLs. Each agent had the checker, a `file://` render tool (desktop 1440 and phone 390) and nothing else. After the runs, one audit script rendered all pages over HTTP against both builds and measured outline duplicates, leaked Markdown, blank Mermaid, Flow label collisions and fixed-chrome overlap. A small repro page isolated the library-level claims.

## Results at a glance

- **13/13 final pages pass the checker with 0 errors/0 warnings.** Every page still had at least one defect visible on screen; most were found only by looking at screenshots.
- Token use reported by the harness (`subagent_tokens`): Sonnet 94.7k, 99.2k and 100.8k; Opus 103.3k and 120.0k; Haiku 103.5k–150.8k. **Only 2 of the 5 higher-tier runs stayed under the 100k target, even with an explicit cap.** Agents' self-estimates were 45–65k, roughly half the measured value. Haiku used more tokens than Sonnet on simpler tasks, mostly from extra render/edit loops (H1: 30 tool calls; H7: 25).
- Mandatory reading floor for a simple page: `presentation-html` SKILL (8.0 KB) + `demistify.md` (6.8 KB) + guide SKILL (19.3 KB) + Markdown baseline (6.8 KB) = 41 KB (~10k tokens). The design-set route adds `design-documents` 17.3 KB, `diagram-rendering-and-fallbacks` 13.7 KB, `engineering-documents` 4.0 KB, and often `diagram-semantics` 10.9 KB (~87 KB in total). **Five agents openly skipped MUST loads to fit the budget**, so the skill's own completion criteria were not met in practice.

## Confirmed library defects (reproduced, not just reported)

| #   | Defect                                                                                                                                                                                                                       | Evidence                                                                       | Checker                                                |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ | ------------------------------------------------------ |
| L1  | Comparison topic `<h3>` per column floods the outline (each topic ×N alternatives). The guide's own `comparisons.md` example uses `<h3>`, and the PoC `call-paths.html` / `comparison-views.html` show the same duplication. | audit: S2 6 dupes; PoC ×3 each. O1, O2 and H4 worked around it with `<strong>` | silent                                                 |
| L2  | Bare Markdown in a plain `<div>` (e.g. an `ex-grid` child) renders literally (`**bold**`, `[link](#)`)                                                                                                                       | repro R1; O2                                                                   | silent                                                 |
| L3  | A literal `<` in bare Markdown corrupts the paragraph, merges the following task list into one line and pushes the next callout/card out of the Reading column                                                               | repro R7; H1, H4                                                               | silent (the guide forbids it, but nothing enforces it) |
| L4  | A live `src` page passes the checker but renders "Loading…" plus a red issues panel when double-clicked (`file://`). This is the guide's headline delivery path.                                                             | S3 baseline, O1 live, PoC `component-guide/*`                                  | silent                                                 |
| L5  | The runtime injects an `ex-nav` on every page, so a **Pages** button appears with no pages                                                                                                                                   | `document.ts:30`; all 13 pages                                                 | n/a                                                    |
| L6  | Fixed bottom-left theme/search controls cover first-screen content on a 390px phone                                                                                                                                          | audit: 15/24 pages overlap 1–4 content blocks                                  | n/a                                                    |
| L7  | `xychart-beta` legend reads "Bar 1 / Line 1", and lines are smoothed between categories                                                                                                                                      | repro R10; H8                                                                  | silent                                                 |
| L8  | Mermaid in an inactive tab is not rendered until opened; checker/render tools see only the visible tab                                                                                                                       | H7, O1 (0/8 SVG on load)                                                       | n/a                                                    |
| L9  | `ex-flow` arrow labels collide with cards and with each other                                                                                                                                                                | audit 1.9.3: H2, O2                                                            | silent                                                 |
| L10 | `ex-stat`/`ex-grid` default to Canvas (1116px at 1440) beside 760px prose; a single stat stretches full-width                                                                                                                | audit widths; PoC `content.html`                                               | n/a                                                    |
| L11 | `## Heading {#id}` renders literally; no documented way to give a bare-Markdown heading a stable ID                                                                                                                          | repro R3; H4                                                                   | silent                                                 |

**Flow A/B (`2a871f4`):** label-on-card/label collisions go to 0 on H2 and O2. However, the branch now **drops** labels it cannot place and shows a red "No clear space for arrow label" panel _to readers_. O2 shows it on desktop and phone, and H2 shows it on phone only, for a three-column flow. The Project card's title also wraps under its icon. Before release, the trade between overlap and a reader-visible error needs an owner decision.

## Guide and skill friction (agent-reported, cross-checked)

1. **Contradictions/gaps between documents.** `ex-callout` accepts `icon` (custom data, checker) but the guide's island table omits it, while `presentation-html` asks for icons on callouts. The checker's icon list includes `path`; the guide's list does not. The guide lists `<ex-mermaid id>`; the custom data lists `label`/`src`. Walkthrough `ex-nav` rules appear three times (presentation-html step 3, guide, `tours.md`).
2. **Undocumented and unusable proof tool.** `demistify check --render` exists (`render-check.ts`: desktop + phone, collects runtime issues), but neither skill mentions it, and it fails without Playwright installed next to the package. Rendering is the gap that hid L1–L3, L7 and L9.
3. **The `presentation-html` step 3 paragraph has about 15 conditional loads in one block.** Every Sonnet/Opus agent called it heavy. Several agents called the "return construct choices / technical atoms / before-send check" returns chat-surface ceremony when the output is an HTML page.
4. **The design-set route has an unhandled branch and a contradiction.** There is no instruction for a missing `index.md` or for a fifth "alternative" document (O1). The rendering reference asks the page to "open on the chunk's lifecycle diagram" yet forbids copying a diagram, and the API cannot show one part of an island. O1's opening slice became a board without arrows, weaker than the source.
5. **Owner decision brief vs comparison.** `owner-decision-brief` wants each option drawn as a change to the current picture, but `options-and-decisions` routes to `ex-compare`, which has no per-option picture slot (O2).
6. **Undocumented details agents had to probe.** These are: slug rules (O1 needed a probe script for `—`), cross-page scoped stop IDs (S1), `index` editing files in place (S1), checker blindness to script-assigned IDs (S3), diff line numbering for `ex-note` (H3), `ex-code` languages (H4), which text the gallery Review title uses (alt vs caption, H6), and whether neutral cards need a tone (the checker warns, the guide implies optional).
7. **Delivery instructions conflict with the agent harness.** `demistify.md` forbids headless Chrome/servers for proof, while the repo's own `verify-demistify` uses headless Chromium. Agents could not satisfy step 4 ("open the actual page") without a tool the skill forbids.

## Visual and component critique

These were observed in screenshots, light and dark, desktop and 390px.

- **Two left edges.** Headings and prose sit on the 760px Reading column while tables, grids, stats, code and compare start at the 1116px Canvas edge. Every mixed page has a jagged left margin (`component-guide/content.html`, H8, O2). A shared left edge with Canvas growing rightward, or Canvas only when content needs it, would read as one document.
- **Vertical rhythm is too loose.** Large H2s plus a full divider above each one cost about 140px per section, so pages run long (H1 6.8k px, O1 7.2k px for modest content). Agents called them "sparse".
- **Compare.** Topic labels repeat as full headings in every column, columns have no surface, and short rows leave large holes. Redesign: one sticky topic gutter (label once, not in the outline), optional per-alternative surface, an optional picture row.
- **Flow / board.** Routing doubles back across rows and labels crowd (1.9.3). Boards have no arrows between regions, so ownership views lose direction (O1). Grid authors cannot hint direction or lanes.
- **Stat.** One number stretched to Canvas width looks empty. Size it to content in a row; Reading width by default.
- **Floating chrome.** The bottom-left theme/search pair overlaps content on phones, and the Walkthrough launcher is an unlabeled icon (S1, H6). Put them in the top toolbar on narrow screens, and give the launcher a text label.
- **Code.** Tab-indented source renders at width 8 (S2 excerpt). In dark mode the removed/added row tint ends before the block edge (`content.html`). Long diffs have no collapsed-context option (H3).
- **Trace / tree.** Steps all start collapsed, so the answer hides behind clicks (H5). Tone icons (`</>` in red/amber/green) are hard to distinguish at a glance (S2).
- **xychart.** The generic legend and interpolated line mislead. Prefer a table or a delta bar until series names are supported.

## Recommendations

**Demistify (library)**

1. Make the checker catch what agents actually get wrong statically: a literal `<` in bare Markdown, Markdown in non-adopting containers, compare-row headings, `src` on a page meant for `file://`, and unnamed xychart series.
2. Publish the existing checker as one portable file beside `runtime.js` (see "Portability and getting started"). Keep rendering proof optional: `check --render` stays an opt-in for environments that already have a browser, never a required step or a bundled browser.
3. Fix L1 in the component (topic label not an outline heading) _and_ in `comparisons.md`; fix L5 (no Pages button without pages), L6 (phone chrome placement), L10 (stat/grid default width).
4. Flow labels (owner decision, below): place an unplaceable label at a fallback position and report it to the author only. `2a871f4` currently drops it and shows readers a red panel.
5. Fix the Walkthrough defects W1–W3 and adopt the layout proposal P1–P7 (sections below).
6. Make authoring issues agent-facing (owner decision). Keep the `[demistify]` console warnings and the checker as the agent channel, and add a small machine-readable count (e.g. a root data attribute) so automated checks need not parse text. Show the panel and outlines only in an explicit author mode (`?ex-check` as implemented in this PR). Where content genuinely cannot render for a reader, such as a missing Markdown file, show a quiet in-place note in that island, not a floating developer panel.

**`presentation-html` and the Demistify guide**

1. Split step 3 into a short decision table (page type → which one or two references to load) and keep the conditional detail in references. Cap the mandatory chain for a simple page at the guide plus one reference.
2. Remove chat-surface returns (atoms/before-send) from the HTML route, or reduce them to one checklist line.
3. Give one owner to each rule now stated in several places (Walkthrough `ex-nav`, IDs/slugs, embed). The guide owns API; the skill links to it.
4. Add the missing branches: no `index.md`, extra alternative documents, absent Verification Design, picture-per-option decisions.
5. Make the proof route one portable command (`node check.mjs page.html`) plus opening the page. Treat rendering automation as optional, and state which file is "the page" for a design set (embedded copy vs live `src`).
6. Publish small documented facts that agents had to probe: slug rule, cross-page scoped IDs, `index` in-place edit, `ex-note` numbering, code languages, callout `icon`, tone optionality.

## Owner decisions (20 October 2026)

- **Evaluation axes:** taste and style, ease of use for agents, and portability, which includes performance and how quickly someone can start using Demistify.
- **Flow labels:** fallback placement, author-only error. Readers do not see a red panel for an unplaceable label.
- **Checks:** small, fast, optional and fully portable; nothing to install. Rendering proof must not be forced or heavy.
- **Layout:** the owner asked for iteration and concrete solutions, and must see before/after comparisons before deciding. The result is below. After reviewing the rendered comparison, the owner approved the layout; it ships in this PR.
- **Checks (21 October):** both a one-file Node checker and an in-page author report. Readers see neither.
- **Authoring issues are for agents, not readers (21 October):** the checker and the runtime's issue reports must not get in a reader's way. Today all 24 `reportIssue` call sites open a floating panel and dashed outlines on the reader's page (`core/issues.ts`, `.ex-issues` in `styles/vocabulary.css`), which also blocks the Walkthrough launcher (W3).
- **Heavy or blocking libraries run in workers (21 October):** diagram layout (beautiful-mermaid with ELK), syntax highlighting (Shiki) and any other library that can block the page run off the main thread. A main-thread fallback for that work is not acceptable. Today `render/diagram-client.ts` falls back to the main thread whenever the worker cannot start, which happens on every double-clicked `file://` page (measured below), so the worker start must be fixed and the fallback replaced, for example by showing the diagram source or a placeholder. `marked` measured 6.8 ms for 80 KB, which is below that bar; re-measure if Markdown gains heavier steps.

## Walkthroughs (driven, not inferred)

A Playwright driver opens the launcher, starts every Walkthrough and Gallery Journey and presses Next through every stop at 1440 and 390. It records the focused target, its position, player overlap and issues. It covered 36 runs: 10 on experiment pages (`file://`) and 26 on the PoC demos (HTTP).

- **Works:** a cross-page Walkthrough across three `file://` pages (S1, 6/6 stops). Gallery Journeys (H6 and the PoC `journey.html`). Deep links and the command-bar list with descriptions.
- **W1 Late layout shift:** O1 stops 2 and 4 land with the target 1,100–2,500 px below the viewport on both widths; diagrams above render after the scroll. The guide promises stops "await content readiness".
- **W2 Tall targets land past their start:** a stop on a tall section or island lands at its middle or end, with the player covering 119–138 px of it. Seen in S3 and in four PoC demo Walkthroughs (`component-guide/content.html`, `layouts.html`).
- **W3 Issues panel blocks the launcher:** with any authoring issue on the page (e.g. a `file://` `src` failure), the panel intercepts clicks on the bottom-left launcher. The reader cannot start a Walkthrough.
- **W4 Discoverability:** the launcher is an unlabeled 34 px icon at the bottom left. On a 390 px phone the player takes 165–219 px (20–26 % of the screen).
- **Teaching:** `tours.md` lacks the combined cross-page scoped ID form (`setup.html#island--heading`) that S1 had to guess. The `ex-nav` launcher rule appears in three documents.

## Portability and getting started

**What a page costs to load.** Built library, measured with a Playwright script, median of 3, local Chromium:

| Page kind                                          | Library requests | Raw / gzip            | First paint | Settled    |
| -------------------------------------------------- | ---------------- | --------------------- | ----------- | ---------- |
| No diagram (tree, code, gallery, 91 KB design set) | 15               | 504 KB / 207 KB       | 84–120 ms   | 236–290 ms |
| Any Mermaid diagram (even a 2-node one)            | 14–16            | 2.6 MB / 0.68–0.71 MB | 72–116 ms   | 423–672 ms |

The diagram cost is one 2.1 MB (500 KB gzip) chunk, mostly the ELK layout engine (`chunks/dist-*.js`). A 1 KB authored page therefore pulls 200–700 KB gzip of shared assets. Rendering is fast, so the cost is transfer size, which matters most on slow or offline connections.

**Getting started today** needs a sunstack-tools checkout, pnpm install and Node 26 to run `check`. `check --render` also needs Playwright installed next to the package (it failed here). The guide documents neither `--render` nor a portable route.

**Measured portable alternative.** The existing CLI, bundled with esbuild into one file (`demistify-check.mjs`, 2.1 MB, 640 KB gzip), runs from an empty folder with plain `node` (22 works). It gives the same findings as the repo checker in 0.37–0.66 s, and its `embed` output is byte-identical to the repo's. Proposal: publish it as `/v1/check.mjs` beside `runtime.js`, then `node check.mjs page.html`. No checkout, package manager or browser is needed. A lighter variant can load diagram validation lazily so `--no-diagrams` skips the ELK code. In-page issues already need nothing installed; keep them author-facing, consistent with the Flow decision.

## Diagrams, workers and Shiki

Questions from the owner: is ELK good, can it load later like Suspense, can layout and other work run in workers, and how should Shiki rendering in a worker fit?

**Today (source, `c191e88`):** diagrams already render off the main thread. `render/diagram-client.ts` starts a module worker (`diagram-worker.ts`) through a same-origin blob shim and falls back to the main thread when a worker cannot start. The engine chunk is a dynamic import, so pages without diagrams never fetch it (measured above). Markdown (`marked`) runs on the main thread.

**Measured (Node 22, local):**

| Work                                                                      | Result                                                                                                  |
| ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| ELK share of the diagram chunk                                            | `elk-worker.min.js` 1,594 KB raw / 464 KB gzip of the chunk's 2,196 KB / 513 KB                         |
| beautiful-mermaid built without ELK                                       | 94 KB raw / 28 KB gzip; sequence and xychart render, flowchart throws (needs ELK)                       |
| ELK layout time                                                           | first call 148 ms (warm-up); 3-node flowchart 7 ms; 31 nodes / 40 edges 71 ms; 4-state diagram 8 ms     |
| Sequence diagram, 3 actors / 28 messages                                  | 0.2 ms (no ELK)                                                                                         |
| `marked`, 80 KB of design-set Markdown                                    | 6.8 ms                                                                                                  |
| Shiki fine-grained (`shiki/core`, JS regex engine, 4 languages, 2 themes) | 428 KB raw / 85 KB gzip in 8 chunks; core 54 KB gzip, each language 3–16 KB gzip and loadable on demand |
| Shiki highlight, 487-line TypeScript file, dual theme                     | 1,651 ms first, 745 ms second                                                                           |

**Is ELK good?** For layered graphs, yes. It is the layout-relevant part of Eclipse ELK, transpiled to JavaScript with GWT ([elkjs README](https://github.com/kieler/elkjs)). Its layout is promise-based and it can run in its own worker. The measured layout times are small next to its download size. In beautiful-mermaid only flowchart/state (`src/layout-engine.ts`), class (`src/class/layout.ts`) and ER (`src/er/layout.ts`) import ELK, through `src/elk-instance.ts`. Sequence and xychart have their own layouts, but the published single `dist/index.js` imports ELK for every diagram.

**Recommendations (proposals, not decided):**

1. **Two-stage diagram loading.** The worker loads the light renderer first (about 28 KB gzip) and imports ELK only when a diagram's first keyword is `flowchart`/`graph`/`stateDiagram`/`classDiagram`/`erDiagram`. This needs a build from beautiful-mermaid's source (or an upstream change) so that ELK sits behind a dynamic import. Pages with only sequence or xychart diagrams then drop about 440 KB gzip.
2. **Suspense-style placeholders that reserve space.** Show a sized placeholder while the engine loads, and swap in the SVG when it arrives. Reserving the height (from a cached size or an estimate from node and message counts) also addresses Walkthrough defect W1, where late diagram rendering pushes stop targets off screen.
3. **Keep Markdown on the main thread for now.** At 6.8 ms for 80 KB, posting text to a worker saves little. Revisit only if Markdown gains highlighting or very large documents.
4. **Shiki belongs in a worker.** CPU time (0.7–1.7 s for one 487-line file) is the problem, not size. Use `shiki/core` with the JavaScript regex engine and languages imported on demand. Create the highlighter once per worker, highlight visible blocks first, and cache by source hash. Shiki's docs recommend the fine-grained core and the JavaScript engine for browsers ([bundles](https://github.com/shikijs/shiki/blob/main/docs/guide/bundles.md), [regex engines](https://github.com/shikijs/shiki/blob/main/docs/guide/regex-engines.md)). The official guide fetched here did not cover workers, so the worker pattern is our design, not upstream guidance.
5. **One worker pool for diagrams and code.** Both are pure string-to-markup transforms. One module worker with lazy-loaded engines avoids a second blob shim and keeps the CDN-relative loading that already works under `file://`.

**Measured in Chromium (21 October).** A module worker in the shape Demistify already uses (a same-origin blob that imports the CDN-hosted worker) runs Shiki 4.5 with the JavaScript regex engine and dual GitHub themes. Served from a separate origin with `Access-Control-Allow-Origin: *`, as the CDN does:

| Step                                                  | Result                                                                              |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------- |
| Highlighter core + TypeScript grammar                 | 15 ms + 17 ms in the worker                                                         |
| Tokenize 487-line TypeScript (`layout/route.ts`)      | 1,671 ms first, 423 ms second                                                       |
| Small JSON block                                      | 13 ms round trip                                                                    |
| Paint the returned tokens as spans on the main thread | 21 ms; no long tasks over 50 ms                                                     |
| Token colours                                         | inline `color` plus `--shiki-dark` per token, so light and dark need no second pass |

**Double-click pages break the worker today.** From a `file://` page, Chromium refuses the blob module shim that `render/diagram-client.ts` uses. A `data:` URL module shim starts from both `file://` and `http`, and so does the blob shim from `http`; a direct cross-origin worker URL throws. With the real 1.9.3 library, `h1-lint` opened from `file://` falls back to main-thread diagram rendering: 311 ms of long tasks (70 + 241 ms). From `http` it has none. The double-click delivery the guide promotes therefore loses the worker. The `data:` shim is the smallest fix; Firefox and Safari behaviour is not verified.

**How hard is Shiki for code blocks?** Moderate, and mostly plumbing that already exists:

- Easy: Shiki is a pure text-to-tokens transform and runs in the worker as measured. `ex-code` already builds one row per line (`render/code-lines.ts`, `islands/code.ts`), so tokens map onto the existing numbered rows, diff kinds and notes without changing them. Markdown fences arrive from `marked` as `pre > code.language-*`; after an island renders, the same client can replace their text with tokens. Mermaid fences keep their current path.
- Needs care: diffs carry `+`/`-` prefixes and two versions of the file, so tokenize the stripped text per file. Highlighting is CPU-heavy (0.4–1.7 s for 487 lines), so show plain text immediately, colour visible blocks first, cache by source hash, and never fall back to the main thread for highlighting; plain text is the fallback. Theme colours should come from Demistify tokens through a custom or CSS-variable Shiki theme rather than GitHub's palette.
- Prerequisite: fix the `file://` worker start first, or double-click pages would lose highlighting entirely.

Not verified here: Mermaid's own default layout engine in its latest major (search results conflict, and the Mermaid docs site was unreachable from this environment), and real-device or Cloudflare timings.

## Layout iteration

A proposal stylesheet was layered over 1.9.3 and applied to eight pages, rendered at 1440 and 390 and re-audited. Iterations, kept honestly:

1. Anchor visual islands to the Reading column's left edge (rejected): alignment improved, but islands lost width. O2's three-option comparison collapsed to one column, wide diagrams clipped more, and a table inside a card was cut off.
2. **Keep Canvas full width and move the Reading column to Canvas's left edge (recommended).** One left edge for headings, prose, callouts, stats, Flow, grids and comparisons; comparisons keep their columns.

Result of the recommended set (P1 left-aligned Reading, P2 rhythm, P3 stat width, P4 comparison topic labels, P5 tables scroll instead of crushing, P6 tab size, P7 phone controls in the top bar; P8, added later, hides authoring issues from readers unless `?check` is present):

| Page                     | Desktop height            | Phone height              | Phone controls covering text |
| ------------------------ | ------------------------- | ------------------------- | ---------------------------- |
| h8-metrics               | 3459 → 3239               | 6007 → 5093               | 1 → 0                        |
| o2-router                | 5024 → 4795               | 10170 → 9996              | 3 → 0                        |
| s2-callpath              | 3711 → 3468               | 6185 → 5813               | 2 → 0                        |
| s3-revise (embedded)     | 6506 → 6304               | 11328 → 9085              | 1 → 0                        |
| s1-sidecar index / setup | 1760 → 1604 / 3143 → 2876 | 2199 → 1924 / 3876 → 3098 | 1 → 0 / 4 → 0                |
| h1-lint                  | 6795 → 6572               | 8992 → 8667               | 0 → 0                        |
| PoC content              | 3656 → 3150               | 4576 → 4106               | 0 → 0                        |

No page gained horizontal overflow. P4 restyles comparison topics as labels, but the outline duplication (L1) still needs a runtime change. P5 restores the readable "Authorized needs" table in S3.

## Limits

- Walkthroughs were driven separately (section below). Gallery fullscreen, sidebar inspection and search remain **unverified** except where S3 and O1 touched them.
- Performance timings are local and unthrottled. Transfer sizes are computed from gzip of the built files, not measured from Cloudflare.
- `subagent_tokens` is the harness figure; whether it counts cumulative input or final context is not documented here.
- One run per cell: model differences are anecdotal, not statistically supported.
- The other agent's WIP teaching spec (in a private lab on the owner's Mac) was not readable from this environment and was not compared.
