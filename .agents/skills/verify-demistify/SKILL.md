---
name: verify-demistify
description: Use when checking a declared Demistify built runtime or Markdown CLI, including choices/comments, navigation, Markdown/diagrams or small-parts behavior, or maintaining this project's verification recipe. Not for product repairs or delivery acceptance.
---

# Verify Demistify

Verify `packages/demistify` through its plain-HTML consumer, `apps/demistify-poc`. Keep the real built library, Markdown files, native custom elements, diagram renderer and browser layout in the path. CLI checking and embedding prove different claims from browser behavior.

**Operable for the existing focused headless browser and CLI routes below.** The choices/comments journey remains a draft wherever its declared-target observations are incomplete; distinguish applicable feature/boundary assertions, component integration and native host observations. Broader journey variants remain explicitly unobserved. Record the actual target and coverage in the task's evidence/trace; a working recipe is not product acceptance or proof of cold-runner skill application.

## Target and contract

Read the [owner task list](../../../docs/wip/demystify/2026-10-16-fixes-todo.md), [project instructions](../../../AGENTS.md) and [handoff](../../../docs/wip/demystify/2026-10-14-demistify-handoff.md). Their outcomes and the current commission are the oracle. Sidebar/source inspection retains rendered main content. Fullscreen may temporarily move live content, but Close/Escape must restore the same nodes and state.

Record the declared commit, permitted dirty-state exceptions, build location/receipt, fixture URL, browser/client, viewport and process/browser ownership. Compare `git status --short`, `git branch --show-current` and `git rev-parse HEAD` with that declaration. Moving source supports preparation, not delivery observation. Stop affected observations if the target changes.

Public-safe fixtures need no account/backend. Existing dependencies are a prerequisite; this recipe grants no installation, browser control or execution beyond the current commission.

The small-parts and fresh-Luna journeys below describe native coverage for the adopted 22 October 2026 small-parts contract (U1–U4/R1–R7). Require its exact source revisions, matching candidate guidance and frozen complete build from the commission. The permanent small-parts fixture and browser test exist in the repository. Their automated assertions and the native fixture, literal-example and fresh-author observations need their own target-bound evidence.

## Frozen built-library launch and health

From the monorepo root, with Node 26+, pnpm 12 and the installed Chromium matching the checkout's Playwright dependency, run the existing repository harness after execution is authorized:

```sh
pnpm test:browser
```

For an explicitly scoped compact-headings correction, run only its permanent case:

```sh
pnpm --filter @sunstack/demistify-poc exec vitest run --config vitest.browser.config.ts tests/browser/document.browser.test.ts -t 'compact headings menu'
```

Report selected tests separately from name-filtered skips and uncollected files. A focused pass is not a full-suite pass.

The root/package scripts route this to `vitest run --config vitest.browser.config.ts`. The existing Vitest Playwright provider launches isolated headless Chromium; no desktop tab is used. With no `PLAYWRIGHT_BROWSERS_PATH` override, use the matching installed default executable. Before launch, resolve its path through the PoC's installed dependency and confirm it exists:

```sh
node --input-type=module -e 'import {createRequire} from "node:module"; import {resolve} from "node:path"; import {existsSync} from "node:fs"; const requireFromPoc=createRequire(resolve("apps/demistify-poc/package.json")); const executable=requireFromPoc("playwright").chromium.executablePath(); const exists=existsSync(executable); console.log(JSON.stringify({executable,exists,browserPathOverride:process.env.PLAYWRIGHT_BROWSERS_PATH??null},null,2)); process.exitCode=exists?0:1;'
```

Retain the actual executable/override receipt. An explicit cache is valid only when selected by the commission and this same check confirms its executable exists; preserve that override for the run. A missing executable is a setup gap: never install, change caches or fall back silently. Its Vite plugin (`tests/browser/built-fixtures.ts`) snapshots the complete `packages/demistify/dist`, the authored fixtures and the built consumer page `apps/demistify-poc/dist/experiments/html-islands-content-surfaces-live.html`, rewrites that page's asset links to the snapshot library, then serves those bytes from a fresh loopback port. The declared target therefore needs both outputs of one root `pnpm build` from the same source: a library-only build fails the whole suite at startup, and a stale PoC build silently serves an older page. Record that page's SHA-256 beside the library hashes. `DEMISTIFY_BROWSER_ASSETS` optionally selects a declared complete library copy and replaces library bytes only; the consumer page still comes from `apps/demistify-poc/dist`. Record any override. Installations require a separate grant. See the [harness README](../../../apps/demistify-poc/tests/browser/README.md) for harness behavior; its installation example grants no install and does not select this run's cache.

Keep stdout/stderr, exit code, printed snapshot directory and API URL. Compare the snapshot's library hashes with the declared library, and the built page's hash with the declared build. Vite writes generated config/cache files under `node_modules/.vite-temp`; if sandbox permissions prevent launch, preserve that startup failure and request the exact access for the same command. It is not a product test failure. Never rebuild a frozen target.

**Readiness/health:** `openFixture` polls for the real document, upgraded navigation controls and stylesheet, then awaits fonts. Each behavior test waits for its actual heading/table/SVG or inspection result. These checks, together with snapshot identity, establish a usable real target; a listening port alone does not. Preserve failed assertions and request/browser errors emitted by the suite. Successful tests do not establish a comprehensive console/network audit.

