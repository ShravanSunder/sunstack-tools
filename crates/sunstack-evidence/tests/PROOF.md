# S2 proof account

Scope: milestone 1 evidence. Governing design: 0f31d9a, plus Lead's whole-message
fixture correction (execution activity 6901) and optional-refusal clarification
at 6d187c7. Plan: 2026-10-16-rust-rule-engine-delivery.md, slice S2.

All tests are integration tests compiled as external consumers of the library.
Filesystem, capture/append/replace, AST-grep, serializers and payload validation
stay real. No provider or JS runtime is used during evidence preparation. Bun
was used only once to write the real Zod schema fixture. See fixtures/README.md
for provenance, literal expectations, licenses and exact generation command.

| Test | Wrong behavior it catches | Independent expectation |
| --- | --- | --- |
| real_markdown_paragraph_and_ast_ranges | A fenced/quoted/heading/code ID resolves, or AST capture points at other bytes | Spec C2/C3; literal Markdown and Rust |
| duplicate_deleted_literal_and_regex_targets_use_frozen_source_ranges | Duplicate treated as missing, deleted treated as current, stale captured bytes change | Spec E11/E12/C2, literal before/current files |
| pointer_preserves_rfc6901_escaping_and_rejects_noncanonical_indices | Escapes misresolve or +1/01 accepted as array index | RFC 6901 section 4 |
| generated_agent_job_schema_admits_reviewer_and_refuses_wrong_variants_without_values | Wrong variant/case/history admitted, or received value copied into errors | Actual generated Zod schema, Spec R34/V28 |
| offline_catalog_allows_local_refs_and_refuses_network_and_file_refs | Offline means reject every ref, or unregistered external ref is accepted | Program Design offline catalog contract |
| external_consumer_registers_new_format_without_library_edit | Custom adapter needs library changes or replaces built-in name | Spec V27/public trait |
| adapter_output_validation_rejects_each_structural_contradiction | Duplicate IDs, out-of-bounds positions, missing results, impossible clipping or foreign turns accepted | Program Design adapter invariants |
| diagnostic_debug_and_json_never_copy_received_content | Received values escape through Debug/JSON, list/message bounds exceeded | R19/V10 and diagnostic bounds |
| real_append_cursor_returns_whole_streamed_message_and_late_result | Delta omits old blocks, unpaired late result, partial line committed, false byte counts | Spec C11/V26 and literal byte lengths |
| late_supplement_repeats_until_transcript_materializes_it | Supplement lost after cursor, treated as injection, or duplicated after materialization | C11/V24/V26 |
| cursor_refuses_same_length_rewrite_truncation_and_identical_replacement | Digest/identity continuity unchecked | C11/V26 and real file rewrites/replacement |
| pinned_public_literals_have_whole_message_records_and_explicit_gaps | Block splitting, wrong tool/category/outcome, unsupported records dropped | Pinned public literals; Lead whole-message disposition |
| generic_projection_windows_precede_filters_and_preserve_unknown_injection | Wrong last-5/final-only filtering, unknown text removed, tool filter ignored or empty projection admitted | C11/V25; literal six-turn composition |
| malformed_unsupported_ambiguous_and_explicit_failure_stay_observable | Unknown/malformed/branch uncertainty erased or shell inferred as read success | C11/V24 and synthetic negative controls |
| native_plan_prepares_real_sources_once_and_serializes_requested_content | Public plan wrapper omits selected content or recaptures one source | Public contracts prepare_evidence; literal file |
| admission_refuses_bad_selectors_before_read_and_required_limits_stay_incomplete | Invalid kind reads missing source, required overflow passes, precancelled request prepares | C3/V1 and plan outcome contracts |
| actual_literal_regex_native_queries_and_path_scope_are_enforced | Native regex loses captures or outside path enters acquisition | C3/V6 and declared path scope |
| optional_unavailable_is_prepared_but_required_unavailable_is_incomplete | Optional unreadability marks whole plan incomplete, or required unreadability passes | Lead clarification at 6d187c7 |
| conversation_aliases_share_capture_and_cursor_through_public_plan | Canonical transcript aliases reread and return duplicate captures/cursors | E30 shared capture and literal bytes |
| normalized_document_aliases_and_native_queries_share_frozen_capture | Normalized aliases/native queries recapture the same file and exhaust max_files | E11 shared frozen capture; max_files=1 control |

## Red and sensitivity evidence

New APIs were first exercised after implementation; no pre-fix red is claimed
for those tests. Two regressions had real pre-fix behavioral reds:

- Optional mirror test: 0 passed / 1 failed, exit 101. An optional unreadable
  source incorrectly made the plan incomplete. After correction, it passes.
- RFC 6901 index test: 0 passed / 1 failed, exit 101. Rust integer parsing accepted
  +1. Requiring canonical ASCII digits fixes it.

After green, five deliberate source mutations were made separately and restored
in `finally` blocks. Each named focused test failed by behavioral assertion,
0 passed / 1 failed, exit 101. These are sensitivity observations, not earlier reds:

| Mutation | Focused test |
| --- | --- |
| Remove prefix digest comparison | cursor_refuses_same_length_rewrite_truncation_and_identical_replacement |
| Disable absent-result/outcome consistency check | adapter_output_validation_rejects_each_structural_contradiction |
| Map Read to unknown | pinned_public_literals_have_whole_message_records_and_explicit_gaps |
| Skip payload validation rejection | generated_agent_job_schema_admits_reviewer_and_refuses_wrong_variants_without_values |
| Classify duplicate paragraph as missing | duplicate_deleted_literal_and_regex_targets_use_frozen_source_ranges |

Raw mutation logs and command receipts: `target/s2-proof/sensitivity-*.log` and
`sensitivity.json` in the implementation worktree. No mutation remains in source.

## Gates and configuration

Run from the worktree root. On this host, Homebrew Clang's configuration points
at an absent Command Line Tools SDK. Use a command-local environment for C builds:

```sh
export CFLAGS="-isysroot $(xcrun --sdk macosx --show-sdk-path)"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo tree -p sunstack-evidence -e normal
cargo tree -p sunstack-evidence -e features -i jsonschema
```

No machine configuration was edited. Setting SDKROOT alone was insufficient;
explicit -isysroot in CFLAGS closes this local setup issue. Initial C compilation
failures were setup failures, not behavioral reds. Test-only lint allowances permit
unwrap/expect/panic assertions; production inherits all workspace deny lints.

Final gate receipts are recorded on the execution thread. The expected test
inventory here is 20 tests. Fixture coverage does not qualify installed hosts,
real subagent transcripts, host clipping metadata or live hook timing. Cursors
currently re-read and parse the bounded committed prefix to validate continuity
and reconstruct joins; bytes_read/bytes_parsed report that actual work, with no
speedup claim. Unix file replacement detection uses device/inode identity;
other platforms use creation metadata and are unverified here. Milestone 2,
provider interactions, CLI/rules integration and release are outside S2 proof.
No open stand-in is used.
