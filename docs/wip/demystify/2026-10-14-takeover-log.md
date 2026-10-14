# Demistify takeover — running log

Append-only. Newest entries at the bottom. Times are UTC. Decisions and directives live in [the decisions record](./2026-10-14-takeover-decisions.md); this log records what happened, evidence, and what is next. Other agents: add your own dated entries with your name; do not edit someone else's entry.

Branch: `claude/serene-pascal-llse28` (base: `feat/demistify-html-islands` at `a31c1da`, plus `main` at `c48b492`).

## 2026-10-14

- **15:20 · Claude (takeover)** Fast-forwarded onto `a31c1da`; read the handoff, AGENTS.md and READMEs. Build and `pnpm check` pass on Node 26.12.0.
- **15:30 · Claude** Measured cold load: ~443 KiB without diagrams, ~949 KiB with one. Causes: 8 base64 fonts in render-blocking `theme.css`, full 1,530-glyph Phosphor font, 500 KiB-gzip Mermaid chunk (beautiful-mermaid + elkjs).
- **15:40 · Claude** Haiku baseline (README only, simple merge-vs-rebase topic): renders, no console errors, but hand-written CSS, invisible arrow, guessed attributes, no theme toggle. Details in the decisions record.
- **15:45–16:05 · Claude + Shravan** Decisions D1–D11 and the five directives recorded.
- **16:00 · Claude** `git push` and the GitHub API both return 403 (Claude GitHub App lacks write access). All commits are local until access is fixed.
- **16:09 · Claude** Starting step 1: small element set (`ex-card`, `ex-callout`, `ex-grid`, one tone list mapped to icon and colour), automatic page chrome (outline + theme toggle), visible issues panel, skill-format guide. The navigation files (`heading-navigation.ts`, `navigation-styles.css`) are left to the other agent's sidebar work.

## Next

1. Step 1 implementation, then Haiku round 1 on the planning-corpus brief.
2. Step 2 load work. 3. Flow container. 4. D9 elements.
