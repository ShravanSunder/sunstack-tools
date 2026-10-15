# Brief: design, planning and proof

Audience: a developer who must understand how an accepted design reaches implementation, who owns each step, and what is already built versus only proposed. Source snapshot dated 2026-10-14; it is not live PR status.

Opening summary: The design sets what must work and how we will know. Planning carries that meaning into executable work. The Lead owns the picture and acceptance; the Implementer and Verifier supply different evidence.

The explanation has three perspectives. Readers usually want one perspective at a time.

## Perspective: The changes and their dependencies

Solid arrows mean “depends on”. The two PRs contain implemented source; the F6 nodes are accepted design awaiting implementation.

Things (id — name — kicker — status):

- `roles` — **Roles + practices** — PR #127 · draft — Implemented source · reviewed
  - Job: Seven responsibilities, Worker/Sidekick execution forms, progressive job packets, and one Verifier paired with each Implementer.
  - Boundary: This is source-reviewed work. Draft PR status does not establish installed behavior or earned trust.
  - Source: PR #127 · head 0bad422c
- `family` — **Five-document family** — PR #128 · draft — Implemented source · reviewed
  - Job: Requirements, Specification, Program Design, Test Design and Verification Design each own a different decision. Each has a local review, followed by integrated reading.
  - Boundary: The family is stacked above #127. Its new design return exists, but current planners still consume the older three-document basis.
  - Source: PR #128 · head ffe30964
- `run1` — **Main planner** — F6 · run 1 — Spec accepted · not started
  - Job: Plan Implementation reads all five documents and current closures, then schedules adopted test and verification strategy into slices, commands and prerequisites. It updates the canonical plan and actual consumers.
  - Boundary: It does not choose a replacement oracle or test layer for an adopted design. Missing strategy returns to the design owner.
  - Source: F6 spec · Run 1
- `run2` — **Improvement planner** — F6 · run 2 — Spec accepted · not started
  - Job: Plan Improve Repo adopts the same five-document basis for design-bearing findings and preserves positively source-proven mechanics-only work.
  - Boundary: Audit-only exits, direct plan-only results, validation receipts and triage stay distinct. An incomplete design is never a mechanics-only fallback.
  - Source: F6 spec · Run 2
- `proof` — **Structural proof** — Later · after implementation — Planned checks · not run
  - Job: Permanent source-contract checks, existing schema tests, and a real source walk through producer → planner → handoff → executor/review. The paired Verifier reports finite observations.
  - Boundary: String checks and schema parsing establish their structural claims; they do not show that a model follows the workflow.
  - Source: F6 spec · Proof and independent review
- `assessment` — **Lead assessment** — Later · after scoped proof — Own reading and interpretation
  - Job: The Lead opens the actual diff and decisive proof, checks them against the accepted need and design, and resolves every material gap before independent implementation review.
  - Boundary: A helper report or a green count cannot replace the Lead’s understanding of what was required and observed.
  - Source: F6 spec · Proof, step 4
- `review` — **Independent review** — Later · same relationships — Findings and correction closure
  - Job: Independent reviewers inspect the implemented skills, their teaching, calls, claims and consumer consistency. The same review relationship checks bounded corrections.
  - Boundary: Today’s accepted SPEC is not a review of an F6 implementation; none exists yet.
  - Source: F6 spec · Review and coordination
- `wrap` — **PR readiness** — Later · separate delivery — Owner retains merge authority
  - Job: Current PR head, checks, comments, review threads and mergeability are examined at the authorized delivery boundary.
  - Boundary: This diagram starts no work, grants no merge, and claims no client refresh or runtime behavior.
  - Source: F6 spec · Coordination and non-goals

Relationships (from → to):

- Roles + practices → Five-document family
- Five-document family → Main planner
- Main planner → Improvement planner
- Improvement planner → Structural proof
- Structural proof → Lead assessment
- Lead assessment → Independent review
- Independent review → PR readiness

## Perspective: What each design contributes to planning

