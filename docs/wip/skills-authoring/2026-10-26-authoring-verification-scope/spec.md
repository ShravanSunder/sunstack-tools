# demistify-explainer: authoring verification scope

Revision: 1. Target: `packages/demistify/skill`. Revision 1 run: update; reviewed.

## Problem and success

Owner Oct26: “remove the phone verification”; component-development checks belong in this repository's AGENTS.md, not repeated in Demistify/presentation authoring skills; “make new prs”; “pointless verifications burn tokens.” User-directed intent, not a reproduced model defect.

Ordinary authoring must not require a phone/narrow viewport, desktop/phone screenshot pair or light/dark regression matrix, including through references. Retain binding/source/markup, authored content and link checks, task-specific interactions and truthful proof limits. Retain responsive component behavior documentation. Component developers own responsive UI checks in repository instructions.

## Decisions

| Decision | Rationale |
| --- | --- |
| Remove viewport/theme matrix obligations from existing homes, no new authoring flag/helper | Less recurring work; one verification owner |
| Keep author-content checks and explicit task requirements | Removing library regressions must not excuse broken authored content |
| Keep responsive API descriptions | They explain what the library provides, without assigning a test matrix |
| No browser/product suite rerun for instruction-only deletion | It cannot establish instruction uptake; no rendering code changes |

## Surfaces and proof

Trigger unchanged. Main removes mandatory viewport/theme observations and calls returning them. Depth removes matching obligations from existing references. No new reference/schema/tool. Proof: inspect complete changed files and references for remaining mandates; validate Markdown/frontmatter/local links and packaging/version consistency with existing tools where available; independent source review. Strongest claim is source agreement with owner intent; fresh-agent application unobserved/deferred, no improvement/reliability claim.

## Coordination

Base origin/main `7403724327af73cd507cdd1bda494b1d4cb842e1`; branch already isolated, no prior skill edits. Separate new PR, ready/unmerged, no release/cache/merge. Product AGENTS.md gets developer-owned responsive verification; patch package Changeset records guide update.

Non-goals: runtime/CSS/test implementation, storage/identity/API changes, deleting responsive behavior documentation, broad verification-policy rewrite.

Run status: reviewed. Revision1 SPEC accepted-to-implement; combined independent implementation review returned great/source-only with no accepted findings. Reports remain in the owning work record; fresh-agent application is unobserved.

## Revision 2 — additional owner-directed variation teaching

Owner Oct26 supplies an agent transcript that treated related screenshot sizes as a filterable list rather than variations, then asks to fix the skills “in a generalized way ... with an example for screen sizes.” This is user-directed teaching, not a faithful reproduced agent failure. Revision1 remains accepted/implemented for the phone-matrix removal; its decisions remain in force.

Success: teach the distinction between a logical item and alternate captures, variations versus filter facets versus separate items/steps, using only the bound component API. Include a worked screen-size example, meaningful capture-backed categories and missing/ambiguous capture behavior. A guided comparison uses one stop per logical screen/state with variations enabled; exact-capture evidence may intentionally omit it. No invented generic variant attribute, runtime change, automatic image inference, manufactured captures or author phone/theme regression matrix.

| Added decision | Rationale |
| --- | --- |
| Teach subject/variation/filter distinction before syntax | Prevent the flat-list interpretation reported by the owner |
| Screen size is a worked example, not a fixed enum | Apply the judgment to supplied evidence without inventing metadata |
| Preserve actual screen+state / viewport contract | General teaching must not claim arbitrary runtime axes |

A second run, same named skill. Main pictures call returns grouping/variation decisions. Depth updates existing pictures.md with one complete gallery/journey recipe and good/bad distinction; tours.md connects its optional variations flag to that grouping. Keep Gallery standalone-page constraint, stable original figure IDs, source-relative references, and truthful file/binding limits.

