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

The native path is exported `prepare_evidence` → `EvidenceSession::open/select/close` → evidence capture/selection → typed plan; it has no backend. Existing tests are external consumers, with crate unit cases adding controlled adapter observation points. Read `crates/sunstack-evidence/tests/PROOF.md`, `crates/sunstack-evidence/tests/SESSION-PROOF.md`, `crates/sunstack-evidence/tests/SUPPLEMENT-CURSOR-PROOF.md` (which supersedes the evidence `PROOF.md` late-supplement row), `crates/sunstack-evidence/tests/ACQUISITION-LIMITS-PROOF.md` and `crates/sunstack-decisions/PROOF.md` for coverage/sensitivity limits; their historical run summaries are not fresh proof.

## Open coverage

At the initial target, `sunstack-rules/src/lib.rs` and CLI are placeholders. Native rules consumption/composition, V3/V7/V12/V14/V20–V23/V31–V35, full V9 cross-RuleSet isolation and process artifact behavior remain draft in the other routes. V5/V16–V18 need milestone 2. Do not extrapolate the available decisions/evidence suite to those obligations.

Complete when the selected tests' real boundaries and rejection cases match the adopted claims, the gates have preserved results, and all remaining V scope is explicitly observed, reused with applicability, or draft with its prerequisite named. No product edits belong to this route.
