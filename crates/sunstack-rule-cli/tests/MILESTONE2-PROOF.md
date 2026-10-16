# Milestone 2 executable proof

S4b-2 exposes the accepted native milestone 2 through the existing four-operation
stdin/stdout executable. S4b-2 needs no new CLI forwarding policy: the existing path already forwards
native inputs, outcomes and exit policy. The accepted M1 integration below
brings its evidence-request decoder into that path. New permanent process tests establish
that connection. Native input repair is limited to rejecting forbidden fields on
empty variants; valid JSON wire shapes remain unchanged.

Governing sources: Specification C14/V2 at `34bee48`; adopted contracts and
connected tables through `410df9d`; reviewed invocation schema `fc754c8`.
Starting native target: accepted `775f1bf`. Execution trace: Router service
`0ff962c5-7fa3-4c18-a5ca-1bbe8db09e89`, root
`01a1446c-0a1b-7c2d-8e3f-4a5b6c7d8e02`. Pre-build plan and paired challenge
were posted before test edits, activities 7368/7373. The accepted M1 merge
also adopts V2 decision `dc756df` and Evidence/request codes `f259d4b`. All seven refinements were
adopted. Acceptance and independent observation belong to separate commissions.

No new hosted observation is claimed. Controlled HTTP supplies provider replies;
Git, filesystem, binary framing, native libraries, signals, child settlement and
HTTP encoding remain real. Historical phase proof records retain their original
targets. PreparationFailed wire/exit coverage remains the existing focused native
enum test; reachable public-stdin production failure was not demonstrated. Amended V2
explicitly accepts the serializer/exit seam plus source non-reachability for
internal-fault-only outcomes; this is not a fabricated process journey.

## Permanent process claims

The process helper uses the built `CARGO_BIN_EXE_sunstack-rule`, the configured
signing runner, literal request bytes, actual CWD, separate stdout/stderr and a
joined child. It records each invocation's input, output, diagnostics, binary,
CWD and exit under `target/cli-process-evidence`. Parsed HTTP and selected
observations are retained under `target/cli-process-observations`.
Every operation result is additionally decoded as its current native outcome;
literal tags, exits, byte content, ownership and source joins remain independent
assertions. Protocol-error cases have no fabricated native result.

| Permanent scenario | Deciding observation; wrong behavior rejected |
| --- | --- |
| `real_process_selects_nonempty_union_after_bytes_from_each_relative_git_cwd` | Two different real Git repositories both spelled `.` from their own process CWDs; literal nonempty after bytes and staged/unstaged member provenance. Dropped scope/discovery, empty success or wrong CWD fails. Disabled discovery refuses/2. |
| `full_document_remains_valid_while_full_changed_units_has_no_change_selection` | Full document succeeds with `change_selections=[]`; required Full ChangedUnits is incomplete/3 with `no_change_selection`. |
| `mixed_process_scope_preserves_exact_selection_reasons_and_http_owner` | Tracked definitions stay unchanged. Preparation selects exactly changed U001 with `[target_changed]`, excludes U002 and sends no HTTP. Evaluation sends one question owned by that verdict; stdout and HTTP source identity join the resolved after source. |
| `moved_process_paragraphs_are_pruned_with_outside_not_sent_and_no_http` | Prefix-only line movement preserves paragraph content: both occurrences outside; request is `not_sent/outside_change_scope`; zero HTTP at joined finish. |
| `process_gate_frontier_stays_pending_until_nonpass_equality_change_or_refusal` | Preparation exposes pending/waiting and only literal gate bytes scanned. Nonpass stops step 2; pass/equal becomes outside; pass/changed judges `[target_changed]`; pass/duplicate is selection_failed/3. Exact HTTP counts 1/1/2/1. |
| `ready_duplicate_target_is_native_execution_refusal_not_protocol_failure` | Ready duplicate gives prepared_with_failures/3 and incomplete_report/3, with zero dispatch. Waiting duplicate in the gate case still prepares pending. |
| `closed_empty_milestone_two_input_variants_refuse_through_the_real_process` | Full/Worktree/None extra fields yield native invalid/2, unknown_field and no received-field echo, through both rule operations; zero HTTP. |
| `root_package_mapping_connects_one_coherent_group_through_all_three_process_operations` | Explicit root mapping connects changed library B and unchanged caller A through all three evidence/rule operations. Literal excerpts, byte ranges, whole-unit membership, import/signature context, after-source joins and one question/group survive to actual HTTP. Only non-Full B has resolved metadata. |
| `missing_mapping_disabled_discovery_and_competing_rule_seed_keep_distinct_process_outcomes` | Missing mapping is execution refusal/3; disabled discovery, competing rule seed and WholeUnit extra fields are native invalid/2. Both rule operations dispatch zero. |
| `partial_connected_context_retains_its_diagnostics_omissions_and_independent_ready_peer` | Valid document target with separate partial related context: required refusal leaves independent peer judging; optional partial context and diagnostics/omissions survive preparation and actual HTTP. One request contains exactly one or two owner questions. |
| `consumed_sigterm_at_a_received_change_scoped_gate_retains_pending_and_resolved_identity` | Complete gate body precedes SIGTERM, consumed signal precedes joined child/server and one terminal cancelled/4 result. Admitted report retains metadata and pending comparisons without fabricated judgments; gate-only scanning and exactly one attempt/request. |

