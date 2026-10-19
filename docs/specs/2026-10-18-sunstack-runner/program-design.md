# sunstack runner: Program Design

How the internal system satisfies the [Specification](specification.md), which the [Requirements](requirements.md) authorize. The rule engine's own [Program Design](../2026-10-12-rust-rule-engine/program-design.md) stays authoritative for evidence selection, request planning and judgment; this document adds the runner and the two engine changes it needs.

## The structure in one picture

```mermaid
flowchart TB
  subgraph bin["sunstack (binary crate, renamed from sunstack-rule-cli)"]
    args["command line: lint · exec · --version"]
    exec["exec: today's JSON protocol, unchanged framing"]
    out["output and exit code"]
  end
  subgraph lintlib["sunstack-lint (new library crate)"]
    cfg["config: discover, decode, validate"]
    sel["targets: globs, ignores, paths"]
    pol["policy: effective severity and off, per file"]
    inv["invocation: binding groups plus scope"]
    res["outcome: findings, exclusions, incomplete, exit"]
    fmt["formats: default · json · github · sarif"]
  end
  subgraph engine["engine crates (existing)"]
    rules["sunstack-rules (modified: per-group policy, error/warn)"]
    evid["sunstack-evidence"]
    dec["sunstack-decisions (Jev / OpenRouter)"]
  end
  args --> exec --> rules
  args --> cfg --> sel --> pol --> inv --> rules
  rules --> evid
  rules --> dec
  rules --> res --> fmt --> out
  exec --> out
  rust["Rust caller"] --> cfg
  npm["npm launcher"] --> args
```

`lint` turns a config into the engine's existing invocation, adding one optional field per binding group, then reads the engine's report back as lint findings. `exec` keeps today's protocol. Nothing else in the engine changes except the severity words.

## Selected structure

| Choice | Selected | Alternatives considered | Why this one | Cost and who pays | Reopen if |
| --- | --- | --- | --- | --- | --- |
| Where lint lives | A new library crate, `sunstack-lint`, called by the binary | Lint code inside the binary crate | R7 needs Rust callers to get the same report without depending on a binary. A library also keeps the binary thin. | One more crate to version (maintainer) | Lint stays CLI-only after a year with no Rust caller |
| Turning a rule `off` and overriding severity | An optional `policy` on each engine binding group, applied before requests are planned and when a verdict is activated | (a) Post-filter the report; (b) write temporary filtered rule-set files; (c) one invocation per file | (a) pays for off rules; (b) breaks source locations and provenance; (c) loses request batching. The engine refuses the same rule set twice, so splitting files into groups by policy is impossible without this field. | An additive field in the `exec` protocol (protocol users ignore it) | Policies turn out to need per-rule-set rather than per-target granularity |
| Change scope | The engine's existing change selection, given every configured target | Lint prefilters targets to changed files | The engine already selects unchanged targets whose rule, rule set or context changed; prefiltering would drop them. | Each run enumerates all configured targets (cheap, local) | Target enumeration becomes slow on very large repositories |
| Config error locations | `serde_json` plus `serde_path_to_error`, with duplicate keys refused, as the engine does for rule files; semantic errors locate their key in the source text | A spanned JSON parser | Same behaviour and dependencies as the engine; no new parser | Locating keys by text search is approximate when a key appears twice (refused anyway) | Configs gain comments or JSONC |
| Release | cargo-dist: one workflow builds four targets once and publishes archives, a shell installer, a Homebrew tap formula and its own npm package | Hand-written workflows; a custom npm package per platform | Least maintenance (research report); the experiment showed it generates every channel | npm installs need GitHub at install time (documented) | A repository needs offline or script-free npm installs |
| Severity words | Hard cutover: `Severity::{Error, Warn}`, `Activation::{Error, Warn, WarnAboveBand}`; `block`/`nudge` refused | Accept both words | The owner chose one vocabulary, and nothing is published | Every fixture and schema changes once | — |

## Entity binding

