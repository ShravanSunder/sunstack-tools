---
name: spec-program-review
description: "Use when independently reviewing a Specification, Program Design or the four-document set, including reader understanding or readability, or whether a spec- or program-only change needs review. Not for implementation review."
---

# Design Review

IF no work reference is in context and the task qualifies, open or resume the trace through `practices-show-me-your-work` before phase work.

Review is independent reconstruction of the smallest system that satisfies the confirmed goal. It is not proofreading, design by committee, or a search for mechanisms to complete.

The reviewer rebuilds and challenges:

```text
Requirements: WHY, for whom, and within what boundary?
  -> Specification: WHAT must be observably true, and against what reference?
  -> Program Design: HOW will the internal system satisfy it?
  -> existing foundation and structural realization
  -> current/proposed calls, state, failure, and observation seams
  -> Verification Design: how will each requirement be shown to hold?
  -> reader understanding and planning readiness
```

Requirements, Specification, Program Design, and Verification Design are separate authoritative documents. Review reconstructs each one independently before judging their agreement; agreement inside a combined `Requirements/spec` document cannot substitute for a separately identifiable Requirements source and Specification.

A persistent 🔎 Reviewer Sidekick, commissioned through `practices-manage-agents`, reads the complete target set and governing sources, then walks the ordered checks in its own session. The Reviewer Sidekick verifies candidates against accepted requirements, Specification obligations, and the confirmed goal boundary, tests deletion before addition, and owns the coverage-bound assessment. The orchestrator disposes and routes that assessment. This skill never edits artifacts, mutates their lifecycle, plans, or accepts a design.

Prefer one independent review-and-correction round. After 🔎 Reviewer Sidekick verification, run further rounds only for a concrete source-backed substantive issue that remains or was introduced inside the accepted boundary; pedantic, stylistic, already-satisfied, confidence-only, and generic-freshness concerns do not qualify. IF such an issue admits a further round, load `references/finding-and-reduction-schema.md` to run rounds under its convergence rule until the review is `ready` or `not-converging`, and return that state. Each round may span several semantic owners in document order (`spec-design -> program-design -> verification-design`), with each affected artifact corrected at most once in that round and every corrected anchor verified by the Reviewer Sidekick.

Disposition comes before remediation. Reject pedantic, stylistic, already-satisfied, or otherwise non-semantic findings with source evidence and continue. Route each valid correction set inside the settled mental model to its semantic owner. If a finding disproves a load-bearing assumption or exposes unmade owner meaning, return the failed assumption, evidence, consequence, and exact owner as `decision-needed` or `blocked`; do not push a mental-model break through correction. The break stops current continuation and goes to the Lead for an owner brief. When the same result also contains an accepted bounded correction, preserve it explicitly as one round in document order after the owner decision, with each affected artifact corrected at most once and one parent verification against the original findings. Once the owner settles the break, review continues under the convergence rule in `references/finding-and-reduction-schema.md`.

## Operations

```text
classify-review-requirement
  requested future mode: specification-only | program-only
  returns: review-required | non-substantial, or blocked input
  dispatches: no reviewer

review
  mode: specification-only | program-only | four-document-design
  four-document-design targets:
    Requirements identity
    Specification identity
    Program Design identity
    Verification Design identity
  result: ready | needs-revision | blocked | decision-needed | not-converging
  coverage: current meaning coverage for every target; uncertain semantic effect is stale
  checks: mode-complete always; sequential chunk passes along artifact seams when
    the set is large; proof-challenge for executable claims; dispel always;
    focused checks for named unresolved risks
```

Review coverage follows meaning, not changed bytes. After each permitted correction round, the parent performs and records a semantic-change check before deciding whether review is complete:

- if meaning changed inside the accepted correction and every accepted finding is resolved, preserve the review as the independent finding source and add parent-verified correction anchors for every accepted finding;
- if the parent verifies that meaning did not change (for example, a formatting, link-repair, process-metadata, or typo-only edit), carry the existing semantic coverage forward without another review;
- run further rounds for concrete source-backed substantive issues inside the accepted boundary until the review is ready or not-converging (see `references/finding-and-reduction-schema.md`); reject a rerun based only on pedantry, style, an already-satisfied finding, confidence, or generic freshness.

Closure is established by the latest permitted review result plus parent-verified correction evidence for the current artifacts. Keep this call-scoped record out of durable design artifacts and do not add persistent review bookkeeping.

