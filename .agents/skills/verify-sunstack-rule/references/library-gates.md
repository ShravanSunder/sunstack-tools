# Native libraries and quality gates

This route checks the Rust workspace and existing native consumers. Inputs are the declared product/design revisions and a permitted evidence directory. Return command exits, test counts and which V obligations the real harness actually covered.

## Run the gates

Use the macOS SDK prefix from `SKILL.md` for each command, from the monorepo root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
```

Record output and the command's own exit status, preserving it when piping to a log. The default local suite excludes ignored hosted tests. A zero-test run is not a behavioral pass. Do not add `--no-tests pass`, skip failures or weaken leak handling. Cargo Clippy checks compilation/types as well as the workspace lint rules.

At the milestone 1 target the default workspace suite has seven deliberate skips: three native hosted cases, two CLI hosted journeys, the fixed measurement workload and the offline preserved-request diagnosis. A skip leaves its claim unknown unless separately observed or validly reused. A workspace run that includes all 23 CLI local cases can also confirm the executable and controlled-journey recipes; reuse that same run instead of repeating the package/subset command.

### Interpret nextest LEAK markers

The Lead-dispositioned full-workspace observations fit a harness/platform explanation; no product defect is established and the exact cause is unproven. Bypassing the signing runner did not eliminate the markers. Recorded comparison rates were 25/8600, 21/8600 and 15/8600 test launches under different instrumentation, not estimates of a runner effect; isolated named-test and cancellation-binary workloads had zero markers.

Reuse that disposition **only** for a matching signature: full-workspace concurrency, exit 0, no assertion failures and a comparable low rate. Record the count, test names, workload denominator, capture settings and limits. A single suite cannot establish a recurrence rate; if comparability is uncertain, retain that uncertainty and triage rather than declaring it benign. Triage anything changed, unexplained or tied to test-owned spawned work, even if concurrency-only. Preserve available participating PID/descriptor evidence before another run; missing ownership remains unknown. The gate stays nextest exit 0. Do not raise timeouts, add retries, bypass the official runner or disable capture to clear a LEAK. A later zero-marker run does not prove the historical recurrence fixed.

For an authorized focused observation, the existing package routes are:

```sh
cargo nextest run -p sunstack-decisions
cargo nextest run -p sunstack-evidence
```

Inspect manifests and, when the assignment permits, run `cargo tree -p <crate> -e normal`. Evidence must have no provider/HTTP dependency; decisions must have no repository/AST/transcript or sibling Sunstack dependency; rules composes evidence and decisions; the CLI calls all three. This is graph evidence, not execution of an independent consumer (V1/V15).

## Check the behavior behind green

Take expectations from Specification C1–C5, C8, C11 and V1/V4/V6/V8–V10/V13/V19/V24–V28. The following existing harnesses drive these boundaries. Open the selected tests, including setup and assertions, before reusing their pass:

| Harness under `crates/` | Required observation and a wrong behavior it must reject |
| --- | --- |
| `sunstack-decisions/tests/native_contracts.rs` | Rich recursive JSON/null, mixed primitives, exact names/kinds/labels/rubric indices; refuse malformed response without salvaging partial answers. Valid additive receipt metadata remains allowed. |
| `sunstack-decisions/tests/http_adapter.rs` | Real reqwest with controlled HTTP, complete structured body, bounded response/attempts and usage. Fixture responses cannot establish hosted Jev. Rules isolation waits for rules integration. |
| `sunstack-decisions/tests/request_budget.rs` | Assessed encoded bytes equal bytes delivered over HTTP; exact/estimated/unavailable policies, escaping/overhead, backend binding and zero dispatch on refusal. Synthetic exact accounting does not prove a real tokenizer. |
| `sunstack-decisions/tests/{diagnostic_privacy,cancellation_contract}.rs` and crate unit tests | Useful bounded safe diagnostics, cancellation before dispatch/admission and after completion, no new attempt and owned settlement. Raw provider content and sentinel credentials must stay absent. Remote billing remains unknown. |
| `sunstack-evidence/tests/document_selection.rs` | Real Markdown paragraphs/ranges, list/wrapped IDs, code/fence/quote exclusions, missing/duplicate/deleted IDs, actual text/AST matches without symbol-identity claims. |
| `sunstack-evidence/tests/{conversation_capture,adapter_extension}.rs` | Qualified public fixture literals, whole messages, tool outcomes/fidelity/categories, explicit gaps/tails, extension and rejection through actual capture. Cursor controls return a first/changed supplement once, suppress unchanged replay, and return a later committed record once while clearing its pending digest. Fixtures do not qualify installed host formats. |
| `sunstack-evidence/tests/payload_inputs.rs` | Real generated AgentJobPacket schema, offline reference catalog, payload pointers, safe mismatch diagnostics. Source provenance is in `crates/sunstack-evidence/tests/fixtures/README.md`. |
| `sunstack-evidence/tests/evidence_preparation.rs` | Public native preparation, requested selected content, required versus optional refusal, limits, lazy reads, projections and cursor continuation/discontinuity controls. |
| `sunstack-evidence/tests/evidence_session.rs` | Original bytes survive later alias selection after mutation/deletion/replacement, one capture/cumulative limits, lazy open, invalid selector before reads and first-capture containment. Session proof does not show rules uses one session. |
| `sunstack-evidence/tests/acquisition_limits.rs` | Failed UTF-8/adapter admission still consumes acquired file/byte allowances; metadata rejection before reading consumes no content budget. |
| `sunstack-rules/tests/` | Public rules composition: admission/binding controls, ordered conditions, router slots, staged/speculative execution, evidence refusal, deterministic splits, equivalence, verdict bands/composites and definition/schema agreement. Full check/source maps are in `crates/sunstack-rules/tests/PROOF.md` and `crates/sunstack-rules/tests/FOLLOWUP-PROOF.md`; controlled HTTP is not hosted inference. |

The native path is exported `prepare_evidence` → `EvidenceSession::open/select/close` → evidence capture/selection → typed plan; it has no backend. Existing tests are external consumers, with crate unit cases adding controlled adapter observation points. Read `crates/sunstack-evidence/tests/PROOF.md`, `crates/sunstack-evidence/tests/SESSION-PROOF.md`, `crates/sunstack-evidence/tests/SUPPLEMENT-CURSOR-PROOF.md` (which supersedes the evidence `PROOF.md` late-supplement row), `crates/sunstack-evidence/tests/ACQUISITION-LIMITS-PROOF.md` and `crates/sunstack-decisions/PROOF.md` for coverage/sensitivity limits; their historical run summaries are not fresh proof.

## Open coverage

The milestone 1 target has real rules and executable implementations. Native library tests do not replace process or hosted observations; use the main skill's scoped reference calls for those. V5/V16–V18 and change-scoped V7 remain milestone 2 drafts. PreparationFailed runtime reachability, platform artifact/release proof, installed-host qualification and exact tokenizer agreement remain separate proof limits. Do not extrapolate the available suite to them.

Complete when the selected tests' real boundaries and rejection cases match the adopted claims, the gates have preserved results, and all remaining V scope is explicitly observed, reused with applicability, or draft with its prerequisite named. No product edits belong to this route.