Existing Full process tests retain their original scenarios while asserting the
current empty metadata and selected/full_scope fields in preparation, judgment,
evaluation failure and cancellation/not_started outcomes. The completed-peer
cancellation case still proves its earlier judgment/receipt and owned settlement.
No old/new shape fallback or second selection owner was added.

## Schema boundary and native repair

`sunstack-rules/tests/contract_schemas.rs` records reviewed-schema validation,
generated-schema validation, native deserialization, scope validation and actual
`EvidenceSession::open` admission separately. It covers every scope variant,
both heads/windows/parent policies, dirty layers, 40/64-hex IDs, optional defaults,
discovery and package mappings. Admission uses distinct real temporary roots;
it does not prove Git-object resolution or execution for schema-only literals.

Semantic distinctions are intentional: generated schema can admit nested/empty
unions, empty/duplicate layers and blank revisions while native scope validation
rejects them. Reversed absolute bounds pass shape but fail semantic admission.
Zero lookback and malformed IDs fail their shape boundaries. Root package prefix
is valid; absolute/parent paths, blank module prefix and unsupported language
fail actual native admission. No schema was relaxed to fit implementation.

Observed behavioral reds before repair:

- `schema-first.log`, run `4ca4775e-7ad3-4e79-911e-9c810782985c`, exit 100:
  eight passed, one aggregate failure. For each Full/Worktree/None extra-field
  case, reviewed/generated=false but native deserialize/scope/admission=true.
- `whole-unit-red.log`: one failed, exit 100, because native WholeUnit accepted
  an extra field rejected by generated schema.

Lead authorized the native repair: empty struct variants for input Full,
RevisionHead Worktree and UnitContent WholeUnit, with mechanical consumers.
EvidenceDiscovery None was an identical temporary M1 fix; the accepted M1
repair is now integrated, retaining the same shape. Output-only variants were not changed. All ten
schema tests subsequently passed. No compile/setup error counts as behavioral
red: an accidental output variant constructor edit was immediately restored.

Most process additions first ran green against existing native forwarding. The
initial connected root-positive fixture incorrectly used module prefix
`@library/`, although the native boundary matcher expects prefix `@library` for
import `@library/lib`. Correcting that fixture alone made its three tests pass;
that setup failure is not a product defect or pre-fix red.

## Controlled HTTP helper correction

The first fixed-source 20-run CLI campaign preserved two failing runs (7 and 17).
Run 7 `4df65b96-d9b6-4ce8-813c-be2d508ffb37` failed on the root-positive
connected test and the zero-dispatch closed-input test. `ControlledServer::next`
imposed a three-second arrival deadline even inside the background auto-answer
loop. The root-positive child reported a sent/provider_failure request after the
loop timed out, consistent with a late request attempt (exact socket arrival
time was not recorded);
the zero-dispatch test legitimately never sends a request. Run 17 repeated the
zero-dispatch timeout. These are harness failures, not product failures.