Proof for added teaching: manual source/API walk against current galleryScreenKey (screen+state) and viewport navigation; static check the literal product example against unchanged matching checker when available; current frontmatter/link/version checks and independent review. Fresh author transfer remains unobserved/deferred. No browser/product-suite or paid evaluation rerun for prose-only changes. Same base/newPR/version allocation as above; patch Changeset/changelog now describes both changes.

Run status: revision1 phone scope reviewed; revision2 variation teaching separately SPEC-accepted and reviewed. Combined independent implementation review returned great/source-only at 1052b99284aa0533be8462a9a33453d5bfff077c; no accepted findings. Fresh-agent application, rendering and installed/released uptake remain unobserved.

## Revision 3 — readable labels within the matching component guide

Run: update demistify-explainer only; behavior-changing/user-directed intent. Status: proposed. Owner Oct27 requests full feature inventory and generalized skill fixes/PRs, with Demistify owning component mechanics and presentation HTML owning when/why. Owner's wide-diagram example asks for multiline labels; “dont over fit.” Revisions1/2 remain effective.

Success: an agent using the component guide can author supported multiline flowchart labels, keep identities and relationships legible, and move full prose into adjacent content without losing source meaning. It retrieves the current controls and component syntax from one bound guide. Existing current feature coverage, Gallery variation teaching and authored-check scope remain intact. No new Mermaid family or rendering behavior is promised.

| Decision | Reason |
| --- | --- |
| Keep trigger/main structure and existing17 references | Component syntax already has coherent owners; a second catalog or new umbrella reference would duplicate them |
| Add a concise flowchart multiline example to diagrams.md | The pinned renderer1.1.3 normalizes <br/> and escaped newlines; this is supported behavior, not guessed upstream Mermaid syntax |
| Label a node by its identity and a short responsibility; use a meaningful line break | Keep the relationship readable without turning labels into arbitrary word-count rules |
| Place full supporting rules in adjacent prose/table/card | Preserve source facts while removing paragraph-long node boxes |
| Choose direction or smaller views by readability and preserve required edges | No blanket horizontal-layout ban or screenshot-specific services |
| Reuse PR82 after merging latest main | Preserve current1.15.3 palette, gallery arrows, one-comment editing and visible Details controls |

Surfaces: trigger/invocation, main path and routing unchanged in this run. Depth: diagrams.md replaces the short generic readability sentence with supported syntax, one small representative example and a concrete good/bad distinction. No other new API teaching, custom colors, new schema/reference/script or runtime edit. Existing revisions1/2 continue to own variations and removing routine device/theme regression matrices. Content/markup/binding/link and requested-interaction observations stay.

Basis: owner-directed success definition, read guide1.15.3 at fe5ea68, package1.1.3 multiline-utils.ts normalization and renderMultilineText, and actual themed render of Review page/Collect feedback → Local review/Retain comments using <br/> (four separate SVG text spans, no literal break markup). This establishes example support only, not all upstream Mermaid syntax. No historical agent failure RED is claimed.

Proof: check frontmatter/relative reference reachability and focused diff; render the exact taught example with the pinned themed renderer and inspect line spans/relationships; check its wrapped HTML with the matching checker; one fresh-context realistic markup task showing guide retrieval/application, with host loaded-copy/action evidence where exposed. Check criteria stay outside subject work directory. Strongest claim is observed candidate example/application, not comparative or repeated reliability. No component device/theme matrix or full product browser rerun for prose-only changes. Any unavailable action record or transfer is named as unverified.

Coordination: branch fix/demistify-authoring-verification-scope, latest main1e986a9 merged at clean8f648f570eb8945d6ff9706ef7d17be4f67dd04b. ExistingPR82, no parallel writer. Existing patch Changeset updated to describe final instruction-only change; public deploy/release not implied by this PR-ready request. Non-goals: API/storage changes, renderer/palette implementation, gallery/walkthrough redesign, broad verification policy or legacy feature promotion.

Review record: revision3 spec pending, no revision3 skill edits. Independent skill-review spec acceptance precedes implementation, followed by example/application proof, Root assessment and independent implementation review. Earlier run acceptance/review is retained without claiming it covers new meaning.

