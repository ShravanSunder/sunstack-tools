# S3a native rules proof

This account covers the rules library and its native preparation/evaluation API. The R37/V20 successor correction and its current 145-test gates are recorded in [FOLLOWUP-PROOF.md](FOLLOWUP-PROOF.md); the aggregate table below preserves the original `9aeca1f` author results. It is not CLI acceptance. The S3b pair owns executable framing/signals, exits and the two executable journeys and their cold/warm measurements. Milestone 2 is deferred under the Lead's split; no connected-change or competing-seed claim is made here.

## Governing inputs and scope

The delivery plan's S3 rows govern the library work, amended by the Lead's S3a/S3b split. Final condition/readiness oracle is design `34bee484935ec4cc8ba42452b527497e9e920256` (including final F6 and null-present F7). Optional backend ceiling oracle is `a32d32f2707a3fbe2e8e2cd80df890dc8687e832`, implemented by accepted upstream `07214f5749b3499276dff6b4f56127d9b9921f31` and merged at `72796df`. Checked additions are the rules crate, schema traits needed by its native contract, and the Lead-approved native diagnostic definition locations. Accepted verification skill files were imported unchanged by the authorized merge; they are not authored here.

The final immutable product commit is declared on execution root `private-coordination-record`, on branch `rule-engine/m1-rules`. This file is associated product proof; the development trace remains that board thread.

## Original `9aeca1f` routes and results

Local integration tests enter exported rules APIs as external Cargo integration-test consumers, load real definition/source files, use the real EvidenceSession and the real Jev reqwest encoder/accounting/validation/controller. Controlled TCP servers supply canned provider responses and observe full request bodies. They do not replace the adapter under test. `ObservedBackend` forwards every adapter operation and adds only admission/failure events; it does not synthesize answers. Pure verdict/schema/strict-decode cases run at their smaller owning boundary.

Cargo commands used the repo's Rust 1.98.1 toolchain and unchanged signing runner, with per-command `SDKROOT="$(xcrun --show-sdk-path)" CC=/usr/bin/clang CFLAGS="-isysroot $(xcrun --show-sdk-path)"`. Logs below are checkout-local under `crates/sunstack-rules/target/s3-proof/`; `run_gate.py` retains the exact command exit and arms immediate PID/stdio-descriptor capture on any LEAK marker before another run.