| E | Semantic owner | Home | Shape at boundaries | Kind | Convention |
| --- | --- | --- | --- | --- | --- |
| E1 Lint config | `sunstack-lint` config module | new: `sunstack-lint` | File: JSON per Specification C4, snake_case. In Rust: `LintConfig { rule_sets: BTreeMap<EntryName, RuleSetEntry>, rules: BTreeMap<RuleSelector, SeveritySetting>, overrides: Vec<OverrideBlock>, ignore_patterns: Vec<GlobPattern>, backend: Option<BackendConfig>, limits: Option<LintLimits> }`, `#[serde(deny_unknown_fields)]`, `JsonSchema` | persisted (file) | engine: serde + schemars + `deny_unknown_fields` |
| E2 Rule-set entry | `sunstack-lint` config module | new | `RuleSetEntry { path: PathBuf, files: Vec<GlobPattern> }`; `EntryName` is a validated newtype (no `/`) | persisted | engine newtype IDs |
| E3 Rule | engine (`sunstack-rules` rule identity) | existing: `RuleId` | Config selector `RuleSelector::{Exact { entry, rule: RuleId }, AllInEntry { entry }}`, written `<entry>/<rule>` or `<entry>/*`; native `RuleId` unchanged | derived | engine `RuleId` |
| E4 Severity | engine (`sunstack-rules` definition contracts) | modified: `Severity` | `Severity::{Error, Warn}` (serde `"error"`, `"warn"`, default `Warn`); config `SeveritySetting::{Error, Warn, Off}` | persisted (rule files, config) | engine closed enums |
| E5 Override | `sunstack-lint` config module | new | `OverrideBlock { files: Vec<GlobPattern>, rules: BTreeMap<RuleSelector, SeveritySetting> }` | persisted | — |
| E6 Lint run | `sunstack-lint` run module | new | In: `LintRequest { config_path: Option<PathBuf>, paths: Vec<PathBuf>, scope: ScopeChoice, format: OutputFormat, max_warnings: Option<u32> }`, `ScopeChoice::{Default, All, Since { rev }}`. Out: `LintReport` (E7 list, exclusions, incomplete parts, counts, usage) and `LintStatus::{Clean, NothingSelected, Findings, Incomplete, Invalid, Cancelled}` | derived | closed variants |
| E7 Finding | `sunstack-lint` outcome module | new | `Finding { rule: RuleSelector, file: PathBuf, span: Option<SourceSpan>, severity: Severity, message: String, judgment: Option<JudgmentSummary> }` | derived | engine `SourceSpan` |
| E8 Backend selection | engine (`sunstack-decisions`) | existing: `BackendSelection` | Config `BackendConfig::{JevOpenrouter { model: ModelName, endpoint: Option<String> }}` maps to `BackendSelection::JevOpenrouter`; default endpoint `https://openrouter.ai/api/v1/systemone`; key read from `OPENROUTER_API_KEY` by the existing factory | persisted (config) | engine `BackendSelection` |
| E9 Release | release workflow | new: `dist-workspace.toml`, `.github/workflows/release.yml` | Git tag `v<semver>`; version from the workspace `Cargo.toml` | persisted (tags, release) | cargo-dist |
| E10 Platform build | release workflow | new | `sunstack-<target>.tar.gz` plus `.sha256`, targets `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` | persisted | cargo-dist naming |
| E11 Install channel | release workflow, tap repository, npm registry | new | Homebrew formula `sunstack.rb` in `ShravanSunder/homebrew-tap`; installer `sunstack-installer.sh`; npm package `sunstack` | persisted | cargo-dist generated |

## Components and interfaces

| Component | Job (one reason to change) | Consumers | Owns |
| --- | --- | --- | --- |
| `sunstack` binary | Parse the command line, dispatch, write output, map status to exit code, handle signals | people, CI, npm launcher | process boundary; exit codes (C2); `--version` |
| `exec` module (in the binary) | Today's protocol, moved without change apart from severity words | programs | protocol framing (unchanged) |
| `sunstack-lint::config` | Find, decode and validate `sunstack.lint.json`; export its JSON schema | lint run, `--check`, `--rules`, `--init` | E1, E2, E5 validity and locations |
| `sunstack-lint::targets` | Enumerate each entry's target files: config globs, minus `ignore_patterns` and Git-ignored files, limited to command-line paths | lint run | which file binds to which entry |
| `sunstack-lint::policy` | Compute each rule's effective setting per target (R10) | invocation, `--rules <file>` | effective severity and `off` |
| `sunstack-lint::invocation` | Build the engine's `RuleInvocationInput`: one `BoundRuleSet` per entry, one binding group per target with its policy, the repository binding with change scope, limits and backend | lint run | scope translation; config-changed rule; default limits |
| `sunstack-lint::outcome` | Turn the engine outcome into findings, exclusions, incomplete parts and `LintStatus` | formats, Rust callers | lint status (C2); "No checks selected" |
| `sunstack-lint::formats` | Render a `LintReport` as default, json, github or sarif | binary | output shapes (C3) |
| `sunstack-rules` (modified) | Apply per-group policy: skip `off` rules when expanding work, use the effective severity when activating a verdict; rename severity words | lint, exec | policy application; `Activation` |
| release workflow | Build four targets once, attach archives and checksums, publish installer, formula and npm package | maintainers | E9–E11 |

