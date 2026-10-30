# sunstack

Lint repository documents with a marked rulebook, ast-grep evidence and explicit
decision steps. Use `sunstack exec` for the native JSON protocol.

## Repository lint

Create a configuration skeleton with `sunstack lint --init`, then add the
referenced Markdown rulebook, JSON rule entries and ast-grep project. Init writes
only `sunstack.lint.json` and refuses to overwrite an existing file.

```json
{
	"rulebook": [{ "markdown": "rulebook.md", "rules": "rulebook.rules.json" }],
	"marker_prefix": "K",
	"check": { "files": ["docs/**/*.md"], "ignore": ["docs/archive/**"] },
	"evidence": {
		"ast_grep": "evidence/sgconfig.yml",
		"kinds": {
			"piece": {
				"rule": "your-rule-id",
				"fields": { "body": { "from": "remainder" } }
			}
		}
	},
	"decision_model": { "model": "jev-latest" }
}
```

Replace `your-rule-id` with a rule in your ast-grep project. An end-of-line marker
such as `<!-- K001 -->` binds a Markdown standard to its `K001` JSON entry.
Each entry names an evidence kind and a decision graph. Steps can check fields
mechanically, ask authored questions, combine answers and route to pass or a
finding. Questions reach the model as authored, with one piece's bounded field
view. The binary embeds six generic Markdown utilities; rulebooks, questions and
domain-specific evidence rules belong to the caller.

Rulebook and ast-grep project paths resolve relative to the configuration file.
Checked-file globs resolve from its Git repository root, or from the config
directory outside Git. Without `--config`, lint searches the current directory
and its parents for `sunstack.lint.json`.

```sh
sunstack lint --config sunstack.lint.json --check
sunstack lint --config sunstack.lint.json --all --evidence
sunstack lint --config sunstack.lint.json --all --evidence piece
sunstack lint --config sunstack.lint.json --all --format json
```

`--check` validates configuration, the rulebook and evidence project, then lists
the assembled rules as JSON. It reads no checked files and sends no model
requests. `--evidence [KIND]` prints picked evidence and its fields as JSON without
asking the model. Both routes work without a credential.

An ordinary lint run uses changed evidence by default and requires Git for that
default scope. Use `--all` for every configured target, `--since REV` for changes
since a revision including worktree edits, or name files or directories for full
scope within those paths. Findings retain their rule, marker and source
location. Available output formats are `default`, `json`, `github` and `sarif`.

The decision model is always configured and enabled for asking rules; there is
no model-off switch. Ordinary runs with asking rules require an OpenRouter
credential before checked-file acquisition, even if a later mechanical branch
would avoid an ask. Mechanical-only rules need no key. Credential lookup and
saving are described below under the native protocol.

Lint exits 0 for clean or nothing selected, 1 for error findings or warnings over
`--max-warnings`, 2 for invalid input, 3 for incomplete work, 4 for cancellation,
and 5 for uncertain decisions. Uncertainty retains the decision step, sent state
and answers for the caller to inspect. Cancellation takes precedence over
incomplete work, then findings, then uncertainty. A warning alone does not fail
unless it exceeds `--max-warnings`.

See the checked-in [worked configuration](tests/fixtures/rulebook-lint/skill-rules.config.json)
and its [rulebook entries](tests/fixtures/rulebook-lint/rulebook/steering.rules.json)
for a complete test example. Copy the fixture directory to an isolated root to
try it; its checked-file globs are relative to that root. The example's
skill-specific rules are test content.

## Native JSON protocol

Run `sunstack exec` (or `cargo run -p sunstack -- exec` from the repository root).
One process consumes one JSON request on stdin and writes one JSON response on
stdout. Relative input paths resolve from the process working directory. Run
from the repository root when using the reviewed examples.

```json
{ "protocol_version": 1, "operation": "evaluate_rules", "input": {} }
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
{ "change_scope": { "kind": "uncommitted", "layers": ["unstaged"] } }
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

The binary uses a non-empty `OPENROUTER_API_KEY` first, otherwise the OS credential
store (`sunstack` / `openrouter-api-key`): macOS login Keychain or Linux Secret
Service. When the environment key is present, a run never reads or writes the store.
Runs never save credentials automatically. Use `sunstack credentials save` to save
the current environment key explicitly; it is listed in `sunstack --help`.
The command reports its destination without printing the key and refuses a missing
or empty variable. Set
`SUNSTACK_CREDENTIAL_STORE=off` to disable both lookup and saving (for CI or a one-off
key). Library callers pass the key as a value to `sunstack_lint::run`.
Without a terminal on stdin, macOS access that needs Keychain approval is treated
as unavailable immediately. Run once from a terminal to approve a changed build,
or supply the environment key. Interactive terminal runs may show the Keychain dialog.
On Linux, a locked Secret Service collection may prompt during fallback lookup or explicit saving;
use `SUNSTACK_CREDENTIAL_STORE=off` for unattended runs.

Credentials never come from stdin. Rule preparation uses the selected backend for encoding and accounting
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

Native RuleSets load `rule_paths` JSONL files, with paths relative to the RuleSet
file. Use the repository lint route above for marked Markdown rulebooks. Native
preparation and evaluation retain their coverage, rule evaluations, skipped rules
and counters. JSONL, RuleSet-header and invocation errors remain strict refusals.

Document targets can select Markdown units independently:

```json
{
	"kind": "markdown_units",
	"mode": "call_site",
	"units": ["paragraph", "list_item"],
	"pattern": "\\bMUST\\s+load\\b"
}
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

A rules entry can use a native mechanical verdict. Put the check under
`verdict.check` and select the checked field in the evidence declaration:

```json
{
	"rule_id": "skill.description",
	"evidence": [
		{
			"id": "description",
			"input": "subject",
			"requirement": "required",
			"select": { "kind": "frontmatter_field", "field": "description" }
		}
	],
	"target": "description",
	"applicability": [],
	"questions": {},
	"verdict": {
		"kind": "mechanical",
		"explanation": "Description names when to use the skill.",
		"check": { "kind": "expect", "expect": { "starts_with": "Use when" } }
	},
	"severity": "error"
}
```

Save each native definition as one JSONL line in a RuleSet's `rule_paths`, and
declare its `subject` input as a document in the RuleSet header. Bind that input
to the document when calling `check_rules` or `evaluate_rules`.

`verdict.check.expect` accepts exactly one of `matches`, `not_matches`, `starts_with` or
`present: true`. Text predicates use the unit's rendered-visible text, excluding
inline code. A frontmatter string uses its decoded text; other values use
canonical JSON text. Null is present. Every predicate fails on an absent field;
a missing or malformed header is unresolved.

For an AST check, use `verdict.check: {"kind":"ast_grep","ast_grep":{...}}`.
`ast_grep` accepts a native rule object validated by ast-grep-config 0.51.0.
It sees the full original Markdown syntax tree, including ancestors outside
the unit. A match fully contained in the target unit is a failure; sibling
matches outside it are excluded. Frontmatter fields use `expect`; runtime unit
kinds do not select the metadata block. Local checks send no model request.

Reports use `checked` occurrences with `completed` pass/fail, `unresolved` or
`cancelled` local outcomes. A failed check retains its declared severity and
can complete with exit 0. Unresolved checks make the report incomplete. Local
checks create no model answers, references, receipts or confidence and consume
no model question/request budget. Applicability and routing still govern them.

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
