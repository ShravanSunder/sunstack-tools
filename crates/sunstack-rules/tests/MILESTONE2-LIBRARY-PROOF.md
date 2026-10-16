# S4b-1 rules library proof

Author evidence for the milestone 2 library slice, governed by Specification34bee48 and the connected-evidence design through410df9d3c6f35b9282b15205907dadc069eb64e1. The work branch integrates accepted evidence33ba680, rules37f4034 and CLI919ca497. The immutable implementation target is declared on execution root01a1446c-0a1b-7c2d-8e3f-4a5b6c7d8e02 after this record is committed. Independent observation and Lead acceptance remain separate.

Rules uses one native EvidenceSession per invocation. It forwards discovery and package mappings, admits ChangeSet selectors, compares dependencies at PATH level before leading conditions, and compares pending content only at the occurrence's acquisition frontier. Unchanged occurrences skip conditions. Unresolved conditions preserve milestone 1's evaluation_failed outcome under both scopes, with the actual step-1 selection. A nonpassing settled gate wins over a speculative comparison refusal. A pending result never fabricates selected reasons.

The additive evidence API owns frozen before/after comparison, capture reuse and cumulative limits. Rules owns selection reasons, scheduling, code targets and request/report projection. Existing evidence selection APIs and behavior remain intact. SelectedReasons validates the closed, nonempty, unique canonical ordering at deserialization and construction; it stores an immutable boxed slice. No new normal production dependency or competing capture owner was added. git2 is a rules test dependency only.

## Final gates

Run from the repository root. The normal configured signing runner remained enabled throughout. On this Apple Silicon host every Cargo check used:

```sh
SDKROOT="$(xcrun --show-sdk-path)" CC=/usr/bin/clang \
CFLAGS="-isysroot $(xcrun --show-sdk-path)" \
CARGO_TARGET_DIR=/private/tmp/2026-10-s4b-rules/target <cargo command>
```

| Command | Observed result |
| --- | --- |
| `cargo nextest run --locked --workspace` | 261 passed, 7 configured hosted tests skipped, exit0; run 81867d56-3fac-4139-88c6-5f864f72ce58 |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Passed, zero warnings/errors, exit0 |
| `cargo fmt --all --check` | Passed, exit0 |
| `git diff --check` | Passed, exit0 |
| `cargo tree --locked -p sunstack-rules --edges normal --depth 1` | Existing DAG preserved: rules depends on decisions and evidence; git2 absent from normal dependencies; exit0 |

The 261-test workspace run includes the accepted CLI process suite, the existing native library suites, and these new library controls. No LEAK marker or assertion failure appeared. Earlier workspace run:258passed/7skipped before the final two reason tests and test-only cleanup. Runtime proof is automated: public prepare_rules/evaluate_rules, real temporary Git repositories/files and actual HTTP through the existing DecisionBackend. No equivalent manual repeat is claimed or needed for those paths.

## Executable claims

Expected values below come from governing contracts and literal fixture bytes, paths and occurrence identities, independently of the production comparison/renderer. Each table row names the mistake the permanent test must catch. Except where the red history below explicitly says otherwise, these are first-green controls with no observed pre-fix behavioral red.

### Native comparison — `sunstack-evidence/tests/change_comparison.rs`

| Test | Wrong behavior caught |
| --- | --- |
| content_comparison_ignores_locations_and_detects_added_deleted_and_changed_paragraphs | Selecting on prefix/line movement, or missing changed/added/deleted paragraphs and deleted files; real Git table |
| path_comparison_precedes_content_and_reuses_one_frozen_capture_after_live_mutation | Treating PATH as semantic change, recapturing live mutation, or charging the same frozen source twice; PATH member0, CONTENT unchanged, original selected bytes and files_read2 |
| full_scope_is_not_change_tracked_and_capture_limits_refuse_instead_of_proving_unchanged | Reading Git for Full or treating an incomplete capture as unchanged; Full zero reads, bounded capture refusal with one charged read |
| native_comparison_results_reject_extra_fields_on_empty_outcomes | Accepting extra members on the closed unchanged outcome; minimal valid JSON remains accepted |
| match_fingerprints_keep_multiplicity_and_capture_text_but_ignore_location_and_order | Comparing match positions/order or dropping duplicate matches/named capture text; real regex fingerprints |
| cancelling_dirty_layers_still_expose_each_changed_member | Comparing only the net worktree delta; staged and unstaged changes remain members0/1 even when final content equals HEAD |
| unavailable_baseline_and_exhausted_history_cannot_prove_irrelevance | Returning unchanged for missing baseline or exhausted shared history; typed unknown reasons and one UTC anchor across two used repositories |

### Public rules selection — `change_selection.rs`