This is the normal dependency path for an adopted substantial design unit. Every document has its own local review; the whole set has an integrated review. Early feasibility research can look ahead without changing who owns a decision.

Things (id — name — kicker — status):

- `requirements` — **Requirements** — WHY · for whom — Authorized outcome and boundary
  - Job: The Lead clarifies the consumer, adopted outcome, constraints and scope with the owner. Requirements receives its local review and owner confirmation or explicit waiver.
  - Boundary: No inferred preference becomes an authorized requirement.
  - Source: Five-document family · Requirements
- `specification` — **Specification** — WHAT · observable — Entities, invariants and obligations
  - Job: It defines what correct means through observable permitted and forbidden behavior, traced to Requirements, including domain identity and states.
  - Boundary: It does not choose internal modules, persistence shapes, test layers or mechanisms.
  - Source: Five-document family · Specification
- `program` — **Program Design** — HOW · structure — Owners, interfaces and proof seams
  - Job: It chooses responsibilities, interfaces, state, failure and concurrency realization. It names sufficient observation seams and what must stay real, with structural confirmation or waiver.
  - Boundary: Test Design chooses scenarios and layers within these structural constraints.
  - Source: Five-document family · Program Design
- `tests` — **Test Design** — WHICH wrong behavior — Scenario → real interaction → oracle
  - Job: It selects scenarios, independent oracles, the cheapest honest layers, substitutions, deterministic completion, and keep/repair/remove decisions.
  - Boundary: A planned red is owed evidence. It is not an executed failing test. The Implementer writes product tests.
  - Source: Five-document family · Test Design
- `verification` — **Verification Design** — HOW the real system shows it — Reference → route → observation
  - Job: It names the independent correctness reference, real build/configuration/client, outcome observations, passing conditions, reuse and limits.
  - Boundary: An acknowledgement, another target or a mocked path cannot prove the adopted outcome. Planned observations remain owed.
  - Source: Five-document family · Verification Design
- `integrated` — **Integrated review** — ALL FIVE · one coherent picture — Whole-set reading + final confirmation
  - Job: After five current local review closures, an independent reader reconstructs the whole set. The owner confirms or waives the final set.
  - Boundary: Five local verdicts cannot be relayed as the integrated reader’s judgment.
  - Source: Five-document family · orchestrator-design
- `payload` — **Ready design data** — PRODUCER owns validation — Five documents + closures + owed proof
  - Job: The cycle producer validates its own result. Planning receives its ready fields as data and opens the actual documents, confirmations and review evidence. Direct source-based planning verifies equivalent readiness.
  - Boundary: The phase never imports an orchestrator-owned validator. Shared document/closure schemas constrain shape, not semantic correctness.
  - Source: F6 spec · D15 and Run 1
- `planning` — **Planning** — F6 · adopted strategy — Schedule it, preserve its meaning
  - Job: Choose slices, concrete commands, dependencies, write scopes and earliest useful integration gates for the adopted strategy. Every owed test or observation is scheduled or returned as an exact gap.
  - Boundary: No silent fallback to three documents; no replacing real proof with a cheaper check that cannot observe the claim.
  - Source: F6 spec · D3 and Run 1
- `canonical` — **Canonical plan** — ONE execution authority — Durable path + basis + delivery intent
  - Job: One immutable Markdown plan links its current basis, proof obligations and intended delivery boundary. Handoff, implementation and review preserve the record.
  - Boundary: A plan-only result stays non-executable. Tickets, handoff and validation do not upgrade authority or create another plan.
  - Source: F6 spec · D5, D14 and D16

Relationships (from → to):

- Requirements → Specification
- Specification → Program Design
- Program Design → Test Design
- Test Design → Verification Design
- Verification Design → Integrated review
- Integrated review → Ready design data
- Ready design data → Planning
- Planning → Canonical plan

## Perspective: Who owns the work and the judgment

This view is role topology, not a dependency DAG. The dotted Implementer–Verifier link means shared facts and reproductions. Advice, observation, findings and acceptance are separate results.

Things (id — name — kicker — status):

