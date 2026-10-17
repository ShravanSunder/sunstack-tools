# S3a R37/V20 follow-up proof

This is the single follow-up to frozen target `9aeca1f6fa63b09a105710e2473193032e4f29ee`. The paired Verifier's full observation found only these two gaps, not a native product defect. Lead direct `private-coordination-record` released the freeze and requested one successor on `rule-engine/m1-rules`, with a bounded R37/V20 recheck and reuse of the other independent observations.

## Changed boundary

- Merged accepted milestone `652506499325b1f2c8e931bd7b4964a69faa8b09` at `a917661609b61ae7728678bd2f98a7cde5f472e3`. Its six docs/schema files are imported unchanged, including the reviewed invocation schema's optional positive ceiling. The other imported passages were already the adopted governing corrections; no new implementation or M2 obligation is claimed.
- Replaced the masked shared-file subcase in `admission_preflight.rs` with `shared_rule_file_is_validated_for_each_owner_with_kind_correct_bindings`. The broad matrix now has 35 cases. The new native preparation case gives both owners valid bindings: first Conversation, then Document with a real allowed repository/path. Both load one cached shared rule file containing a Conversation selector. It requires exactly `RuleDiagnosticCode::InputKindMismatch`, the shared rule file's definition location at line 2/column 1, zero normalizer callbacks and zero HTTP. Definition and binding metadata reads are not claimed absent.
- Retained `empty_applicability_and_question_maps_and_reused_ids_across_rulesets_are_admitted` unchanged as the valid shared-file reuse control: two distinct RuleSets load the same valid file and produce two requests/occurrences from one normalized transcript capture.
- Extended `generated_backend_ceiling_schema_matches_native_optional_positive_only_contract`. It now checks full invocation values against native decoding, generated invocation schema **and the reviewed invocation schema**, in addition to the generated/native backend checks. Independent literal expectations: absence, 1 and 262144 accepted; zero, null, negative and fractional values refused.
- Updated the per-test proof inventory. No production Rust, manifests, lock, runner, verification skill, support harness or hosted test changes relative to `9aeca1f`.

## V20 isolation and sensitivity chronology

All work was prepared in the authorized separate worktree `private verification archive` on a local branch from exact `9aeca1f`. The original `m1-rules` tree stayed unchanged during its full observation. The temporary source mutation below is author sensitivity, never a production repair or a claim of a pre-existing defect.

1. Deliberately skipped only cached shared-file per-owner validation in the separate worktree. The original broad admission test stayed green: run `cb468de9-ac6c-44a4-a452-dadbb669ea0d`, exit 0, one passed/three filtered. Its second owner's invalid Conversation binding independently caused refusal, demonstrating the mask.
2. Added the kind-correct binding case and removed the masked subcase, keeping the same deliberate cache bypass. New test failed: run `80f9d3f8-2f9d-4bfd-afa0-02e260b8e643`, exit 100, one failed/four filtered. Instead of Invalid at admission, it reached PreparedWithFailures and read 86 transcript bytes. This is the wrong behavior the isolated control must catch, not a timeout or an unrelated binding error.
3. Restored `invocation_admission.rs` byte for byte before any green gate. All five admission tests passed: run `a6f4ec75-a55c-4f82-bfc9-1b44fa59b8a1`, exit 0, five passed/zero filtered. The dedicated negative retained exact code/location and zero capture/HTTP; the unchanged valid reuse control passed. `git diff 9aeca1f -- crates/*/src` is empty.

Raw logs under the separate worktree's `crates/sunstack-rules/target/v20-proof/`:

- `old-fixture-cached-validation-bypass.log`
- `corrected-fixture-cached-validation-bypass-red.log`
- `corrected-fixture-restored-green.log`

## R37 red and merge green

The extended full-invocation comparison first ran against the unchanged stale reviewed schema. Run `1aaeefe9-e841-49e0-a817-e02ab8c0379a`, exit 100, one failed/four filtered: the reviewed schema rejected the positive ceiling `1`, while native and generated checks accepted it. This is the Verifier's exact counterexample exercised through validators, not a new native decoding defect.

After importing accepted `6525064`, both changed test binaries passed: run `c43c89b0-9e94-4acf-8d32-78aa47e19d6a`, exit 0, ten passed/zero skipped. Positive, absent and every negative ceiling control agreed across the reviewed/generated/native invocation boundaries. Raw `reviewed-ceiling-agreement-red.log` and `consolidated-rows-green.log` retain that chronology. No reviewed schema was hand-edited here.

## Final author gates on the consolidated tree

Every Cargo command ran at the separate worktree root with the pinned toolchain, unchanged signing runner and command-local SDKROOT from xcrun, CC=/usr/bin/clang and matching CFLAGS sysroot. The inherited author log runner immediately captures PID/stdio ownership on a LEAK marker before another run; none occurred.

| Exact command | Result | Raw evidence under `crates/sunstack-rules/target/v20-proof/` |
| --- | --- | --- |
| `cargo fmt --all -- --check` | exit 0 | `followup-final-fmt.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, no warnings | `followup-final-clippy.log` |
| `cargo nextest run --workspace` | exit 0; 145 passed, three hosted ignored; zero LEAK; run `f2fe31dd-577f-4ec1-8fc6-2577a06ebfde` | `followup-final-workspace.log` |
| `cargo tree -p sunstack-decisions -e normal` | exit 0; no Git/AST/jsonschema/other sunstack dependency | `followup-final-decisions-tree.log` |
| `cargo metadata --no-deps --format-version 1` | exit 0; exact four-crate DAG asserted | `followup-final-crate-graph.json` |

The full gate now contains 59 local rules tests plus the 86 accepted library baseline; two rules hosted cases and one decisions hosted case remain ignored by default. No hosted test was rerun for this test/docs-only correction. The two genuine hosted V22/V34 results from the independent `9aeca1f` observation (run `cdd68f8c-fe88-471c-8f71-85bc99c18070`) are explicitly reused after checking unchanged production, support and hosted-test files. Default skips never count as fresh hosted success.

## Independent observation and limits

The new target's R37/V20 observation and Lead acceptance are pending. The broader independent `9aeca1f` result is reused, not presented as a new run. Historical S1 LEAK and the separate S3b recurrence remain unattributed here; these zero-marker runs do not close that investigation. No new production stand-in, CLI, model accuracy, tokenizer, M2, deployment or release claim. The root remains `private-coordination-record`; the exact successor commit is declared there after commit/push.