Public library entry points of `sunstack-lint`:

```rust
pub fn discover_config(start: &Path, explicit: Option<&Path>) -> Result<ConfigLocation, LintInvalid>;
pub fn load_config(location: &ConfigLocation) -> Result<LintConfig, LintInvalid>;
pub async fn check(config: &LintConfig, cancellation: CancellationToken) -> CheckResult;
pub fn list_rules(config: &LintConfig, checked: &CheckResult, file: Option<&Path>) -> Vec<RuleListing>;
pub async fn run(request: LintRequest, cancellation: CancellationToken) -> LintReport;
pub fn render(report: &LintReport, format: OutputFormat) -> String;
pub fn config_schema() -> schemars::Schema;
```

`LintInvalid` carries `{ file: PathBuf, line: Option<u32>, column: Option<u32>, json_path: Vec<String>, code: LintInvalidCode, message: String }`; `LintInvalidCode` is a closed enum (`NoConfigFound`, `Syntax`, `UnknownField`, `DuplicateKey`, `DuplicateEntry`, `SameRuleSetTwice`, `UnknownSelector`, `BadGlob`, `MissingRuleSet`, `UnsupportedInputs`, `OldSeverityWord`, `PathNotFound`, `NoGitForDefaultScope`, `BackendMissing`, `CredentialMissing`).

The engine change, as an additive field on `BindingGroup`:

```rust
pub struct BindingGroup {
    pub bindings: BTreeMap<SourceName, InputBinding>,
    #[serde(default, skip_serializing_if = "GroupPolicy::is_empty")]
    pub policy: GroupPolicy,
}
pub struct GroupPolicy {
    pub off: BTreeSet<RuleId>,                 // never expanded, never requested
    pub severity: BTreeMap<RuleId, Severity>,  // replaces the authored default at activation
}
```

An unknown `RuleId` in a policy is refused by invocation admission with the rule-set path and group position. A rule `off` in a group produces one rule-evaluation result `disabled_by_policy` for that group, so `exec` callers see it.

## The lint call path

There is no current predecessor for `lint`. The proposed path:

```mermaid
sequenceDiagram
  participant U as caller
  participant B as sunstack binary
  participant C as lint config
  participant T as targets and policy
  participant I as invocation
  participant R as sunstack-rules
  participant D as sunstack-decisions
  participant O as outcome and formats
  U->>B: sunstack lint [paths] [scope flags]
  B->>C: discover_config, load_config
  C-->>B: LintConfig or LintInvalid (exit 2)
  B->>R: check_rules on every entry (no targets, no key)
  R-->>B: assembled rules or refusal (exit 2)
  B->>T: enumerate targets; effective policy per target
  T-->>I: entries x targets with GroupPolicy (targets with every rule off are dropped)
  I->>I: scope: Default (uncommitted) / All / Since(rev); config changed → All, recorded
  I->>R: evaluate_rules(RuleInvocationInput)
  R->>D: only reachable judged rules and model gates (key read lazily)
  D-->>R: answers or provider failure
  R-->>O: RuleInvocationOutcome
  O-->>B: LintReport and LintStatus
  B-->>U: rendered report, exit code
```

`exec` keeps its current path: `main` → `protocol_boundary::read_frame` → `decode_request` → `operation_execution::execute_operation` → library → response. The only edge that changes is that `main` now dispatches on the command line first.

## Run outcome

| Engine outcome or fact | Lint result | Exit |
| --- | --- | --- |
| `LintInvalid` before the engine runs; engine `Invalid` | `Invalid` with located diagnostics | 2 |
| `Cancelled` | `Cancelled`, keeping completed findings | 4 |
| `PreparationFailed`; `IncompleteReport`; `rule_coverage` not `full`; any occurrence `selection_failed`, `evaluation_failed` or `not_started`; a request-limit or provider failure | `Incomplete`, keeping completed findings | 3 |
| Any `Activation::Error`, or warn findings above `--max-warnings` | `Findings` | 1 |
| No targets selected for any entry | `NothingSelected`, printed as "No checks selected" | 0 |
| Otherwise | `Clean` | 0 |

`disabled_by_policy`, `not_applicable`, `outside_change_scope` and `zero_matches` are counted as exclusions, never as incomplete.