## 1. Classify Review Requirement

IF operation is `classify-review-requirement`, load `references/classifying-review-requirement.md` and return its `review-required | non-substantial` result or blocked missing-input result.

Dispatch no reviewer, select no checks, and return no review verdict. `review-required` instructs the caller to make a separate fresh review invocation.

Completion: classification returns immediately with zero reviewer dispatches.

## 2. Prepare the Review

For operation `review`, require:

```text
mode and complete target identities: file paths or separately labeled in-chat records
governing sources, authority states, and coverage basis
confirmed goal boundary and accepted requirements set, or the exact authority gap
constraints and non-goals
risk predicates
claimed proof evidence or gaps
review question when narrower than readiness
prior review coverage and semantic-change record when coverage is being reused
bounded design-review status: no prior review | prior rounds with their findings | earlier results unavailable
```

When prior results are unavailable, inspect the current target and governing sources, record the reason, and treat this review as the baseline; the convergence rule in `references/finding-and-reduction-schema.md`, loaded at step 8, counts from it. Reject stale, wrong-source, or unverified inputs. Route any material design break or newly exposed owner decision to its owner.

MUST load `../../shared-references/design-documents.md` and return the identity status of each document the selected mode needs. `specification-only` inspects separately identifiable Requirements and Specification sources. `program-only` adds a separately identifiable Program Design. `four-document-design` inspects all four, the Verification Design included. Reuse resolvable file pointers for file-backed records. Copy separately labeled in-chat records verbatim into the tmp review packet, because a fresh reviewer cannot follow a pointer into a conversation, and never into a durable combined review artifact.

Use the owner-confirmed requirements record and confirmed goal boundary when available. Otherwise use the last inspectable owner-accepted governing source. If neither exists, or they conflict, return the authority gap. Mutually narrowed current files never establish the accepted requirements set by themselves.

A combined `Requirements/spec`, a Requirements-titled artifact that also stands in for the Specification, or an absent separate Requirements or Specification identity is a concrete blocker-level design finding, not permission for review to infer the missing identity. Record `needs-revision`, route the smallest correction to `spec-design`, and do not repair or create either artifact during review. Continue only with bounded findings that the available sources can support; never return `ready`.

`program-only` also requires the governing Specification. `four-document-design` requires the current Requirements, Specification, Program Design, and Verification Design. A missing confirmed goal boundary may produce `decision-needed`; review does not infer acceptance from silence or a status label.

Completion: the complete target set, governing sources, accepted requirements, boundaries, open authority decisions, and any prior-coverage semantic-change record are unambiguous; any missing-history baseline is recorded with its reason.

If a review-and-correction round already ran, admit another only under the convergence rule and accepted boundary in `references/finding-and-reduction-schema.md`. Switching modes, checks, target labels, or caller skills does not reset the convergence comparison.

## 3. Select the Mode

The selected mode reference owns its review judgment:

```text
specification-only   -> references/reviewing-specification.md
program-only         -> references/reviewing-program-design.md
four-document-design -> references/reviewing-four-document-design.md
```

Specification review judges the governing Requirements authority and the Specification's observable What. Program review judges structural How and realization of the governing Specification, including each entity's binding, the shape at each boundary, and closed result and decision variants. Four-document design review independently repeats both, judges the Verification Design, walks each requirement from its obligations through their seams to its planned observation, walks the trace table row by row, checks that each term means the same thing in all four documents, and judges integration and planning readiness. A revision to the Verification Design alone is reviewed in `four-document-design`; there is no standalone Verification Design mode. A design noun with no entity routes `Route: spec-design` when it names no `E` or `R` it serves, or carries its own identity, lifecycle, or relationships; a synonym for an existing entity routes `Route: program-design`.

Completion: exactly one mode and its complete required artifact set are selected.

## 4. Resolve Reviewer Runtime and Authority

The caller completes classification and admission. For an admitted substantive review, MUST use `practices-manage-agents` to commission or resume one persistent 🔎 Reviewer Sidekick with read-only workspace access. Resume that exact Reviewer Sidekick through corrections. Candidate findings never become final disposition without its verification.

A later semantic change outside an accepted correction invalidates affected check coverage. Further rounds follow the convergence rule in `references/finding-and-reduction-schema.md`.

## 5. Walk the Mode and Chunk Checks