Lead push `01a14642-9efb` authorized a cancellation-safe receive path for that
background loop, whose finish signal owns its lifetime. The causal `next()`
three-second bound is unchanged. All auto-answer users assert exact counts at
finish, including zero-dispatch; joined finish also checks captured request and
body counts, so queued unexpected requests cannot silently disappear. Owned
children finish before server shutdown. No deadline was raised, retry added,
capture disabled or runner bypassed. The campaign is restarted after this change.

## Final gates and LEAK comparison

The corrected campaign ran the unchanged CLI-only command 20 times: 680/680
passed, every run exit 0, four default skips per run. Nine LEAK markers appeared
across seven tests (maximum two on one test). The accepted `919ca49` baseline
ran the same command 20 times: 460/460 passed, every run exit 0, six LEAK markers
across four existing tests. All three campaigns' tracked/untracked source hashes
were unchanged throughout their own runs.

| Test with a marker | First campaign | Baseline | Corrected |
| --- | ---: | ---: | ---: |
| moved_process_paragraphs | 0 | 0 | 2 |
| mixed_process_scope | 0 | 0 | 2 |
| consumed_sigterm_change_scoped_gate | 1 | 0 | 1 |
| ready_duplicate_target | 1 | 0 | 1 |
| real_process_union_two_cwds | 0 | 0 | 1 |
| full_document_control | 1 | 0 | 0 |
| evidence_worker_settles_after_sigterm (existing) | 0 | 0 | 1 |
| root_package_mapping | 0 | 0 | 1 |
| real_binary_mixed_judgments (existing) | 0 | 2 | 0 |
| real_router_provider_failure (existing) | 0 | 2 | 0 |
| invalid_payload_and_malformed_requests (existing) | 0 | 1 | 0 |
| malformed_judgment_input (existing) | 0 | 1 | 0 |

All other tests have zero markers. Overall observed rates: corrected 9/680
(1.324%), baseline 6/460 (1.304%). The common 23 tests have 1/460 corrected
versus 6/460 baseline; 11 new tests have 8/220. Different suite sizes and
sequential periods limit comparison; no statistical equivalence is claimed.
No leaked descriptor owner or root-cause mechanism was observed. The isolated
original line-movement case also passed without LEAK. The full per-test table,
all 60 run IDs/exits and untouched logs are in
`/private/tmp/2026-10-s4b-cli/cli-leak-report.md` and sibling campaign directories.

Lead disposition `01a14647-3c61`: CLI-suite process-concurrency signature,
cause unknown, not test-owned. The higher new-test split may reflect more real
children per test; that explanation is plausible and unproven. Re-triage if any
single test reaches three of 20 markers, or a LEAK coincides with an assertion
failure or nonzero exit. The first campaign retained 668 passed observations,
three failed observations over two failed runs, and nine tests not run after
fail-fast. Its separate helper failure is closed by the lifetime correction and
the corrected 20 successful runs.

The strengthened ignored offline measurement test also passed: 40 actual child
invocations and exactly 70 observed HTTP requests (`measurement-helper-green.log`).
This checks the helper's finish counts, without refreshing hosted or performance
claims.

The accepted M1 pin `095b3f7e2b95ac6132fb0aff06d27446349c3abd` is merged as
`f973021b72870bb16a94ed2c6350dcb03f9ec718`. Its strict variants and evidence
request decoder are retained; unknown/duplicate/invalid input now uses the
accepted request code and schema-owned safe path. The M1 process regressions
run alongside the M2 cases. No M2 selection policy moved into the CLI.

Seven conflicts were resolved at existing owners: keep M2 chrono schema feature
plus new serde_path_to_error dependency; keep M2 bounded-document return shape
with WholeDocument{}; retain ChangeScope solely in change_contracts.rs with
Full{}; combine M2 diagnostics with M1 request diagnostics; export both M2 and
new decoder modules; keep ChangeSet selector compatibility with strict input
variants; preserve the M2 admitted evidence plan and discovery/package forwarding.
Eight further consumers needed constructor/pattern syntax only, listed in
`/private/tmp/2026-10-s4b-cli/m1-merge-consumers.txt`. No semantic test assertions
were weakened. The merge's code tree is the tested source; the final successor
only completes this proof document.