| Test | Wrong behavior caught |
| --- | --- |
| mixed_changed_and_unchanged_paragraphs_select_the_right_owner_through_real_http | Always select/skip or leak a peer question/content; exact selected reason, one question and one HTTP request |
| line_movement_alone_becomes_outside_before_any_encoding_or_http | PATH change used as content change; preparation prunes both owners and evaluation sends zero HTTP |
| definition_changes_select_only_their_owners_and_common_instructions_select_both | Ignoring rule/common definition dependencies or assigning a peer's reason |
| staged_gate_preserves_pending_frontier_and_pass_exposes_equality_change_or_refusal | Eager CONTENT work, premature selected reasons or hidden refusal; waiting preparation stays pending, bytes_scanned equals only gate bytes, pass decides equal/changed/duplicate target |
| speculative_real_comparison_refusal_is_subordinate_to_the_observed_gate_outcome | Replacing a nonpassing gate with selection failure, or hiding refusal after a passing gate; real duplicate-target diagnostic, HTTP counts and work retained |
| cancellation_and_request_limit_leave_unreached_content_comparison_pending | Fabricating a settled reason after cancellation/dispatch limit; pending survives in typed/JSON partial reports with resolved metadata |
| a_later_http_barrier_mutation_does_not_replace_the_binding_capture_used_at_content_frontier | Recapturing after a gate HTTP barrier; the original prefix-only target stays outside and its returned source hash stays original |
| target_only_deletion_is_a_selection_failure_and_a_new_ruleset_selects_unchanged_reused_rule | Declaring deletion irrelevant or missing an added RuleSet; deleted_target refusal and new_rule reason on an unchanged reused rule |

`request_planning/comparison_gate_order_tests.rs::settled_gate_reduction_wins_over_comparison_refusal_in_both_application_orders` checks four typed reduction cases: passing/nonpassing gate, with refusal applied before/after the settled gate. This is a unit reduction proof, paired with the real Git/HTTP speculative test above. It does not claim two independently forced public runtime completion orders.

### Dependencies and metadata — `scope_dependencies.rs`, `change_reasons.rs`

| Test | Wrong behavior caught |
| --- | --- |
| leading_stops_keep_path_resolution_but_skip_refused_content_and_keep_a_ready_peer | Running step2 after a false leading condition or allowing a refused peer to suppress a ready owner; RuleSet/rule conditions each have false and passing controls |
| each_nontracked_context_or_condition_dependency_prevents_false_irrelevance | Treating a Full document, conversation, payload context, payload condition, origin condition or outside-bound definitions as unchanged; isolated FullScope reasons with other dependencies tracked and unchanged |
| pattern_condition_compares_its_whole_input_even_when_the_condition_stays_true | Comparing only the verdict target or before/after booleans; change outside U001 still gives applicability_changed under RuleSet/rule conditions |
| repeated_groups_reuse_captures_and_report_one_resolved_binding_joined_to_the_judged_source | Resetting capture ownership per group, duplicating resolved metadata or losing source attribution; two groups, eight physical reads, one resolved binding and exact judged-source joins |
| unresolved_current_condition_keeps_the_path_selection_without_a_fabricated_full_scope_reason | Keeping the initial FullScope placeholder after a leading scan fails; tracked path_pending becomes evaluation_failed/cause condition |
| unchanged_paths_skip_leading_scans_entirely | Evaluating a would-fail condition for an irrelevant occurrence; all outside, zero scanned bytes, zero HTTP |
| distinct_group_sources_share_one_cumulative_capture_budget | Resetting limits per group/repository; first group judged, second refused, exactly three physical reads and two resolved bindings |
| context_content_change_selects_the_unchanged_target_with_only_its_context_reason | Losing a changed context dependency; unchanged U001 selected only by context_changed and literal U002 context reaches HTTP |
| unknown_before_sides_conservatively_select_and_judge_the_current_side | Treating unknown before-side as irrelevant or preventing current-side judgment; missing baseline and bounded history expose their precise reasons in preparation and completed HTTP reports |

### Connected evidence — `connected_rules.rs`

| Test | Wrong behavior caught |
| --- | --- |
| connected_rule_forwards_mapping_one_group_and_exact_cross_repository_evidence_to_http | Dropping package mapping/caller/code target or judging chunks separately; one question, exact library/caller text, repository/revision/ranges/signatures/import spans, whole-unit membership |
| missing_mapping_and_disabled_discovery_are_distinct_without_inference | Treating unmapped context as complete or ignoring disabled discovery; distinct refusals and zero HTTP |
| competing_rule_seed_fields_refuse_before_session_open_and_data_words_remain_valid | Admitting any of six competing seed/query spellings or rejecting the same words in free-form question data; exact UnknownField definition path/line, valid positive performs three reads |
| required_connected_failures_isolate_the_owner_and_optional_partial_context_reaches_http | Losing native readiness or peer independence; eight required/optional cases cover unresolved relationship, minimum, whole-unit overflow and visit cutoff; partial diagnostics and omissions survive unchanged |
| reused_related_selector_is_checked_against_each_owners_kind_correct_binding | Trusting cached shared-file validation from its first ChangeSet owner; second owner has a valid Document binding and fails only selector compatibility at the shared rule location |

