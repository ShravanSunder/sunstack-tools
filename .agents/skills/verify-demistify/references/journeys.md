# Demistify journeys

The [owner task list](../../../../docs/wip/demystify/2026-10-16-fixes-todo.md) supplies the expected outcomes. The adopted clarification is: sidebar/source inspection retains main content; fullscreen may move live content and must restore the same nodes/state on Close/Escape. Source pointers locate entry paths, not observed results. Recheck them at the pinned build.

## Fixtures and prerequisites

- **Navigation/Markdown:** `/experiments/agent-written/navigation-markdown-fix/index.html`, sibling `details.html` and `notes.md`; [authored fixture directory](../../../../apps/demistify-poc/experiments/agent-written/navigation-markdown-fix/). It uses `/experiments/islands/runtime.js` and `theme.css`. The PoC builder copies these authored files and the built library into static output.
- **Layout/tree:** [layout-regression-fix/index.html](../../../../apps/demistify-poc/experiments/agent-written/layout-regression-fix/index.html) supplies `#wide-diagram`, `#narrow-diagram`, `#full-width-diagram`, `#tight-labels`, `#loose-labels`, `#map-labels`, generic SVG `#gesture-canvas` and actual picture `#inspection-picture`. Its final authored links use `/experiments/islands/` correctly.
- **Headless routes:** `openFixture` uses `/__demistify/navigation/index.html`, `/__demistify/layout/index.html` or `/__demistify/embedded/index.html` in fresh same-origin frames. The harness rewrites library URLs to `/__demistify/library/`. Its embedded variant is produced by the real `embedMarkdownFiles` owner and has no sibling `notes.md`.
- **Additional variants:** pre-rendered SVG, inline source inspection, genuine failure/loading and broader visual classes still need applicable finished fixtures/proof. Keep absent cases unknown; do not invent routes.

Use one browser surface. Useful width cases include 1600, 1440, 1439, 900, 899 and 390 CSS pixels; record actual dimensions. Include resizing while inspection is already open. Current navigation/inspection thresholds are 1440/900px; expected usability does not depend on preserving those implementation constants.

## Presentations, pages and fragments

1. Open the navigation fixture without a fragment. Expect Architecture selected, with Implementation plan and Review absent from stacked main content. Open **Browse presentations**, inspect all titles/descriptions and the separate-page destination, and select each local presentation. Check visible selection and menu dismissal, not only URL mutation.
2. At wide width inspect **Page navigation**: selected presentation headings under **On this page**, page links under **Pages**, presentation choices in their menu, current page and breadcrumbs. Hidden presentations/tabs must not pollute active headings. At narrow width open **Browse page headings** by pointer and keyboard, then activate a link. The permanent compact case uses 390×900px: hover the trigger, require the panel visible, click the same trigger and require it still visible; confirm the Implementation detail page link is available; click Recovery and require the panel closed with the Markdown target inside the viewport. This distinguishes hover-only success from the original click-dismissal failure.
3. From Architecture activate **Open the implementation diagram directly** (`#plan-diagram`). Expect Implementation plan and its Diagram tab, with actual visible SVG. An ID in hidden DOM does not pass.
4. Navigate cold to the same built page with `#plan-diagram`. Independently navigate cold to `#notes--recovery`; expect Architecture, the external Markdown Recovery heading in view and the relevant outline entry. This catches initial-fragment/async-loading behavior that same-page clicks miss.
5. Follow Implementation detail, then its link back to Recovery. Verify relative page URL and the correct presentation/heading; exercise back/forward through the selection transitions under test. A Markdown heading and Mermaid component are separate cases.

Keep URL, selected presentation/tab, visible target and active headings for each entry. A source-load failure remains open when a different static link works.

Source path: registration in [`runtime.ts`](../../../../packages/demistify/src/runtime.ts); menu/selection in [`presentations.ts`](../../../../packages/demistify/src/layout/presentations.ts); hash/click/`island-ready` callbacks → `revealElement` in [`document.ts`](../../../../packages/demistify/src/layout/document.ts); tab state in [`tabs.ts`](../../../../packages/demistify/src/islands/tabs.ts); IDs in [`heading-fragments.ts`](../../../../packages/demistify/src/core/heading-fragments.ts); grouped links/hidden-heading filtering in [`navigation.ts`](../../../../packages/demistify/src/layout/navigation.ts). Source alone does not establish runtime timing; the cold-fragment assertions below cover their declared entry cases.

