# Engine evidence additions: implementation plan

```text
plan path: docs/specs/2026-10-24-engine-evidence-additions/plans/2026-10-24-engine-evidence-additions.md (in sunstack-tools once copied; working copy in the Router scratch root)
originating planner: plan-implementation
supersedes: none
planning result: ready
governing planning basis:
  kind: reviewed-four-document-design
  Requirements, Specification, Program Design, Verification Design: requirements.md, specification.md, program-design.md, verification-design.md in this set
  review: engine-additions-design-review.md, correction-1, and closure (9 → 2 → 0, ready)
  current applicability anchors: sunstack-tools main c49d2f2f
delivery context:
  requested terminal: pr-ready-unmerged
  delivery grouping: single:engine-evidence-additions
  PR topology: one-pr
  repositories: sunstack-tools
planned at: sunstack-tools main c49d2f2f (confirm origin/main at start; re-anchor if it moved)
```

## Goal

Implement R42–R45 so that conduct rules over session logs and change rules over diffs run mechanically, with item locations, on the CLI built from this branch. Release goes through the repository's own changeset process; nothing is published from this branch.

## Scope and non-goals

- **In scope:** the owners the Program Design binds. That's `sunstack-evidence` (transcript normalization, conversation contracts and selection, evidence contracts, frozen queries, evidence admission) and `sunstack-rules` (definition admission, request state, request materialization, local check evaluation). Plus the generated schemas, the owner tests, and the design set copied verbatim into `docs/specs/2026-10-24-engine-evidence-additions/`.
- **Not in scope:**
  - lint input kinds (`sunstack-lint` stays document-only);
  - judged rules and the decision backend;
  - triggers or hooks;
  - any CI change;
  - publishing a release.

## Slices

| Slice | Obligation | Reads | Write surfaces | Focused proof (the wrong behavior it fails) | Layer, oracle |
| --- | --- | --- | --- | --- | --- |
| 0 | Set the home | the whole design set | `docs/specs/2026-10-24-engine-evidence-additions/`, copied verbatim, plans included | Files identical to the scratch copies (`cmp`) | static |
| 1 | R42 | Spec R42 and V36; PD's "Codex custom calls" row; `transcript_normalization.rs:297–388` | `transcript_normalization.rs` and its tests | Owner tests: an `exec` custom call is `shell` even when its input looks like a patch; `apply_patch` is `edit_file`; another name is `other`; an output pairs by call ID; an orphan output stays a gap; outcome is unknown unless the output states it. A name-blind or input-sniffing build fails | unit; oracle R42 |
| 2 | R43 | Spec E34 and R43; PD's "Input pattern" row; `conversation_contracts.rs:31,94`; `conversation_selection.rs:89`; `evidence_admission.rs:126` | `conversation_contracts.rs` (`Included.input_pattern`), `evidence_contracts.rs` (the `input_pattern` omission reason with its count), `conversation_selection.rs`, `evidence_admission.rs`, tests, schemas | Owner tests: a string input and a structured input, each with quotes and a newline, give the specified answer; the exclusion count counts only pattern rejects, separate from name and projection omissions; an invalid pattern refuses before any read. A build that matches escaped JSON for string inputs, or merges the omission counts, fails | unit; oracle R43 |
| 3 | R44 | Spec sides and R44; PD's "Sides" row; `evidence_contracts.rs:183`; `frozen_queries.rs:306` | `evidence_contracts.rs` (`MatchSide`, required `sides`), `frozen_queries.rs`, `evidence_admission.rs`, tests, schemas | Owner tests for each side value: `added` keeps only matches overlapping added ranges; `removed` keeps before-side overlaps; the two `*_file` values keep the whole side; missing and empty sides refuse. A whole-file build fails `added` | unit; oracle R44 |
| 4 | R45 | Spec E12, R45 and its settlement table, V41; PD's target-resolution, materialization and settlement rows; `request_state.rs:125,210`; `request_materialization.rs:69,180–187`; `local_check_evaluation.rs:37,69`; `occurrence_reporting.rs:103`; `definition_admission.rs:142–170`; `evidence_contracts.rs:375` | `evidence_contracts.rs` (`TargetLocation::ChangeMatches`; `ConversationRecords.records` with `{record_id, position}` as a hard cutover, updating every consumer and test), `request_state.rs`, `request_materialization.rs`, `local_check_evaluation.rs`, `definition_admission.rs`, tests, schemas | V41 truth table in `crates/sunstack-rules/tests/mechanical_execution.rs`, one test per R45 row, plus multiple items and a refused non-presence check. Builds that always apply `present: false`, ignore completeness, report only the first item, or admit another check each fail a row | unit and integration; oracle R45 table |
| 5 | V36–V40 on real inputs | Verification Design; the spike's saved requests (scratch `sunstack-evidence-spike/`) | Saved requests and reports under the branch's ignored `tmp/`; a scratch Git fixture outside every worktree | Each V row's passing condition on the CLI built from this branch, structured results compared, and zero model requests on `evaluate_rules` | real-path; oracles: the named transcript lines and fixtures |
| 6 | Release record | the repository's release and changeset process (`package.json`, `.changeset/`, `AGENTS.md`) | one changeset entry | The repository's own release check passes; nothing is published | static |

## Dependency edges

- Slice 0 comes first.
- Slices 1 to 4 run in order: slices 2, 3 and 4 all change `evidence_contracts.rs`, and slice 4 consumes slices 2 and 3's selections.
- Slice 5 comes after slice 4. Slice 6 comes last.

## Gates

- **Per slice:** the repository's own checks, read from `AGENTS.md` and `package.json` (for example `cargo test --workspace`, `cargo clippy` with the workspace lints, `cargo fmt --check`), plus schema regeneration with no drift, plus the slice's focused proof.
- **Batch boundary:** after slice 5. The paired Verifier observes slices 0–5 once, at the declared head.
- **Review:** the Lead assesses the diff, then one independent implementation review by Sol xhigh. Corrections are made as one pass per review result.
- **PR:** `implementation-pr-wrapup`. The owner merges.

## Risks and stop conditions

- **Stop:** the hard cutover of `ConversationRecords` breaks a consumer outside the named owners, for example the lint report or the CLI schema in a way the design doesn't name. Return the consumer and its anchor to the Lead.
- **Stop:** an anchor has moved on `origin/main` so a binding no longer applies. Return it with the evidence.
- **Stop:** a V row can't be earned as written (`verification-design-gap`).
- **Risk:** real transcripts contain private content. Keep reports under the ignored `tmp/` and the Router scratch root, and quote the minimum.
