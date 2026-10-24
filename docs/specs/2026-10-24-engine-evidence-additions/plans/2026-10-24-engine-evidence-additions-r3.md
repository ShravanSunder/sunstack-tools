# Engine evidence additions: implementation plan, revised at slice 4

```text
plan path: docs/specs/2026-10-24-engine-evidence-additions/plans/2026-10-24-engine-evidence-additions-r3.md (in sunstack-tools once copied; working copy in the Router scratch root)
originating planner: plan-update
supersedes: docs/specs/2026-10-24-engine-evidence-additions/plans/2026-10-24-engine-evidence-additions-r2.md
planning result: ready
governing planning basis:
  kind: reviewed-four-document-design
  Requirements, Specification, Program Design, Verification Design: this set, as corrected in delivery
  review: design review 9 → 2 → 0 (ready); delivery corrections verified in engine-additions-design-review-delivery-corrections.md and engine-additions-design-review-delivery-corrections-2.md (0 open, no new round)
  current applicability anchors: sunstack-tools main c49d2f2f
delivery context:
  requested terminal: pr-ready-unmerged
  delivery grouping: single:engine-evidence-additions
  PR topology: one-pr
  repositories: sunstack-tools
planned at: branch feat/engine-evidence-additions, HEAD 0210a2b9 (slices 0–4 in progress or done)
```

## Changed from r2

| Change | Evidence | What it invalidates |
| --- | --- | --- |
| Slice 4 writes `crates/sunstack-rules/src/request_planning/change_selection.rs` (`finish_change_selection`). A mechanical presence rule whose target is its own change-set selection skips the change-scope comparison, and settlement applies R45's table. `change_comparison.rs` is unchanged | Implementer's slice 4 break, with a 9/10 log ending in `selection_failed/discovery_limit`. Program Design row "Change-scope comparison". Reviewer closure 2 | Slice 4's write surfaces. Nothing completed earlier |
| Slice 4 writes `crates/sunstack-rules/src/rule_source_decoding.rs` (`:147–148`). For a presence check targeting a newly admitted selection, the early check is skipped, and `validate_rule_definition` still owns the pairing | The Implementer's `mechanical_admission` red, `InvalidMechanicalCheck`. Program Design row "Markdown-sourced rule decoding". Reviewer closure 2 | Slice 4's write surfaces |
| V41's owner tests add three things: the real-Git limited cases (a retained match with `present: false` and `present: true`, and an empty limited selection); the judged peer that still refuses `DiscoveryLimit`; and admission through a real `rule_sources` RuleSet. That admission accepts `present: false` on change-set and conversation targets and still refuses it on Markdown and frontmatter targets | Verification Design's V41 row as corrected | Slice 4's focused proof |
| The design set changed again | Both closures | Slice 0. Requalify by copying the current set and all three plans verbatim, then `cmp` each |

Everything else in r2 stands: slices 1–3, slice 5 on the frozen inputs with the companion `prepare_evidence` counts, the schema gate, no release record, the gates and the stop conditions.

## Slice 4, as it now reads

| Slice | Obligation | Write surfaces | Focused proof (the wrong behavior it fails) | Layer, oracle |
| --- | --- | --- | --- | --- |
| 4 | R45 | `evidence_contracts.rs`, `request_state.rs`, `request_materialization.rs`, `local_check_evaluation.rs`, `definition_admission.rs`, `change_selection.rs`, `rule_source_decoding.rs`, `crates/sunstack-lint/src/outcome.rs` (the existing `None` arm only), tests, reviewed schema contracts | The V41 truth table, one test per R45 row, plus: multiple items; a refused non-presence check; the real-Git limited cases; the judged peer's unchanged refusal; and `rule_sources` admission accepting the new targets and refusing the old ones. Each of these fails a wrong build: one that always applies `present: false`, ignores completeness, reports only the first item, admits another check, refuses a found match on a limited selection, or loosens the old refusals | unit and integration; oracle: the R45 table and Program Design's two delivery rows |