### Wire and coverage — `milestone_two_contracts.rs`

| Test | Wrong behavior caught |
| --- | --- |
| omitted_discovery_and_mappings_have_explicit_native_defaults | Missing the None/empty default cutover |
| bounded_discovery_and_package_mappings_round_trip_the_governing_tables | Losing either field in typed serialization |
| full_scope_preparation_and_real_http_report_expose_selection_without_resolving_git | Breaking the M1 Full path, omitting selected reasons or SingleSource coverage, or fabricating resolved Git metadata |
| selected_reasons_are_closed_nonempty_unique_and_in_the_canonical_order | Accepting empty/unknown/duplicate/unordered reasons or extra fields on comparison_pending |
| payload_context_keeps_its_authored_input_and_group_in_single_source_coverage | Leaking internal mapped input names or group0 attribution into group1; two actual HTTP requests preserve flags and group positions0/1 |

## Red history and sensitivity

The initial integration build failed at four request_state coverage type uses; this was a compile failure, not a behavioral red. Three wire/default controls then failed against the old shape and passed after cutover. The first two connected tests failed with InputKindMismatch before selector admission, then passed. The first three selection tests failed against always-Full behavior, then passed. The first native API run was a missing-symbol compile failure; its runtime controls were first-green. The final leading-order controls failed twice against pre-correction ordering and passed after implementing410df9d. A pending-variant decoding test found that a Serde unit variant ignored extra fields; changing it to an empty struct variant preserved its JSON shape and closed that hole. The native unchanged variant had the same observed red and received the same JSON-preserving repair. Fixture compilation/path/byte-offset/newline assertion errors are not counted as product behavior reds.

Five deliberate defects were tested in a separate AgentStudio worktree, never on the declared target:

| Deliberate defect | Deciding test | Result |
| --- | --- | --- |
| Use PATH changes as settled content reasons | line_movement_alone | Intended failure, exit100 |
| Run CONTENT during initial planning | staged_gate_preserves_pending | Intended failure, exit100 |
| Drop forwarded package mappings | connected_rule_forwards | Intended failure, exit100 |
| Drop report change_selections | repeated_groups_reuse_captures | Intended failure, exit100 |
| Disable the competing seed-field decoder guard | competing_rule_seed_fields | Intended failure, exit100; guard bypass returned Prepared with files_read3 |

Each mutated file was restored byte for byte. The five restored deciding tests passed together, exit0, run caf9d140-633e-4e6d-af82-7ff332e6cad6. The final source review compares that sensitivity snapshot with the target: subsequent changes deduplicated test imports, added two reason controls, corrected a literal newline assertion, and changed the new empty native Unchanged variant to an empty struct variant with the same JSON. The latter closes the same Serde extra-field defect as ComparisonPending, with an observed red and a passing permanent decoding regression. No compared fingerprint, mapping, admission guard, report projection or acquisition-frontier semantics changed after sensitivity.

## Proof boundaries and risk

V20 has structural admission-before-EvidenceSession::open evidence plus a guard-bypass sensitivity that reaches real capture. It does not have a syscall observer excluding hypothetical acquire-then-discard behavior. Binding resolution may read Git and freeze files before leading conditions; step2 subject comparison/acquisition remains deferred. Those are separate costs and claims.

No production stand-in remains. Controlled HTTP supplies deterministic replies and proves transport/request bytes, scheduling, selection and report behavior; it does not establish hosted model accuracy or current hosted availability. Seven configured hosted tests were skipped. M2 native JSON fields follow the reviewed prose/tables; the separately reviewed invocation schema remains M1 and is not claimed to validate M2 input. Independent observation, Lead acceptance, S4b-2 CLI adaptation, PR readiness, merge and release are unverified or outside this handback.

The principal merge risk is incorrect narrowing suppressing a required judgment. Exact owner/reason controls, unchanged neighbors, line movement, missing history, nontracked dependencies, cumulative limits and frozen-source mutation address that risk at the real library boundary. Runtime completion-order claims remain bounded as described above.

Raw author logs, source hashes and the sensitivity driver are under `/private/tmp/2026-10-s4b-rules/`; this temporary location is not durable storage. The permanent tests and this record travel with the branch.
