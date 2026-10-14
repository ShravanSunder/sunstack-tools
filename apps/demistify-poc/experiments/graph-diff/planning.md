# Design, planning & proof

The design sets what must work and how we will know. Planning carries that meaning into executable work. The Lead owns the picture and acceptance; the Implementer and Verifier supply different evidence.

:::risk
Source snapshot: 2026-10-14. This is a visual comparison of the supplied planning DAG, not live PR status or new workflow authority.
:::

## The changes and their dependencies

Solid arrows mean “depends on”. Each node’s job, boundary and source are listed below. The two PRs contain implemented source; the F6 nodes are accepted design awaiting implementation.

::diagram{src="fixtures/roadmap.d2" title="The changes and their dependencies"}

### Roles + practices

**PR #127 · draft — Implemented source · reviewed**

Seven responsibilities, Worker/Sidekick execution forms, progressive job packets, and one Verifier paired with each Implementer.

**Boundary:** This is source-reviewed work. Draft PR status does not establish installed behavior or earned trust.

Source reference: PR #127 · head 0bad422c

### Five-document family

**PR #128 · draft — Implemented source · reviewed**

Requirements, Specification, Program Design, Test Design and Verification Design each own a different decision. Each has a local review, followed by integrated reading.

**Boundary:** The family is stacked above #127. Its new design return exists, but current planners still consume the older three-document basis.

Source reference: PR #128 · head ffe30964

### Main planner

**F6 · run 1 — Spec accepted · not started**

Plan Implementation reads all five documents and current closures, then schedules adopted test and verification strategy into slices, commands and prerequisites. It updates the canonical plan and actual consumers.

**Boundary:** It does not choose a replacement oracle or test layer for an adopted design. Missing strategy returns to the design owner.

Source reference: F6 spec · Run 1

### Improvement planner

**F6 · run 2 — Spec accepted · not started**

Plan Improve Repo adopts the same five-document basis for design-bearing findings and preserves positively source-proven mechanics-only work.

**Boundary:** Audit-only exits, direct plan-only results, validation receipts and triage stay distinct. An incomplete design is never a mechanics-only fallback.

Source reference: F6 spec · Run 2

### Structural proof

**Later · after implementation — Planned checks · not run**

Permanent source-contract checks, existing schema tests, and a real source walk through producer → planner → handoff → executor/review. The paired Verifier reports finite observations.

**Boundary:** String checks and schema parsing establish their structural claims; they do not show that a model follows the workflow.

Source reference: F6 spec · Proof and independent review

### Lead assessment

**Later · after scoped proof — Own reading and interpretation**

The Lead opens the actual diff and decisive proof, checks them against the accepted need and design, and resolves every material gap before independent implementation review.

**Boundary:** A helper report or a green count cannot replace the Lead’s understanding of what was required and observed.

Source reference: F6 spec · Proof, step 4

### Independent review

**Later · same relationships — Findings and correction closure**

Independent reviewers inspect the implemented skills, their teaching, calls, claims and consumer consistency. The same review relationship checks bounded corrections.

**Boundary:** Today’s accepted SPEC is not a review of an F6 implementation; none exists yet.

Source reference: F6 spec · Review and coordination

### PR readiness

**Later · separate delivery — Owner retains merge authority**

Current PR head, checks, comments, review threads and mergeability are examined at the authorized delivery boundary.

**Boundary:** This diagram starts no work, grants no merge, and claims no client refresh or runtime behavior.

Source reference: F6 spec · Coordination and non-goals

## What each design contributes to planning

This is the normal dependency path for an adopted substantial design unit. Every document has its own local review; the whole set has an integrated review. Early feasibility research can look ahead without changing who owns a decision.

::diagram{src="fixtures/design.d2" title="What each design contributes to planning"}

### Requirements

**WHY · for whom — Authorized outcome and boundary**

The Lead clarifies the consumer, adopted outcome, constraints and scope with the owner. Requirements receives its local review and owner confirmation or explicit waiver.

**Boundary:** No inferred preference becomes an authorized requirement.

Source reference: Five-document family · Requirements

### Specification

**WHAT · observable — Entities, invariants and obligations**

It defines what correct means through observable permitted and forbidden behavior, traced to Requirements, including domain identity and states.

**Boundary:** It does not choose internal modules, persistence shapes, test layers or mechanisms.

Source reference: Five-document family · Specification

### Program Design

**HOW · structure — Owners, interfaces and proof seams**

It chooses responsibilities, interfaces, state, failure and concurrency realization. It names sufficient observation seams and what must stay real, with structural confirmation or waiver.

**Boundary:** Test Design chooses scenarios and layers within these structural constraints.

Source reference: Five-document family · Program Design

### Test Design

**WHICH wrong behavior — Scenario → real interaction → oracle**

It selects scenarios, independent oracles, the cheapest honest layers, substitutions, deterministic completion, and keep/repair/remove decisions.

**Boundary:** A planned red is owed evidence. It is not an executed failing test. The Implementer writes product tests.

Source reference: Five-document family · Test Design

### Verification Design

**HOW the real system shows it — Reference → route → observation**

It names the independent correctness reference, real build/configuration/client, outcome observations, passing conditions, reuse and limits.

**Boundary:** An acknowledgement, another target or a mocked path cannot prove the adopted outcome. Planned observations remain owed.

Source reference: Five-document family · Verification Design