## Revision 4 — consolidated component-guide inventory corrections

Status: proposed; supersedes unaccepted revision3. Same named target and owner, user-directed intent, no historical RED claim. The completed independent B whole-source review confirmed main+17references cover all current families and accepted two missing HOW examples in diagrams.md. Root opened both absence homes, exact pinned adapter and actual render output. Existing R1/R2 changes stay; no new main-path or trigger change.

| Decision | Basis and rationale |
| --- | --- |
| Keep B-T1's qualified multiline flowchart example from R3 | Teach supported breaks rather than guessed upstream syntax; full meaning remains adjacent |
| Add B-T2's minimal named numeric-chart recipe in the same diagrams.md home | chart-source.ts already captures quoted names on bar/line statements and diagram.ts wires it into real rendering. Root's actual example retains Accepted/Rejected legend labels; this is current adapter support, not a new API |
| Explain quoted series names, actual axis categories/units and truthful fallback together | A useful recipe lets the author identify what values mean; generic legend labels or unsupported scientific meaning remain a reason to use a table |
| Keep A's when/why and direction/disclosure judgment separate | B owns exact supported source and controls, no duplicate workflow chooser or generic syntax catalog |
| Defer B-RA1 generic zoom metadata wording to its external vocabulary/generated-metadata owner | Current guide correctly says Mermaid is icon-only fullscreen and Grid/Flow do not gain container inspectors merely because a flag parses. The minor common-attribute description does not require a guide/runtime fix or widen this skill run |

Scope: diagrams.md adds the compact quoted-flowchart multiline example, a small named bar/line example and adapter-specific limits; retain safe inert source carriers, palette/ER/fullscreen contracts and source-versus-execution distinction. Keep existing grouping and matrix-ownership corrections in their current homes. No vocabulary/runtime/test implementation, new attributes/families, shared reference, second guide, new schema/script or global policy change.

Proof: exact taught chart source runs through the existing themed renderer and must retain series names plus supplied categories/units, not generic labels; the multiline example retains all branch relationships and separate lines. Wrap actual examples in ordinary matching-bound HTML and check them with the existing CLI; no permanent test is added for prose examples. Static guide/caller/link/packaging checks and the planned fresh bounded candidate application follow. Source/example observation is not a RED→GREEN, complete UI matrix or general author-reliability claim. Runtime/physical-phone behavior is unchanged and not rerun. Source-review findings B-P1/P2 already correspond to pendingPR82 and remain included, without duplicating PRs. Any fresh-run action/loaded-copy gap is named honestly.

Coordination/base/patch Changeset/PR82 and non-goals remain from R3. A new library release/deploy or installed-copy refresh is not established by this PR. Review target is R4 as a whole including effective retained decisions; independent skill-review SPEC acceptance is pending. No R3/R4 skill-file changes have been made.

R4 execution record (2026-10-27): independent skill-review SPEC accepted-to-implement, selected depth-coverage/rule-agreement complete, zero candidates (retained b-spec-r4-review.md). Root read the whole report and current sources. R4 status: implemented, not reviewed/shipped. Only diagrams.md and the allocated Changeset changed beyond the spec; trigger/callers/API/runtime unchanged. Exact source examples were already rendered at example strength; final extraction/checking, bounded application, Root assessment and implementation review remain owed.

R4 final status (2026-10-27): reviewed at source/example strength, not shipped. Independent implementation review returned great: all three selected checks complete, zero new accepted findings; four in-scope baseline source findings resolved in the draft. Reviewed semantic headdf55b5160bd6ced1736a8f8da1c01c5a482a015d; this final record changes process status only, not skill meaning. External generic zoom metadata remains separately deferred. Actual loaded-copy/action provenance, empirical uptake/trigger/transfer/reliability and installed/released uptake remain unverified. Full independent reports are retained in the private work record. PR82 remains open/unmerged; no release or stronger behavior claim follows.
