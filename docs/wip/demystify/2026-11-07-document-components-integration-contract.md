# Release the approved document components

This 7 November planning snapshot records the approved consumer outcome and the pre-cutover source boundaries. Its source inventory and work list describe base `9b4d85a`, rather than the later implementation. The [component guide](../../../packages/demistify/skill/references/design-documents.md) owns the implemented syntax; actual publication still requires the protected release and public consumer proof.

## The outcome already requested

The owner asked to formalize and integrate the selected document experiment after an independent review, without losing information or changing its design. The owner's 7 November reminder says these components were already requested for release. The other Lead relayed the chosen form as `requirements.html?need=U2&stage=program-design#program-design-U2`, and confirmed the earlier layout choice is decided.

The reusable delivery consists of the requirements browser, its four selected document views, related-documents workspace, and the contextual/standalone call-stack browser. The approved capability picker, equal-size Overview / Specification / Program Design / Verification controls, selected row/detail behavior, one docs-only related workspace, nested call detail, Comments coexistence, motion, source details in each corresponding card, and quiet selected surfaces are preservation constraints. Archived color/corner/layout comparisons do not reopen those choices.

A separately released drawer shell does not deliver these views. Root owns the reusable component and distribution work; the design-documents Lead owns workflow instructions that compose the released contract. Workflow teaching names this work as a dependency until the real distribution contains its API, checker and guide.

## Pre-cutover source at 9b4d85a

Public Demistify is 1.20.0, source `9b4d85a97fa479edb4239bf3bf3e736a60a0557c`. The protected release completed. Current public runtime, theme, stories reference and inspection reference match the published manifest. Story / Section / Map / fold toggle are released, as is the shared bottom drawer. The exact document browsers are app-private `lab-*` elements.

| Responsibility                            | Current source                                                  |
| ----------------------------------------- | --------------------------------------------------------------- |
| Browser registration                      | `apps/demistify-poc/src/understanding-lab/register.ts`          |
| Requirements selection and document stage | `requirement-browser.ts`, `requirement-stages.ts`               |
| Related documents and calls workspace     | `requirement-related-view.ts`, `requirement-related-markup.ts`  |
| Call-stack selection/detail               | `execution-review.ts`, `execution-markup.ts`, `hidden-calls.ts` |
| Shared private picker                     | `search-picker.ts`                                              |
| Authored and prepared records             | `collection-data.ts`, `study-model.ts`                          |
| Page/fragment addresses                   | `study-links.ts`                                                |
| HTML preparation                          | `apps/demistify-poc/scripts/collect-understanding-records.ts`   |
| Private app bundling                      | `apps/demistify-poc/scripts/build-understanding-lab.ts`         |
| Reusable shell already released           | `packages/demistify/src/islands/drawer.ts`                      |

The Requirements, Specification, Program Design and Verification source pages each contain the current requirements-browser island. The execution page contains the call-stack island. Thus the existing consumer is a collection of coordinated views, not merely a single requirements-list widget.

## One source of meaning

Authors own each page's rich HTML at stable IDs and one inert record block in that page. This is not an external sample JSON file serving as canonical content.

Current record kinds are need, specification, program-design, scenario, observation, frame and stack. Metadata supplies identity and forward relationships. Body-bearing records supply `bodyRef` and `summaryRef`, resolving to HTML in the owning page. Rich tables, prose, code and diagrams remain HTML. A program-design record can serve multiple rules without copying its body. Observations attach to scenarios; no observation means Not run, and is not fabricated as a stored result.

The preparer reads the owning HTML files, parses/validates metadata, body/summary references and forward edge kinds, then derives one prepared snapshot per collection and embeds it into output pages. Browser views consume that snapshot; they do not fetch siblings or reconstruct facts from rendered title strings. Backlinks, entry defaults and requirement projections are derived from the same authored records.

The released form must preserve this one-owner model. It must not require a framework, backend, per-document application bundle, Router reader, duplicate JSON prose corpus, inferred evidence, or application-specific fixture IDs. Source preparation remains a document-authoring step; generated snapshots are output, not an extra author-maintained truth.

An HTML-only semantic-markup extractor is not currently implemented. Replacing current inline metadata with that different authoring contract would require its own explicit design; the workflow must not teach it as already supported.

