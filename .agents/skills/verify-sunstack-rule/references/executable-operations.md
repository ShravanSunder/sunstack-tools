# Executable operations — milestone 1

Drive the real `sunstack-rule` process at SKILL.md's pinned product target. Oracle: Specification C14/V2/V13 and exact contracts, read with the main skill's pinned `git show` method. Each cited Specification row is the full checklist; the controls below do not narrow it. V15 artifact integrity is release proof, open unless separately requested.

Inputs: declared checkout/build/config/client, explicit CWD, and an authorized evidence directory. Return the run ID, declared binary/readiness result, relative transcript/cursor path control result, actual process stdout/stderr/exits, operation coverage and proof limits. The established route is `crates/sunstack-rule-cli/README.md`, permanent tests and `crates/sunstack-rule-cli/tests/support/mod.rs`; `crates/sunstack-rule-cli/tests/PHASE1-PROOF.md` is historical two-operation proof, while `crates/sunstack-rule-cli/tests/PHASE2-PROOF.md` describes the integrated target.

## Build and run locally

From the monorepo root, apply the SDK prefix in `SKILL.md` to each Cargo command:

```sh
cargo build -p sunstack-rule-cli
cargo nextest run -p sunstack-rule-cli
```

The default package run executes 25 local cases (24 process tests and one native-enum serializer seam); four CLI cases remain ignored: two hosted journeys, the measurement workload and an offline diagnosis. Apply `library-gates.md`'s same-run reuse rule and LEAK interpretation; require exit 0.

The harness starts `CARGO_BIN_EXE_sunstack-rule` through the repository's absolute `scripts/cargo-debug-signing-runner.sh`, with real pipes and explicit child CWD. Local helpers inject a synthetic environment credential and use real loopback HTTP fixtures. They retain the actual binary/library/filesystem/serializer boundaries and await each child/server. They do not dispatch to hosted Jev.

For an authorized manual invocation with a complete caller-supplied request file, the README's route is `scripts/cargo-debug-signing-runner.sh target/debug/sunstack-rule < request.json`. `request.json` means that supplied file, not a committed fixture. It must wrap a full native operation input in the versioned envelope. The permanent tests construct valid complete inputs and are the reproducible recipe above.

## Observe framing, readiness and outcomes

The process installs signal handlers, writes `sunstack-rule: ready` to stderr, then reads one JSON request on stdin. Registration is not operation completion. Require exactly one complete parsed response and its child exit. `sunstack-rule: cancellation observed` is a separate signal-consumption witness. A killed process, missing/truncated JSON or an empty exit 0 is execution failure.

Raw stdin is bounded at **8,388,608 bytes**; the next byte refuses with `input_limit` without waiting for EOF or parsing. Large evidence remains bound by path. Unknown operation/version, malformed/concatenated framing and duplicate/unknown envelope fields yield `protocol_error`, exit 2, no fabricated known-operation result. Success framing is `operation_result` with protocol_version 1 and `result: { operation, outcome }`.

| Operation | Exit 0 | Exit 2 | Exit 3 | Exit 4 |
| --- | --- | --- | --- | --- |
| `prepare_evidence` | `prepared`, requested selected content/captures/cursors/measurements | `refused` | `incomplete` for required refusal; optional refusal remains prepared/0 | `cancelled`, before or after plan admission |
| `prepare_rules` | `prepared { preparation }` | native `invalid` | real `prepared_with_failures`; adopted V2 method below | `cancelled`, correct preparation progress |
| `run_judgments` | `completed`, exact correlated native answers/receipt/attempts | `invalid_request`, budget refusal with zero dispatch | `provider_failure` or `invalid_response`, no answers fabricated | `cancelled`, attempts/remote uncertainty retained |
| `evaluate_rules` | `complete_report`, including failed block verdicts | native `invalid` | real `incomplete_report`; adopted V2 method below | `cancelled`, no report before admission or retained partial report afterwards |

