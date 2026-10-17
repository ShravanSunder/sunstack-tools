# sunstack-rule

One process consumes one JSON request on stdin and writes one JSON response on
stdout. Relative input paths resolve from the process working directory. Run
from the repository root when using the reviewed examples.

```json
{"protocol_version":1,"operation":"evaluate_rules","input":{}}
```

Replace `input` with the native operation input. Supported operations are
`prepare_evidence`, `run_judgments`, `prepare_rules` and `evaluate_rules`.
The response is either `protocol_error` with diagnostics, or `operation_result`
whose `result` contains the operation and its unchanged native outcome.
The raw stdin limit is 8,388,608 bytes. Bind large evidence by path.

Exit 0 means the operation completed, including an unfavorable rule verdict.
Exit 2 means invalid input or protocol refusal; 3 means failed or incomplete
execution; 4 means caller cancellation. SIGINT and SIGTERM cancel owned work
and await its settlement before publishing the terminal response. The static
`sunstack-rule: ready` stderr line witnesses signal registration, and
`sunstack-rule: cancellation observed` witnesses signal consumption.

Milestone 2 uses the same four operations and protocol version. Native inputs
accept non-Full `change_scope` values (dirty layers, revision ranges, commit lists,
time windows and unions), bounded discovery, ChangeSet sources and explicit
package bindings. A relative repository root resolves from this process's CWD.
For example, select current unstaged changes on a repository binding with:

```json
{"change_scope":{"kind":"uncommitted","layers":["unstaged"]}}
```

This is a binding fragment, not a complete invocation. ChangeSet selectors need
`discovery.kind = "bounded"` with native discovery limits. A package binding's
empty `target_path_prefix` maps to the target repository root. Use the reviewed
[invocation schema](../../docs/specs/2026-10-12-rust-rule-engine/schemas/rule-invocation.schema.json)
for complete inputs.

Preparation and reports expose native `change_selections`; Full-only work has
an empty list. Rule occurrences retain selection reasons such as
`target_changed` or `full_scope`, pending comparisons, and outside-scope results.
Preparing rules sends no model request. A waiting gate leaves target comparison
pending; evaluation resolves it only when the native gate permits the work.
The CLI forwards these native outcomes and uses the same exit meanings above.

The Jev backend reads `OPENROUTER_API_KEY` from the process environment, never
stdin. Rule preparation uses the selected backend for encoding and accounting
and therefore requires valid backend configuration; it sends no provider
request. Evidence preparation does not require a backend credential.

Build with the pinned root toolchain and the SDK environment described in the
project verification skill. For a local development invocation, pass a request
file to `scripts/cargo-debug-signing-runner.sh target/debug/sunstack-rule`.
The reviewed skill-lint input lives in
`docs/specs/2026-10-12-rust-rule-engine/examples/skill-lint.invocation.json`.
The handoff example contains illustrative transcript/schema paths; the permanent
journey fixture supplies the accepted generated schema and a documented Codex
transcript composition.

Run `cargo nextest run -p sunstack-rule-cli` for local process checks.
The explicitly ignored `journey_measurements` test runs the fixed 40-process
controlled workload. The two ignored `hosted_journeys` cases make four genuine
provider invocations and require a command-local credential. They are deliberate
paid observations; preserve failures and obtain a new campaign decision rather
than repeating until favorable. The ignored `hosted_request_diagnosis` is an
offline encoder observation for a preserved failed invocation, with no dispatch.

See [milestone 2 proof](tests/MILESTONE2-PROOF.md) for change-scoped and connected
process coverage and its proof limits. See [phase 2 proof](tests/PHASE2-PROOF.md) for the observed gates, measurements,
fixture provenance, failed campaign and proof limits. [Phase 1 proof](tests/PHASE1-PROOF.md)
is a historical account of the earlier two-operation target.
