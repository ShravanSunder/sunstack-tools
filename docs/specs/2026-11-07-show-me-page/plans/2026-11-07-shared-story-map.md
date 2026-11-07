# Deliver the shared story and Map composition

## Governing basis and delivery context

- Plan path: `docs/specs/2026-11-07-show-me-page/plans/2026-11-07-shared-story-map.md`
- Originating planner: `plan-implementation`
- Supersedes: none
- Planning result: ready
- Governing planning basis: reviewed-four-document-design; [Requirements](../requirements.md), [Specification](../specification.md), [Program Design](../program-design.md), [Verification Design](../verification-design.md).
- Current review/remediation identities: `tmp/show-me-page-review/spec-review-1-f10-closure.md`, `program-review-1-correction-2.md`, `four-document-review-1-correction-2.md`. The Lead read the complete reviews, their correction sources and decisive code anchors. The closures admit planning; none is product acceptance.
- Planned-at branch / HEAD: `feat/demistify-show-me-page`, `26b316c29ed3797c54bd639573f361e54b65207a`; isolated worktree `/Users/shravansunder/dev/sunstack-tools.feat-demistify-show-me-page`.
- Requested terminal: `pr-ready-unmerged` for source implementation. The Lead then carries the separately authorized release and final private evaluation through the existing release workflow; that continuation is not an automatic expansion of the executor's terminal.
- Delivery grouping: `single:shared-story-map`; PR topology: `one-pr`; repository: `ShravanSunder/sunstack-tools` only. No tracker selected; existing work thread/trail remains the tracking home.

The owner commissioned getting the approved page deployed and verified before use, quoted in the 7 November commission named by Requirements. That is the authority for the delivery continuation, distinct from available GitHub/CI access. The generic source-push allowance alone does not authorize deployment. No unrelated PR, application deployment, release tag or public upload of owner content is included.

## Goal and scope

Replace the frozen page's private story/section/Map UI code with the designed native-HTML library composition. Deliver the new public markup, matching checker/vocabulary/guide, tree marker and in-page icon, shipped icon font, sanitized complete example and real proof. Preserve all accepted U1–U6/R1–R12, rich content, states and original identities. No Router reader, story store, framework, backend, generated story meaning or persisted disclosure preference.

The library owns rendering/styles/interactions. The app owns examples and public-safe content. The private frozen page and full source-derived input manifest remain private working artifacts; neither enters the product PR. Do not widen the neutral surface token to existing controls or readers.

## Current evidence and prerequisites

- Grounded source: native Trace enhancement, shared head/icons, tree conversion, singular `openDrawer`, independent bottom workspaces, document fragment resolution, original review capture, vocabulary/checker and font packaging. Program Design records each current→target delta.
- Complete unchanged base build succeeded with `pnpm build`. Frozen assets/consumer and build receipt: `/private/tmp/2026-11-show-me-base-26b316c`. Baseline runtime/theme and source fixture identities are in that receipt; do not rebuild that frozen copy.
- R10 before-state: `tmp/verification/demistify/show-me-baseline/receipt.json`, twelve captures at 1280/390 × light/dark × trace/summary/card, from the frozen build. Compare candidate against these exact existing fixture targets. The parent-path grant failure is preserved; the subsequent actual host grant and one successful capture are separate evidence.
- Node 26.13.0, pnpm 12.11.2, OXC/TypeScript versions and frozen dependency installation checked. Default matching Playwright Chromium exists. Existing isolated native Chrome/CDP route ran under exact host grants; the final product native input remains to be exercised.
- GitHub auth/read route, CI environment secret names and main-only deployment policy are present. The current hosted v1 manifest is 1.19.0 at source `e8c6a13a`; source HEAD is not a released binding. Actual deployment authentication is tested only by the protected release job, not by listing secret names. No token is copied locally.
- Existing release-version PR #132 is outside this feature's source diff. Do not merge it as a shortcut. Reconcile the feature's changeset with the normal Changesets version PR after source integration.
- Existing dependencies in earlier worktrees and all baseline/capture folders are retained while used by proof. Free disk was 75GiB, enough for the scoped build and snapshots. Do not remove another task's directories.

## Execution and proof order

