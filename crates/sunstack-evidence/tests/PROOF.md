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
| late_supplement_repeats_until_transcript_materializes_it (superseded) | Current cursor cases and results: [SUPPLEMENT-CURSOR-PROOF.md](SUPPLEMENT-CURSOR-PROOF.md) | C11/V24/V26; limits repair: [ACQUISITION-LIMITS-PROOF.md](ACQUISITION-LIMITS-PROOF.md) |
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

Initial new APIs were first exercised after implementation; no pre-fix red is claimed
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
inventory here is 34 tests. Fixture coverage does not qualify installed hosts,
real subagent transcripts, host clipping metadata or live hook timing. Cursors
currently re-read and parse the bounded committed prefix to validate continuity
and reconstruct joins; bytes_read/bytes_parsed report that actual work, with no
speedup claim. Unix file replacement detection uses device/inode identity;
other platforms use creation metadata and are unverified here. Milestone 2,
provider interactions, CLI/rules integration and release are outside S2 proof.
No open stand-in is used.

## Correction after independent observation at 77cdde02

The independent report at `/private/tmp/2026-10-s2-verification-77cdde0/report.md`
found an actual V4 defect while all original 20 tests passed. The initial list
case asserted only `is_ok`; it could not detect truncated text. The Lead accepted
the finding against unchanged C2 and authorized an additive correction.

The selector now tracks Markdown block context. For tight lists, where the
parser omits Paragraph events, it accumulates the entire inline source range
until a block boundary. Headings, code fences and quoted blocks are excluded by
context. Source slices retain continuation indentation and markup delimiters.
No contract, dependency, or other source owner changed.

Ten permanent cases were added before the repair. At the old production source,
`cargo test -p sunstack-evidence --test document_selection paragraph_` exited 101:
**6 failed, 5 passed, 1 filtered**. Six named regressions failed for the exact
observed text/eligibility errors. The four new controls passed, as did the existing
paragraph test selected by that filter. After repair, the entire document test
binary passed **12/12**, exit 0. Raw before/after evidence is in
`target/s2-proof/correction/paragraph-{red,green}.log`.

| Added test | Wrong behavior it catches | Oracle / layer |
| --- | --- | --- |
| paragraph_regression_tight_list_wrapped | First line returned without continuation | C2, observed literal, real file + public capture/resolver; pre-fix red |
| paragraph_regression_tight_list_emphasis | Tail after emphasis missing | C2, observed literal; pre-fix red |
| paragraph_regression_tight_list_link | Link and following tail missing | C2, observed literal; pre-fix red |
| paragraph_regression_tight_list_inline_code | Inline code/tail missing | C2, observed literal; pre-fix red |
| paragraph_regression_list_heading_excluded | Heading inside list treated as target | C2 exclusion, observed literal; pre-fix red |
| paragraph_regression_list_fence_excluded | Fenced example inside list treated as target | C2 exclusion, observed literal; pre-fix red |
| paragraph_control_ordinary_wrapped | Fix drops ordinary continuation | C2 control, real file/resolver; already passed before repair |
| paragraph_control_loose_list_wrapped | Fix breaks loose-list paragraphs | C2 control; already passed before repair |
| paragraph_control_inline_code_id | Fix treats a code-span ID as target | C2 exclusion control; already passed before repair |
| paragraph_control_ordinary_emphasis | Fix drops ordinary inline markup | C2 control; already passed before repair |
| category_and_name_filters_exclude_contrasting_tools_and_remove_present_results | Category ignored or present results not excluded | V25: real Read/Bash/Search calls paired with observed results; assert selected IDs/categories and retained call/input/outcome with absent projected result |
| unknown_format_is_refused_at_capture_entry_before_file_acquisition | Registry collision test mistaken for unknown-format acquisition refusal | V24: public capture on nonexistent path with unregistered format must return UnknownTranscriptFormat before file error |
| valid_clipped_results_keep_known_and_unknown_sizes_through_capture_and_selection | All clipped results refused or fidelity lost during projection | V24: concrete external adapter, real capture and selection; literal retained text, known size40 and unknown size |
| invalid_external_output_is_refused_by_real_capture_entry | Helper matrix passes while capture bypasses validator | V27: concrete invalid adapter registered then actual file capture; assert AdapterOutputInvalid |

The four new proof-gap tests passed before any production change in their owners;
they close previously unobserved paths, not additional observed product failures.
Five separate post-green sensitivity probes then established their ability to fail:
ignore category filter; skip present-result exclusion; bypass acquisition output
validation; reject every clipped result; delay unknown-format validation until
after opening the file. Each focused run exited 101 with **0 passed / 1 failed**,
for its behavioral assertion. Each production file was restored in a `finally`
block. Logs and exact commands: `target/s2-proof/correction/sensitivity-*.log` and
`sensitivity.json`. These are sensitivity observations, not pre-fix reds.

The original schema fixture and provenance were unchanged. The Verifier separately
reproduced the actual Zod export byte-for-byte at 77cdde02. Final correction gates
and the new fixed commit are declared on the execution thread; their raw logs
are in `target/s2-proof/correction/`.