The 🔎 Reviewer Sidekick reads the complete target set and governing sources first, every artifact whole. MUST load `references/coordination-and-chunking.md` and return a sequential chunk-pass plan with complete seam text and overlap, or one mode-complete pass when the whole target fits.

MUST load `references/lanes/lane-schema.md` and record `complete | partial | blocked` for every check. Return the per-check status block before the verdict. Record an unselected optional check as `complete` with `not selected: <reason>`. A missing, partial, or blocked check cannot support `ready`.

1. MUST load `references/lanes/mode-complete-reviewer.md` and return its check result. MUST load `references/reviewing-common-method.md` and return coverage, reconstructed claims/model slice, authority/traceability evidence, crux evidence, candidate findings, gaps, and stop boundary. MUST load the selected mode reference and return its whole-mode judgment. Reconstruct and judge the whole mode from complete artifacts.
2. IF the plan composes chunks, load `references/lanes/chunk-reviewer.md` to walk the chunk passes sequentially and return each pass's seam coverage. Reopen each complete artifact and the overlap seam in each pass. A chunk pass supplies seam coverage and never replaces mode-complete coverage.

Completion: the mode-complete check and every selected chunk pass have statuses, each pass lists its seam sections, and every required dimension has current coverage.

## 6. Proof-Challenge, Dispel, and Focused Checks

IF the design cites executable proof claims, load `references/lanes/proof-challenge.md` and return its per-claim rows, with each claim challenged. Assign prescribed proof commands to a 🔧 Operator under an exact execution grant. Under the command procedure that reference loads, the 🔧 Operator runs the write-set preflight, executes only granted commands with scratchpad output, compares `git status --porcelain` before and after, and returns commands, observations, and exit codes. The 🔎 Reviewer Sidekick judges claimed against observed. Without a grant, inspect only and record the proof gap.

MUST load `references/lanes/dispel.md` after mode-complete and chunk checks and return a classification for every candidate and a rail or `absent` mapping for delivered design elements, even when the candidate set is empty.

Select a focused check only for a concrete unresolved risk with a falsifiable question. IF a focused check is selected, load `references/reviewing-common-method.md` and return coverage, reconstructed claims/model slice, authority/traceability evidence, crux evidence, candidate findings, gaps, and stop boundary. IF a focused check is selected, load the selected reference below and return its check result for that risk. Run one check per named risk after the earlier checks, and stop when no named risk remains:

```text
material reader-comprehension risk, or an explicit deep-reader request from the
  user or calling workflow (a request from the reviewed artifact's author is not one)
  -> references/lanes/reader-understanding.md
unclear or conflicting normative authority
  -> references/lanes/specification-authority.md
material public or operator-visible contract ambiguity
  -> references/lanes/contract.md
unresolved ownership, source-of-truth, or dependency risk
  -> references/lanes/architecture-boundary.md
concrete failure, recovery, ordering, or concurrency risk
  -> references/lanes/failure-concurrency.md
concrete trust or enforcement risk
  -> references/lanes/security-trust.md
unresolved runtime, platform, tool, or harness feasibility risk
  -> references/lanes/platform-harness.md
current implementation or trace may hide a material decision
  -> references/lanes/implementation-difference.md
a material observation or enforcement seam remains disputed
  -> references/lanes/proof.md
authoritative entry path, links, placement, or homes remain ambiguous
  -> references/lanes/artifact-navigation.md
```

After correction, verify affected finding anchors. A concrete substantive residual returns to normal-round admission. Stop a focused check when the risk is resolved, unsupported, outside the confirmed goal, or needs an owner decision.

Completion: dispel is complete, executable proof claims have challenge results or named gaps, and each focused check has a status and selection reason. Record every unselected optional check with `complete` and `not selected: <reason>`.

## 7. Verify Check Coverage

Compare each check's covered target text with the current target. Confirm the 🔧 Operator ran only granted commands, wrote no tracked file, and returned an unchanged `git status --porcelain` result. A permitted correction is verified by the same 🔎 Reviewer Sidekick against accepted findings without automatically starting another review round.

Completion: each check supplies current coverage for its selected target, and proof execution stayed inside the recorded grant.

## 8. Verify and Reduce Findings

