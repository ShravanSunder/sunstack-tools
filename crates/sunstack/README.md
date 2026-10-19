# sunstack exec

Run `sunstack exec` (or `cargo run -p sunstack -- exec` from the repository root).
One process consumes one JSON request on stdin and writes one JSON response on
stdout. Relative input paths resolve from the process working directory. Run
from the repository root when using the reviewed examples.

```json
{"protocol_version":1,"operation":"evaluate_rules","input":{}}
```

Replace `input` with the native operation input. Supported operations are
`prepare_evidence`, `run_judgments`, `check_rules`, `prepare_rules` and `evaluate_rules`.
The response is either `protocol_error` with diagnostics, or `operation_result`
whose `result` contains the operation and its unchanged native outcome.
The raw stdin limit is 8,388,608 bytes. Bind large evidence by path.

Exit 0 means the operation completed, including an unfavorable rule verdict.
Exit 2 means invalid input or protocol refusal; 3 means failed or incomplete
execution; 4 means caller cancellation. SIGINT and SIGTERM cancel owned work
and await its settlement before publishing the terminal response. The static
`sunstack: ready` stderr line witnesses signal registration, and
`sunstack: cancellation observed` witnesses signal consumption.

Change-scoped work retains the same protocol version. Native inputs
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
when model work is reachable; it sends no provider request. Local-only rule
preparation and evaluation construct no backend and need no credential. A
model-based judgment, applicability gate or Router still requires valid backend
configuration when it becomes reachable. Evidence preparation needs no credential.

`check_rules` assembles every requested RuleSet and validates its rules without
constructing a backend, reading reviewed documents or sending model requests.
Its input contains `definition_root`, `definitions`, `repositories`,
`payload_schemas`, `limits`, optional `discovery` and `output` (`diagnostics` or `assembled_rules`).
Discovery defaults to `none`, as in runtime input. Connected rules need bounded
discovery settings for admission; check never executes discovery or acquisition.
Repository-root metadata and declared payload schemas are still validated.
It accepts no backend, attempt or request-budget settings. A clean check exits 0;
refusals exit 2 with definition file, line and column. Printed rules are returned
only after a clean check and include each native definition and its source.

RuleSets can combine existing `rule_paths` JSONL files with
`rule_sources: [{"markdown":"standards.md","rules":"standards.rules.json"}]`.
Paths resolve relative to the RuleSet file. A RuleSet with sources needs a `marker_prefix`
such as `K`; an end-of-line `<!-- K014 -->` binds its standard to the `K014`
entry in the strict rules object. Entry `name` becomes the native rule ID;
`target.input` is always explicit, and optional `context` uses native evidence
declarations. The target declaration reserves ID `target`. The assembled question
uses visible Markdown wording, with a default cap of 400 words configurable by
`max_question_words`. Both the Markdown and rules file are definition dependencies.

At runtime, invalid source entries are listed in `skipped_rules`, while valid
rules continue. An unreadable or malformed source contributes one source-level
skip. `rule_coverage` is `full`, `partial` or `none`; partial runs can complete
with exit 0, while no valid rule yields incomplete exit 3. Measurements keep
`invalid_rule_entries` and `skipped_rule_sources` separate from applicability's
`rules_skipped`. Existing JSONL, RuleSet-header and invocation errors retain
their strict refusal behavior.

Run the complete credential-free check/print example from the repository root:

```sh
scripts/cargo-debug-signing-runner.sh target/debug/sunstack exec \
  < docs/specs/2026-10-12-rust-rule-engine/examples/marked-standards.check.request.json
```

The [check input schema](../../docs/specs/2026-10-12-rust-rule-engine/schemas/check-rules-input.schema.json)
and [rules-object schema](../../docs/specs/2026-10-12-rust-rule-engine/schemas/rule-source.schema.json)
describe these boundaries. Preparation and evaluation reports now require coverage,
rule evaluations, skipped rules and their counters; strict report consumers must
update together.

Document targets can select Markdown units independently:

```json
{"kind":"markdown_units","mode":"call_site","units":["paragraph","list_item"],"pattern":"\\bMUST\\s+load\\b"}
```

