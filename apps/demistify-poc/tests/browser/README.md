# Demistify browser regressions

Twenty-three Vitest Browser Mode scenarios exercise the built native Web Components in headless Chromium. Each case gets a fresh same-origin document. Vite serves copied runtime, theme, font, chunk and worker files as bytes, so the worker resolves beside the runtime just as it does for a plain HTML consumer. Tests do not import the runtime's TypeScript sources. An SVG outcome proves rendering; this suite does not separately assert that the worker, rather than the supported main-thread fallback, did the render.

Build the library before testing. During concurrent work, use a frozen copy of the complete library dist directory instead of rebuilding another writer's target. `DEMISTIFY_BROWSER_ASSETS` selects that copy; without it, the harness snapshots `packages/demistify/dist`. The harness prints its private snapshot path and chooses a free loopback port. It does not use the PoC preview port.

From the monorepo root:

```sh
pnpm --filter @sunstack/demistify build
PLAYWRIGHT_BROWSERS_PATH=/private/tmp/2026-10-demistify-browser-cache pnpm --filter @sunstack/demistify-poc exec playwright install chromium
PLAYWRIGHT_BROWSERS_PATH=/private/tmp/2026-10-demistify-browser-cache pnpm test:browser
```

Keep the same `PLAYWRIGHT_BROWSERS_PATH` for install and test. This environment variable confines browser downloads to task storage; no desktop profile or global browser configuration is used. The browser provider is exactly 5.0.4 to match Vitest; Playwright is pinned to 1.64.0. There is no separate Playwright test runner. The Node configuration and its include patterns are unchanged: `tests/browser` is outside its existing includes.

| Scenario                       | Forbidden behavior caught                                                                        | Independent expectation                                                                                                               |
| ------------------------------ | ------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------- |
| External and embedded Markdown | File renders as source, prose/table disappears, or lazy diagram never renders                    | Authored heading, literal registry text, table outcome, Client/Registry/Session labels and a nonempty connection                      |
| Exact Markdown source          | Panel truncates or transforms source, or removes the main rendered view                          | Original `notes.md` bytes and visible original heading/table                                                                          |
| Presentations                  | Menu loses descriptions, paths hide existing content, or cold/scoped targets resolve incorrectly | Literal names/descriptions, retained content, qualified Markdown/Mermaid targets and Back/Forward                                     |
| Initial fragments              | Async heading or inactive diagram stays hidden until the reader clicks                           | Direct navigation before runtime startup reveals Recovery or the Diagram tab and real SVG                                             |
| Diagram scrolling              | Left or right endpoint becomes unreachable, controls scroll away, or page overflows              | Visible endpoint bounds within the island at each end; controls within viewport; 1600px and 390px viewports                           |
| Tree labels                    | Long labels drop below icons, inline code splits flow, or loose paragraphs flatten               | First-line icon/text bounds, inline-code bounds, separated paragraphs and operable nested folding                                     |
| Inspection retention           | Sidebar moves the original, or fullscreen loses content/focus on close                           | Both rendered views, visible main island, restored original SVG and invoking-control focus after Escape/Close                         |
| Modifier gestures              | Ordinary wheel zooms, modifier wheel does not enlarge content, or drag does not pan              | Unchanged zoom on ordinary scrolling, larger visual on Cmd-wheel, both scroll axes moving on Cmd-drag and reset restoring 100%/origin |

The compact-navigation case additionally exercises a 390px pointer hover/click, the visible Pages destination and a heading link that dismisses the menu and reveals Recovery. Its final oracle rejects the original frozen runtime at the post-click visibility assertion. The bounded repair associates the trigger with its native popover using the show action.

The two native visual gesture cases use an ordinary figure/image click and the actual rendered `#wide-diagram` sidebar control. They test rendered enlargement, normal wheel without zoom, native modifier pan on an available scrolling axis, reset and retained main content. They add no fixed-size wrapper or product override. The generic SVG case remains a separate both-axis observation.

At the original frozen runtime `7fdfeb4d…` / theme `bf7b030b…`, the native picture case passed and the native Mermaid case exposed a product failure: zoom enlarges the SVG from 419px to 843.75px at 201%, but the scrollable copy is the inner `ex-mermaid` (211px horizontal overflow). Cmd-drag writes the outer inspection viewport, which has no overflow, so the actual scroll offset remains zero. The normal-wheel portion passes; reset/close after that failing pan step remain unobserved. This product failure was returned to the Lead and preserved. The Lead's subsequent theme `d330d90d…` leaves inspected Mermaid overflow to the outer viewport; both native cases then pass on the unchanged runtime, including reset and close. Test code contains no product override.