For separately authorized PoC preview work, root `pnpm build` refreshes the library and copied static host; `pnpm --filter @sunstack/demistify-poc preview --port 4174` serves it on loopback. This is a distinct delivery path from the harness snapshot. The owner desktop surface needs its own release. Do not run these commands during a source/build freeze.

## Drive and reuse

Inspect the finished browser configuration and assertions before reuse. The separate `apps/demistify-poc/vitest.config.ts` uses Node; the package `test` command is not browser evidence. The Implementer owns product tests. Report the current selected browser cases from the actual run and read their assertion limits at source; historical case counts in a journey are not the current suite inventory.

For direct UI observation, use host computer-use/Chrome on one released surface with sole control. Use ordinary links, tabs, buttons, wheel and pointer input. Read-only DOM geometry corroborates visible outcomes. Internal setters, synthetic events and mocked rendering/fetch do not establish user interaction. An adopted component-integration case may call an existing action to decide its lifecycle boundary; label that result separately from native input, as the choices/comments reference requires for controls behind a phone modal. Do not introduce a separate Playwright driver.

Inspect a passing journey's target, fixtures, interactions, assertions and result before reuse. Reuse applicable evidence. Add direct observation for a missing interaction, changed dependency or specific unresolved doubt; independence does not require repeating equivalent proof.

## Journeys

Read [journey instructions and source map](references/journeys.md) for the selected scope.

IF the claim includes choices or comments, load `references/choices-comments.md` and return the declared target, selected O1/O2 cases, deciding observations with evidence paths, and every unobserved case/gap. Use that return in the evidence/limits report below; selecting cases without observing them is not coverage.

| Journey                          | Deciding observation                                                                                                                                                                                                                   |
| -------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Presentations/pages/fragments    | Cold and same-page links reveal the correct presentation/tab; selected-only content and grouped navigation remain useful                                                                                                               |
| Markdown/Mermaid/source          | Referenced text and real SVG render; source text/reference stay exact while main content stays rendered                                                                                                                                |
| Diagram layout/inspection        | Local scrolling reaches both ends without page overflow, displacement, lost controls or arrow/node drift; fullscreen restores state                                                                                                    |
| Tree labels                      | Multiline, inline-code and loose-paragraph labels wrap beneath text with the icon on the first line                                                                                                                                    |
| Visual gestures                  | Cmd+wheel anchors zoom at the pointer; Cmd+drag pans; normal scroll, buttons, reset and restoration remain usable                                                                                                                      |
| CLI checking/embedding           | Local sources check/embed and missing sources fail; transformation stays distinct from runtime/offline proof                                                                                                                           |
| Small parts in cells/prose/Trace | Exact numeric states, stable phase slots, both themes and desktop/390px containment; permanent `review-ui/small-parts.html` and `tests/browser/small-parts.browser.test.ts` exist in the existing harness                              |
| Frozen fresh-Luna artifact       | Supported native action capture establishes the loaded candidate copies; matching CLI and the actual frozen HTML establish only the named case's factual/composition behavior                                                          |
| Choices and comments             | Native cards/table/inline details, exact identified feedback, same-revision reload and actual Copy/Export; shared panels, reconnect and read-only-copy isolation through the existing frozen harness plus assigned native observations |

The representative pass uses the complete existing suite: real frame launch/readiness, navigation selection and cold fragment cases, external/embedded rendered text/SVG, exact-source inspection with original heading/table retained, Close, then automatic teardown. Cases use fresh frames rather than sharing one continuous user session. A pass covers only the assertions actually exercised.

## Evidence, ownership and cleanup

Use a separate run directory under ignored `tmp/verification/demistify/`. Record target/build/resource and fixture/source identities, route, viewport, actions, before/after observations, actual rectangles/scroll offsets, screenshots, console/network failures, CLI output and exit codes. Report test pass/fail/skip counts only from actual runs; preserve failures alongside later success. Keep captured content public-safe.

For each journey report **supported**, **unsupported** or **unknown**, evidence paths and undriven entry points. Source checks, suites, browser behavior, file-protocol operation, hosting and release are separate claims. Name missing sensitivity if a known failure has not shown the observation can reject it. Measure actual geometry instead of copying production formulas. Hidden overflow that clips content fails even when document width looks contained.

The Verifier owns this recipe and assigned apparatus. Product failures/test gaps go to the Implementer; unsettled expectations/targets go to the Lead. Run history belongs in the authorized trace/evidence. This recipe grants no product/test edits, public uploads, release or acceptance authority.

The non-watch Vitest command exits and tears down its server/browser. Confirm the recorded API port has no listener; preserve/reopen evidence afterward. Keep the immutable temp snapshot if it is still needed as input evidence. Stop only a hung process/session this run started, by recorded identity. Leave shared instances/tabs and others' files alone. Directory removal needs explicit repository permission.

Report the exact build and exercised scope on every use. Other journey variants stay unknown until adequate applicable evidence exists. Embedding/HTTP rendering alone does not establish offline/file viewing. Normal main-diagram scroll input, precise pointer anchoring, drawer resize and narrow-navigation interactions beyond the scoped 390px hover/click/Recovery case need separate proof.
