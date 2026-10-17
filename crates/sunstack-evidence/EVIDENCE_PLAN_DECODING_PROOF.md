# Evidence input decoding — successor proof

`prepare_evidence` now delegates input decoding to `sunstack_evidence::decode_evidence_plan_input`. Malformed request objects receive Evidence/request diagnostics, with `unknown_field`, `duplicate_field` or `invalid_request`, static repair guidance and a schema-owned path. A malformed repository scope no longer receives selector/projection advice. The operation remains `prepare_evidence/refused`, exit 2; valid preparation and valid JSON stay unchanged.

This is the Lead-commissioned successor to `bc5c722d3779f060857fe6c8e93d02ced9335e21` on `rule-engine/m1-strict-variants`, in `private verification archive`. Authority: Lead direct `private-coordination-record`; governing design: `f259d4b28f5131478e574e123335904c78b0d347`, Program Design Diagnostics table, Evidence/request row, plus Specification R10/V10 and R19. The Lead accepted the paired Verifier's finding about the pre-existing CLI mapping. The Verifier's complete base observation is `private verification archive`; its cited raw process refusal and the responsible source were inspected before this correction.

Risk: clients interpreting malformed evidence input now receive these intentional diagnostic additions and Request origin. The old strict-variant proof in `sunstack-rules/STRICT_VARIANTS_PROOF.md` remains historical evidence for bc5c722; its old invalid_selector process assertion is superseded here. No schema/design files or rules source were changed. Hosted calls remain outside the grant and were not run. Independent successor observation and delivery acceptance are separate from this implementer proof.

## Implementation boundary

- `src/evidence_plan_decoding.rs` owns one typed Serde decode and end-of-input check, classifies the error, then discards library error text. It uses the already locked serde_path_to_error dependency, now direct in evidence. No JSON tree or second validator was added.
- `src/evidence_diagnostics.rs` adds the three closed EvidenceCode variants and Request origin with static actionable guidance and RepairRequest advice. Unknown fields point to the containing object; duplicate fields point to their containing schema object; malformed/type/trailing input uses invalid_request.
- `src/lib.rs` exports the decoder. The return type is `Result<EvidencePlanInput, Box<Diagnostic>>`; boxing only refusal avoids a 240-byte Err variant and required no lint suppression. The CLI moves that owned diagnostic into its existing bounded list.
- `sunstack-rule-cli/src/protocol_boundary.rs` changes only the prepare_evidence branch to use that native decoder; framing, operation routing, all other operations and response/exit shapes are preserved.

The safe path keeps only schema-owned map-field prefixes, omits sequence indices, and caps at 32 segments. It stops at the actual `sources` map before any caller-owned source name, including names such as repositories, limits and max_files. All arbitrary payload JSON is below that map. Unknown segments stop traversal rather than being copied or skipped to later values. Trailing JSON, malformed root JSON and non-object roots report an explicit Field location with an empty path. This intentionally sacrifices the authored source name in exchange for the required privacy boundary; it still identifies sources as the containing schema field. Error text, authored keys/values, paths received as values and raw JSON never enter guidance or logs.

## Permanent tests and deciding failures

The new native tests are in `tests/evidence_plan_decoding.rs`. Before implementing the new classification, the existing CLI behavior was extracted unchanged into the actual exported native function and the CLI was delegated to it. That runnable baseline still returned InvalidSelector for every decode error. The tests failed on the returned code, not on a missing API or compilation failure. No test double supplied classification or path behavior.