`S1 → S2 → S3 → S4 → independent implementation review → source PR → authorized release → S5`. Slices are serial because they share vocabulary, runtime registration, built assets and one controlled composition. One source PR is coherent: all controls serve one reader composition, with one guide and no useful separately released fragment. No parallel writers share these files.

### S1 — Tree state, local-link identity and shipped glyphs

**Reads:** Specification R6–R8; Program Design tree/document-link contracts, icon packaging edge and call delta; Verification O3/O4; current tree, document/target resolver, icons and packaging.

**Write surfaces:** `src/core/document-links.ts` and focused tests; tree prefix parsing helper if needed and its focused table tests; `src/islands/tree.ts`; `src/node/packaging.ts`; tree guide; permanent existing tree browser case/fixture. All paths below are under `packages/demistify` unless the app prefix is shown.

Add shared resolved-URL classification with its closed outcomes. Consume unformatted branch-only `folded:` after native checklist and before tone; retain leaf/code/formatted literals and unmarked default. Keep fold/navigation independent. Add the exact jump and bulk-pair names to the existing packaged font subset. No global subject-icon expansion or unrelated icon system.

**Pre-change signal / oracle:** existing runtime always opens branches and uses arrow-up-right; new literal combinations and same-document URL cases must distinguish it. Expectations come from C7/C8 and the owner's exact bulk pair, not production functions. Use tables of the classification/prefix rules, plus real parsed Markdown DOM for combinations; a pure string test alone cannot earn R8.

**Checks:** focused Node cases through `pnpm --filter @sunstack/demistify-poc exec vitest run` with the new package test paths; built-runtime tree cases through the existing `test:browser` configuration. Run scoped lint/format/types. Build with `pnpm build` before browser proof. Inspect actual subset CSS/font and rendered glyphs, not only class names.

**Earns:** U4's R7/R8 through O4 on the built candidate; the final hosted glyph part remains S5. Stop for any need to change R8 syntax or native navigation expectations.

### S2 — One complete story and section through native state

**Reads:** Specification E2/E3/E4/E6 and R1–R4/R9; Program Design binding, markup, membership and styling contracts; Verification O1/O2/O5 and baseline.

**Write surfaces:** new story/section/fold-toggle islands, shared story DOM membership and disclosure-scope helpers, their styles, vocabulary and registration; `src/node/story-markup.ts` plus focused checker cases; `src/node/check.ts` delegation; app sanitized `experiments/agent-written/show-me-work/` fixture and permanent `tests/browser/show-me-work.browser.test.ts` plus fixture wiring.

Implement story native-details enhancement, compact rich children/metadata, optional attachments, explicit carried states, section split counts/sticky head and structurally scoped controls. A focused fixture may omit Map until S3; do not claim Map totals yet. Use the shared document-card token only on the new surfaces. Keep original live nodes and original-source capture. Define named child contracts once in vocabulary; both guards and checker consume those definitions. `ex-section` becomes supported and its obsolete unknown-element suggestion is retired. Move new component-specific checker work out of the already large check owner instead of growing it past 900 lines.

**Pre-change signal / oracle:** unsupported markup/checker rejects the valid case at base; first complete composition must show prescribed native state and all supplied text. Malformed cardinality/identity must produce exact error classes. Per-fold readback must reject parent-only expansion, a cached action, skipped entry or altered excluded note. Expected membership is authored independently of the collector.

**Checks:** new focused checker tests and permanent native disclosure cases, actual table/diagram attachments, all optional/absent controls. Run `pnpm check`, focused Node suite, build and new browser file. Compare existing Trace/card against frozen R10 baseline before advancing; never repair a new-style regression by changing its expected baseline. A body wrapper change that retains behavior is allowed.

**Earns:** local U1 and U2's story/bulk portions, O1/O2, R9 structure. Map-total and full four-section integration wait for S3. Stop for any ownership/identity/state-policy change outside Program Design.

### S3 — Map, Comments and ordinary navigation integrated

**Reads:** Specification R3/R5/R6/R10; Program Design Map counts, non-modal presentation, setModalControlHost ordering, control host and fragment path; Verification O2/O3/O5 and paired feasibility report.

**Write surfaces:** Map island, shared section-membership read, shared reader-control host, existing drawer/comments/navigation/document/target-resolution integration; shared styles and runtime/vocabulary; Map-related checker validation; complete four-section app fixture and the permanent browser cases.

