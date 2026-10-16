---
name: verify-demistify
description: Verify Demistify's built browser runtime and Markdown CLI through presentation navigation, fragments, Markdown/source inspection, diagram scrolling, tree wrapping and visual inspection journeys. Use when checking a declared Demistify build or maintaining these project-specific verification instructions.
---

# Verify Demistify

Verify `packages/demistify` through its plain-HTML consumer, `apps/demistify-poc`. Keep the real built library, Markdown files, native custom elements, diagram renderer and browser layout in the path. CLI checking and embedding prove different claims from browser behavior.

**Operable for the focused headless browser and CLI routes below.** Broader journey variants remain explicitly unobserved. Record the actual target and coverage in the task's evidence/trace; a working recipe is not product acceptance.

## Target and contract

Read the [owner task list](../../../docs/wip/demystify/2026-10-16-fixes-todo.md), [project instructions](../../../AGENTS.md) and [handoff](../../../docs/wip/demystify/2026-10-14-demistify-handoff.md). Their outcomes and the current commission are the oracle. Sidebar/source inspection retains rendered main content. Fullscreen may temporarily move live content, but Close/Escape must restore the same nodes and state.

Record the declared commit, permitted dirty-state exceptions, build location/receipt, fixture URL, browser/client, viewport and process/browser ownership. Compare `git status --short`, `git branch --show-current` and `git rev-parse HEAD` with that declaration. Moving source supports preparation, not delivery observation. Stop affected observations if the target changes.

Public-safe fixtures need no account/backend. Existing dependencies are a prerequisite; this recipe grants no installation, browser control or execution beyond the current commission.

## Frozen built-library launch and health

From the monorepo root, with Node 26+, pnpm 12 and the existing task Chromium cache, run the finished repository harness after execution is authorized:

```sh
PLAYWRIGHT_BROWSERS_PATH=/private/tmp/2026-10-demistify-browser-tests/chromium pnpm test:browser
```

For an explicitly scoped compact-headings correction, run only its permanent case:

```sh
PLAYWRIGHT_BROWSERS_PATH=/private/tmp/2026-10-demistify-browser-tests/chromium pnpm --filter @sunstack/demistify-poc exec vitest run --config vitest.browser.config.ts tests/browser/document.browser.test.ts -t 'compact headings menu'
```

Report selected tests separately from name-filtered skips and uncollected files. A focused pass is not a full-suite pass.

The root/package scripts route this to `vitest run --config vitest.browser.config.ts`. The existing Vitest Playwright provider launches isolated headless Chromium; no desktop tab is used. Its Vite plugin snapshots the complete `packages/demistify/dist` and authored fixtures, then serves those bytes from a fresh loopback port. `DEMISTIFY_BROWSER_ASSETS` optionally selects a declared complete library copy; record any override. Keep the same existing browser cache. Installations require a separate grant. See the [harness README](../../../apps/demistify-poc/tests/browser/README.md).

Keep stdout/stderr, exit code, printed snapshot directory and API URL. Compare the snapshot's asset hashes with the declared library. Vite writes generated config/cache files under `node_modules/.vite-temp`; if sandbox permissions prevent launch, preserve that startup failure and request the exact access for the same command. It is not a product test failure. Never rebuild a frozen target.

**Readiness/health:** `openFixture` polls for the real document, upgraded navigation controls and stylesheet, then awaits fonts. Each behavior test waits for its actual heading/table/SVG or inspection result. These checks, together with snapshot identity, establish a usable real target; a listening port alone does not. Preserve failed assertions and request/browser errors emitted by the suite. Successful tests do not establish a comprehensive console/network audit.

For separately authorized PoC preview work, root `pnpm build` refreshes the library and copied static host; `pnpm --filter @sunstack/demistify-poc preview --port 4174` serves it on loopback. This is a distinct delivery path from the harness snapshot. The owner desktop surface needs its own release. Do not run these commands during a source/build freeze.

## Drive and reuse

Inspect the finished browser configuration and assertions before reuse. The separate `apps/demistify-poc/vitest.config.ts` uses Node; the package `test` command is not browser evidence. The Implementer owns product tests. The eleven browser scenarios cover selected real interactions; their exact assertion limits are in the journey reference.

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

The representative pass uses the complete existing suite: real frame launch/readiness, navigation selection and cold fragment cases, external/embedded rendered text/SVG, exact-source inspection with original heading/table retained, Close, then automatic teardown. Cases use fresh frames rather than sharing one continuous user session. A pass covers only the assertions actually exercised.

## Evidence, ownership and cleanup

Use a separate run directory under ignored `tmp/verification/demistify/`. Record target/build/resource and fixture/source identities, route, viewport, actions, before/after observations, actual rectangles/scroll offsets, screenshots, console/network failures, CLI output and exit codes. Report test pass/fail/skip counts only from actual runs; preserve failures alongside later success. Keep captured content public-safe.

For each journey report **supported**, **unsupported** or **unknown**, evidence paths and undriven entry points. Source checks, suites, browser behavior, file-protocol operation, hosting and release are separate claims. Name missing sensitivity if a known failure has not shown the observation can reject it. Measure actual geometry instead of copying production formulas. Hidden overflow that clips content fails even when document width looks contained.

The Verifier owns this recipe and assigned apparatus. Product failures/test gaps go to the Implementer; unsettled expectations/targets go to the Lead. Run history belongs in the authorized trace/evidence. This recipe grants no product/test edits, public uploads, release or acceptance authority.

The non-watch Vitest command exits and tears down its server/browser. Confirm the recorded API port has no listener; preserve/reopen evidence afterward. Keep the immutable temp snapshot if it is still needed as input evidence. Stop only a hung process/session this run started, by recorded identity. Leave shared instances/tabs and others' files alone. Directory removal needs explicit repository permission.

Report the exact build and exercised scope on every use. Other journey variants stay unknown until adequate applicable evidence exists. Embedding/HTTP rendering alone does not establish offline/file viewing. Normal main-diagram scroll input, precise pointer anchoring, drawer resize and narrow-navigation interactions beyond the scoped 390px hover/click/Recovery case need separate proof.