- `owner` — **You** — OWNER — Meaning, tradeoffs and merge
  - Job: You build the governing mental model with the Lead, settle owner-controlled choices, confirm adopted design decisions and retain merge authority.
  - Boundary: You do not need to ferry routine logs or errors between helpers.
  - Source: Roles · Lead charter
- `advisor` — **🦉 Advisor** — CHALLENGE — Competing model and tradeoffs
  - Job: Challenge the Lead’s assumptions and the inference from observation to claim. Bring a credible competing picture with source evidence, gains, costs and who pays.
  - Boundary: Advice is not approval or independent review. It does not author governing design.
  - Source: Roles · Advisor charter
- `lead` — **🦁 Lead** — GOVERNING AUTHOR — Design, plan, integration and acceptance
  - Job: The owner’s main conversation. Writes the design and plan, cuts bounded assignments, reads decisive evidence, integrates results and accepts only claims it understands.
  - Boundary: It cannot become a delegator who forwards “done.” A Verifier may run the observation; interpretation stays here.
  - Source: Roles · Lead charter
- `support` — **📚 Researcher / 🔧 Operator** — BOUNDED HELP — Checked answers / prescribed observations
  - Job: The Researcher answers a supplied question from inspected evidence. The Operator scrapes, extracts, monitors or runs scripts under supplied criteria.
  - Boundary: Neither owns architecture or acceptance. Cheap models do not acquire implementation authority. Worker/Sidekick describe how these roles run.
  - Source: Roles · practices-manage-agents
- `implementer` — **🐒 Implementer** — SIDEKICK — Code + product tests + associated proof
  - Job: Execute the Lead’s bounded plan, write feature code and product regression tests, run associated proof, and correct accepted implementation defects. Every Implementer has a paired Verifier.
  - Boundary: Material design/plan meaning returns to the Lead. It does not author governing replacements.
  - Source: Roles · Implementer
- `verifier` — **🔬 Verifier** — PAIRED SIDEKICK — Outcome observation + test adequacy
  - Job: Challenge proof feasibility early, check pyramid/TDD/sensitivity, inspect applicable real-path evidence and observe the adopted outcome. Own only assigned verification drivers/scripts.
  - Boundary: No feature repairs or product-suite edits. Missing cases return to the Implementer; wrong expected meaning returns to the Lead. It does not accept delivery.
  - Source: Roles · Verifier charter
- `accept` — **🦁 Lead assessment** — BEFORE independent review — Required outcome ↔ actual observation
  - Job: Read the diff, the expected outcome and the actual target-bound observations. Determine whether the evidence establishes the claimed result and what remains open.
  - Boundary: Verifier judgment is evidence to inspect. A receipt can be real yet test the wrong requirement.
  - Source: Roles · Lead acceptance
- `reviewer` — **🔎 Reviewer** — INDEPENDENT · later — Code, contracts, tests and corrections
  - Job: Inspect ownership, types, state transitions, assertions and scope against governing sources. Detect tautological tests and assertions that invent restrictions. Verify corrected findings.
  - Boundary: No editing or delivery acceptance. Review does not replace the Verifier’s real-system observation.
  - Source: Roles · Reviewer charter

Relationships (from → to):

- You → 🦁 Lead
- 🦉 Advisor → 🦁 Lead
- 📚 Researcher / 🔧 Operator → 🦁 Lead
- 🦁 Lead → 🐒 Implementer
- 🦁 Lead → 🔬 Verifier
- 🐒 Implementer → 🔬 Verifier (facts)
- 🐒 Implementer → 🦁 Lead assessment
- 🔬 Verifier → 🦁 Lead assessment
- 🦁 Lead assessment → 🔎 Reviewer

## What exists today versus what is proposed

Implemented source today is only Roles + practices and Five-document family, with one relationship between them (Roles + practices → Five-document family).
The proposal adds: Main planner, Improvement planner, Structural proof, Lead assessment, Independent review, PR readiness. 'Added' means added to this plan, not implemented software.
