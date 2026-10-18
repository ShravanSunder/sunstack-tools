---
name: orchestrator-design
description: Use when a user asks to run, resume, or finish one bounded Requirements, Specification, Program Design, and independent three-artifact design cycle before planning. Not for a direct design phase, long-horizon delivery, implementation, or PR work.
---

# Design Orchestration

The orchestrator advances the smallest design owner that can resolve the work, verifies its material result against current evidence, and continues until the three artifacts form one coherent reviewed design or an owner decision or blocker stops the run. This skill owns coordination and the final claim. The user-facing orchestrator loads `spec-design` and `program-design` in this session and authors those artifacts with the user. Those skills own phase judgment; they are not a reason to spawn a writer. Independent review is assigned out through `spec-program-review`.

```text
spec-design          Requirements and observable Specification
program-design       structural realization
spec-program-review  independent three-artifact review
discuss-pathfinding  genuinely unwritten owner meaning
```

MUST load `../../shared-references/requirements-specification-program-design.md` and return the separate concept and identity boundaries used to validate every downstream handoff.

MUST load `practices-show-me-your-work` once at the start and return the shared work reference (or explicitly unshared checkpoint path) plus whether this workflow is responsible for the whole work or contributes to it. Reuse supplied work context for nested work. The tracker owns recording, fallback, and conditional readable views. If tracking cannot initialize, report the gap and continue independent design/review work; tracking availability is not a review-admission prerequisite unless the user explicitly made it one.

MUST load `practices-preflight` at entry and return `ready` or the gap list; route only the work no open gap blocks.

Record only consequential design decisions, owner confirmations, accepted or rejected review findings, corrections, results with evidence, and unresolved meaning. A historical event helps orientation but never proves the current artifact or review state. User corrections append a record referencing the earlier record; do not rewrite history.

## Orient And Route

1. Reopen the current user intent, governing sources, Requirements, Specification, Program Design, and any inspectable current phase or review result. Use the trail to find relevant context, then verify load-bearing claims in current sources.
2. IF assigning research or independent review, load `practices-manage-agents` and return the selected responsibility, continuity, and assignment boundary. Do not assign any helper, Worker or Sidekick, as Requirements, Specification, or Program Design author. The orchestrator retains material design work with the user and writes those artifacts in this session.
3. Invoke the smallest owner that can advance the current evidence. Invoke means this session loads that skill. Do not spawn an executor to be the phase owner. Only independent review is assigned out:
   - fresh full-design request, or missing Requirements or Specification -> `spec-design`;
   - current Requirements and Specification but missing Program Design -> `program-design`;
   - three current artifacts without completed review coverage -> assign or resume the persistent independent 🔎 Reviewer Sidekick through `spec-program-review` in `three-artifact-design` mode;
   - genuinely unmade owner meaning -> `discuss-pathfinding`, returning to the phase that owns it.

If the user says to write the spec, or not a subagent, write immediately. Loading `spec-design` or `program-design` in this session is required. Starting a skill-legalization patch instead of writing is the failure. Portability does not transfer authorship: only explicit user direction designating a named successor Lead and the transferred design or planning scope permits another session to continue that governing authorship. Record that direction in the successor packet. Without all three signals, a helper, executor, role label, handoff, or assistant continuation returns the authoring gap to the current Lead.

| rationalization | reality |
| --- | --- |
| "spec-design owns artifacts, so spawn a Sidekick" | those skills own judgment; this session loads them and writes |
| "the executor writes the spec" | executor means implementation, research, or review support, not design authorship |
| "the section is already settled, so delegation is only wording" | selecting structure and prose for governing content is authorship; helpers return evidence, not sections |
| "the handoff says continue, so this session is the new Lead" | a successor transfer also needs the named recipient, transferred scope, and explicit user direction |
| "I'll check the instructions first" | "you write it" is already the instruction; load the phase skill and write |

4. Preserve each producer's current route and compact handoff. Accept only `discuss-pathfinding | spec-design | program-design | spec-program-review | stop`, or a return token from a lower level; a contradictory destination blocks without the orchestrator inventing a route. MUST load `../../shared-references/phase-return-tokens.md` and map a returned token to its next owner: `requirements-gap` -> `discuss-pathfinding`; `specification-gap` -> `spec-design`; `program-design-gap` -> `program-design`; `ready-for-planning` -> design terminal, then `plan-implementation` only when continued delivery was requested. Mapping names the next owner; it never widens the requested task, so a review-only or phase-only request ends at the returned token.
5. Verify what materially changed before continuing. Check artifact identity and resolution, returned owner, current evidence, and the finding or decision that justifies the next route. For each newly authored or substantively revised substantial Requirements, Specification and Program Design artifact, inspect that phase's visual result from the shared rendering owner: its reader question, selected medium, durable relative embed when generated, semantic/readability inspection and exact capability or preview gap. Reject a pass whose medium is fenced plain text for a picture-type view when Mermaid can render. Requirements coverage is each changed screen, not one overview image. One chat image or another artifact's visual does not establish all three artifacts' coverage. Require exact source identity where correctness depends on it, such as a reviewed commit or PR head; ordinary decisions need no opaque identity.
6. Route a `specification-gap` to `spec-design`, an owner-controlled structural choice through `discuss-pathfinding` back to `program-design`, a bounded How correction to `program-design`, and a blocker or owner decision to the exact stop. Continue across phase boundaries instead of returning a progress checkpoint as completion.

