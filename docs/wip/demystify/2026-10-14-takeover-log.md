# Demistify takeover — running log

Append-only. Newest entries at the bottom. Times are UTC; times before 16:09 are approximate. Decisions and directives live in [the decisions record](./2026-10-14-takeover-decisions.md); this log records what happened, evidence, and what is next. Other agents: add your own dated entries with your name; do not edit someone else's entry.

Branch: `claude/serene-pascal-llse28` (base: `feat/demistify-html-islands` at `a31c1da`, plus `main` at `c48b492`).

## 2026-10-14

- **15:20 · Claude (takeover)** Fast-forwarded onto `a31c1da`; read the handoff, AGENTS.md and READMEs. Build and `pnpm check` pass on Node 26.12.0.
- **15:30 · Claude** Measured cold load: ~443 KiB without diagrams, ~949 KiB with one. Causes: 8 base64 fonts in render-blocking `theme.css`, full 1,530-glyph Phosphor font, 500 KiB-gzip Mermaid chunk (beautiful-mermaid + elkjs).
- **15:40 · Claude** Haiku baseline (README only, simple merge-vs-rebase topic): renders, no console errors, but hand-written CSS, invisible arrow, guessed attributes, no theme toggle. Details in the decisions record.
- **15:45–16:05 · Claude + Shravan** Decisions D1–D11 and the five directives recorded.
- **16:00 · Claude** `git push` and the GitHub API both return 403 (Claude GitHub App lacks write access). All commits are local until access is fixed.
- **16:09 · Claude** Starting step 1: small element set (`ex-card`, `ex-callout`, `ex-grid`, one tone list mapped to icon and colour), automatic page chrome (outline + theme toggle), visible issues panel, skill-format guide. The navigation files (`heading-navigation.ts`, `navigation-styles.css`) are left to the other agent's sidebar work.
- **16:09–16:41 · Claude** The entries below happened in this order within that window; individual times were not recorded.
- **Claude** Step 1 committed: `ex-card`, `ex-callout`, one tone list, `ex-grid`, automatic outline + theme toggle, issues panel, skill-format guide (`packages/demistify/skill/SKILL.md`), planning fact brief.
- **Claude** Haiku round 1 (planning brief): 5,737 tokens, all facts, no issues; it hand-wrote one layout rule and the diagram duplicated the cards. Led to: diagram nodes whose id matches a card take its tone and open it; grid auto-fit default; `done` blue to differ from `added`.
- **Claude** Step 2: fonts moved out of CSS into six linked files (only used faces download, after paint); Phosphor subset to 24 icons. theme.css 302 KB → 27 KB, icon font 147 KB → 2 KB. Main page 443 → 143 KiB transferred, FCP 324 → 152 ms; diagram page 949 → 650 KiB; offline bundle 570 → 241 KiB. Fixed a nav re-render loop on pages without `ex-nav` (inserted nav now scoped to the document).
- **Claude** Haiku round 2 (planning + new page-loading brief): 4,300 and 2,309 tokens, zero issues, zero style blocks, every diagram node linked.
- **Claude** Step 3: `ex-flow` routes arrows only through the gaps between rows/columns with per-channel lanes and spread ports; arrows outside a flow are reported when they cross a box. Fixed-column grids collapse on mobile.
- **Claude** Step 4: `ex-code` (code or diff with `ex-note` margin notes; tone marks the line number, diff colours stay), `pick` on cards for chosen options, charts via Mermaid `xychart-beta` with named series. Unrendered diagrams reserve height and render 300 px before view (a final-position diagram never rendered before).
- **Claude** Merged the local agent's `6249bf0`, `36f73a6` (sidebar lifecycle, detached-target fix, circular triggers, navigation demo). Implemented its relayed owner requirement 5: every content island now has sidebar + expand controls by default (`zoom="none"` opts out; innermost hovered island only; always visible on touch). Committed the missing `subset-font` lockfile entry.
- **Claude** Haiku round 3 (three briefs, final guide 1,472 tokens): planning 4,434, page-loading 2,353, code-review 1,669 tokens; no style blocks; zero authoring issues; all linked nodes resolve. One model wrote bare Markdown between islands → the library now renders bare text inside `ex-document`/tab sections as Markdown. Untitled callout icons moved to a gutter. Pages saved verbatim in `apps/demistify-poc/experiments/agent-written/`.
- **Claude** Push still 403 from this container; all of the above is local on `claude/serene-pascal-llse28` on top of the local agent's `36f73a6`.
- **16:45 · Claude** Focused rail check the local agent asked for, on `/experiments/html-islands-navigation-live.html` (body fallback, no wrapper): at 1024 px (hover) and 390 px (touch, focus) the preview marks exactly the marker's own section, separate from the current-section marker; page stays responsive (evaluate round-trip 6–8 ms); no page errors.

## Open

- Diagram engine chunk is still 500 KiB gzip (beautiful-mermaid + elkjs); a lighter path for simple flowcharts is unexplored.
- Mermaid has no cross-tab node linking; nit and question share a tone.
- Long arrow labels on short horizontal flow arrows can overlap card edges (legible via halo).
- The five-prototype report still measures the hand-written `planning.html` with the older vocabulary; the agent-written pages are separate evidence.
