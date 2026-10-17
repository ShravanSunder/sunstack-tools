# Executable phase 1 proof

Base: accepted milestone 1 `07214f5749b3499276dff6b4f56127d9b9921f31`.
Oracle: Specification C14/V2/V13 and Program Design at `34bee48`, with
contracts amended by `92d6bc6` (8 MiB), `8ac4133` (stderr readiness) and
`a32d32f` (optional provider ceiling). Execution trace:
`private-coordination-record` on Router service
`0ff962c5-7fa3-4c18-a5ca-1bbe8db09e89`.

This target implements **prepare_evidence and run_judgments only**.
Prepare_rules/evaluate_rules remain unsupported until phase 2, after the
Lead accepts S3a. This is not completion of all V2 or the milestone journeys.

The executable serializes the accepted milestone 1 EvidencePlanOutcome
unchanged. `change_selections` is milestone 2 and absent at this target,
as the Lead explicitly confirmed in push `private-coordination-record`.
No evidence API repair or CLI shape shim was added.

## Real path and ownership

Permanent tests spawn the actual `CARGO_BIN_EXE_sunstack-rule` through the
unchanged absolute Cargo signing-runner path. Each child has explicit CWD,
pipes and a synthetic environment credential; no credential is in stdin.
They inspect the child exit and parse exactly one complete stdout JSON value.
The process owns one Tokio runtime. Protocol code bounds stdin before parsing;
library types own input admission and outcomes. Judgment input decoding calls
the decisions crate's existing `judgment_validation::decode_run_input`, retaining
its closed codes and schema-only diagnostic paths. The CLI adds only the envelope,
operation delegation, signals and exits. Its closed protocol diagnostics copy
no received decoder message, key or value. No casts or duplicate domain shapes
were introduced. RawValue preserves operation-input JSON until native decoding.

Both Unix signal streams are installed before the contracted ready line is
flushed and before stdin is read. The harness permits the signing runner's
documented best-effort unsigned warning before the actual ready line; that
warning never counts as readiness. Tests await that real child's ready line,
signal its PID, and await the static `cancellation observed` stderr line before
continuing. Thus registration and token observation are separate witnesses.
Static CLI operational text is below 256 bytes per invocation, including error
paths. The signal loop keeps polling owned library work after cancellation;
terminal serialization follows the awaited operation, never a detached worker.

## Scenarios and independent oracles

Expected envelope/operation tags, outcome names and exits below are literals
from C14 and the contracts. The real HTTP response fixtures are reused from
the accepted decisions test support; expectations are not computed by CLI
exit mapping or production verdict/accounting code. Transcript compositions
are explicitly synthetic, not live-host qualification.

| Permanent test | Wrong behavior it rejects / real boundary |
| --- | --- |
| `raw_protocol_errors_never_fabricate_an_operation_result` | Empty/truncated JSON, non-object/multiple frames, raw duplicate/unknown envelope keys, unknown version/operation must give protocol_error/2 without result; actual process framing |
| `evidence_success_and_required_optional_refusals_use_real_selections_and_exits` | Missing selected text, wrong operation, unselected-text leakage, required refusal treated as success or optional refusal treated as incomplete; actual Markdown selection and 0/2/3 exits |
| `real_binary_runs_mixed_judgments_with_admitted_receipt_and_attempts` | Fake completion or dropped native primitive/receipt; real process to reqwest to recording HTTP, literal mixed answers and one actual dispatch |
| `malformed_judgment_input_keeps_the_native_code_and_safe_schema_path` | CLI replacing the native invalid_question code/path or echoing a dynamic question name; actual binary to the public native decoder |
| `real_binary_distinguishes_invalid_request_provider_failure_and_invalid_response` | Known malformed input becoming protocol_error, budget refusal dispatching, failed work becoming answers or success; operation results/2, both failure variants/3, zero-call observer |
| `real_binary_keeps_configured_provider_attempts_finite` | Hidden extra retry or missing attempt; actual three HTTP requests and three attempt records under retry_count=2 |
| `stdin_raw_byte_bound_accepts_exact_limit_and_refuses_excess_without_eof_or_parse` | Rejecting exact 8,388,608 bytes, parsing over-limit malformed input, or waiting on an open writer past limit+1; real byte stream |
| `both_real_signals_are_observed_before_operation_admission_without_dispatch` | Killing before handler registration, mistaking kill return for consumption, dispatch after cancellation, fabricated evidence progress; both SIGINT/SIGTERM and both operations, real consumed-signal witness and zero observed HTTP |
| `in_flight_sigterm_keeps_remote_uncertainty_and_starts_no_retry` | Losing cancellation/remote uncertainty or starting retries; full real HTTP request received then held, SIGTERM, socket EOF, one attempt despite retry_count=3, joined child |
| `evidence_worker_settles_before_terminal_stdout_after_sigterm` | Publishing while owned local acquisition is blocked; real FIFO schema read, writer-open rendezvous, signal consumed, bounded no-output observation, release, admitted-plan cancellation/4 |
| `relative_transcripts_and_wire_cursor_paths_follow_each_actual_process_cwd` | Resolving relative transcript/cursor paths against a shared wrong root or replaying old records; same relative spelling in two CWDs, append/resume and whole new assistant message |
| `invalid_payload_and_malformed_requests_keep_received_values_out_of_diagnostics` | Echoing author keys/payload values into errors; real offline schema refusal and malformed stdin, useful closed diagnostics |