For newly created file-backed artifacts, pass `new artifact home: <project-root>/docs/specs/` to `spec-design` and `program-design` and validate their distinct returned paths beneath that home. Preserve authoritative pre-existing artifacts wherever they already live. Decision records and linked detail may live in the central trail; do not create a second lifecycle ledger, replay store, digest, generic approval record, or per-turn report in the project.

## Review And Correction Rounds

Prefer one review-and-correction round. Further rounds follow `spec-program-review`'s convergence rule in its `references/finding-and-reduction-schema.md`. Pedantic, stylistic, or already-satisfied findings do not justify another round. IF the review returns `not-converging`, load `../../shared-references/owner-decision-brief.md` and return a brief of what keeps failing.

Reduce findings against the sources:

- `ready` -> design terminal;
- non-semantic or unsupported finding -> reject with evidence;
- IF a mental-model break or unmade owner meaning appears, load `../../shared-references/owner-decision-brief.md` and return the brief with the failed assumption, evidence, and consequence; dependent work stops;
- accepted bounded findings -> correct through the semantic owners; the retained independent 🔎 Reviewer Sidekick checks corrected anchors and current affected evidence, then the orchestrator accepts the result.

Each round may call `spec-design` then `program-design`, correcting each affected artifact once. The retained 🔎 Reviewer Sidekick verifies those corrections within the same review relationship; this verification does not start another full review or count as a round. Pre-review authoring does not consume a review round. `spec-program-review` owns review admission and coverage.

## Missing Review Evidence

If prior review results are unavailable, verify current artifacts and governing sources, record why, and run an ordinary review; the current review sets the baseline. Route design breaks to their owner.

## Trail And Completion

At each owner return, record a meaningful checkpoint with decision, reason, evidence, and result through the tracker. At every terminal response, leave the actual design outcome and continuation context, including blocked, deferred, or stopped work. Nested work contributes without resolving the outer thread; the responsible whole-work agent follows the tracker's current-history and completion checks before resolution. Produce a readable view when requested or substantial synthesis is needed. Report unshared fallback or view gaps honestly; preserve the real design result and continue independent work unless shared recording was made a delivery gate.

Return one status and the next skill or stop. `ready` requires distinct, current Requirements, Specification, and Program Design artifacts, independent review, retained 🔎 Reviewer Sidekick verification of corrected anchors and current affected evidence, and orchestrator acceptance. It means design-ready, not implemented.

## Design-Ready Continuation

When the caller or user has requested continued delivery, preserve this design terminal and the next phase's existing input/result contract for the delivery loop; the orchestrator does not implement or claim delivery complete. The reviewed artifacts, task boundary, completion or escalation, and design proof/results remain authoritative. Use `plan-handoff` only when a current implementation plan needs portability, or `spec-handoff` only when reviewed design context needs portability before a plan exists. Do not require either handoff merely because the executor uses a separate session, and do not create a management packet schema, route, or work trail.

For a substantive design loop that uses agent help, the orchestrator uses the owner's model choice. It works with the user on material design meaning and authors every governing section, diagram, artifact and visual brief through this workflow. Helpers are chosen through `practices-manage-agents` and return evidence only. A tool may realize pixels, typography, spacing and style from an unchanged orchestrator-authored brief, while the orchestrator owns semantic composition, labels, relationships, invariants and acceptance. A helper may run only the prescribed generation/copy/preview procedure and report its result. Advice is neither authorship nor acceptance.

If continued delivery is requested and no ready plan exists, the orchestrator MUST load `plan-implementation` in this session to author the implementation plan before commissioning implementation, and return `ready | revision-requested | blocked`. Once the plan is ready, MUST load `practices-manage-agents` to assign or resume one persistent 🐒 Implementer Sidekick per useful planned PR assignment, and return the selected responsibility, continuity, and assignment boundary; apply and verify a title under the session-title convention `practices-manage-agents` owns, or report the title capability gap. The Lead remains the default user conversation; the user may explicitly choose direct contact with the assigned Sidekick inside its scope. In either branch, that Sidekick owns direct implementation, associated proof, and corrections by default. It returns material design/plan questions, cross-assignment integration conflicts, permission boundaries, and concise completion evidence to the orchestrator; the Lead does not relay every internal progress turn or poll merely to keep the conversation active. Returning to the Lead for conversation does not pause authorized implementation or transfer execution ownership. The Lead retains all governing Requirements, Specification, Program Design, diagrams, and implementation-plan authorship plus material decisions, integration, assessment, and acceptance.

The orchestrator keeps the coordination root and each execution root as `orchestrator`; each 🐒 Implementer Sidekick joins only its execution root as `implementer`. Board seats describe thread participation, not authority; `practices-collaboration` owns the role-to-seat mapping, and `practices-manage-agents` owns the concrete-benefit delegation test. Design or plan gaps return to the orchestrator and user. Independent PR assignments may proceed concurrently; dependent assignments wait for verified prerequisites.