## Preservation and cutover

Move reusable presentation, interaction and its generic data validation/preparation into the library's existing ownership structure. Keep the 42 illustrative records, selected lab pages and comparison archives in the demo app. The demo must then consume the same public entry points and guide syntax as another standalone HTML document; it cannot retain an alternate private browser implementation.

Preserve existing need/stage/stack/call addresses and source destinations through the cutover. Preserve relationships and body meaning, initial states, examples and gaps. Respect supplied IDs and comment-source ownership when rendering a projection. Do not copy a full authored ID into several live places or silently move it onto a different fact.

The approved 1% document-content surface must remain scoped to document content and its compact header. Capability rows, controls and full workspace header retain their selected surfaces. Reuse the existing token/component system; moving a CSS file by cascade order is not an acceptable style extraction.

## Current consumer contract

The four-document set under docs/specs/2026-11-07-design-documents becomes normative once accepted; this draft remains a source-boundary inventory. The workflow needs Requirements-only and partial collections, including complete designs with zero call paths. The existing canonical owner filenames are the bounded initial convention; requirements.html is required, later pages are optional until authored, and extra home/plan HTML pages are tolerated. Body Markdown islands and ordinary non-record sections remain supported. No missing stage or call path is fabricated. Registration also separates archived color/corner studies from reusable browser registration. study-markup.ts and requirements/related/call styles are part of that move.

## Planned integration work, owned by Root

1. Turn implicit related-view hooks into one explicit documented authoring contract. Current `ruleBody` extracts the last paragraph; other projections depend on private selector hooks. A trailing note must not silently replace a rule. Preserve the actual bodies and selected presentation while replacing that accident.
2. Give each reusable view's styles one owner, removing conflicting old overrides when the demo adopts the public implementation.
3. Remove private source-type imports across the package boundary; establish the supported element/controller typing boundary alongside the real public interface.
4. Expose the supported standalone HTML/collection preparation path through existing library packaging/CLI conventions, with real errors for malformed IDs, missing references and unsupported structure.
5. Register the actual public elements, authoring/checker metadata and complete component reference. Workflow teaching depends on this final reviewed interface; this draft does not prematurely choose names.
6. Preserve/extend the real collection and browser cases; verify the selected desktop and 390px layouts in affected themes, Comments, nested sheets, native controls, exact source links, and a second independently authored collection with non-fixture identities. A guided authoring trial with a corrected attempt already exists; it does not prove unaided general capability.
7. Independent style/component review, one coherent source PR, protected version/release and actual published-byte smoke. Release completion means these actual browsers can be authored against the published runtime/guide, not merely that PR130 was merged.

Historical G1 fixture-dependent entry defaults were repaired on the branch at `5d8d113` and reached current main in the PR130 squash `2f8db1d`; do not reopen them without a current reproduction. Historical G2/G3/G4 were formalization findings; recheck current decisive owners rather than claiming every old concern remains a current defect.

## Evidence and limits

- Original owner request and quote: sibling worktree `sunstack-tools.fix-demistify-document-drawers/tmp/review-handoffs/2026-11-05-document-system-formalization/owner-request.md`.
- Original integration/review boundaries and findings: its `implementation-handoff.md`, and `tmp/review-handoffs/2026-11-05-current-documents-review/`.
- Collection-driven entry and first-author evidence: `tmp/2026-11-05-collection-entry/checkpoint-5d8d113.md`. Its old PR state is historical; GitHub now confirms PR130 merged.
- Public snapshot at drafting: `https://demistify.getagentstudio.dev/v1/release.json`, 1.20.0/9b4d85a, 71 files, manifest SHA-256 `b699d374bbfdf55eafc18ffc8f15ca57d9c769c543b4f656f62f015eb28410d3`.
- Peer workflow design: the other Lead's design-workflow set in `ai-tools`, revision `fbc2eefb` before its reconciliation. Workflow adoption follows component publication.
- Root has reopened the existing app demo through native Chrome. Actual AX content is observed; screenshot output is grey and is not a reliable visual pass.
- The independent show-me private-page O6 is a separate remaining proof gate. Its browser/tool issue is not a prerequisite to writing this integration contract or releasing the document components.