Use the existing sidebar owner and closed presentation variants. Map always uses its non-modal aside; standard narrow Comments/inspection keep modal defaults. Rehome the same Map trigger using the synchronous before-hide/after-show edge at open/close/replacement/resize. Derive Map section totals from the same section membership as headers; Needs you is For-you's linked branch. No extra fifth branch, copied tree, remembered count or separate sidebar manager. Share ordinary link resolution for encoded/local/repeated/history cases and open story-local target disclosures. Preserve target/download/modifier and nonlocal links.

**Pre-change signal / oracle:** baseline has no Map integration. Native pointer/keyboard both-order replacement and per-member identity/focus are the oracle; a programmatic click behind a modal cannot pass. Include live count changes, Map-branch exclusion, missing/malformed and competing fragment cases, 899/900 resize, normal/reduced motion, text selection and visible focus.

**Checks:** new permanent integration browser file plus kept `document.browser.test.ts`, `drawer.browser.test.ts`, `review-ui.browser.test.ts` and `explanation.browser.test.ts` affected cases. `pnpm check`, focused Node tests, `pnpm build`, then actual full `pnpm test:browser` on one frozen complete build. Run `pnpm --filter @sunstack/demistify-poc exec vitest run` for Node consumers. Keep actual commands, counts/exits and failures.

At this first complete interaction gate, freeze source/build/fixture identity and commission paired Terra's O1/O2/O3/O5 native observation at 1280/420/390, light/dark and reduced motion, using existing recipe and owned browser only. Root owns product fixtures/tests; Terra owns assigned journey apparatus and `.agents/skills/verify-demistify` updates. Explicitly grant only that recipe's files when commissioning maintenance. Native input and read-only DOM geometry remain separate from implementation-owned integration setters. A missing driver becomes an assigned apparatus task, not a weaker outcome.

**Earns:** all local U1–U4 and R10 interaction/layout obligations; no released-use claim. This is the sole pre-guide independent product batch boundary. Fix findings against unchanged authority and rerun only affected proof before proceeding.

### S4 — Matching guide, independent author and source PR

**Reads:** Specification R9/R11/R12; Program Design authoring/vocabulary and trust boundaries; Verification O1/O6/O7, all U5/U6 fields; existing packaged guide and references.

**Write surfaces:** `packages/demistify/skill/SKILL.md`, new `references/stories.md`, relevant trees/navigation/inspection reference links, public-safe complete app example, focused packaging/checker cases, one minor `.changeset` for `@sunstack/demistify`. The workflow skills in another repository are outside scope.

Document the complete story/section/Map recipe and exact child/default/count/marker semantics with rich HTML. Guide and generated editor metadata match actual code. Use skill-creation for the named component guide from this accepted design; no independent workflow-policy rewrite. Preserve exact source/bindings/IDs and actual available APIs.

Freeze matching candidate assets/guide and the sanitized expected set. Commission a fresh Luna author with only those allowed inputs and output destination; no inherited conversation, implementation, private reference or design. Record and judge its first submitted HTML before feedback with real checker and rendered O1/O3. Failure repairs the guide, then a different fresh first-composition trial; never silently repair the author's output into a pass. Keep counts of authored usage separately from library costs if reported.

**Checks:** `pnpm check`; `pnpm build`; full Node/browser gates once at the final changed build; real `demistify check` on complete example and first-author output; packaging/changeset check using `pnpm release:check` with the declared base ref, and `pnpm changeset status --since origin/main`; existing Wrangler compile-only dry run from release workflow. Reuse unchanged earlier native proof only after source/dependency comparison. Record any remaining actual-hosted gaps.

Freeze final source/build and prepare independent implementation review through the same Claude reviewer, with all raw proof and author-case inputs/first result. Root inspects each finding and fixes accepted implementation defects as one bounded batch. Push/open one source PR, validate CI/comments/threads/mergeability through `implementation-pr-wrapup`. The source implementation terminal remains unmerged. U6 is not declared satisfied at that point.

### S5 — Authorized delivery continuation and released-only evaluation