Every decoded response recursively checks diagnostic list length <=100,
guidance <=512 characters, safe field-path length <=32, and sentinel absence
from diagnostic content. Requested selected text remains allowed on stdout.
Provider failure bodies and credentials stay out of stdout/stderr diagnostics.
Evidence preparation has no backend in its public API; source inspection of
that actual path corroborates zero provider execution. An unrelated listener
alone is not treated as universal egress proof.

The FIFO's 100 ms negative observation is explicitly bounded, not a claim
about arbitrary delays. It starts after cancellation consumption, not before
registration. The real native schema-reader await and CLI operation await
corroborate ordering; release permits the actual worker to settle before the
cancelled outcome. A deliberate early-publication fault was caught there.
Socket closure is local ownership evidence, not proof that remote inference
or billing stopped. Native controller ownership tests also run in the full gate.

## Observed chronology and gates

Every Cargo command used this command-local SDK prefix from the monorepo root:

```sh
SDKROOT="$(xcrun --show-sdk-path)" CC=/usr/bin/clang CFLAGS="-isysroot $(xcrun --show-sdk-path)"
```

- Initial `cargo nextest run -p sunstack-rule-cli --test process_protocol`:
  `d55cf97c-9609-4291-9586-10ecec8ed608`, exit 100, two failed against the
  successfully built empty main: missing response / exit 0 instead of 2.
  Log: `target/cli-protocol-initial-red.log`.
- First implementation exposed a test-fixture error: `MISSING` is not a valid
  TargetId and reached decode refusal/2. Source inspection corrected it to
  valid absent `U999`, reaching required selection refusal/3. No product
  behavior was weakened for that correction.
- Scoped 11-process green: `006e7630-5f0f-40aa-852b-9e37bfcc1d8e`, exit 0.
  New judgment, cancellation, bounds, privacy and cursor cases first ran green;
  no pre-existing defects or pre-fix reds are claimed for those additions.
- Post-green sensitivity `6763e4f8-9648-4346-9bb3-5fa468130e32`: intentionally
  publishing stdout upon cancellation and forcing judgment exits to 0 made
  the worker-order and failure-exit cases fail (two failed, exit 100).
  Log: `target/cli-publication-exits-sensitivity.log`.
- Post-green sensitivity `1565b5c9-5093-487e-8418-2f00ce0831d2`: increasing
  the bounded reader to limit+2 made the open-writer limit+1 scenario time out
  for its intended reason (one failed, exit 100). Log:
  `target/cli-bound-sensitivity.log`. All deliberate faults were restored.
- Final source inspection caught a duplicate CLI judgment-input error classifier
  despite the existing native decoder. Regression `9956653d-3297-460a-a55b-ca6f11ac6838`
  failed for invalid_configuration instead of the native invalid_question.
  Delegating to that existing decoder passed in `8cdaa2e6`; the safe path is
  [request, questions], never the dynamic author name. This was reversible
  implementation drift, fixed without an API or contract change. Logs:
  `target/cli-native-decoder-{red,green}.log`.
- Final `cargo fmt --all -- --check`: exit 0, `target/cli-final-fmt.log`.
- Final `cargo clippy --workspace --all-targets -- -D warnings`: exit 0,
  `target/cli-final-clippy.log`.
- `cargo build -p sunstack-rule-cli`: exit 0, `target/cli-final-build.log`.
- Final `cargo nextest run --workspace`: exit 0, 98 passed, one hosted ignored,
  zero LEAK markers, run `747e99ea-158e-43ce-b553-5d2a9a3b1beb`;
  `target/cli-final-workspace.log`.

Rerun the process suite with `cargo nextest run -p sunstack-rule-cli` and the
same SDK prefix. Runtime artifacts are under `target/cli-process-evidence/`:
PID/sequence directories keep stdin, stdout, stderr, CWD, binary and exit.
Raw HTTP request bodies and connection counts, plus the cancellation barrier
observation sequences, are in `target/cli-process-observations/`. Headers and
environment credentials are excluded. Fixture paths are disposable and rebuilt
by the tests; captured raw requests and outputs are retained.

## Open scope

No fresh hosted CLI inference is claimed: phase 1 uses controlled HTTP through
the real adapter. Accepted repair V11 is separate hosted library evidence.
Rules preparation/evaluation, pre/post-rule-report cancellation with completed
peers, V29/V30 genuine Jev journeys, V14 workload/comparator/repetition design
and measurements, other operating systems, deployment and release stay open.
The project's executable verification recipe remains DRAFT; activating it is
a separate maintenance assignment, not a write made under this product grant.
The paired Verifier has not observed this phase 1 target yet. No production
stand-in was added; controlled remote replies do not prove hosted inference.