MUST load `references/finding-and-reduction-schema.md` and return its dispositions, merged duplicates/conflicts, coverage gaps, goal-relevance record, deletion-first decision, scope effect, and final coverage-bound result. In that reference, `parent` means the 🔎 Reviewer Sidekick, never the author or orchestrator.

Re-anchor before accepting a finding: identify the confirmed requirement or goal-boundary field it serves, whether the existing foundation already satisfies it or can supply the correction, the concrete failure if unresolved, the smallest correction, whether the confirmed obligations still hold without the questioned mechanism, whether the proposed mechanism is the smallest change that serves the quoted clause or one of several, and whether the correction stays inside the confirmed goal boundary. Then return the existing `accepted | rejected | contested | unverified` disposition; do not create a second checkpoint status. When evidence is missing, follow that reference's evidence-lookup branch before accepting. Question whether a proposed mechanism is needed before accepting findings that merely complete its missing contracts.

Every finding uses the complete Finding shape from the loaded schema. For the caller-facing explanation, write every returned finding in ordinary language, whether it is an unreduced reviewer candidate or has the parent disposition `accepted | rejected | contested | unverified`. Each explanation includes:

- a title naming the concrete problem rather than a review method;
- what is wrong and where the evidence appears;
- the affected confirmed requirement or design relationship and concrete consequence;
- the smallest correction, followed by a separate `Route:` line naming `spec-design`, `program-design`, `verification-design`, `caller`, or several owners in document order, such as `spec-design -> program-design`;
- the evidence or affected review coverage that would confirm the correction.

Do not leave the route implicit from the artifact or section named. Record parent disposition separately after reduction; unreduced candidate status does not waive the useful finding fields. A review label may summarize the explanation but may not replace any field.

For each material proof claim, compare the claimed outcome with the supplied evidence's actual observation boundary. State what the evidence proves, what it cannot observe, and the smallest missing planned observation (a Verification Design correction) or observation seam (a Program Design correction). Do not collapse distinct unsupported claims into generic “runtime proof”; match each claim with the smallest observation that could confirm or falsify it. For each applicable diagram, name the reader question it should answer and compare its visible owners, direction, state or effect, normal and error behavior, and changed edges with the written requirements and design. Rendering or repeated labels alone do not establish usefulness or agreement.

Classify each requested correction by the concept it affects: `Requirements`, `Specification`, `Program Design`, `Verification Design`, or a named combination. A correction to one document preserves the documents upstream of it unless the owner explicitly changes them.

Route accepted corrections by semantic owner:

- `Requirements` returns to `spec-design`, using `discuss-pathfinding` when owner meaning is missing;
- `Specification` returns to `spec-design`;
- `Program Design` returns to `program-design`;
- `Verification Design` returns to `verification-design`;
- any correction spanning Requirements or Specification plus Program Design returns to `spec-design` first, then `program-design` after the observable contract is settled.
- a correction that also touches Verification Design returns to `verification-design` last, after the documents it depends on are settled.
- `caller` returns the exact owner/caller decision; review does not resume until that decision is resolved.

Reject prose taste without reader or design effect. Missing evidence is `unverified`. Preserve real disagreement as contested. Two kinds of scope expansion get two different dispositions. A reviewer candidate that proposes adding scope with no rail anchor (new persistence, history, identity, governance, certification, control planes, external services, resilience layers) is `rejected: scope expansion` and listed for the owner as an observation, never `decision-needed`; anchoring it one altitude up does not rescue it when the mechanism is one of several ways to serve the quoted clause, and an accepted finding then names the unrealized obligation, never the reviewer's mechanism. A design element the artifacts already contain that dispel mapped `absent` is never rejected as harmless: it returns `decision-needed` with deletion as the default recommendation, or `needs-revision` routed to `program-design` when the owner has already declined it. When the parent's disposition contradicts dispel's correction class, record the quoted rail text that overrides it.

Completion: every candidate has a source-backed disposition; every finding uses the complete loaded Finding shape and contains each caller-facing field above regardless of disposition or final verdict; and no accepted finding silently changes the confirmed goal or accepted requirements.

## 9. Return the Coverage-Bound Result

Return every field in the `Coverage-Bound Result` owned by `references/finding-and-reduction-schema.md` for the current mode and targets.

`ready` means the reviewed artifact meaning satisfies the invoked mode. When accepted bounded findings are remediated, downstream callers may continue only with the original result plus complete parent-verified remediation evidence. Rejected non-semantic findings need no remediation. A mental-model break remains a stop until its owner supplies settled meaning. Durable artifacts remain about their subject matter and do not acquire review lifecycle or acceptance status.