### Integrated review

**ALL FIVE · one coherent picture — Whole-set reading + final confirmation**

After five current local review closures, an independent reader reconstructs the whole set. The owner confirms or waives the final set.

**Boundary:** Five local verdicts cannot be relayed as the integrated reader’s judgment.

Source reference: Five-document family · orchestrator-design

### Ready design data

**PRODUCER owns validation — Five documents + closures + owed proof**

The cycle producer validates its own result. Planning receives its ready fields as data and opens the actual documents, confirmations and review evidence. Direct source-based planning verifies equivalent readiness.

**Boundary:** The phase never imports an orchestrator-owned validator. Shared document/closure schemas constrain shape, not semantic correctness.

Source reference: F6 spec · D15 and Run 1

### Planning

**F6 · adopted strategy — Schedule it, preserve its meaning**

Choose slices, concrete commands, dependencies, write scopes and earliest useful integration gates for the adopted strategy. Every owed test or observation is scheduled or returned as an exact gap.

**Boundary:** No silent fallback to three documents; no replacing real proof with a cheaper check that cannot observe the claim.

Source reference: F6 spec · D3 and Run 1

### Canonical plan

**ONE execution authority — Durable path + basis + delivery intent**

One immutable Markdown plan links its current basis, proof obligations and intended delivery boundary. Handoff, implementation and review preserve the record.

**Boundary:** A plan-only result stays non-executable. Tickets, handoff and validation do not upgrade authority or create another plan.

Source reference: F6 spec · D5, D14 and D16

## Who owns the work and the judgment

This view is role topology, not a dependency DAG. The dotted Implementer–Verifier link means shared facts and reproductions. Advice, observation, findings and acceptance are separate results.

::diagram{src="fixtures/roles.d2" title="Who owns the work and the judgment"}

### You

**OWNER — Meaning, tradeoffs and merge**

You build the governing mental model with the Lead, settle owner-controlled choices, confirm adopted design decisions and retain merge authority.

**Boundary:** You do not need to ferry routine logs or errors between helpers.

Source reference: Roles · Lead charter

### 🦉 Advisor

**CHALLENGE — Competing model and tradeoffs**

Challenge the Lead’s assumptions and the inference from observation to claim. Bring a credible competing picture with source evidence, gains, costs and who pays.

**Boundary:** Advice is not approval or independent review. It does not author governing design.

Source reference: Roles · Advisor charter

### 🦁 Lead

**GOVERNING AUTHOR — Design, plan, integration and acceptance**

The owner’s main conversation. Writes the design and plan, cuts bounded assignments, reads decisive evidence, integrates results and accepts only claims it understands.

**Boundary:** It cannot become a delegator who forwards “done.” A Verifier may run the observation; interpretation stays here.

Source reference: Roles · Lead charter

### 📚 Researcher / 🔧 Operator

**BOUNDED HELP — Checked answers / prescribed observations**

The Researcher answers a supplied question from inspected evidence. The Operator scrapes, extracts, monitors or runs scripts under supplied criteria.

**Boundary:** Neither owns architecture or acceptance. Cheap models do not acquire implementation authority. Worker/Sidekick describe how these roles run.

Source reference: Roles · practices-manage-agents

### 🐒 Implementer

**SIDEKICK — Code + product tests + associated proof**

Execute the Lead’s bounded plan, write feature code and product regression tests, run associated proof, and correct accepted implementation defects. Every Implementer has a paired Verifier.

**Boundary:** Material design/plan meaning returns to the Lead. It does not author governing replacements.

Source reference: Roles · Implementer

### 🔬 Verifier

**PAIRED SIDEKICK — Outcome observation + test adequacy**

Challenge proof feasibility early, check pyramid/TDD/sensitivity, inspect applicable real-path evidence and observe the adopted outcome. Own only assigned verification drivers/scripts.

**Boundary:** No feature repairs or product-suite edits. Missing cases return to the Implementer; wrong expected meaning returns to the Lead. It does not accept delivery.

Source reference: Roles · Verifier charter

### 🦁 Lead assessment

**BEFORE independent review — Required outcome ↔ actual observation**

Read the diff, the expected outcome and the actual target-bound observations. Determine whether the evidence establishes the claimed result and what remains open.

**Boundary:** Verifier judgment is evidence to inspect. A receipt can be real yet test the wrong requirement.

Source reference: Roles · Lead acceptance

### 🔎 Reviewer

**INDEPENDENT · later — Code, contracts, tests and corrections**

Inspect ownership, types, state transitions, assertions and scope against governing sources. Detect tautological tests and assertions that invent restrictions. Verify corrected findings.

**Boundary:** No editing or delivery acceptance. Review does not replace the Verifier’s real-system observation.

Source reference: Roles · Reviewer charter

## Implemented source and the planned extension

The current view contains two implemented-source nodes and one dependency relationship. The proposal adds six planned nodes and six dependency relationships, producing eight nodes and seven relationships. “Added” means added to this graph proposal; it does not mean implemented.

Current preserves the captured base. Changes keeps that base and marks additions. Proposed shows the resulting graph without change labels.

### Implemented source

::diagram{src="planning-changes.json#current" title="Implemented source"}

### Changes

::diagram{src="planning-changes.json#diff" title="Changes"}

### With the accepted plan

::diagram{src="planning-changes.json#proposed" title="With the accepted plan"}