## Markdown, Mermaid and exact source

1. Let `#notes` finish the real `notes.md` fetch. Observe Session ownership, Ownership, Recovery, the table, literal `<registry>` prose, local heading link and nonempty Mermaid SVG. Check the request resolves from the HTML page URL. Mocked content or an inert island tag does not establish this pipeline.
2. Open **Show Markdown source** within `#notes`. Compare panel code `textContent` against the actual decoded source file, including whitespace, line breaks and fenced Mermaid; do not use product-generated metadata as the expected source. Confirm the reference is shown and original main text/table/SVG remain rendered. At wide width check the copy beside the main view; at narrow width the source may use a sheet, with original content retained behind it. Close with the inspected accessible **Close** control and confirm focus/reachability.
3. Repeat for script-backed `#review-notes`, an unwrapped-inline fixture and the declared embedded/pre-rendered variant. Capture authored inline text before rendering; compare with that input, not reserialized HTML. Distinguish browser HTML line-ending normalization from fetched-file text.
4. Use the finished genuine missing-source fixture. Observe visible loading/busy state while genuinely pending, then the visible failure and absence of a false prepared-success state. If transient loading cannot be captured, leave that part unknown. A mocked fetch/delay does not establish real loading behavior.
5. Compare external-file and inline Mermaid through the shipped renderer. For packaged content require the generated artifact, embedded source and absence of a sibling Markdown fetch; repeat rendered text/source/SVG observations. HTTP success does not establish file-protocol/offline operation.

Source path: fetch/retained record in [`markdown.ts`](../../../../packages/demistify/src/islands/markdown.ts) → shared [`markdown-content.ts`](../../../../packages/demistify/src/render/markdown-content.ts) → upgraded [`mermaid.ts`](../../../../packages/demistify/src/islands/mermaid.ts) → [`diagram-client.ts`](../../../../packages/demistify/src/render/diagram-client.ts) worker or real fallback → visible SVG. Markdown source inspection creates an independent literal `pre` view and calls the existing inspector. [`markdown-source.ts`](../../../../packages/demistify/src/core/markdown-source.ts), [`source-text.ts`](../../../../packages/demistify/src/core/source-text.ts), [`markdown-files.ts`](../../../../packages/demistify/src/node/markdown-files.ts) and [`packaging.ts`](../../../../packages/demistify/src/node/packaging.ts) retain source/reference across the boundary. Fetch/worker acknowledgement is not the rendered outcome.

## Diagram layout and inspection

Require the working wide-comparison fixture with its real columns/wrappers and fitting small diagram. SVG width alone does not recreate parent sizing pressure. Retain an observed pre-fix failure when available; otherwise reproduction sensitivity is unknown.

1. At pinned-navigation width record viewport/document/body/main, navigation, island/SVG and control rectangles; relevant nodes/arrows; page/local offsets and client/scroll widths. Main content stays between navigation and viewport/drawer boundaries. Inspect visible content as well as bounds: hidden overflow can mask loss.
2. By normal UI scroll the wide diagram to left, middle, rightmost and back. Confirm local offset changes while page horizontal position stays fixed, endpoint labels/arrowheads remain readable/reachable and connections stay attached. Corroborate screenshots with actual node/edge geometry; do not copy production sizing formulas. Programmatic `scrollLeft` changes alone leave normal UI scrolling unproved.
3. Hover/focus and operate **Open Wide comparison in a sidebar** at both scroll ends. Controls remain attached/reachable. Check actual original text/SVG, not merely an empty owner or reserved dimensions. Resize via **Resize panel**, scroll panel/main content and close; no outer overflow, displacement or blank main island may remain.
4. Use **Expand Wide comparison**, including expansion from the drawer. Record original node identity, state and geometry before opening. Fullscreen may temporarily move content. Inspect, resize, then close through Close and Escape in separate transitions; require the same live nodes/state, dimensions, controls and focus restored. Do not require main content visible behind fullscreen.
5. Narrow/widen across navigation, inspection and tab/column transitions with inspection open and closed; repeat deciding scroll/control checks. Cover runtime and pre-rendered Mermaid, plus a Markdown-fenced diagram where scoped. The fitting small diagram may remain centered without scrolling.