Preparation of evidence has no backend. Rule preparation requires valid backend configuration/environment credential for encoding/accounting but makes zero provider calls. It leaves routers/decisions not evaluated, prepares only requests ready before their answers and records deferred requests as `waiting { on }` without acquisition. Use the synthetic local fixture credential, not the owner's key, for those local tests. Milestone 1 intentionally omits the milestone 2 change-scope shapes (`change_selections`, judged `selected_by`, occurrence/preparation `selection`, and `outside_change_scope`); the exact contracts' SelectedReason paragraph states that accepted milestone 1 shapes omit selection. Do not demand those future shapes from this full-scope target.

**Adopted V2 PreparationFailed method:** prove both native `preparation_failed` variants' wire tags, diagnostics/measurements and exit 3 through the real CLI serializer/exit seam in `crates/sunstack-rule-cli/src/protocol_boundary.rs`, **plus source reading that no valid input reaches the internal invariant/worker-fault outcome**. Public-stdin reachability is not claimed; do not label the seam as a process journey or add a fault flag to force it. Read `rule_preparation.rs` and `rule_execution.rs` at the product pin for the internal-fault return boundaries and their non-input-reachable worker/invariant paths.

## Match the observations to their harness

| Tests under `crates/sunstack-rule-cli/tests/` | Deciding observation |
| --- | --- |
| `process_protocol.rs`, `process_bounds.rs` | Real framing/error results, request-decoder native refusals with safe field paths, exact raw-byte limit and open-writer excess-byte refusal, literal selected bytes versus excluded content, required/optional evidence outcomes. |
| `process_judgments.rs` | Mixed noul/choice/score, receipt and actual HTTP body/count; native diagnostic path/code; 0/2/3 outcomes; finite configured attempts; no fabricated answer on failure. |
| `process_rules.rs` | Actual prepared evidence; working HTTP positive control followed by zero additional preparation calls; failed block verdict with exit 0; native invalid, selection and provider/invalid-answer incomplete outcomes. |
| `process_cancellation.rs`, `process_rule_cancellation.rs` | Both signals before evidence/judgment admission, SIGTERM before rule admission and in flight; real ready/consumed-signal barriers. Causal completed peer retains judgment/receipt, held second request cancels without retry and third stays not_started/unread; actual HTTP/read counters corroborate. FIFO-held evidence worker cannot publish terminal stdout until it settles. |
| `process_evidence_paths.rs` | Same relative transcript and cursor source paths in two real CWDs resolve to distinct captures; append/resume returns the new whole record. Payload/author/provider/credential sentinels remain absent from safe diagnostics/stderr. |
| `process_journeys.rs` | Controlled skill-lint/handoff paths and router outcomes, covered by the milestone 1 recipe as part of this same default run. |

The FIFO's 100 ms no-output observation is bounded and corroborated by source awaits; it is not proof about arbitrary timing. Socket EOF and cancellation do not establish remote inference/billing reversal. The accepted phase 1 report additionally observed in-flight SIGINT outside the permanent suite; reuse only with an explicit unchanged-source/config applicability check, or leave that extra case unobserved in your new run.

## Evidence and cleanup

Each child action is retained under `target/cli-process-evidence/<test-pid>-<sequence>/`: stdin.json, stdout.json, stderr.log and result.json (CWD, binary, exit and input byte count). Associated request bodies/counts and barrier sequences are in `target/cli-process-observations/<test-pid>-*.json`. These locations are rooted in the checkout, even with a separate CARGO_TARGET_DIR. Record new test PIDs/artifact paths so stale output cannot count as the current result. Copy owned artifacts to the granted evidence directory before checkout cleanup; synthetic content is allowed, credentials/headers are not.

Reopen stdout/result and relevant HTTP/barrier observations beyond the nextest summary. Confirm tags, exits, answer references and actual effects; a successful test discovery/build proves no operation. The harness owns temporary fixtures and joined child/server cleanup. Retain artifacts after completion, leave shared caches and other sessions alone, and follow the main skill's cleanup permission boundary.

Complete when each scoped operation/control is observed or validly reused with evidence, the adopted V2 method above is covered with its public-stdin limit explicit, and no owned process remains. Return this process coverage separately from any main-skill-selected hosted or measurement route; controlled process tests alone do not establish V29/V30 hosted proof.