`mode: unit` produces one occurrence per requested section, paragraph or list
item, with an optional `starts_with` or `pattern` filter. Requested unit kinds
can overlap. `mode: call_site` requires exactly one nonempty filter and produces
one occurrence per match, owned by the innermost requested unit; list items own
their contained paragraphs. Regexes that can match empty text are refused.
Filters use visible text with inline code excluded; fenced content remains.

Occurrence keys include source identity, raw unit ranges, and call-site raw and
visible match ranges. Raw spans use original document bytes and character
coordinates. Several visible matches may share a raw entity span; generated-only
matches use zero-width raw boundary anchors. Question owners carry the full key,
so budget splitting preserves each target's evidence and answer references.

Preparation exposes `rule_evaluations`, including `waiting_for_expansion` behind
a staged different-input gate. No child keys or target selections are invented
while it waits. Evaluation reports `expanded`, `zero_matches`, stopped, failed
or cancelled work explicitly. Zero matches is complete when other work completes;
it is never a passed verdict. Same-input speculative carriers remain permitted.
Existing change-scope repository snapshots may already hold the reviewed bytes;
canonical target selection and expansion still wait for their gate frontier.

The evidence library also supports `frontmatter_field` with a literal top-level
`field`. An initial YAML `---` block yields a decoded JSON value or an absent
field. Missing or malformed blocks yield explicit unavailable data without an
invented span. Noul and composite rules reject this mechanical-only target.

A rules entry can replace its judgment threshold with one local check:

```json
{"name":"skill.description","target":{"input":"skill","select":{"kind":"frontmatter_field","field":"description"}},"expect":{"starts_with":"Use when"},"severity":"error"}
```

`expect` accepts exactly one of `matches`, `not_matches`, `starts_with` or
`present: true`. Text predicates use the unit's rendered-visible text, excluding
inline code. A frontmatter string uses its decoded text; other values use
canonical JSON text. Null is present. Every predicate fails on an absent field;
a missing or malformed header is unresolved.

`ast_grep` accepts a native rule object validated by ast-grep-config 0.51.0.
It sees the full original Markdown syntax tree, including ancestors outside
the unit. A match fully contained in the target unit is a failure; sibling
matches outside it are excluded. Frontmatter fields use `expect`; runtime unit
kinds do not select the metadata block. The marked standard becomes the local
result's explanation and is not sent to a model.

Reports use `checked` occurrences with `completed` pass/fail, `unresolved` or
`cancelled` local outcomes. A failed check retains its declared severity and
can complete with exit 0. Unresolved checks make the report incomplete. Local
checks create no model answers, references, receipts or confidence and consume
no model question/request budget. Applicability and routing still govern them.

The [mechanical invocation](../../docs/specs/2026-10-12-rust-rule-engine/examples/mechanical.invocation.json)
checks a description and a load instruction without a credential. Wrap that
native input in the usual `evaluate_rules` protocol frame. The companion
[check request](../../docs/specs/2026-10-12-rust-rule-engine/examples/mechanical.check.request.json)
validates and prints both standards without reading the reviewed skill.

Rust callers supply `RuleBackendFactory` in `RuleExecutionContext`. Selection
is credential-free; the invocation calls `prepare` once when its first model
request reaches encoding and caches the result. Concrete built backends already
implement the factory trait and can still be passed with `Arc::new(backend)`.

The backend-free
[unit-target check request](../../docs/specs/2026-10-12-rust-rule-engine/examples/unit-targets.check.request.json)
checks a rule that selects every paragraph of the existing example document.
Like other checks, it validates the rule without reading that document.

Build with the pinned root toolchain and the SDK environment described in the
project verification skill. For a local development invocation, pass a request
file to `scripts/cargo-debug-signing-runner.sh target/debug/sunstack exec`.
The reviewed skill-lint input lives in
`docs/specs/2026-10-12-rust-rule-engine/examples/skill-lint.invocation.json`.
The handoff example contains illustrative transcript/schema paths; the permanent
journey fixture supplies the accepted generated schema and a documented Codex
transcript composition.

Run `cargo nextest run -p sunstack` for local process checks.
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