Source owners: prepared/runtime sizing in [`mermaid.ts`](../../../../packages/demistify/src/islands/mermaid.ts); SVG/scroll/control layout in [`components.css`](../../../../packages/demistify/src/styles/components.css); body reservation in [`navigation.css`](../../../../packages/demistify/src/styles/navigation.css) and [`vocabulary.css`](../../../../packages/demistify/src/styles/vocabulary.css); copy/move/restore in [`inspector.ts`](../../../../packages/demistify/src/islands/inspector.ts), [`inspection-copy.ts`](../../../../packages/demistify/src/islands/inspection-copy.ts), [`drawer.ts`](../../../../packages/demistify/src/islands/drawer.ts). Source alone proves no geometry.

## Tree wrapping

Inspect `#tight-labels`, `#loose-labels` and `#map-labels` at narrow/wide widths: short/long, nested, inline-code/emphasis/link, loose paragraphs and long-root labels. Each tone icon stays beside the first line; wrapped lines align beneath label text rather than the icon. Preserve inline flow, paragraph spacing and readable text. Collapse/reopen a branch by pointer and keyboard and confirm descendants remain reachable. Capture close-up screenshots plus actual icon/text bounds; CSS strings or wrapper presence cannot prove wrapping.

Source path: [`TreeIsland.render`/`convertList`/`bindControls`](../../../../packages/demistify/src/islands/tree.ts) parses Markdown, separates label/children, adds icons and toggles branches. `.ex-tree-node`, `.ex-tree-label` and map rules in [`vocabulary.css`](../../../../packages/demistify/src/styles/vocabulary.css) own layout. Prepared real rows signal readiness, not a fixed sleep.

## Visual inspection gestures

Require real inspected pictures, Mermaid and each other claimed visual class, with enough overflow to pan after zoom; cover sidebar and fullscreen. Missing visual variants remain gaps.

1. Over a recognizable off-center feature, perform **Cmd+wheel**. Observe magnification with that feature staying at the pointer, subject to edge clamping. A percentage label cannot prove anchoring; capture feature/pointer positions and rendered result.
2. At a pannable zoom, **Cmd+drag**; observe visible pan without zoom/outer-page displacement. Release and perform another normal action to catch stuck capture; include release/cancel at the viewport edge where the route permits it.
3. Wheel without Cmd and confirm ordinary scrolling without magnification. Operate **Zoom in**, **Zoom out**, **Reset zoom** by keyboard/pointer. Reset restores the centered view, not only `100%`. Close/reopen and confirm original content and normal scrolling remain intact.

Use genuine modifier/pointer input through the authorized route. An inability to deliver/observe it is a driver capability gap; synthetic DOM events do not close the claim. Source path: inspector scale/buttons in [`inspector.ts`](../../../../packages/demistify/src/islands/inspector.ts) → wheel/pointer registration and viewport scroll in [`inspection-gestures.ts`](../../../../packages/demistify/src/islands/inspection-gestures.ts), styled by [`inspection-gestures.css`](../../../../packages/demistify/src/styles/inspection-gestures.css). The native assertion coverage below establishes only the exercised gestures, not every expected result above.

## CLI checking and embedding

After target/execution authorization, run from the repository root. Commands come from [`bin/demistify.ts`](../../../../packages/demistify/bin/demistify.ts). Use a unique evidence directory/output for each run:

```sh
node packages/demistify/bin/demistify.ts check apps/demistify-poc/experiments/agent-written/navigation-markdown-fix/index.html apps/demistify-poc/experiments/agent-written/navigation-markdown-fix/details.html
```

Keep diagnostics/exit code. A valid fixture should exit 0; findings remain findings rather than a reason to disable checks. The CLI resolves referenced Markdown relative to the input page. The missing-source fixture must demonstrate `markdown-source` plus nonzero exit; that fixture is currently a setup gap. Use checking without `--fix` during observation.

Create the allowed ignored evidence directory and ensure the following output does not already belong to another run:

