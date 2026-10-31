# Requirements: the lint report as a nudge

31 Oct 2026. Author: 🦁 Lead. Draft, not yet reviewed.

This is a normalized Requirements record for an engine change. Every row comes from already-settled meaning: the accepted skills design set "rules as nudges" in `ai-tools` (branch `skills/rules-as-nudges`, accepted-to-implement at `398d2190`, folder `docs/wip/skills-authoring/2026-10-31-rules-as-nudges/`) and the owner's words quoted there. No row adds owner meaning. The set is one concern, the lint report and the evidence behind it, so it carries no capability level.

## Who this is for

| Class | Who | What they need from the engine |
| --- | --- | --- |
| Receiving agents (direct users, through a runner) | the agent that runs a rule over skill text: a writer, a review lead, a Lead, an orchestrator, an auditor, a rule's writer or reviewer | a report that tells them what a rule saw, where, why it matters, how it was decided, and where else to read; and what the run couldn't see |
| Runner maintainers (developer users) | whoever keeps a caller such as `ai-tools`' `tests/skills/lib/skill-rules/report.ts` | report fields they can read and print without guessing |
| Rule authors (developer users) | whoever writes rulebooks, evidence projects and rule files | chunks that hold what a question needs, and JSON files that can be checked like Markdown |
| Other lint callers (stakeholders) | anyone running `sunstack lint` with their own config | a generic engine whose existing fields keep their meaning |
| The owner (decision authority) | Shravan | the purpose: rules are nudges; the engine stays generic |

## The need, in the owner's words

From the accepted skills set, all 31 Oct 2026:

- "rules are in enses nduges to leads and reviwe agetsn or diffrent rkoesl"
- "even the sunhstack litning tool which sir ealted to skill is about providign formation to a agentic reviwer and the direcitons of reateidnh locations"
- "our system was to be generic enokugh that any evidence and rules shoudl have been able to go in, so why would we not able to extend it for ruels too"
- "we have rules that need to check json to amek srue its doen well thesrar eall nduges to agesnt"
- On the three kinds of "where else to read": "yes this amke sense"
- On going ahead: "after ferficaiotn yoku can star"

And from the engine's own 12 Oct requirements (`docs/specs/2026-10-12-rust-rule-engine/requirements.md`, U26): "the severity should be by rule, can have a default severity of nudge unless overridden"; the runner spec renamed `nudge` to `warn` on 18 Oct.

## Today, and the gap

A rule's result reaches an agent today as one file and line, a message and a severity. The report doesn't say where else to read, keeps no answers on a pass or a gate's exclusion, points a section-sized finding at the section's first line, hides that a model view was cut, and attaches a child section's loads to its parents. Lint can't read JSON evidence, so rule files can't be checked. The evidence for each is in the skills set's requirements (evidence 2 to 10) and the 🦉 Advisor's probes on the signed 0.5.0 engine (`advisor-view-2.md`, `spike-1/`, `probe-2x2/` in the owner's memory-logs).

The desired difference: a receiving agent can act on a result as a nudge, and a rule's evidence holds what its question needs, without the engine learning what a skill is.

## Requirements

Each row is `authorized` (its source is the accepted skills set, accepted by the independent design review and released for work by the owner on 31 Oct). Priority: all rows are needed for Gate 1 of the skills set's work plan, except U7, which no accepted rule depends on yet (see its note). The Lead assigned this priority from the accepted set's decisions.

| U | Need | Why it matters | Source |
| --- | --- | --- | --- |
| U1 | A finding carries the related places the run read, each labelled with what it is: the standard's own line, the loaded file, the enclosing section, the caller. A place the run never read stays out of these links. | The receiving agent reads them before deciding; the owner confirmed the three kinds of "where else to read". | skills R1; C17 |
| U2 | A finding on a section-sized subject can point at the line it's about. | Today the line-17 nudge lands on the section's first line (the spike, line 7), so the agent is told "this section", not "this line". | skills C14 (D20) |
| U3 | Every judged result keeps its answers, passes and gate exclusions included. | An agent can see why a gate dropped a piece and challenge a wrong exclusion. | skills R5b; C4 |
| U4 | Every result says whether it rests only on mechanical checks or on a model's answer somewhere on its path. | A probability supports an answer, it doesn't establish a fact; the agent needs to know which. | skills R1 (basis); C1 |
| U5 | A piece says when the model saw less than was collected. | Collected completeness and model-visible completeness are different; a cut must show as a gap, not as "fine". | skills R2; C5 |
| U6 | A section's loads are the loads in its own text, not its child sections'. | Today one child's load attaches to the H1, the parent and the child at once, blaming the wrong section. | skills spec "What the engine must do"; Advisor probe |
| U7 | A rule can ask for the full text of the section a piece sits in, and for a loaded reference's fields, not only its opening. | Listed by the accepted skills specification for chunks that need more context. No accepted rule depends on it yet: placement uses the loaded file's opening (D18b, D18c). | skills spec "What the engine must do" |
| U8 | Lint can check JSON files with rules, the same way it checks Markdown. | Rule files are JSON and need their own nudges; the engine already collects JSON natively. | skills R8; C6, C7 |
| U9 | A rule's authored severity is kept: `warn` is the nudge and doesn't fail a run unless the caller sets a warning limit; `error` stops the run; `uncertain` keeps its own status. The engine adds no new level. | The owner set severity per rule; `nudge` was renamed `warn`. | skills R5a; C1, C2; engine U26, runner U4 |
| U10 | The engine stays generic: chunks, related places and anchors are declared by the caller's config and rules; nothing in the engine knows what a skill, a placement rule or a plugin is. | "our system was to be generic enokugh that any evidence and rules shoudl have been able to go in". | skills R9 |

## Goal boundary

- **Goal:** the lint report carries what a receiving agent needs to act on a nudge, and lint's evidence can hold what a rule's question needs, including JSON files.
- **Foundation reused:** `sunstack lint`, its rulebook, evidence projects, field sources and report; the native JSON collector behind `prepare_evidence`.
- **May change:** `sunstack-tools` crates for lint, evidence, rules and reporting; the CLI's report formats; the engine's docs. Released as 0.6.0 under the Lead's engine grant.
- **Protected:** existing report fields keep their meaning; the native protocol operations keep their contracts.
- **Non-goals:**
  - a new severity level or a claim field (D21);
  - a per-rule coverage field: the runner derives "unread" from the denominator, counts and results (C3);
  - carrying a skill's description as context on every chunk (D18b);
  - per-load units with per-target attribution (a later refinement, C12);
  - following loads more than one hop; reading another repository;
  - judging agent behavior (evals, later);
  - `AGENTS.md` as a source, which is config work after this change.
- **Acceptable evidence:** engine tests on fixtures for each row; a sample report showing every new field; the skills set's rule proof after the release.
- **Open:** nothing that blocks; U7's need is carried as written, sequenced last.

## Path to the other documents

Requirements → [Specification](specification.md) → Program Design → Verification Design, each a separate file in this folder.