| Test | Wrong behavior rejected / oracle | Initial native observation |
| --- | --- | --- |
| valid_plan_decodes_without_source_io_and_preserves_its_wire | Valid canonical input must deserialize without opening its source and retain its structural JSON; its serialized canonical form must reserialize byte-identically | passed |
| unknown_scope_field_reports_request_class_and_scope_path | An extra full-scope field must be unknown_field/request at repositories/change_scope, not a selector problem | failed: invalid_selector |
| duplicate_field_reports_request_class_and_containing_path | Literal bytes containing max_files twice must be duplicate_field/request at limits; the duplicate is inserted into raw JSON after serialization, never round-tripped through Value | failed: invalid_selector |
| invalid_value_reports_request_class_and_owned_field_path | A string in numeric max_files must be invalid_request/request at limits/max_files, without echoing the value | failed: invalid_selector |
| trailing_json_reports_invalid_request_at_the_root | A second JSON body must be invalid_request at the root, never accepted | failed: invalid_selector |
| caller_source_keys_never_become_location_segments_even_when_they_match_schema_keys | Extra fields under a valid caller-named source must be unknown_field at exactly sources; authored names/fields/values cannot appear in rendered diagnostics | failed: invalid_selector |
| malformed_json_reports_invalid_request_at_the_root | Incomplete root JSON must produce invalid_request at the root | added after initial green; no separate pre-fix red |
| non_object_root_reports_invalid_request_at_the_root | An array root must produce invalid_request at the root | added after initial green; no separate pre-fix red |
| arbitrary_payload_with_schema_like_keys_remains_open | Valid authored payload containing kind, unexpected and nested repositories/limits/max_files must parse unchanged | added after initial green; positive compatibility guard |

The three classification rows have independent permanent red/green cases. The last three controls implement the Verifier's feasibility refinement after the initial six cases passed. The caller-key control was extended to all five names author_secret_source, repositories, limits, max_files and change_scope. The shared native assertion checks exact code/origin/path/advice, bounded guidance and absence of synthetic secret keys/values. Byte identity is claimed only for the explicit canonical serialized positive form, not arbitrary accepted whitespace, field order or omitted defaults.

Two real-process cases in `sunstack-rule-cli/tests/process_protocol.rs` run the actual signed executable with the unchanged existing driver:

1. `unknown_field_inside_full_scope_is_refused_at_the_real_process_boundary`: positive preparation returns the exact document paragraph/exit 0. The negative input differs only by the extra full-scope field and must return request/unknown_field at repositories/change_scope, refused/exit 2, no selections/measurements/evidence or echoed unknown field. The earlier invalid_selector assertion was changed to the governing successor expectation.
2. `malformed_named_source_reports_only_schema_owned_location`: positive preparation succeeds; a coherent renamed source with an extra field refuses at exactly sources. Both complete stdout and stderr must exclude the synthetic caller name, unknown key and value.

Both process cases failed at the extracted baseline on invalid_selector versus unknown_field, then passed through the real boundary. Raw stdin/stdout/stderr/result artifacts are retained. No provider, HTTP fixture or credentials are involved in these local preparation cases.

## Execution evidence

Every Cargo command ran from the dedicated worktree root. The task gate runner sets the command-local xcrun SDK correction, removes any inherited hosted credential, leaves repository runner/profile configuration unchanged, preserves Cargo's exit code and writes raw logs plus sibling `.exit` files under `private verification archive`:

```sh
SDKROOT="$(xcrun --show-sdk-path)" CC=/usr/bin/clang \
CFLAGS="-isysroot $(xcrun --show-sdk-path)" \
CARGO_TARGET_DIR=private verification archive cargo <arguments below>
```

