# Demistify response and feedback authoring

Revision 1. One update run targets `demistify-explainer`, owned by `packages/demistify/skill/` in sunstack-tools. Root authors this proposal; the authorized skill Worker implements only after acceptance.

## Promise, source and success

This skill helps agents compose a release-matched Demistify document when a reader must choose an answer, inspect supporting material or leave precise feedback. The baseline is the complete current guide at `c49d2f2fceaef2462d318317e451ee2c8883ac4d`, including its existing reference chooser and component/checking conventions.

Success: given a new, realistic review request, an agent retrieves the actual matching choice/comments API, authors native choice cards with a clear Markdown title/body hierarchy and optional full extras, binds stable review identities, and delivers a page whose selection, details and feedback work together. An explanation-only request does not acquire response UI merely because it compares alternatives.

Authoring basis: **user-directed intent**, from the owner's explicit Web Components, both-skills and fresh usability request, and the accepted UI corrections. This is behavior-changing: the reference chooser, depth and completion gain a response/feedback branch. It is not a causal repair claim about a reproduced historical skill failure. No RED or improvement claim is made.

Source owners: the four documents at `docs/specs/2026-10-24-choice-comments/` define the proposed product contract; existing `render/markdown-content.ts`, inspector/drawer, vocabulary and checker establish integration constraints. Their approval and implementation must precede final API examples. The private experiment is a UI reference, never a public API guide.

## Decisions

| ID | Default and rationale | Failure form addressed |
|---|---|---|
| D1 | Keep the present description and both model/user invocation capabilities: writing/checking Demistify markup already includes this work. No client-specific encoding changes. | Trigger expansion would overlap the workflow skill without improving retrieval. |
| D2 | Keep the document-first mental model and existing five-step spine. Add one conditional chooser row for reader input/feedback, returning the requested response/feedback arrangement with applicable identities, binding and interactions. | New API detail otherwise stays undiscovered or requires page scripts. |
| D3 | New `references/choices-and-comments.md` owns coherent API teaching and one complete plain-HTML recipe. `SKILL.md` keeps the branch and completion visible. | Long syntax would obscure the all-run spine; a component-name list gives no usable result. |
| D4 | Teach live responses separately from `ex-card pick` and comparison display selection. Cards provide a constrained Markdown title and walkthrough narration body; extras go through native inspection, with table/inline alternatives only when useful. | Recommendation/chosen display would otherwise be mistaken for reader input; long cards would repeat the earlier hierarchy failure. |
| D5 | The product guide is the sole syntax/schema owner. Workflow guidance links back to the matching guide. Existing inspection/layout/content references receive only needed cross-links, with no parallel API explanation. | Competing owners drift after release. |
| D6 | Teach explicit document/revision and target/question/option identity; local saved comments versus transient answers; unified Copy/Export and local-only limits. | Anonymous feedback, implied agent submission and claimed answer persistence mislead consumers. |
| D7 | Candidate source proof uses the complete matching local runtime and this guide; package version remains unchanged and one minor Changeset declares the feature. | A public old runtime cannot prove an unreleased API. |

## Surface allocation and calls

- Trigger: unchanged current description; existing true condition is authoring/checking Demistify markup. Near miss: ordinary chat answer or explanation with no artifact.
- Main path: preserve binding, view chooser, compose once, actual checks/observation and evidence return. The new chooser call is `IF the reader must select answers or attach feedback, load references/choices-and-comments.md and return the requested response/feedback arrangement, its required identities and binding, optional supporting-detail placement and interactions to observe.` Its result is consumed during composition and delivered-page checking. Feedback-only work needs no choice; a choice needs no extras; choice-only work needs no Comments binding unless feedback or a combined review handoff is requested.
- Depth: the new reference begins from the selected response/feedback task and owns syntax, complete example, native semantics, rich-content limits, stable identities, supported lifecycle and actual interaction checks. It does not decide when to load itself. Examples use fictional public-safe content. Existing `small-parts.md` remains the owner of identity/badge/check/timing cells.
- Proof: checker/vocabulary consistency and static skill validation establish structure; a fresh agent applies this candidate to a new review artifact. Root inspects actual loaded-copy/action records and the artifact's interactions. Explanation-only recognition is separately checked.

Every new rule serves one of the failure forms above. Completion demands a checked, opened artifact with each requested interaction working: actual selection when answers are requested, details only when extras were authored, and comments when feedback is requested. It does not require all three in every artifact. An API list cannot substitute for the requested behavior. Existing all-run completion remains in `SKILL.md`.

## Proof and coordination

Write hidden checks outside the author work directory before starting fresh no-fork agents. Prompts are ordinary authoring requests, not tests, and never expose the checks or ask which files the agent read. Candidate-path reads and actions come from host records, not the agent's final self-report. Named-candidate application establishes application only; an unchanged installed description or a recognition exercise does not establish implicit installed trigger routing. Report named examples and remaining limits honestly; there is no universal repetition count or invented baseline delta.

Root records one response/feedback artifact application plus an explanation-only recognition case; a failed runtime/environment run is retried as an execution gap. Inspect the actual emitted markup, supplied facts, API matching, selection exclusions, native details and unified feedback. Rework only wording implicated by observed author actions, then recheck that changed behavior.

Base `github/main` and branch `feat/demistify-choice-comments` start at `c49d2f2fceaef2462d318317e451ee2c8883ac4d`. Pending edits at proposal time: Root's design documents only; no skill files edited. The runtime and guide land in the same source PR, minor Changeset, package version 1.12.0 left untouched. Publication, version consumption and installed-cache refresh are outside this run. The workflow companion has its own separate one-skill proposal.

Sensitive surface decision: **allowed** for prose examples of existing local checker commands and release-matched asset loading; no new executable, subprocess implementation, third-party adoption, secret, hook, cache/home write or network authority. Existing command/binding safety stays intact. Required proof is syntax/checker correctness and actual local candidate application; no private transcript text enters public examples.

Non-goals: changing renderer semantics, adding backend submission, controlled-framework state, migrating private prototype storage, maintaining another schema or forcing choices on ordinary explainers.

## Status and review record

Run: **implemented**. Revision 1 is **accepted-to-implement** after independent teaching and structure reviews, all four checks complete/great. Root verified and closed the conditional-return/completion finding at D2 and the main-path paragraphs; no open finding remains. Review records: `/private/tmp/2026-10-choice-components-delivery/skills-teaching-review/product-final.md` and `skills-structure-review/demistify-explainer-revision1.md`. Root accepts this one-skill proposal's current meaning. No skill files changed at acceptance. After implementation, fresh application proof and Lead assessment precede independent implementation review; PR readiness does not mean shipped.

Implementation evidence, 2026-10-24: the unchanged trigger/five-step spine gains one requested-interaction chooser row and `references/choices-and-comments.md`. The diff matches D1–D7, including feedback-only, choice-only and optional extras; no API/schema ownership moved. The complete HTML recipe was extracted unchanged and the actual implemented source checker reported `1 file: 0 errors, 0 warnings`, exit 0. Source comparison covered current vocabulary, registrations, choice/comments getters/events, narration, review model/store and native view composition. Relative references and scoped `git diff --check` passed. The repository formatter explicitly excludes this skill tree; Codex quick validation was not run because PyYAML is absent, as the platform rule permits. Evidence: `/private/tmp/2026-10-choice-components-delivery/skill-implementation/`. These are source/structural results; U4 fresh application, actual rendered interaction observations, Lead assessment and independent implementation review remain open.
