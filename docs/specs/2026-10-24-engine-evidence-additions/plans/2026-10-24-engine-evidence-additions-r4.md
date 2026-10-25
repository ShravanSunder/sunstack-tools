# Engine evidence additions: implementation plan, revised after implementation review

```text
plan path: docs/specs/2026-10-24-engine-evidence-additions/plans/2026-10-24-engine-evidence-additions-r4.md (in sunstack-tools once copied; working copy in the Router scratch root)
originating planner: plan-update
supersedes: docs/specs/2026-10-24-engine-evidence-additions/plans/2026-10-24-engine-evidence-additions-r3.md
planning result: ready
governing planning basis:
  kind: reviewed-four-document-design
  Requirements, Specification, Program Design, Verification Design: this set, including the completeness amendment (Specification R45 completeness; Program Design "Claude empty thinking", Local settlement and Failure; Verification Design V41)
  review: design review 9 → 2 → 0 (ready); delivery corrections 1 and 2 (0 open); implementation review engine-additions-implementation-review.md (needs-revision: IR1 blocker, IR2 important; the amendment admitted)
  current applicability anchors: sunstack-tools main c49d2f2f
delivery context:
  requested terminal: pr-ready-unmerged
  delivery grouping: single:engine-evidence-additions
  PR topology: one-pr
  repositories: sunstack-tools
planned at: branch feat/engine-evidence-additions, HEAD 4ddee364 (slices 0–5 done; declaration withdrawn)
```

## Changed from r3

| Change | Evidence | What it invalidates |
| --- | --- | --- |
| **IR1.** `local_check_evaluation.rs` (the completeness check at `:38–58`, the empty-selection branch at `:94`): a conversation capture's `MalformedRecord` gaps make an empty selection incomplete, exit 3, for both `present` values. Nonempty rows are unchanged. Unsupported records, unknown injections and ambiguous turns stay positioned and don't gate | Implementer's malformed probe; implementation review IR1; Specification R45 completeness | R45 conversation completeness, V41's completeness claim, and every absence claim over a capture with a malformed gap |
| **IR2.** `transcript_normalization.rs` (Claude thinking read at `:202`, via `:16`): a thinking block with an empty `thinking` string normalizes as a thinking block. Wrong-type and missing-field diagnostics and every other adapter are unchanged | The gap breakdown (all 4,935 real malformed records are this shape); implementation review IR2; Program Design "Claude empty thinking" | Claude normalization for that shape; the V38 control's completeness and pass; slice 5's V38 rows |
| The design set changed again (the completeness amendment) | Lead push `01a1705d-0e33…`; admitted in the implementation review | Slice 0: copy the current set and all four plans verbatim, then `cmp` each |

## Slice 6: the correction pass

| Slice | Obligation | Write surfaces | Focused proof (the wrong behavior it fails) | Layer, oracle |
| --- | --- | --- | --- | --- |
| 6 | R45 completeness (IR1), Claude empty thinking (IR2) | `local_check_evaluation.rs`, `transcript_normalization.rs`, their owner tests, and the design and plan copies | **Permanent V41 cases:** a newline-terminated invalid-JSON record gives incomplete (exit 3) for an empty selection under both `present` values, and keeps its row for a nonempty one, with zero model requests and inference attempts. An empty-thinking Claude record normalizes with no `MalformedRecord`. A build that ignores capture gaps, gates on every gap, or still rejects empty thinking fails a case. **Requalify slice 5 on the frozen inputs:** the V38 positive and control show zero malformed gaps, the same four positive IDs and positions failing, the control passing with four pattern exclusions, and zero model requests and inference attempts; the malformed CLI control returns incomplete. The nearby protected cases still pass: nonempty and multiple locations, the limited-change cases, the judged peer, and the nongating unsupported gaps | unit and real-path; oracles: Specification R45 completeness, Program Design rows, frozen inputs |

## Gates

- **Slice 6:** `cargo nextest run --locked --workspace`, `cargo clippy` with `-D warnings`, `cargo fmt --check`, and the schema gate. Record the schema export and compare command exits. Use a scratch `CARGO_TARGET_DIR` for any run that mustn't touch the declared binary.
- **Then:** declare a new frozen target (commit, binary built with the declared command, its hash, config) to the paired Verifier and the Lead. The Verifier observes V36–V41, including the completeness cases, once at that target. The same Reviewer then reopens every corrected file and the affected seams in a new meaningful review. Then `implementation-pr-wrapup`; the owner merges.

## Stop conditions

Unchanged from r3. A correction that would need a broader gap policy, a new gap kind or a changed public shape stops and returns to the Lead.
