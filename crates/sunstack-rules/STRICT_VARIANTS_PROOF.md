# Milestone 1 strict input variants — implementation proof

This repair makes fourteen formerly unit variants reject unknown fields through native Serde deserialization. They are empty struct variants now; canonical valid JSON stays byte-identical. Mechanical Rust constructors and patterns follow that change. No parser, schema file, diagnostic contract, dependency, output-only enum, or CLI production code changes.

Branch: `rule-engine/m1-strict-variants`, based on accepted milestone 1 `d4933678f88fff507c8a630be7e55db65b0470d9`. Dedicated worktree: `/Users/shravansunder/dev/sunstack-tools.rule-engine-m1-strict-variants`. The final commit is declared separately after this note and the tested source are committed together.

Scope comes from the Lead's repair packet and direct correction: `EvidenceDiscovery::None` exists in milestone 1 and belongs here. `InputDeclaration::ChangeSet` is shape-reachable even though milestone 1 runtime does not implement change-set evaluation. RevisionHead, later ChangeScope variants, and UnitContent are absent at the base. Hosted calls are forbidden for this repair and were not attempted.

Risk: downstream Rust code constructing the fourteen variants must use `{}`. JSON clients that previously supplied undeclared fields now receive the existing invalid-input outcome. Public valid JSON and intentionally open authored content retain their existing forms. Independent target observation and acceptance belong to the paired Verifier and Lead respectively; this document records implementer observations only.

## Sweep

The roots are the CLI protocol envelope's four operation inputs: `EvidencePlanInput`, `RunJudgmentsInput`, `RuleInvocationInput` (both prepare/evaluate), and rule-set/rule/router definition files. Traversal follows struct fields, maps, vectors and enum payloads, rather than counting every `Deserialize` derive. All twenty input-reachable internally tagged enums have `deny_unknown_fields`. Source anchors below are crate-relative, and the lines hold at both base and repaired source.

| Crate / source | Enum | Former units converted | Other variants / proof |
| --- | --- | --- | --- |
| evidence/src/evidence_contracts.rs:103 | SourceSpec | none | four fielded variants tested |
| evidence/src/evidence_contracts.rs:124 | QueryPath | none | two fielded variants tested |
| evidence/src/evidence_contracts.rs:130 | ChangeScope | Full | isolated native red/green |
| evidence/src/evidence_contracts.rs:173 | EvidenceSelection | WholeDocument, WholePayload | isolated native red/green; six fielded alternatives tested |
| evidence/src/conversation_contracts.rs:24 | TurnWindow | All, SinceCursor | isolated native red/green; Last tested |
| evidence/src/conversation_contracts.rs:31 | ToolFilter | All | isolated native red/green; Named/Categories tested |
| evidence/src/conversation_contracts.rs:38 | ToolUseSelection | Excluded | isolated native red/green; Included tested |
| evidence/src/preparation_contracts.rs:20 | EvidenceSelector | none | five fielded variants tested |
| evidence/src/preparation_contracts.rs:72 | EvidenceDiscovery | None | isolated native red/green; Bounded tested |
| decisions/src/decision_contracts.rs:98 | JudgmentQuestion | none | type-tagged; three fielded variants tested |
| decisions/src/decision_contracts.rs:171 | BackendSelection | none | one fielded variant tested |
| rules/src/definition_contracts.rs:60 | InputDeclaration | Document, Conversation, ChangeSet | isolated native red/green; Payload tested |
| rules/src/definition_contracts.rs:104 | VerdictDeclaration | none | two fielded variants tested |
| rules/src/definition_contracts.rs:150 | ApplicabilityEntry | none | two fielded variants tested |
| rules/src/definition_contracts.rs:165 | ConditionTest | none | five fielded variants tested |
| rules/src/definition_contracts.rs:202 | PresencePattern | none | three fielded variants tested |
| rules/src/invocation_contracts.rs:27 | InvocationDefinitions | none | two fielded variants tested |
| rules/src/invocation_contracts.rs:53 | InputBinding | none | four fielded variants tested |
| rules/src/invocation_contracts.rs:85 | InvocationOrigin | Direct | isolated native red/green; Hook tested |
| rules/src/invocation_contracts.rs:103 | OriginAgent | Main, Unknown | isolated native red/green; Subagent tested |

