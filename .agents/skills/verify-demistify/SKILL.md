---
name: verify-demistify
description: Verify Demistify's built browser runtime and Markdown CLI through presentation navigation, fragments, Markdown/source inspection, diagram scrolling, tree wrapping and visual inspection journeys. Use when checking a declared Demistify build or maintaining these project-specific verification instructions.
---

# Verify Demistify

Verify `packages/demistify` through its plain-HTML consumer, `apps/demistify-poc`. Keep the real built library, Markdown files, native custom elements, diagram renderer and browser layout in the path. CLI checking and embedding prove different claims from browser behavior.

**Draft: source-mapped only.** A representative launch-to-cleanup pass has not established that this recipe is operable. Observation needs a pinned commit/build, frozen fixture and released browser surface or finished authorized browser harness. The new harness command is not yet published. All unobserved journeys remain unknown.

## Target and contract

Read the [owner task list](../../../docs/wip/demystify/2026-10-16-fixes-todo.md), [project instructions](../../../AGENTS.md) and [handoff](../../../docs/wip/demystify/2026-10-14-demistify-handoff.md). Their outcomes and the current commission are the oracle. Sidebar/source inspection retains rendered main content. Fullscreen may temporarily move live content, but Close/Escape must restore the same nodes and state.

Record the declared commit, permitted dirty-state exceptions, build location/receipt, fixture URL, browser/client, viewport and process/browser ownership. Compare `git status --short`, `git branch --show-current` and `git rev-parse HEAD` with that declaration. Moving source supports preparation, not delivery observation. Stop affected observations if the target changes.

Public-safe fixtures need no account/backend. Existing dependencies are a prerequisite; this recipe grants no installation, browser control or execution beyond the current commission.

## Build, launch and health

From the monorepo root, with Node 26+ and pnpm 12, after execution is authorized:

```sh
pnpm check
pnpm build
pnpm --filter @sunstack/demistify-poc preview --port 4174
```

These commands come from root/package scripts and [README](../../../README.md). The root build builds the library, then the PoC, which copies the library into public assets and builds static output. A library-only build leaves the PoC copy stale. Preview binds loopback; record its actual URL/port and process/session identity. Leave occupied ports/processes alone and select an unused port.

For an already built frozen instance, inspect its build receipt/resources instead of rebuilding it. An answering port or Vite ready message alone is insufficient.

**Read-only health check:** confirm the declared fixture and expected heading at the recorded origin; served runtime/theme match declared built assets; relevant elements have upgraded; visible Markdown is prepared with `aria-busy` cleared; visible Mermaid has a real nonempty SVG. Inspect relevant failed requests/browser errors. Use bounded readiness checks, ordinarily revealing lazy diagrams through their tab/scroll position first. Missing served-build identity is a setup gap.

## Drive and reuse

Use the finished repository browser suite after inspecting its published command, configuration, fixtures and real-path assertions. At this draft's source cutoff, `apps/demistify-poc/vitest.config.ts` uses Node, and the package `test` script builds experiments then runs Vitest. That command alone is not browser evidence. The Implementer owns product tests and the pending harness; integrate its finished command when the Lead publishes it.

For direct UI observation, use host computer-use/Chrome on one released surface with sole control. Use ordinary links, tabs, buttons, wheel and pointer input. Read-only DOM geometry corroborates visible outcomes. Internal setters, synthetic events and mocked rendering/fetch do not establish user interaction. Do not introduce a separate Playwright driver.

Inspect a passing journey's target, fixtures, interactions, assertions and result before reuse. Reuse applicable evidence. Add direct observation for a missing interaction, changed dependency or specific unresolved doubt; independence does not require repeating equivalent proof.

## Journeys

Read [journey instructions and source map](references/journeys.md) for the selected scope.

| Journey | Deciding observation |
| --- | --- |
| Presentations/pages/fragments | Cold and same-page links reveal the correct presentation/tab; selected-only content and grouped navigation remain useful |
| Markdown/Mermaid/source | Referenced text and real SVG render; source text/reference stay exact while main content stays rendered |
| Diagram layout/inspection | Local scrolling reaches both ends without page overflow, displacement, lost controls or arrow/node drift; fullscreen restores state |
| Tree labels | Multiline, inline-code and loose-paragraph labels wrap beneath text with the icon on the first line |
| Visual gestures | Cmd+wheel anchors zoom at the pointer; Cmd+drag pans; normal scroll, buttons, reset and restoration remain usable |
| CLI checking/embedding | Local sources check/embed and missing sources fail; transformation stays distinct from runtime/offline proof |

The first representative live pass uses the built navigation/Markdown fixture: load, switch presentation, open a cold Markdown fragment, observe text/SVG, open exact source with main content retained, close, save evidence and clean up. A pass covers only interactions actually driven.

## Evidence, ownership and cleanup

Use a separate run directory under ignored `tmp/verification/demistify/`. Record target/build/resource and fixture/source identities, route, viewport, actions, before/after observations, actual rectangles/scroll offsets, screenshots, console/network failures, CLI output and exit codes. Report test pass/fail/skip counts only from actual runs; preserve failures alongside later success. Keep captured content public-safe.

For each journey report **supported**, **unsupported** or **unknown**, evidence paths and undriven entry points. Source checks, suites, browser behavior, file-protocol operation, hosting and release are separate claims. Name missing sensitivity if a known failure has not shown the observation can reject it. Measure actual geometry instead of copying production formulas. Hidden overflow that clips content fails even when document width looks contained.

The Verifier owns this recipe and assigned apparatus. Product failures/test gaps go to the Implementer; unsettled expectations/targets go to the Lead. Run history belongs in the authorized trace/evidence. This recipe grants no product/test edits, public uploads, release or acceptance authority.

Stop only the process/session this run started, by recorded identity. Leave shared instances/tabs and others' files alone. Preserve evidence and reopen it after cleanup; record intentionally shared state. Directory removal needs explicit repository permission; retain scoped residue if absent.

Before calling the recipe operable, integrate the published browser command where applicable and exercise launch, readiness, health, the representative journey, evidence and cleanup on the frozen real build. Other journeys stay unobserved until adequate applicable evidence exists. Embedding/HTTP rendering alone does not establish offline/file viewing.