| Log stem | Exact Cargo arguments | Observed exit/counts |
| --- | --- | --- |
| decoder-red-native | nextest run -p sunstack-evidence --test evidence_plan_decoding --no-fail-fast | 100; 6 run: 1 positive pass, 5 intended diagnostic failures, 0 skip |
| decoder-red-cli | nextest run -p sunstack-rule-cli --test process_protocol -E 'test(unknown_field_inside_full_scope) \| test(malformed_named_source)' --no-fail-fast | 100; 2 failed, 2 filtered |
| decoder-green-native | nextest run -p sunstack-evidence --test evidence_plan_decoding --no-fail-fast | 0; 6 passed, 0 skipped |
| decoder-green-cli | nextest run -p sunstack-rule-cli --test process_protocol -E 'test(unknown_field_inside_full_scope) \| test(malformed_named_source)' --no-fail-fast | 0; 2 passed, 2 filtered |
| decoder-clippy | clippy --workspace --all-targets -- -D warnings | 101; result_large_err, corrected by boxing Diagnostic |
| decoder-focused-final | nextest run --workspace -E 'binary(evidence_plan_decoding) \| test(unknown_field_inside_full_scope) \| test(malformed_named_source)' --no-fail-fast | 0; 8 passed, 207 filtered |
| decoder-nextest | nextest run --workspace | 0; 208 passed, 7 ignored cases skipped, 0 LEAK, before three added controls |
| decoder-focused-complete | nextest run --workspace -E 'binary(evidence_plan_decoding) \| test(unknown_field_inside_full_scope) \| test(malformed_named_source)' --no-fail-fast | 0; 11 passed, 207 filtered; final workspace run also covers the later stderr assertion |
| decoder-fmt-complete | fmt --all -- --check | 0 |
| decoder-clippy-complete | clippy --workspace --all-targets -- -D warnings | 0 |
| decoder-nextest-complete | nextest run --workspace | 0; 211 passed, 7 ignored cases skipped, 0 failures, 0 LEAK |
| decoder-build-complete | build --workspace | 0 |

The backslash before a vertical bar in this Markdown table only escapes the table separator; the executed nextest expression contains the ordinary `|` operator. Intermediate fmt and Clippy after boxing also passed, with their logs retained under decoder-fmt-final and decoder-clippy-final.

Initial process diagnostic red artifacts: `target/cli-process-evidence/53596-{0,1}` (named-source control) and `53597-{0,1}` (scope control). Initial green: `57521-{0,1}` and `57522-{0,1}` respectively. Final full-suite scope pair: `74684-{0,1}`, prepared/exit 0 then request/unknown_field at repositories/change_scope, refused/exit 2. Final named-source pair: `74222-{0,1}`, prepared/exit 0 then request/unknown_field at exactly sources, refused/exit 2. All four final stderr streams contain only `sunstack-rule: ready` plus newline. Both refused outputs contain diagnostics only; no selected evidence or authored-key/value sentinel is echoed. These raw frames were read, not inferred from the test summary. Each directory contains stdin.json, stdout.json, stderr.log, result.json identifying the actual executable, working directory and exit status. Artifacts are also copied under the task temp directory's cli-process-evidence/.

## Standards, trace and remaining work

Walked every changed Rust source/test against domain-rust and global Rust rules. Decoding owns the translation from transport bytes to typed EvidencePlanInput and closed native diagnostics; the CLI remains a caller. The closest separation principle is “Parse once at the boundary”, and the proof follows “Fixed in CI?” by showing the same diagnostic assertions red then green on the real native/process path. Production introduces no Value trees, casts, unwrap/expect/panic allowances, second runtime, unowned task or storage mechanism. Test Value mutations deliberately construct malformed wire cases. The initial large Err warning was fixed without allowing the lint. No language-standard departure is retained; new modules/tests remain below 200 lines.

Own traces and pairing remain on execution root `private-coordination-record`, including the successor pre-build plan and red checkpoint. The Verifier adopted feasibility under f259d4b with no oracle disagreement and has Lead authority to extend its base report when the successor is declared. No stand-ins, unresolved design questions or owner decisions block this correction. Hosted behavior, other milestones, historical LEAK root cause and final acceptance are not established by these local tests. Build/process artifacts are held while the separately owned observation is active; nothing was removed.

Final checkpoint: the seven deliberate default skips remain five hosted cases, the fixed measurement workload, and offline reconstruction. They are unverified, not counted as passed. No LEAK disposition or rerun was needed on the complete 211-test run. Hold the task build (8.9 GiB) and process artifacts (76 MiB) for independent observation; free space is 84 GiB. No directory removal or integration into a milestone branch was performed.