After parent reduction, return either a stop or exactly one recommended next skill using the route and compact-handoff procedure in the loaded reference. Reviewer candidates never select this route. When validated findings span documents, recommend the most upstream owner first and carry the complete accepted set; each owner's result decides whether the next one follows. Recommend `discuss-pathfinding` only when complete review evidence establishes that the current model fails and replacement owner meaning is still unmade, and include the review-selected return owner. A current authoritative correction routes directly to its semantic owner; missing evidence stops.

Completion: the result names the first required revision, coverage gaps, any owner decision, what a downstream program designer or planner would still have to invent, and one parent-selected compact continuation handoff or an exact stop reason.

## Completion Blockers

Do not return `ready` while any of these hold:

- the current target identity or semantic scope is missing or ambiguous;
- Requirements and Specification are not separately identifiable in any review mode, or a combined `Requirements/spec` is being used as both;
- the complete target or governing-source set was not read;
- the confirmed goal boundary or accepted requirements are missing or conflicting without an explicit returned authority gap;
- no complete semantically current mode-complete check exists;
- a selected check has no status;
- partial or blocked coverage affects a required dimension;
- a finding lacks its anchor (an accepted requirement identity, confirmed goal-boundary field, or domain-skill rule), its plain-language meaning, the observable outcome that fails, source-backed evidence, deletion test, scope effect, semantic correction route, or disposition;
- specification, program, or four-document-design mode boundaries are conflated;
- `program-only` or `four-document-design` omits an applicable current/proposed call path, explicit no-predecessor case, added/removed/changed edge status, or a preservation-critical or contested unchanged edge;
- `program-only` or `four-document-design` accepts a Specification entity with no binding, a boundary shape written only as prose, or an open-string result or decision contract; or `four-document-design` passes traceability without walking every trace-table row and every requirement to its planned observation, or leaves a design noun unresolved against the entity table;
- `four-document-design` proceeds without a separately identifiable Verification Design, or accepts a requirement with no Verification Design row;
- a material proof claim is accepted without evidence that can observe it at the required layer, or an applicable diagram is accepted without checking that it answers its reader question and agrees with the written meaning;
- a UI image is accepted that invents a control or skips a changed screen;
- required file-backed visuals or relative embeds are missing or broken, generated labels/edges/boundaries disagree with governing meaning, or unavailable image/preview access was treated as complete visual coverage;
- four-document-design mode trusts author or local checks without independent reinspection;
- the Reviewer Sidekick did not read the complete target set and governing sources before planning checks, or an optional check lacks a selection reason;
- a chunk plan split one requirement's trace to its obligation, realization, and planned observation, or one call-path delta with its owners and proof seam, across chunks with no overlap seam; or a chunk pass was treated as mode-complete coverage;
- any reviewer judged an artifact it did not read completely, or the common method's coverage rows leave a target section neither covered nor excluded with a reason;
- the dispel check is missing, or a design element dispel mapped `absent` lacks either a `decision-needed` return or an accepted removal routed to `program-design`, since "well drawn" is not a disposition;
- a reviewer candidate that adds unrequested scope was returned as `decision-needed` or accepted instead of rejected as scope expansion;
- a 🔧 Operator ran a command outside the recorded grant, or an executable proof claim was accepted without a proof-challenge result or a named proof gap;
- focused review began before reduction of the earlier checks, a focused check was selected without a named predicate, or a broad predicate was treated as sufficient selection;
- a round ran after not-converging, or outside the accepted boundary without an owner brief;
- the downstream consumer must invent meaning owned by the reviewed artifact;
- the result recommends no next skill, more than one next skill, or a route selected from an unreduced reviewer candidate when a validated continuation exists;
- a continuation omits the current boundary status or makes the destination choose among correction alternatives instead of carrying one smallest verified correction;
- a pathfinding recommendation omits the review-selected return owner, or complete current authority/evidence already settles the correction;
- a continuation copies artifact contents or unrelated history instead of returning the destination's compact pointer-based handoff;
- the result implies edit, remediation, lifecycle, planning, or acceptance authority.

IF a trace is open, at phase completion record the outcome, evidence, and next owner or return token as a checkpoint through `practices-show-me-your-work`.