The embedded fixture is created through the real `embedMarkdownFiles` owner. Its directory contains only HTML, so sibling Markdown requests return 404. Tests verify that no such request occurred. This establishes embedded loading without a sibling file over HTTP; it does not claim file-protocol/offline delivery, CDN/CORS correctness or other engines.

All waits are bounded by observable readiness or assertion polling. Geometry uses browser bounds and broad visibility/alignment tolerances, never production sizing formulas or path snapshots. The one custom browser command performs a native modifier pointer drag because Vitest's `dragAndDrop` API models HTML drag/drop rather than pointer capture. It runs only inside the provider-owned browser and targets the test iframe.

## Post-fix sensitivity

After a clean passing run, this command copies the built assets once, makes a separate private copy for each of eight forbidden behaviors, then runs only that behavior's test. It requires one matching failed assertion and exit 1 per mutation, writes the full outputs in task temporary storage and exits 0 only when all eight were caught. Runtime mutations require one exact owning section match; a build shape change fails setup rather than silently skipping a mutation.

```sh
PLAYWRIGHT_BROWSERS_PATH=/private/tmp/2026-10-demistify-browser-cache pnpm --filter @sunstack/demistify-poc exec node tests/browser/sensitivity.ts
```

`DEMISTIFY_BROWSER_ASSETS` can pin this command to the same frozen build as the clean run. Mutations affect copied runtime/theme bytes only; product sources, shared dist and fixtures are unchanged. Run the clean suite again afterward. This is post-fix sensitivity, not pre-fix TDD evidence. Deliberate broken cases do not establish a previously present product defect.

Run just the required native cases against the selected built assets:

```sh
PLAYWRIGHT_BROWSERS_PATH=/private/tmp/2026-10-demistify-browser-cache pnpm --filter @sunstack/demistify-poc exec vitest run --config vitest.browser.config.ts tests/browser/native-gestures.browser.test.ts
```

The following optional mode makes one private handler-disabled copy and runs only those two native cases. It expects both to fail at the zoom assertion. Against the original theme, the unmodified Mermaid case reached the later pan failure; against the Lead's corrected theme, both unmodified native cases pass. It does not rerun the original eight mutations:

```sh
PLAYWRIGHT_BROWSERS_PATH=/private/tmp/2026-10-demistify-browser-cache pnpm --filter @sunstack/demistify-poc exec node tests/browser/sensitivity.ts --native-gestures
```

## Mermaid hover regression

The node-hover case uses an actual Markdown-fenced Client → Registry → Session SVG. Native hover must keep both incident connectors at the page accent, retain their resting stroke width and node dimensions, and clear tracing on pointer exit. It repeats after the real theme button switches to a distinct second colour. The original runtime fails the visible-stroke check because the SVG declares `--accent: var(--accent)`; its traced stroke resolves to none. The prior hover width also scales default SVG markers. The renderer now inherits the page accent and diagram tracing changes colour without changing connector width; HTML flow overlays retain their existing stroke emphasis. This is a genuine pre-fix failure plus current real-browser proof, not a deliberate mutation.

## Integrated document vocabulary

The full suite now adds two independent galleries, reviewer/grid selection, shared sidebar and modal dismissal,200ms settled filtering with exact image-node/zoom retention, input-safe screenshot/viewport keys, gallery-only narrow containment with no empty outline, tight/loose/nested Markdown tasks, relative images, separate tree go-to, literal ex-pre text and a long scrolling outline. Presentations cross real files, return home and show a missing-stop message. A marker must be visible before its outside-diagram bounds can count as proof. The full explorer case uses49 nodes and350 relationships, native search/history/current/proposed/tracing, arrow-table correspondence and retained evidence tables.

The cold Markdown-fence node case opens the actual scoped Registry fragment before lazy rendering; its positive bounds must be on screen. Assigning the diagram's ID before observing visibility lets the shared resolver reveal it. The original runtime leaves the target unresolved and fails this case.

The earlier optional sensitivity script targets the old frozen contracts and is historical apparatus; its selected-wrapper mutation is superseded by ordered paths. It has not been adapted or claimed as current sensitivity evidence. Current suite coverage comes from the permanent assertions described here, not the earlier mutation run.

## Hover settling

The hover DOM cases call the real Mermaid trace, native arrow installer and Explorer renderer in a browser, controlling only the clock. They require no paint at99ms, the latest target at100ms, the same delay on exit, immediate focus/presentation feedback, and no late mutation of a disconnected picture. These are timing-owner integration checks; the existing built-runtime journeys and native browser observation cover wiring and pointer delivery separately. The old immediate handlers fail the entry assertions.

The Explorer stroke/arrowhead case waits for the settled traced state before reading its stroke. It retains the original geometry and theme assertions.