```sh
node packages/demistify/bin/demistify.ts embed apps/demistify-poc/experiments/agent-written/navigation-markdown-fix/index.html tmp/verification/demistify/embedded.html
node packages/demistify/bin/demistify.ts check tmp/verification/demistify/embedded.html
```

Expect local Markdown embedded successfully. Inspect retained text/reference against `notes.md`, removed external Markdown `src` and retained library links. This proves transformation only. Output is outside the PoC preview root; browser proof needs the approved serving/packaging route. The command does not bundle runtime assets or establish no-server viewing. Actual standalone packaging uses [`bundleStandaloneHtml`](../../../../packages/demistify/src/node/standalone.ts); its published harness/artifact and file-protocol observation are separate prerequisites.

## Finished harness assertion coverage

[`vitest.browser.config.ts`](../../../../apps/demistify-poc/vitest.browser.config.ts), [`built-fixtures.ts`](../../../../apps/demistify-poc/tests/browser/built-fixtures.ts) and [`fixture-page.ts`](../../../../apps/demistify-poc/tests/browser/fixture-page.ts) own launch, byte serving and readiness. The config runs the existing headless provider; no desktop control or ad hoc UI driver is needed. Record the snapshot and any assets override. The browser suite does not rebuild assets.

| Source | Assertion scope | Still outside these assertions |
| --- | --- | --- |
| [document cases](../../../../apps/demistify-poc/tests/browser/document.browser.test.ts) | External/embedded prose, table, literal text and real SVG; exact file source with heading/table retained; menu descriptions, selected-only content and outline; cold heading/component fragments; sidebar/main SVG and fullscreen content/focus after Escape/Close; generic SVG modifier input; Mermaid native-hover connector colour/width and node-size stability in two distinct themes plus pointer-exit clearing; 390px compact-menu hover then pointer click, available page link and Recovery heading activation | Inline source forms, source-panel SVG retention, transient loading/error UI, full relative-page/back-forward behavior and narrow-menu inputs/transitions beyond the compact case; same-node/state fullscreen restoration beyond checked text/focus |
| [layout cases](../../../../apps/demistify-poc/tests/browser/layout.browser.test.ts) | Endpoint geometry/control viewport bounds and document overflow at 1600/390px; selected tree first-line/code/paragraph geometry and pointer folding | Normal main-diagram scrolling input (offsets are assigned), measured arrow/node alignment, drawer resizing/open-resize transitions, narrow/map tree and keyboard coverage |
| [native gesture cases](../../../../apps/demistify-poc/tests/browser/native-gestures.browser.test.ts) | Actual picture/Mermaid magnification, ordinary wheel without zoom, native Cmd-drag on an available axis, reset size/origin and retained originals | Precise pointer anchoring, Ctrl-specific input, fullscreen gestures, comprehensive centered reset and other visual classes |

The source close control is **Close**, scoped to the source panel. Native pointer drag is implemented by the existing [provider command](../../../../apps/demistify-poc/tests/browser/gesture-commands.ts); it does not dispatch synthetic DOM events. The suite has twelve cases across three files. A successful run supports only the scope above. Its 390px diagram case alone says nothing about narrow-menu activation; the separate compact case covers only the hover/click/Recovery path described above, not an actual relative-page hop. Existing post-fix sensitivity is documented in the harness README; do not repeat mutations just to establish independence, and do not treat them as pre-fix TDD. Keep historical ten-case results bound to their original commit/assets rather than relabeling them as eleven-case results.

Return each journey's target, entries driven, actions, observed consequence, evidence path and supported/unsupported/unknown result. Assign fixture/setup, driver, product or disputed-expectation gaps only when evidence supports that kind. Preserve failures and route them to Implementer/Lead. Operable apparatus and green focused assertions do not establish delivery acceptance or the unobserved variants.

The hover regression in the document cases uses real rendered edges and native pointer input. It compares the highlighted connector colour to the actual page accent, width to its resting value and node dimensions before/after; the theme button must change that expected colour. It does not measure rasterized arrowhead polygons or claim keyboard tracing coverage. Its Lead-owned red/green evidence is in the fixes proof; earlier independent Verifier observations remain bound to their original targets.