Default limits when `limits` is omitted (tunable with evidence; listed in the README and recorded in every `json` report): files 2000; bytes per file 1 MiB; total bytes 64 MiB; matches 10,000; transcript bytes 8 MiB; Git-ignored files excluded; model requests per run 200; input tokens per request 32,000; request bytes 256 KiB; questions per request 32; attempt timeout 60 s; 2 retries with 0.5–8 s backoff; response bytes 1 MiB; 4 concurrent requests; speculation off.

## Failure, cancellation and concurrency

- Ctrl-C and SIGTERM reach the existing `ProcessSignals`, which cancels the shared `CancellationToken`; the engine settles owned work and lint reports `Cancelled` (exit 4). Through npm, the cargo-dist launcher forwards signals to the child.
- A model or network failure is the engine's provider failure; the engine's retry settings apply once; lint never retries a run.
- Lint runs one invocation per command, so there is no concurrency beyond the engine's own request scheduler (bounded by `max_concurrent_requests`).
- Lint reads the repository and writes only to standard output and standard error. `--init` is the only command that writes a file, and it refuses to overwrite.

## Trust boundary

| Asset | Entry | Policy owner | Enforcement |
| --- | --- | --- | --- |
| OpenRouter key | environment variable | `sunstack-decisions` factory | read lazily, only when model work is reachable; never in config, report, diagnostics or logs (existing engine rule U2) |
| Repository text | files under the repository root | `sunstack-evidence` | paths outside the root are refused (existing); text leaves the machine only inside a judged request to the configured endpoint |
| Config and rule files | repository | `sunstack-lint::config`, `sunstack-rules` admission | strict decoding; unknown fields refused |

## portable-builds and install

```mermaid
flowchart LR
  tag["git tag v0.1.0"] --> wf["release.yml (cargo-dist)"]
  wf --> b1["macos-14: aarch64-apple-darwin"]
  wf --> b2["macos-14: x86_64-apple-darwin (cross-compiled)"]
  wf --> b3["ubuntu: x86_64-unknown-linux-musl"]
  wf --> b4["ubuntu-arm: aarch64-unknown-linux-musl"]
  b1 & b2 & b3 & b4 --> rel["GitHub Release: .tar.gz + .sha256, installer.sh, schema"]
  rel --> tap["ShravanSunder/homebrew-tap: sunstack.rb with checksums"]
  rel --> npm["npm: sunstack (fetches the archive at install)"]
```

The three crates that use `git2` turn on `vendored-libgit2` and keep default features off. cargo-dist is configured with `unix-archive = ".tar.gz"` so minimal systems need no `xz`. A release check in the workflow inspects each binary's dynamic dependencies (`otool -L`, ELF headers) and fails on anything but operating-system libraries. The config schema is generated from `LintConfig` by `config_schema()` and attached to the release.

## Proof seams

| Seam | Real | Stand-in | Observes |
| --- | --- | --- | --- |
| Process tests of `sunstack` with fixture repositories | binary, engine, files, Git | an unreachable backend endpoint for runs that must not call a model | exit codes, all four formats, scope selection, severity policy, cancellation |
| Engine tests of `GroupPolicy` | `sunstack-rules` | a recording backend (existing test pattern) | an `off` rule makes no request; severity override and band activation |
| Hosted run | Jev on OpenRouter with the owner's key | none | one real judged rule from the static Linux build with no CA bundle |
| Container matrix | release builds in clean Alpine and Debian (x64, arm64) | none | no shared libraries; `--version`, `lint --check`, a lint run |
| Release rehearsal | a private test repository running the generated workflow | a local mirror serving those exact release bytes to the installers | archives, checksums, formula, npm package; installs and exits through each channel |

## Trace

### runner