Crate aliases in this table mean `crates/sunstack-{evidence,decisions,rules}`. Nine affected enums contain fourteen affected variants. The eleven already fielded enums remain unchanged. No decisions production change is needed.

Output-only enums are excluded even if they derive Deserialize: reported origin/agent, ObservedField, diagnostics, selection/report/capture outcomes and normalized conversation output. Provider answers are not operation or authored-definition input. MonitoringCursor is decoded from its returned string and contains no tagged enums. Ordinary string enums (for example Inclusion, AssistantSelection, AccountingPolicy, Severity, Speculation) remain strings. StructuredJson, StructuredEntry and SharedEvidenceState represent open authored JSON and are intentionally untagged. The protocol envelope is a strict struct with a raw input; OperationInput is a dispatch enum without Deserialize.

The locked serde/serde_derive version is 1.0.229. Its primary source explains the defect: `serde_derive/src/de/enum_internally.rs:82-104` chooses InternallyTaggedUnitVisitor for a unit and the struct deserializer for a struct variant. `serde/src/private/de.rs:2956-3005` consumes the unit's map through IgnoredAny and returns success. This was checked against the installed registry source, then reproduced through actual native types and the binary. No replacement deserializer or recursive JSON validator was added.

## Permanent scenarios and pre-fix failures

`tests/strict_input_variants.rs` has fourteen independently scheduled tests. Each parses the canonical literal `{"kind":"<variant>"}`, serializes it back to the exact original bytes, injects `unexpected: 1`, and requires native deserialization to fail. Every one failed before the repair because the injected object was accepted. A first panic cannot conceal a later variant.

`tests/strict_fielded_inputs.rs` has thirteen unchanged-behavior controls. It checks every fielded variant of the twenty enums: valid native roundtrip and refusal of an extra field at the variant root. It also preserves ordinary string enum wires and nested authored content containing legitimate `kind` and `unexpected` keys in payloads and decision instructions. These checks do not use a schema as a substitute for native parsing.

`tests/strict_input_boundaries.rs` has five containing-input scenarios, each with a positive decode before changing one nested object: EvidencePlanInput; decode_rule_invocation (direct plus main/unknown hook agents); decode_rule_set (all three empty input declarations); decode_router; and decode_rule_definition (conversation window/tool projection). The rule decoders must return their existing UnknownField diagnostic at a Definition location. These tests stop at parsing, without unsupported runtime admission standing in for a shape refusal.

`sunstack-rule-cli/tests/process_protocol.rs::unknown_field_inside_full_scope_is_refused_at_the_real_process_boundary` launches the actual signed debug executable via the existing unchanged process driver. It creates a real document and sends a valid prepare_evidence envelope. The positive result must prepare the exact paragraph. It then injects only `/repositories/0/change_scope/unexpected`. Before repair both invocations returned operation_result / prepare_evidence / prepared, exit 0. After repair the second returns operation_result / prepare_evidence / refused, invalid_selector, exit 2, with no selections, measurements, selected content, or echoed unexpected field. This is the existing operation-input classification, not a protocol-envelope error.

The initial native run had one unrelated test-fixture mistake: the fielded ApplicabilityEntry roundtrip omitted EvidenceDeclaration's serialized default `requirement`. That fixture was corrected without changing production. The original log is retained; the corrected pre-fix run establishes the complete intended baseline.

## Commands and artifacts

Commands execute from the dedicated worktree root with a command-local environment:

```sh
SDKROOT="$(xcrun --show-sdk-path)" CC=/usr/bin/clang \
CFLAGS="-isysroot $(xcrun --show-sdk-path)" \
CARGO_TARGET_DIR=/private/tmp/2026-10-m1-strict-variants/build cargo <arguments below>
```