This is the Lead's post-source-PR continuation, governed by the owner's explicit deployment commission and normal protected release runbook, outside `implement-plan`'s PR-ready terminal. No release occurs from a feature checkout or by copying a newer guide under old v1.

After the reviewed source is integrated through the authorized merge route, inspect the current Changesets version PR, its exact diff/checks/comments/threads and mergeability; version/changelog consumption remains separate from feature source as the existing runbook requires. Use the protected `demistify-release.yml` main deployment. Watch its real checks with `gh run watch <id> --exit-status --interval 240`; do not add credentials, disable gates or bootstrap over retained history. A failed release remains a failure. Retain all outcomes; never call an older hosted manifest the new release.

Prepare a source-derived fixed private manifest from the frozen approved HTML before migration: every story/entry field, rich form, supplied attribute/state, ID, link, order and group. Keep it in ignored private task evidence. Root composes the private page from the new released parts without custom presentation/interaction code. Sensitive content uses a trusted complete local copy of the actual matching released distribution over HTTP, per the packaged guide's trust rule; provenance/readback verifies that this copy is the released artifact, with no candidate fallback. Public-safe hosted smoke separately proves public runtime/theme/font/chunks/guide. Do not execute the denied reference file or expose a copied reference as HTTP.

Commission the same independent Terra for full O6 manifest/readback and native comparison at1280/420 after all actual artifacts exist. It compares supplied inert reference pictures and explicit contracts, not invented exact-size golden pixels. Authorised Map correction and new tree behaviors are judged against C5/C7/C8. Report release notes, complete guide locations and usable private page to the design-documents Lead. Only then close the owner delivery ask; required unfinished observations remain open.

## Obligation-to-proof schedule

| Verification row | First local earning slice / command | Final required gate |
| --- | --- | --- |
| U1: R1/R2/R9 | S2 native story/checker cases; S3 complete `show-me-work.browser.test.ts` + paired O1/O5 | S5 O6 full private preservation and reader comparison |
| U2: R3/R4/R10 | S2 scope tests; S3 permanent complete browser case, full suite and paired O2/O5 | S5 actual released interaction and glyphs |
| U3: R5/R6/R10 | S3 permanent native Map/navigation cases + paired O3/O5 | S5 released-only native Map/Comments/reveal flow |
| U4: R7/R8 | S1 focused rule tables and real parsed tree/browser/font; S3 full wiring | S5 released packaged glyph/syntax check |
| U5: R1/R2/R6/R9/R10/R11 | S4 first independent O7 artifact, real checker and rendered composition | S5 O6 exact private readback |
| U6: R11/R12 | S4 matching build/guide/author case and independent implementation review establish source-PR prerequisites only | S5 protected release, actual artifact identity and independent O6 outcome; cannot be earned before release |

## Existing checks and false greens

Keep the existing Markdown/source, fragments, tree label, Trace/card, Comments/inspection, bottom workspace and icon packaging cases. Repair only fixtures/expectations that directly express a changed authorized contract; keep their failure strength. Remove none. Existing tests do not prove Map until new assertions drive it. Add no broad temporary test suite or separate Playwright driver.

False greens to reject: current source tested against stale dist; only library built while PoC consumer is stale; a font class without shipped glyph; mirrored action state; descendant-only/parent-only reveal; counts inferred from output; a title changed to satisfy the baseline; hidden clipping masked by document width; synthetic `.click()` behind a modal; a guide authored by someone who saw the answer; a private record sampled instead of fully compared; prepared assets or source merge passed off as actual release.

## Stops and completion boundaries

Stop the dependent slice for a real model break, missing grant/credential, unavailable native proof, changed public meaning, conflicting ownership or a proof gate that cannot observe its obligation. Preserve raw failure; route to the owning design/plan phase. Small fixture/implementation repairs preserving the governing outcome stay in scope. No unasked infrastructure, security mechanism, compatibility shim, history rewrite, tag, destructive cleanup or public private-content upload.

Slice completion requires its relevant lint/types/format, meaningful automatic proof and the native observation the slice names; then advance. Source-PR readiness requires complete local proof, first-author case and independent implementation review, plus checked CI/review state. Delivery completion additionally requires the actual release and final private reader observation. Name unverified physical-device/Safari coverage and historical text truth; never infer them from desktop CSS widths or the component proof.
