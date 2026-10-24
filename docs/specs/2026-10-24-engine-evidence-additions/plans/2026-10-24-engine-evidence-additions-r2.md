# Engine evidence additions: implementation plan, revised in delivery

```text
plan path: docs/specs/2026-10-24-engine-evidence-additions/plans/2026-10-24-engine-evidence-additions-r2.md (in sunstack-tools once copied; working copy in the Router scratch root)
originating planner: plan-update
supersedes: docs/specs/2026-10-24-engine-evidence-additions/plans/2026-10-24-engine-evidence-additions.md
planning result: ready
governing planning basis:
  kind: reviewed-four-document-design
  Requirements, Specification, Program Design, Verification Design: requirements.md, specification.md, program-design.md, verification-design.md in this set, as corrected in delivery
  review: engine-additions-design-review.md, correction-1, closure (9 → 2 → 0, ready), and engine-additions-design-review-delivery-corrections.md (delivery corrections verified, 0 open, no new round)
  current applicability anchors: sunstack-tools main c49d2f2f
delivery context:
  requested terminal: pr-ready-unmerged
  delivery grouping: single:engine-evidence-additions
  PR topology: one-pr
  repositories: sunstack-tools
planned at: branch feat/engine-evidence-additions, slices 0–3 in progress or done
```

## Changed from the first plan

| Change | Evidence | What it invalidates |
| --- | --- | --- |
| The design set was corrected in delivery: R42's outcome reads only a native status field; V36's oracle is unknown status; V37 selects `shell` and `edit_file` against a frozen control; V37 and V38's exclusion counts come from a companion `prepare_evidence`; real inputs run as frozen prefixes; the requirements' account of Codex edits | Verifier `engine-additions-verifier-feasibility.md` and its addendum; Implementer findings at `report_contracts.rs:450–463`; Reviewer closure of the corrections | Slice 0's copy (requalify: copy the corrected set and this plan, `cmp` identical). Slice 1's R42 tests if they assumed an outcome from text (requalify: the slice 1 owner tests include an output whose content holds a nested `exit_code` and no native status, giving unknown status). Slice 5's V36–V38 runs |
| Slice 4 writes one more file: `crates/sunstack-lint/src/outcome.rs:64–78`, where `TargetLocation::ChangeMatches` joins the existing arm that returns `None` | Implementer preflight; Lead authorized | Nothing completed. Lint's inputs, refusals and reports are unchanged; its existing tests stay unchanged and passing |
| The per-slice schema gate follows the repository's way: update each affected reviewed contract in `docs/specs/2026-10-12-rust-rule-engine/schemas/`; the owner contract-schema tests validate against both the reviewed and the `schemars`-derived schemas; `scripts/release/export-lint-schema.sh` reruns unchanged with no drift. No generator is added | `tests/support/contract_schemas.rs:9–43`; no engine schema generator exists | Each slice's gate wording; nothing completed changes |
| Slice 6 (release record) is removed. A Rust crate change in this repository carries no release record: the workspace version has stayed 0.1.0 since #55, and `.changeset` covers Demistify only. Release stays the owner's separate cargo-dist step, and the PR description says so | `.changeset/README.md`, `Cargo.toml:11–12`, `git log -- crates/` | The goal's "release goes through the repository's changeset process" sentence. Nothing completed |
| Slice 5 runs on the Verifier's frozen inputs (`engine-additions-verifier-frozen-inputs.json` in the scratch root), including V37's control `engine-v37-control-01a16f99-frozen.jsonl` | The control session is live again and its rollout grows | Nothing completed |

Slices 1–3 keep their evidence, except the slice 1 requalification above.

## Goal

Implement R42–R45 so that conduct rules over session logs and change rules over diffs run mechanically, with item locations, on the CLI built from this branch. This PR adds no release record; releasing is the owner's separate step.

## Slices

| Slice | Obligation | Write surfaces | Focused proof (the wrong behavior it fails) | Layer, oracle |
| --- | --- | --- | --- | --- |
| 0 | Set the home | `docs/specs/2026-10-24-engine-evidence-additions/`, the corrected set and both plans | Files identical to the scratch copies (`cmp`) | static |
| 1 | R42 | as before | as before, plus: a custom output with a nested `exit_code` and no native status gives result present, unknown status. A build that parses the nested text fails | unit; oracle R42 as corrected |
| 2 | R43 | as before | as before | unit; oracle R43 |
| 3 | R44 | as before | as before | unit; oracle R44 |
| 4 | R45 | as before, plus `crates/sunstack-lint/src/outcome.rs` (the existing `None` arm only) | as before; lint's existing tests unchanged and passing | unit and integration; oracle R45 table |
| 5 | V36–V40 on frozen real inputs | saved requests and reports under the branch's ignored `tmp/`; a scratch Git fixture outside every worktree | Each V row's passing condition as corrected. `evaluate_rules` gives verdicts, every item location and zero model requests and inference attempts. Companion `prepare_evidence` runs give V37's 107 and V38's 4. Each report records its frozen source's byte length and sha256 | real-path; oracles: the frozen inputs and the fixture |

The first plan's slice rows, reads and dependency edges stand wherever this table says "as before".

## Gates

- **Per slice:** the repository's checks (`cargo test --workspace`, `cargo clippy` with the workspace lints, `cargo fmt --check`), the schema gate above, and the slice's focused proof.
- **Batch boundary:** after slice 5. The paired Verifier observes slices 0–5 once, at the declared head.
- **Review:** the Lead assesses the diff, then one independent implementation review by Sol xhigh. Corrections are made as one pass per review result.
- **PR:** `implementation-pr-wrapup`. The PR description names the owner's release step. The owner merges.

## Stop conditions

Unchanged from the first plan, except that slice 6's are removed.