The local gate runner `/private/tmp/2026-10-m1-strict-variants/run_gate.py` sets those exact values, removes any inherited hosted credential, streams output to a persistent log, and preserves Cargo's exit code. The repository's signing runner and default nextest profile are unchanged. Rust is 1.98.1; nextest is 0.9.137. Logs and sibling `.exit` files live in `/private/tmp/2026-10-m1-strict-variants/`.

| Log stem | Exact Cargo arguments | Exit / observed counts |
| --- | --- | --- |
| red-native | nextest run -p sunstack-rules --test strict_input_variants --test strict_fielded_inputs --test strict_input_boundaries --no-fail-fast | 100; 32 run: 12 pass, 20 fail; includes one fixture mistake |
| red-native-corrected | nextest run -p sunstack-rules --test strict_input_variants --test strict_fielded_inputs --test strict_input_boundaries --no-fail-fast | 100; 32 run: 13 pass, 19 intended fail, 0 skip |
| red-cli | nextest run -p sunstack-rule-cli --test process_protocol -E 'test(unknown_field_inside_full_scope)' | 100; 1 fail, 2 filtered out |
| green-native | nextest run -p sunstack-rules --test strict_input_variants --test strict_fielded_inputs --test strict_input_boundaries --no-fail-fast | 0; 32 pass, 0 skip |
| green-cli | nextest run -p sunstack-rule-cli --test process_protocol -E 'test(unknown_field_inside_full_scope)' | 0; 1 pass, 2 filtered out |
| fmt | fmt --all -- --check | 0 |
| clippy | clippy --workspace --all-targets -- -D warnings | 0 |
| nextest | nextest run --workspace | 0; 201 pass, 7 ignored cases skipped, 0 fail |
| build | build --workspace | 0 |

Raw real-process observations are in `target/cli-process-evidence/` in this worktree. Pre-fix: `59417-0` valid / `59417-1` malformed, both exit 0 prepared. Post-fix focused: `82662-0` valid exit 0 prepared / `82662-1` malformed exit 2 refused. Each directory contains stdin.json, stdout.json, stderr.log and result.json identifying binary, working directory, exit and input size. Both focused post-fix stderr files contain only `sunstack-rule: ready` plus newline. These are synthetic fixtures, not hosted responses.

No LEAK markers occurred in the focused red/green runs. The full-workspace default run also had zero LEAK markers. Its seven skipped cases are five hosted tests, the fixed journey measurement workload, and offline reconstruction requiring a preserved failed invocation; none was counted as passed. A LEAK is not treated as an assertion pass or retried away: the runner captures process/stdio inventory and the existing Lead disposition applies only if the observed signature matches it.

## Standards and proof limits

Reviewed the changed Rust files against domain-rust and the repository/global Rust rules. Production edits are exclusively enum shape declarations and mechanical constructors/patterns. Existing typed boundary validation and thiserror diagnostics remain owners. There are no new dependencies, casts, runtime instances, tasks, locks, storage formats, production Value trees or production unwrap/expect/panic allowances. Test-only JSON mutations construct intentionally malformed wire inputs; test unwrap/expect/panic allowances follow existing test convention. Existing file responsibilities and lengths are preserved; new test files are below 200 lines. No language-rule departure remains or was kept for this repair.

The closest verification-principle case is “Fixed in CI?”: the same permanent checks were observed failing before and passing after the fix on the isolated target. The reader-principle move is to publish only commands actually run and bind each claim to its log, rather than counting summaries as proof. No stand-ins are used for the repaired parsing behavior; the CLI scenario runs the real boundary. Local checks do not establish hosted-provider behavior or merged milestone behavior. Those are outside this repair's grant. Independent final observation remains unverified until separately commissioned and returned.

Cleanup checkpoint: hold the 5.4 GiB task build and 25 MiB process artifacts while the immutable target observation is pending. The worktree is unmerged and its evidence is an active review input, so removal is premature. Free space after proof is 95 GiB. Raw process artifacts are also copied to `/private/tmp/2026-10-m1-strict-variants/cli-process-evidence/`; no directories were removed.