| U | R | E | Owner | Interface | Shape and home | State | Failure | Proof |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| U1 | R1 lint a repository from its config | E1, E2, E6, E7 | `sunstack-lint::run` | `run(LintRequest)` | `LintReport` in `sunstack-lint` | none persisted | `LintInvalid` → exit 2 | process tests (V1) |
| U1 | R2 exit codes (C2) | E6 | binary + `sunstack-lint::outcome` | `LintStatus` → `ExitCode` | closed `LintStatus` | none | each row of Run outcome | process tests (V2) |
| U1, U2 | R3 formats | E7 | `sunstack-lint::formats` | `render(report, format)` | default text, json (`LintReport`), GitHub annotations, SARIF 2.1.0 | none | — | format comparison (V1) |
| U1, U7 | R4 change scope | E6 | `sunstack-lint::invocation` | `ScopeChoice` → engine `ChangeScope` | `RepositoryBinding.change_scope` | none | outside Git with default scope → `NoGitForDefaultScope` | scripted Git history (V3) |
| U1, U5 | R5 `--check`, `--rules`, `--init` | E1, E3 | `sunstack-lint::config`, `::policy` | `check`, `list_rules`, init writer | `CheckResult`, `RuleListing` | `--init` writes one file | overwrite refused | process tests (V4) |
| U3 | R6 `exec` unchanged | — none: protocol | binary `exec` module | stdin/stdout JSON | engine wire contracts | none | existing protocol errors | existing protocol tests (V5) |
| U2 | R7 Node and Rust | E6 | npm launcher; `sunstack-lint` library | `npx sunstack`; `run()` | same `LintReport` | none | — | npm and library comparison (V6) |

### config

| U | R | E | Owner | Interface | Shape and home | State | Failure | Proof |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| U6 | R8 discovery | E1 | `sunstack-lint::config` | `discover_config` | `ConfigLocation` | none | `NoConfigFound` | process tests (V7) |
| U5, U6 | R9 validation | E1, E2, E5 | `sunstack-lint::config` | `load_config` | `LintConfig`, `LintInvalid` | none | each `LintInvalidCode` | config cases (V7) |
| U4, U5 | R10 effective severity | E3, E4, E5 | `sunstack-lint::policy` → `sunstack-rules` activation | `GroupPolicy` | `GroupPolicy` in `sunstack-rules::invocation_contracts` | none | unknown rule → admission refusal | severity table and engine policy tests (V8) |
| U5 | R11 document-input rule sets only | E2 | `sunstack-lint::config` (after `check_rules`) | `check` | `UnsupportedInputs` | none | refused at check | process test (V9) |
| U1 | R12 targets and paths | E2, E6 | `sunstack-lint::targets` | target enumeration | `Vec<PathBuf>` per entry | none | `PathNotFound` | ignore and context cases (V9) |
| U7 | R13 backend only when reached | E8 | `sunstack-decisions` factory (lazy) with lint's pre-check | `BackendConfig` → `BackendSelection` | engine `BackendSelection` | none | `BackendMissing`, `CredentialMissing` → exit 2 | key and no-key runs (V10) |
| U4 | R14 one vocabulary | E4 | `sunstack-rules` | serde of `Severity`, `Activation` | `"error"`, `"warn"`, `"warn_above_band"` | rule files | `OldSeverityWord` | fixture sweep (V11) |
| U5 | R15 published schema | E1 | `sunstack-lint::config` + release workflow | `config_schema()` | `sunstack.lint.schema.json` release asset | release asset | — | schema vs binary cases (V12) |

### portable-builds

| U | R | E | Owner | Interface | Shape and home | State | Failure | Proof |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| U8 | R16 four standalone builds | E10 | release workflow | cargo-dist targets | `sunstack-<target>.tar.gz` | release assets | dependency check fails the release | container matrix and linkage check (V13) |
| U8 | R17 HTTPS without a CA bundle | E8 | `sunstack-decisions` (rustls with bundled roots) | `reqwest` client | existing | none | private-CA limit documented | hosted run from static build (V10) |
| U9 | R18 git2 with libgit2 compiled in | — none: build | Cargo features | `vendored-libgit2` | three `Cargo.toml` files | none | — | linkage check (V13) |

### install

| U | R | E | Owner | Interface | Shape and home | State | Failure | Proof |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| U10, U11 | R19 archives and checksums | E9, E10 | release workflow | GitHub Release | `.tar.gz`, `.sha256` | release | partial publish is resumed, never rewritten | release rehearsal (V14) |
| U10 | R20 Homebrew | E11 | release workflow + tap repository | `brew install shravansunder/tap/sunstack` | `sunstack.rb` | tap commit | — | release rehearsal (V14) |
| U10, U11 | R21 shell installer | E11 | release workflow | `sunstack-installer.sh` | release asset | — | — | release rehearsal (V14) |
| U2, U10 | R22 npm | E11 | cargo-dist npm package | `npx sunstack` | npm `sunstack` | installed package | install needs GitHub | release rehearsal (V14) |
| U10, U11 | R23 one version | E9 | workspace version | `--version` | `Cargo.toml` `[workspace.package] version` | tag | mismatch fails the release | release rehearsal (V14) |