Commands run from this worktree root with
`SDKROOT="$(xcrun --show-sdk-path)" CC=/usr/bin/clang CFLAGS="-isysroot $(xcrun --show-sdk-path)"`
and `CARGO_TARGET_DIR=/private/tmp/2026-10-s4b-rules/target`:

| Cargo arguments | Result |
| --- | --- |
| `fmt --all -- --check` | exit 0 |
| `clippy --locked --workspace --all-targets -- -D warnings` | exit 0, zero warnings |
| `nextest run --locked --workspace` | exit 0; 322 passed, 0 failed, 7 skipped; 0 LEAK |
| `build --locked --workspace` | exit 0 |
| `tree --locked -p <crate> -e normal`, each of the four crates | all exit 0; evidence has no provider/HTTP dependency, decisions no Git/AST/transcript/sibling dependency; rules composes both and CLI all three |

Workspace run `f569a30e-b7bc-480a-8c24-da23d0c28427` contains all 36 local CLI
cases and ten schema controls, plus native M1/M2 regressions. The seven default
skips are five hosted cases and two offline CLI workloads, never counted as
passes. The separately changed offline measurement was observed earlier as
recorded above; final suite does not refresh that performance measurement.

Logs/commands/exits: `/private/tmp/2026-10-s4b-cli/final-gates/manifest.json`.
Final real-process evidence comprises 109 child invocations and 42 HTTP/barrier
observation files; no child stderr contained an unsigned-runner warning.
`process-artifacts.json` identifies only files created after this gate's log
started; `process-artifacts.tar.gz` preserves their separate inputs, outputs,
stderr, CWD, binary paths and exits. Each operation has success and 2/3/4 controls;
11 protocol-error outcomes have no fabricated operation result. No manual
repeat was needed: the permanent tests drive the actual runnable boundary.

Author proof supports S4b-2's bounded process/schema obligations at this source.
Independent target observation, Lead acceptance, M2 verification-skill activation
and PR readiness remain separate. The merged skill has activated M1 recipes but
its old target/M2 draft is not treated as current M2 proof authority. Lead has
already assigned the later upkeep route. No hosted inference, installed-host,
other-platform, release or artifact-signing qualification is claimed.

Risk: Rust consumers constructing repaired variants must use `{}`; JSON clients
sending undeclared fields now receive the existing invalid/refused contract.
External consumers outside this workspace were not built. Concurrency markers
retain the explicit unknown-cause boundary above; final zero markers do not
prove the historical signature fixed. No stand-in or unsettled owner decision
remains within the commissioned implementation scope.

Cleanup: retain the owned Cargo build and raw process artifacts for the pending
paired observation, along with unmerged worktrees. Their consumers remain open.
An earlier exact-target cargo clean removed 2.8 GiB of sensitivity build residue;
proving logs stayed outside it. No worktree or directory was removed here.

## Mechanical native test adaptations

Only constructors/patterns for Full, Worktree, WholeUnit and the temporary None
variant changed in these existing test files; their assertions are preserved:

- `crates/sunstack-evidence/tests/change_admission.rs`
- `crates/sunstack-evidence/tests/change_comparison.rs`
- `crates/sunstack-evidence/tests/change_selection.rs`
- `crates/sunstack-evidence/tests/change_union.rs`
- `crates/sunstack-evidence/tests/connected_groups.rs`
- `crates/sunstack-evidence/tests/connected_preparation.rs`
- `crates/sunstack-evidence/tests/connected_scope_guards.rs`
- `crates/sunstack-evidence/tests/connected_session.rs`
- `crates/sunstack-evidence/tests/discovery_limits.rs`
- `crates/sunstack-evidence/tests/document_selection.rs`
- `crates/sunstack-evidence/tests/evidence_preparation.rs`
- `crates/sunstack-evidence/tests/frozen_queries.rs`
- `crates/sunstack-evidence/tests/public_limits.rs`
- `crates/sunstack-evidence/tests/repository_context.rs`
- `crates/sunstack-evidence/tests/semantic_chunks.rs`
- `crates/sunstack-evidence/tests/support/connected_inputs.rs`
- `crates/sunstack-rules/tests/change_reasons.rs`
- `crates/sunstack-rules/tests/connected_rules.rs`