| Command | Result | Evidence |
| --- | --- | --- |
| `cargo fmt --all -- --check` | exit 0 | `s3a-declaration-fmt.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, zero warnings | `s3a-declaration-clippy.log` |
| `cargo nextest run --workspace` | exit 0; 144 passed, 3 hosted ignored; zero LEAK; run `04158c4e-3da0-4129-8c5b-d359e42fb4ea` | `s3a-declaration-workspace.log` |
| `cargo tree -p sunstack-decisions -e normal` | exit 0; no git2, ast-grep, jsonschema or other sunstack crate | `s3a-final-decisions-tree.log` |
| `cargo metadata --no-deps --format-version 1` | exit 0; exact four-crate graph checked | `s3a-final-crate-graph.json` |
| `cargo nextest run -p sunstack-rules --test hosted_rules --run-ignored all --success-output immediate` | exit 0; 2 passed, zero skipped, zero LEAK; run `7b75b4a9-1d85-4092-a989-0acd9402de5e` | `hosted-v22-v34.log` |

The default local gate excludes credentials. The two rules hosted cases are ignored by default and require an explicit hosted run. That run used the owner-authorized key only in child-process memory. Actual endpoint/model were OpenRouter System One and `jev-latest`. Both resolved to `typesafe/jev-1.14-20261014`, one attempt each: applicability input1099/output48/costUSD0.000046158, routing input1133/output60/costUSD0.000047586. These are protocol/runtime observations, not model-accuracy measurements. The third default ignored case is decisions' hosted V11; its independent fresh pass belongs to accepted upstream 07214f5, not to this author's S3a hosted command.

Exact normal workspace edges are cli -> rules, decisions, evidence; rules -> evidence, decisions; evidence/decisions independent. No production unwrap/expect/panic/todo/unimplemented was found in rules source; workspace Clippy and unsafe denial remain active. Rules source modules are each below 600 lines.

## Test claims

Each row names a permanent test and the wrong behavior it catches. Expected values come from the fixed literals and governing V rows stated in the row, not from calling the planner to generate its own expected result. Unless explicitly named in the chronology section, **no pre-fix behavioral red was observed**: new controls first ran green. Missing APIs, malformed fixture syntax, wrong test field/type names and corrected expected locations are setup failures, not claimed product regressions.

### `admission_preflight.rs`

Layer: Public native library; real files/adapter/controlled HTTP (schema/decode cases use their native validators).

| Test | Wrong behavior and independent oracle |
| --- | --- |
| `definition_and_binding_preflight_refusals_never_reach_source_capture_or_http` | Invalid definitions or bindings reach source capture; 35 literal refusals require zero real normalizer callbacks and HTTP bodies, with the positive control below. The formerly masked shared-file case is now its own isolated test. |
| `empty_applicability_and_question_maps_and_reused_ids_across_rulesets_are_admitted` | Empty applicability/questions or reused IDs across distinct RuleSets are incorrectly refused; fixed positive input expects two occurrences, two requests and one normalized capture. |
| `raw_nested_duplicate_keys_are_refused_before_any_typed_map_can_erase_them` | Duplicate nested keys disappear into typed maps; authored duplicate input/question/instruction/binding objects must be refused before map construction. |
| `primitive_and_selector_diagnostics_keep_their_native_codes_and_definition_locations` | Forwarded semantic errors lose their native codes or source location; empty score rubric and empty conversation projection retain native codes and definition line 2. |
| `shared_rule_file_is_validated_for_each_owner_with_kind_correct_bindings` | A cache hit skips the second owner’s selector compatibility check; valid Conversation/Document bindings leave the shared Conversation selector as the sole mismatch. Native preparation must refuse InputKindMismatch at shared rules.jsonl line 2, column 1, before a normalizer callback or HTTP. Deliberate skip-cached-validation fails; original validation is restored. See FOLLOWUP-PROOF.md. |

### `binding_groups.rs`

Layer: Public native library; real files/adapter/controlled HTTP (schema/decode cases use their native validators).

| Test | Wrong behavior and independent oracle |
| --- | --- |
| `three_groups_keep_threshold_answers_and_identical_bindings_as_separate_occurrences` | Group identities or threshold answers bleed together; literal .5999/.6/.6001 and two identical bindings require separate authored occurrences with one capture for the identical source. |
| `file_and_byte_limits_are_cumulative_across_binding_groups` | Global file/byte limits reset per group; three equal sources and a two-source bound require two calls and an evidence failure for the third. |

### `budget_splitting.rs`

Layer: Public native library; real files/adapter/controlled HTTP (schema/decode cases use their native validators).

| Test | Wrong behavior and independent oracle |
| --- | --- |
| `rules_split_greedily_at_rule_boundaries_and_reassess_real_bodies` | Over-budget grouping refuses everything or splits inside a rule; max_questions=3 requires gate+two rules in part 0, the third rule in part 1, with real bodies inside the bound. |
| `carried_gate_that_cannot_share_split_zero_is_sent_concurrently_once` | A gate is repeated or becomes an artificial prerequisite when it cannot fit with a rule; two held real bodies prove concurrent one-question gate/rule requests. |
| `oversized_rule_is_refused_without_preventing_a_fitting_peer_and_request_limit_counts_splits` | An oversized rule blocks a fitting peer or split requests escape the cumulative request bound; literal limits require peer continuation and explicit refusal/not-started outcomes. |
| `optional_content_is_removed_once_before_splitting_but_required_content_is_never_trimmed` | Optional evidence remains over budget or required evidence is silently trimmed; escaped real bodies under a byte cap prove optional omission and required overflow. The one omission pass is corroborated by scheduler ownership, not inferred from body count. |

### `carried_gate_execution.rs`

Layer: Public native library; real files/adapter/controlled HTTP (schema/decode cases use their native validators).

| Test | Wrong behavior and independent oracle |
| --- | --- |
| `same_source_gate_rides_once_and_its_nonpass_discards_the_rule_answer` | A same-source gate is staged/repeated or a nonpassing gate leaks a verdict; exactly one HTTP request and one discarded rule answer are required. |

### `completed_equivalence.rs`

Layer: Public native library; real files/adapter/controlled HTTP (schema/decode cases use their native validators).

| Test | Wrong behavior and independent oracle |
| --- | --- |
| `handwritten_fixed_completed_answer_oracles_hold_for_staged_speculative_and_split_plans` | Grouping changes completed-answer meaning; three hand-written expected outcome pairs are checked separately against staged, speculative and split real HTTP execution. |
| `random_completed_answers_and_actual_admission_orders_match_an_independent_staged_oracle` | Completion order changes reduction; 32 generated probability/priority cases compare all three layouts to an independent numeric .6/.8 oracle. Admission events come from the real adapter. |

### `condition_execution.rs`

Layer: Public native library; real files/adapter/controlled HTTP (schema/decode cases use their native validators).

| Test | Wrong behavior and independent oracle |
| --- | --- |
| `leading_payload_conditions_pass_stop_or_fail_without_eager_document_reads` | True/false/missing payload fields collapse into one result or stopped rules are acquired; fixed values require judged/not-applicable/unresolved and exact source/call counts. |

### `condition_ordering.rs`

Layer: Public native library; real files/adapter/controlled HTTP (schema/decode cases use their native validators).

| Test | Wrong behavior and independent oracle |
| --- | --- |
| `nonleading_payload_condition_never_prunes_its_earlier_carried_decision` | A later false/missing payload condition suppresses an earlier decision; D must still ride, and C is not evaluated after a nonpass but genuinely fails unresolved after D passes. |
| `payload_presence_distinguishes_value_null_and_missing_without_reading_stopped_rules` | Present null is treated as missing/false; value and null pass presence, missing fails unresolved before rule-source acquisition. |

### `condition_patterns.rs`

Layer: Public native library; real files/adapter/controlled HTTP (schema/decode cases use their native validators).

| Test | Wrong behavior and independent oracle |
| --- | --- |
| `literal_regex_and_ast_conditions_scan_real_sources_and_keep_a_bounded_unresolved_outcome` | Literal, regex or AST presence uses a synthetic result or turns a scan limit into false; real Rust source variants require pass/nonpass/unresolved and exact lazy acquisition. |

### `consumer_reuse.rs`

Layer: Public native library; real files/adapter/controlled HTTP (schema/decode cases use their native validators).

| Test | Wrong behavior and independent oracle |
| --- | --- |
| `lint_and_eval_consumers_share_definitions_but_keep_independent_expectations_and_failure_controls` | Consumers treat unfavorable or missing judgments as a clean pass; separate lint/eval expectation tables reuse identical files and test threshold equality, fail, provider failure, invalid response, cancellation, budget and selection refusal. |

### `contract_schemas.rs`

Layer: Public native library; real files/adapter/controlled HTTP (schema/decode cases use their native validators).

| Test | Wrong behavior and independent oracle |
| --- | --- |
| `every_reviewed_milestone_one_definition_and_invocation_deserializes_and_validates_generated_schema` | Reviewed examples cannot be consumed by the exported Rust types/generated schemas; every example definition and invocation is decoded, with referenced bare rule lines semantically validated. |
| `generated_and_reviewed_rule_shapes_agree_on_shape_refusal_controls` | Generated shape admits invalid rule IDs, evidence, targets, thresholds or primitive shapes; reviewed schema and generated schema are compared against fixed positive/negative literals. |
| `generated_invocation_shape_refuses_zero_budgets_and_empty_binding_groups` | Generated invocation shape permits zero budgets/timeouts or empty binding groups; reviewed and generated validators must both refuse those fixed mutations. |
| `generated_backend_ceiling_schema_matches_native_optional_positive_only_contract` | Generated or reviewed invocation schema disagrees with native optional-positive ceiling decoding; absence/positive integers pass and null/zero/negative/fraction fail native decoding, generated backend/invocation schemas and the reviewed full invocation schema. See FOLLOWUP-PROOF.md for the stale-reviewed-schema red and merge green. |
| `reviewed_invocations_pass_native_cross_file_admission_and_real_evidence_preparation` | Example invocations only pass shallow decoding; native cross-file admission and real evidence preparation run with explicit local path substitutions, zero HTTP. The generated packet schema fixture is reused, not hand-recreated. |

### `definition_admission.rs`

Layer: Pure native contract/logic.

| Test | Wrong behavior and independent oracle |
| --- | --- |
| `reviewed_rule_set_and_bare_rule_lines_admit_native_questions` | Authored native questions or bare JSONL rule files cannot be admitted; reviewed RuleSet/rule examples are decoded and semantically validated. |
| `raw_duplicate_fields_refuse_before_typed_maps_erase_them` | Duplicate raw object fields are overwritten; literal duplicates must be refused before typed maps. |

### `evidence_refusal.rs`

Layer: Public native library; real files/adapter/controlled HTTP (schema/decode cases use their native validators).

| Test | Wrong behavior and independent oracle |
| --- | --- |
| `refused_owner_leaves_peer_questions_and_nonpassing_gate_dominates_its_selection_failure` | One refused owner removes peers or overrides a controlling nonpass; only its questions disappear and the settled gate controls the occurrence. |
| `all_refused_owners_and_gate_required_refusal_record_evidence_refused_without_http` | All-required evidence refusal fabricates a sent request/budget/timeout; explicit evidence_refused and zero HTTP are required, including a refused gate owner. |

### `failure_reduction.rs`

Layer: Public native library; real files/adapter/controlled HTTP (schema/decode cases use their native validators).

| Test | Wrong behavior and independent oracle |
| --- | --- |
| `settled_gate_controls_failed_speculative_rule_regardless_of_real_adapter_completion_order` | Provider completion order overrides the controlling gate; real adapter admission/failure barriers force both orders, preserving the failure/attempt log and only actual admitted discards. |
| `malformed_informational_answer_fails_the_combined_request_without_salvaging_rule_verdicts` | One malformed informational answer is salvaged per question; every rule in its real combined request must fail without a verdict, with a valid-response control. |
| `controlled_server_counts_received_bodies_even_when_a_response_is_held_at_shutdown` | The HTTP observer counts replies instead of received bodies; a held response at shutdown must still count as one received request. |
| `caller_cancellation_retains_a_completed_peer_and_never_dispatches_the_queued_group` | Cancellation loses a completed peer or starts queued acquisition; one-permit causal progression keeps the first answer/receipt, cancels the held second and leaves the third unread/not-started. |

### `first_rule_request.rs`

Layer: Public native library; real files/adapter/controlled HTTP (schema/decode cases use their native validators).

| Test | Wrong behavior and independent oracle |
| --- | --- |
| `real_rules_evidence_and_http_preserve_owner_labels_verdict_collision_and_locations` | The first real integration flattens owner data, confuses informational verdict with the verdict question, leaks policy, or loses native source locations; fixed wire/answer/paragraph expectations check each boundary. |

### `hosted_rules.rs`

Layer: Real hosted library/evidence/HTTP adapter.

| Test | Wrong behavior and independent oracle |
| --- | --- |
| `hosted_applicability_decision_preserves_native_answer_and_receipt_through_rules` | Only the controlled service works for applicability; an actual Jev decision must admit a native noul answer, resolve its reference and retain a receipt/one attempt. No probability accuracy assertion. |
| `hosted_router_preserves_native_choice_confidence_and_source_linked_receipt` | Only the controlled service works for routing; an actual Jev choice must produce a contracted completed routing outcome and resolvable native choice/receipt. No label accuracy assertion. |

### `router_execution.rs`

Layer: Public native library; real files/adapter/controlled HTTP (schema/decode cases use their native validators).

| Test | Wrong behavior and independent oracle |
| --- | --- |
| `router_groups_have_independent_controlling_answers_and_authored_reports_under_reverse_completion` | Router groups share a controlling slot or completion order rearranges outputs; two groups, distinct labels and reverse responses require authored group order and resolvable references. |
| `native_choice_confidence_controls_uncertain_routed_and_no_route_outcomes` | Distribution maximum substitutes for native confidence; .59/.60 and confidence .4 with [.6,.3,.1] check the floor, plus a confident unrouted label. |
| `same_ruleset_under_two_route_labels_keeps_opposite_wired_gates_separate` | Two route labels to one RuleSet share differently wired gate answers; opposite gate values require the selected label to use its own binding, entry and request identity. |
| `routed_leading_scan_uses_router_sources_early_and_waits_for_other_sources` | Leading scans use a router-only frontier or acquire scan-only sources before routing; seven final-F6 cases assert real body counts, scan pass/fail and no-route laziness. |

### `router_identity.rs`

Layer: Public native library; real files/adapter/controlled HTTP (schema/decode cases use their native validators).

| Test | Wrong behavior and independent oracle |
| --- | --- |
| `carrying_compares_canonical_payload_values_and_conversation_versions_instead_of_input_names` | Input names replace source identity; equal payload values with reversed authored key order and identical transcript aliases/version carry once, while unequal values/supplements stage. |
| `two_rulesets_named_subject_wire_document_and_conversation_separately_and_refuse_cross_kind` | Two subject inputs of different kinds are wired interchangeably; document/conversation RuleSets receive only their mapped contents and cross-kind wiring is refused. |

### `router_planning.rs`

Layer: Public native library; real files/adapter/controlled HTTP (schema/decode cases use their native validators).

| Test | Wrong behavior and independent oracle |
| --- | --- |
| `leading_pruning_moves_router_control_to_the_next_carrier_or_a_router_only_request` | A pruned first candidate strands router control; a later carrier or router-only fallback must ask the choice exactly once. |
| `noncarrying_candidates_dispatch_before_routing_only_when_speculation_is_enabled` | Genuinely noncarrying candidates run early with speculation off or cannot run early with it on; different rule/router source identities and held HTTP bodies prove the positive concurrency boundary. |
| `a_failed_router_is_incomplete_and_never_turns_candidate_answers_into_judgments` | A failed router yields a complete report or salvaged candidate judgments; a real 503 requires router_failed, incomplete, retained native failure and no judgments. |
| `two_candidates_with_a_split_resolve_exactly_one_router_and_one_decision_per_group` | Splits repeat controlling questions or references/costs become per-question; two candidates and one split require three requests, one router/two decisions, resolvable references and total observed cost .003. |
| `declared_two_ruleset_workload_measures_router_requests_against_per_ruleset_decisions` | Routing request counts are asserted without a baseline; a fixed two-RuleSet workload measures four per-RuleSet gate/rule requests versus three router/rule requests, preserving two favorable occurrences and two source reads. |

### `rule_preparation.rs`

Layer: Public native library; real files/adapter/controlled HTTP (schema/decode cases use their native validators).

| Test | Wrong behavior and independent oracle |
| --- | --- |
| `preparation_readies_only_pre_answer_requests_and_waiting_records_acquire_nothing` | Preparation acquires/sends answer-dependent work; off/on ready-vs-waiting records and files_read prove the pre-answer frontier and zero HTTP. |
| `preparation_assesses_carried_and_split_requests_without_dispatching_or_inventing_answers` | Preparation fabricates answers or ignores final split budgets; actual encoded counts and split records are inspected without dispatch. |
| `preparation_reports_evidence_budget_and_accounting_failures_without_dispatch` | Preparation hides evidence, byte-budget or unverifiable accounting failures; each has explicit failure without dispatch or fabricated counts. Unverifiable now uses native require_exact. |
| `routed_preparation_waits_for_scan_only_sources_and_respects_speculation_for_noncarrying_candidates` | Preparation differs from the final-F6 router readiness predicate; scan-only work waits under both modes, noncarrying work follows speculation. |
| `preparation_pre_abort_returns_no_preparation_and_starts_no_acquisition` | Pre-abort starts acquisition or invents preparation; cancelled-before-admission must read/send nothing. |

### `staged_conditions.rs`

Layer: Public native library; real files/adapter/controlled HTTP (schema/decode cases use their native validators).

| Test | Wrong behavior and independent oracle |
| --- | --- |
| `deferred_scan_performs_no_acquisition_after_nonpass_failure_or_cancellation_with_a_passing_control` | A deferred scan reads before a stop/failure/cancel; rewriting the source only after the first real body plus the passing/unresolved controls establishes capture timing and zero deferred acquisition. |
| `a_same_input_then_condition_then_different_input_gate_preserves_the_lazy_frontier` | The A-condition-C sequence acquires C too early; false/unresolved B prevents C evidence/question, while passing B gives the literal A/C/rules request sequence. |
| `adjacent_different_input_gates_over_the_same_source_keep_separate_staged_selections` | Adjacent G/H over an otherwise unread source merge their selections; G nonpass/failure prevents H-only selection, while passing G selects H later with exact parse counts. |
| `adjacent_different_input_gates_speculate_concurrently_with_one_control_each` | Speculation silently stages adjacent gates or repeats them; all three real bodies arrive held, with exact owner set, one control each and discarded unused H/rule answers on G nonpass. |

### `staged_gate_execution.rs`

Layer: Public native library; real files/adapter/controlled HTTP (schema/decode cases use their native validators).

| Test | Wrong behavior and independent oracle |
| --- | --- |
| `staged_gates_wait_in_authored_order_skip_later_acquisition_and_keep_first_captured_bytes` | Authored gates reorder, C is read after B stops, captured A is reopened, or cancellation loses A receipt; real HTTP barriers, deletion of A and pass/nonpass/failure/cancel controls establish the frozen/lazy sequence. |

### `transcript_request.rs`

Layer: Public native library; real files/adapter/controlled HTTP (schema/decode cases use their native validators).

| Test | Wrong behavior and independent oracle |
| --- | --- |
| `transcript_projections_keep_owner_labels_deduplicate_content_and_carry_an_alias_gate` | Different projections lose owner labels or identical content is duplicated; real user/assistant transcript selections keep distinct labels, two contents, all attributions, exact generated question names and one alias-gate request. |

### `verdict_evaluation.rs`

Layer: Pure native contract/logic.

| Test | Wrong behavior and independent oracle |
| --- | --- |
| `blocking_band_table_preserves_inclusive_threshold_and_native_basis` | Threshold/band boundaries are exclusive or fabricated confidence is used; exact V33 .30/.31/.79/.80 table expects block/nudge/nudge/pass and native answer references. |
| `composite_normalizes_fractional_scores_and_keeps_each_native_reference` | Composite normalization loses fractional score/native references; literal 1.5/3 and .8 with fixed weights require .575, no noul confidence, and failure when dimensions are unavailable. |
| `verdict_admission_refuses_every_invalid_band_and_composite_dimension_shape` | Invalid bands/dimensions slip through admission; literal missing/wrong-kind/single-level/nonpositive/nonunit-sum cases and band-at/above-threshold/nudge cases must refuse. |
| `composite_threshold_equality_and_default_nudge_use_native_values_without_fabricated_confidence` | Composite equality fails, nudge is not the default, or severity activates on pass; literal .75/.7499 values and omitted-severity decoding check those boundaries. |

## Observed reds and sensitivity

These are author observations retained in raw logs, not independent Verifier mutations. Every deliberate fault below was restored before final gates.

- `leading-condition-red.log`, run14bc: the earlier ungated path could not realize the leading-condition stop. `carried-gate-red.log`, run61bb: old execution did not carry the same-source gate. These were behavioral reds, followed by the planner/scheduler cutover; the old path was removed.
- Generated invocation-shape zero-budget red run d275 preceded the numeric schema bounds; it was a schema refusal defect, not a provider behavior claim.
- Non-leading condition red run905f showed the later condition removing its earlier decision question. The 01405fd route identity controls also failed before optional route labels were carried through candidate owners; those behavioral corrections preceded current green. Earlier ab4 null/nonpass expectations were superseded by final F7 and are excluded.
- `budget-splitting-red-valid-fixture.log`, runb653: the valid tight-budget fixture refused the whole request rather than splitting. Earlier malformed fixture setup is excluded.
- `failed-request-no-discard-red.log`, run19b463: a failed provider request incorrectly listed an answer as discarded; it returned no admitted answer. Projection now lists only actual completed admitted answers.
- `definition-semantic-location-red.log`, run73a614: native semantic diagnostic had no definition location. The original partially specified noul case failed shape decoding earlier, so it was not the deciding semantic red. The score/selector semantic cases established the missing location; the selector's expected code was corrected to its owning native `empty_projection`.
- `22d6345-red.log`, run6bfd and `riding-readset-red.log`, run9b87: final reviewed null-present and eligible-rules-read-set controls rejected the earlier implementation. Earlier null-as-nonpass expectations were superseded and are not evidence for current semantics.
- `preparation-waiting-sensitivity.log`, run24d561: deliberately making every request ready acquired three sources instead of one. This is post-green sensitivity, not a pre-existing bug. The original source was restored byte for byte.
- `completed-oracle-sensitivity.log`, rune91fae: deliberately bypassing applicability reduction failed the hand-written equivalence oracle. It is sensitivity, not a pre-fix red. Source was restored byte for byte before the next full gate.
- `held-response-counter-red.log`, run4cb76289: a full received body with its response still held counted zero. The product-test server observer now counts immediately after full parse. No production engine defect is claimed for this observer correction.
- `ceiling-schema-red.log`, runc70b982a: generated schema accepted explicit null while native Serde refused it. The explicit positive NonZero field schema fixes that mismatch without changing native decoding.
- Router identity/frontier test setup corrections are not product reds: unregistered `claude_code` corrected to native `claude_code_jsonl`; noncarrying fixture changed to actually use a different source (same-source candidates legitimately fan out with speculation off); wrong public test field/type names corrected; canonical definition path expected instead of a relative path. The failed-router test's extra empty-occurrences assumption was dropped: the governing obligation is an incomplete report, clear failure, and no salvaged judgments, which it now asserts. No product change was made for these test assumptions.

Additional passing focused run IDs: schema/preparation06b0432e (9 tests), owned cancellation9fd8fbb8 (8), router/verdict/staged9390ea00 (11), mixed-kind wiring/split e16aa804 (6), consumer/native examples36d2d62c (6), transcript/mixed missing3b1995d9 (3), real pattern matrix29b35af4 (1), adjacent speculation3396f448 (4), and declared routing workload95f17b37 (1). The aggregate above applies to original9aeca1f; FOLLOWUP-PROOF.md records the successor gates.

## Boundaries and remaining observation

There is no production stand-in. The controlled provider supplies canned answers for deterministic scheduling/refusal/provenance scenarios; it cannot establish model accuracy or hosted availability. Two genuine hosted calls close the S3a protocol path for V22/V34, without asserting accurate classifications. Generated packet schema and qualified transcript literals are reused from accepted evidence fixtures; the example-preparation test labels its added Codex shell records synthetic.

No executable/CLI, live hook, deployed service, release, non-macOS platform, exact Jev tokenizer or known provider context-capacity claim is made. Milestone 2 shapes/changes and related-unit competing-seed proof remain assigned to later slices. The historical S1 Verifier LEAK at bc9c1ddd remains unattributed; zero markers here do not explain it. Any future marker must capture ownership before retrying.

The primary paired Verifier completed the full `9aeca1f` observation: all 60 original named tests passed independently (144 local workspace plus two genuine hosted). It supported the commissioned rows except the stale reviewed R37 schema and masked V20 shared-file negative. Its report established no native product contradiction. The successor adds the isolated V20 test (61 current rules claims, 59 local plus two hosted) and repairs R37 by merging the accepted docs and extending the agreement test. Its bounded independent R37/V20 recheck and Lead acceptance remain pending; other observations are reused only because production/support/hosted code is unchanged.
