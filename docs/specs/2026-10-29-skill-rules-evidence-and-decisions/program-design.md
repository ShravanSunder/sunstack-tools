# Skill rules on evidence and decisions: program design

How the engine realizes [Specification](specification.md) R1–R28 for [Requirements](requirements.md) U1–U12. The current-system map behind it is the 29 Oct engine map (crate paths below are at `origin/main` 010da0f).

## What changes, in one picture

`sunstack lint --config` gets a new pipeline: load the config, load the evidence project with the standard util library, assemble and admit the rulebook, build evidence pieces, pick them, walk each rule's decision, ask the decision model only when a walk needs answers, and report. It sits beside the existing `exec` pipeline for JSONL RuleSets and conversation rules, which stays as it is. It reuses what already works: the Markdown parse and marker binding, the frontmatter YAML reader, git change capture, the decision model contract and its HTTP backend, credential resolution, and lint's scopes, formats and exit codes.

```mermaid
flowchart TB
  subgraph cli["sunstack (CLI)"]
    cmd["command_line + lint_command<br/>flags, credential at the edge"]
  end
  subgraph lint["sunstack-lint (command policy)"]
    cfg["rules_config_*<br/>decode, validate, resolve paths"]
    run["lint_run<br/>scope → pipeline → outcome"]
    rep["report_contracts + outcome + formats<br/>status, exit code, uncertain"]
  end
  subgraph rules["sunstack-rules (rules and decisions)"]
    asm["rulebook_assembly<br/>markers ↔ entries"]
    adm["rulebook_admission<br/>the R24 checks"]
    walk["decision_walking<br/>steps → result or needs answers"]
    exe["rulebook_run<br/>request building, bounded dispatch"]
  end
  subgraph evidence["sunstack-evidence (Markdown evidence)"]
    proj["evidence_project_loading<br/>sgconfig + standard utils"]
    std["standard_markdown_utils<br/>embedded YAML"]
    build["evidence_piece_building<br/>matches → pieces → fields"]
    cut["evidence_piece_cutting"]
    pick["evidence_piece_picking<br/>changed spans → picked pieces"]
    md["markdown_document (existing)<br/>parse, bind_markers"]
    fm["markdown_frontmatter (existing)"]
    chg["change_selection + repository_capture (existing)"]
  end
  subgraph decisions["sunstack-decisions (existing, unchanged)"]
    jv["judgment_validation"]
    je["judgment_execution + decision_backends"]
  end
  cmd --> cfg --> run
  run --> proj
  std --> proj
  run --> asm --> adm
  asm --> md
  adm --> jv
  run --> chg
  run --> build --> cut
  build --> fm
  build --> pick
  run --> exe --> walk
  exe --> je
  run --> rep
```

## Structure decision

**The crux:** the confirmed design's rule (one evidence kind, a decision graph, named structured questions, results including uncertain) doesn't fit the current `RuleDefinition` (evidence declarations bound by binding groups, one verdict, per-occurrence applicability) or its request planner (one carrier per RuleSet group, budget splitting across occurrences).

| Option | What it means | Gain | Cost | Verdict |
| --- | --- | --- | --- | --- |
| A. Extend `RuleDefinition` | add an evidence-kind selector and a `DecisionGraph` verdict variant; keep binding groups and the request graph | one rule type | binding groups stay (the stand-in the runner needs today); the request graph must learn per-piece lazy asks, uncertain terminals, routes and cut evidence; every existing admission path grows a second meaning | rejected: it keeps the workaround the owner asked to remove and doubles the meaning of a shared type |
| B. A rulebook pipeline beside it, in the existing crates (selected) | new modules for evidence pieces, rule entries, decision walking and a bounded request runner; reuse parse, markers, frontmatter, change capture, decisions | each piece has one job; nothing needs binding groups; the decision model contract is reused unchanged | two rule pipelines exist side by side: `exec` JSONL rules (conversations, Router) and lint's rulebook rules; a small dispatcher that overlaps the request scheduler's limits | selected |
| C. A new crate | same as B in its own crate | clear separation | a new crate boundary that splits Markdown evidence from its parser and rules from their checker for no consumer that needs it | rejected: the existing crates already own these concerns |

Falsifiers that would reopen it: a second consumer needs rulebook rules through `exec` (then the run entry moves behind the protocol), or conversation rules need evidence pieces (then the two pipelines should merge).

The Markdown-marker route through RuleSets (`rule_sources`) is deleted, not kept beside the new one: it is the same job done the old way (R3).

## Entity binding

| E | Semantic owner | Home | Type home and shape | Kind | Convention |
| --- | --- | --- | --- | --- | --- |
| E1 Rules config | sunstack-lint | modified: `config_contracts`, `config_decoding`, `config_validation`, `config` (discovery unchanged) | `RulesConfig { schema?, rulebook: Vec<RulebookPair { markdown, rules }>, marker_prefix: MarkerPrefix, max_question_words: NonZeroU32, check: CheckedFiles { files: Vec<GlobPattern>, ignore }, evidence: EvidenceSettings { ast_grep: PathBuf, kinds: BTreeMap<EvidenceKindName, EvidenceKindDefinition> }, decision_model: DecisionModelSettings { model: ModelName, endpoint: Option<Url> }, limits: Option<LintLimits> }`, `deny_unknown_fields`; replaces `LintConfig` | derived from the file | strict Serde, field-path errors (`config_decoding`) |
| E2 Rulebook sentence | sunstack-rules (assembly), via sunstack-evidence's marker binding | existing: `MarkdownDocument::bind_markers`, `MarkdownMarker` | `RulebookSentence { marker: MarkerId, rulebook: PathBuf, line: u32, standard: String }` | derived | existing marker contracts |
| E3 Rule entry | sunstack-rules | new: `rulebook_entry_wire`, `rulebook_assembly`, `rulebook_admission`; replaces `rule_source_entry_wire` | wire `RuleEntryWire { name, evidence, questions: NamedQuestionMap, decision: DecisionGraphWire, severity: Option<Severity> }`; admitted `RulebookRule { name: RuleId, sentence: RulebookSentence, kind: EvidenceKindName, questions: NamedQuestionMap, decision: DecisionGraph, severity: Severity }` | derived | `RuleId`, `Severity` (existing, `definition_contracts`); `NamedQuestionMap` (existing, sunstack-decisions) |
| E4 Evidence project | sunstack-evidence | new: `evidence_project_loading` | `EvidenceProject { globals: GlobalRules, rules: RuleCollection<SupportLang>, rule_ids: BTreeSet<EvidenceRuleId> }` from ast-grep-config 0.51 | derived | vendor types, mirrored from ast-grep's CLI loader |
| E5 Evidence kind | sunstack-evidence | new: `evidence_kind_contracts` | `EvidenceKindDefinition { rule: EvidenceRuleId, fields: BTreeMap<FieldName, FieldSource>, max_chars: Option<NonZeroU32> }`; `FieldSource` closed enum: `FirstLine`, `Capture { regex }`, `Inside { rule }`, `Enclosing { rule }`, `SameFile { rule }`, `Yaml { key }`, `Remainder`, `Target { regex, resolve: TargetResolution, opening_chars, as_kind: Option<EvidenceKindName> }`; `TargetResolution`: `Path`, `Skill`, `PathOrSkill` | derived | Serde tag `from`, snake_case |
| E6 Evidence piece | sunstack-evidence | new: `evidence_piece_building`, `evidence_piece_cutting` | `EvidencePiece { kind, where: PieceLocation { file, first_line, last_line, span: SourceSpan }, text, fields: BTreeMap<FieldName, FieldValue>, cut: Vec<FieldName> }`; plus `FileMatchIndex { by_rule: BTreeMap<EvidenceRuleId, Vec<SourceSpan>> }` per file | derived per run | `SourceSpan` (existing, `evidence_contracts`) |
| E7 Field | sunstack-evidence | same as E6 | `FieldValue` closed enum: `Text { value, spans, uncut_chars }`, `Texts { values, spans }`, `Targets(Vec<LoadTarget { name, path, exists, opening: Option<String>, fields: Option<BTreeMap<FieldName, FieldValue>> }>)` | derived | — |
| E8 Question | sunstack-decisions | existing: `JudgmentQuestion`, `NamedQuestionMap` | unchanged | authored | existing |
| E9 Decision step | sunstack-rules | new: `decision_graph_contracts` | `DecisionGraph { start: StepName, steps: BTreeMap<StepName, DecisionStep> }`; `DecisionStep` closed enum: `MechanicalCheck { tool: MechanicalTool, yes: Route, no: Route }`, `AskNoul { question, yes_at_or_above, no_at_or_below, yes, no }`, `AskChoice { question, min_probability, routes: BTreeMap<ChoiceLabel, Route> }`, `AskScore { … like noul }`, `Combine { weights: BTreeMap<QuestionName, Weight>, yes_at_or_above, no_at_or_below, yes, no }`; `MechanicalTool`: `AstGrep { rule }`, `Regex { field, pattern }`, `Contains { field, text }`, `MaxChars { field, max }`, `FileExists { field }`; `Route`: `Step(StepName)`, `Pass`, `NotApplicable`, `Uncertain`, `Finding { message, severity: Option<Severity>, at: Option<FieldName> }` | authored | the wire's step keys (`mechanical_check`, `ask`, `combine`) select the variant |
| E10 Result | sunstack-rules (produced), sunstack-lint (reported) | new: `decision_walking`, `rulebook_run`; modified: lint `report_contracts` | `RuleResult` closed enum: `Pass`, `Finding { severity, message, line, path_taken, answers?, value? }`, `NotApplicable`, `Uncertain { step, state, answers }`, `Incomplete { reason: IncompleteReason }`; lint adds `LintStatus::Uncertain` (exit 5), `IncompleteReason::RequestTooLarge` | derived per run | lint's existing enums |
| E11 Run | sunstack-lint | modified: `lint_run`, `outcome`, `targets`, `invocation` (replaced by the pipeline) | `LintRequest` unchanged in shape (config path, paths, scope, max_warnings) plus `evidence_listing: Option<Option<EvidenceKindName>>` | per invocation | existing |
| E12 Standard util library | sunstack-evidence | new: `standard_markdown_utils` and data `crates/sunstack-evidence/ast-grep-utils/markdown/*.yml` | embedded `&'static str` YAML per util, parsed with `ast_grep_config::from_str` into `SerializableGlobalRule` | shipped in the binary | `include_str!` |

## Components, owners and edges

| Component | One job | Reason to change | Consumers |
| --- | --- | --- | --- |
| `standard_markdown_utils` | hold the shipped utils and their reserved IDs | a new common Markdown shape | `evidence_project_loading` |
| `evidence_project_loading` | turn an `sgconfig.yml` plus the standard utils into globals and a rule collection, Markdown only, reserved IDs enforced | ast-grep's project format | lint check and run |
| `evidence_piece_building` | parse each checked file once, run every project rule that applies to its path once, build pieces and fields per kind, resolve targets inside the repository root | the field-source vocabulary | lint run, `--evidence` |
| `evidence_piece_cutting` | apply R12 | the size rule | piece building |
| `evidence_piece_picking` | select pieces under a scope from `CapturedChange` spans and target changes | the picking rule | lint run |
| `rulebook_assembly` | pair markers and entries both ways and produce admitted-or-skipped rules | the rule file wire | check and run |
| `rulebook_admission` | every R24 refusal, against the evidence project and kinds | the decision vocabulary | check and run |
| `decision_walking` | walk one rule on one piece: settle, or stop at the first ask needing answers, then finish with answers | decision semantics | `rulebook_run` |
| `rulebook_run` | for each picked pair, walk, build the request, enforce `request_max_chars`, dispatch with bounded concurrency and request count, collect results in order | dispatch policy | lint run |
| lint `rules_config` modules | decode, validate and resolve the config, refusing old keys by name | the config contract | CLI |
| lint `lint_run` | order the pipeline and map scope, skips and results into the outcome | the command's flow | CLI |
| lint report modules | statuses, exit codes, uncertain, findings, evidence listing, formats | the report contract | CLI, the #171 runner |

Allowed edges: `sunstack` → `sunstack-lint` → (`sunstack-rules`, `sunstack-evidence`, `sunstack-decisions`); `sunstack-rules` → (`sunstack-evidence`, `sunstack-decisions`). Forbidden: `sunstack-evidence` → `sunstack-rules` (evidence never knows about decisions); `decision_walking` → any I/O (it is pure, given a piece, its file's match index and answers); `rulebook_run` → the filesystem (it receives pieces). Violations show up as a Cargo dependency cycle or an import in review.

## The representative path: `sunstack lint --config <cfg> --since origin/main`

```mermaid
sequenceDiagram
  participant CLI as sunstack lint
  participant Cfg as rules_config
  participant Proj as evidence_project_loading
  participant Asm as rulebook_assembly + admission
  participant Chg as change_selection (existing)
  participant Ev as evidence_piece_building + picking
  participant Run as rulebook_run + decision_walking
  participant Jev as judgment_execution (existing)
  participant Rep as outcome + report
  CLI->>Cfg: read and validate config (refuse old keys)
  Cfg-->>CLI: RulesConfig | invalid (exit 2)
  CLI->>Proj: load sgconfig + standard utils
  Proj-->>CLI: EvidenceProject | invalid
  CLI->>Asm: markers ↔ entries, admit against project and kinds
  Asm-->>CLI: rules + skipped rules (R28)
  CLI->>CLI: any rule asks and no credential? → invalid before reading files (R4)
  CLI->>Chg: capture RevisionDiff(origin/main → worktree)
  Chg-->>CLI: changed files with after_spans; config inputs changed? → widen to full
  CLI->>Ev: parse each selected file once; run every applicable project rule once
  Ev-->>CLI: pieces per kind (cut per R12) + match index per file; picked by span overlap or target change
  loop each picked (rule, piece), in rule order then file and byte order
    CLI->>Run: walk decision
    Run-->>Run: mechanical steps settle locally
    alt walk reaches an ask or combine
      Run->>Run: build request (all the rule's questions, one piece's state), check request_max_chars
      Run->>Jev: run_judgments (bounded by max_concurrent_requests, max_model_requests, cancellation)
      Jev-->>Run: answers | failure → Incomplete
      Run-->>Run: finish walk with answers
    end
    Run-->>CLI: RuleResult
  end
  CLI->>Rep: results, skips, scope → status and exit code (R26)
  Rep-->>CLI: report (default | json | github | sarif)
```

`--check` runs the first three steps only and lists rules (R24). `--evidence [kind]` runs through picking and prints pieces, with no credential and no request (R13).

The current path for comparison: lint builds a native RuleSet invocation with a stand-in binding, calls `sunstack_rules` `evaluate`, which plans requests per RuleSet group and occurrence (`request_graph.rs:157`), and projects verdicts into the lint report (`lint_run.rs:142`). Added edges: config → evidence project; assembly → admission against evidence kinds; pieces → picking by span; run → `run_judgments` per pair. Removed edges: lint → native invocation with bindings; marker sources → `RuleDefinition` lowering. Preserved: credential resolution at the CLI edge; `run_judgments` with its attempt controller, retries and receipts; lint's scope mapping, widening on config change and formats.

## State

Nothing persists. Per run: the config, project, rules and pieces are immutable once built; the dispatcher owns the only mutable state (in-flight requests, request count). A walk's state is the current step and the answers once fetched.

| From | Event | To |
| --- | --- | --- |
| walk at a mechanical step | tool says yes/no | the routed step or terminal |
| walk at the first ask or combine | no answers yet | `NeedsAnswers` (dispatcher sends one request) |
| `NeedsAnswers` | answers arrive | walk continues from that step with those answers |
| `NeedsAnswers` | request too large, provider failure, malformed answer, cancel, request limit | `Incomplete { reason }` |
| any step | a terminal | `Pass`, `Finding`, `NotApplicable` or `Uncertain` |

Illegal states are unrepresentable or refused at the trusted entry: a route to a missing step, a cycle, an unrouted choice option, an ask naming an undefined question, weights not summing to 1, or no above yes are refused by `rulebook_admission`, so the walk never meets them. A piece without a field a step names can't happen because admission checks field references against the kind.

## Failure and recovery

| Failure | Detected by | Contained to | Result |
| --- | --- | --- | --- |
| bad config, unreadable rulebook or rule file, bad project | config decoding, assembly, project loading | the whole run | `invalid` (2), nothing read or asked (R28) |
| one bad marker or entry | assembly, admission | that rule | skipped with diagnostics; `incomplete: coverage_partial` (R28) |
| missing credential with asking rules | lint run, before reading files | the whole run | `invalid` naming the credential (R4) |
| a checked file over lint's file limits or unreadable | piece building | that file's pieces | `incomplete` with the existing selection reason |
| request over `request_max_chars` | `rulebook_run` | that pair | `Incomplete { request_too_large }` (R21) |
| provider error, timeout, malformed answer | `run_judgments` (existing attempt controller and validation) | that pair | `Incomplete { provider_failure }` (R21); retries are the existing controller's |
| request limit reached | dispatcher | remaining pairs | `Incomplete { request_limit_reached }` |
| cancellation (signal) | existing cancellation token | the run | `cancelled` (4); finished results kept |
| a target outside the repository root | piece building | that target | `exists: false`, never read (R11) |

## Concurrency

Only model requests overlap. The dispatcher uses one `JoinSet` with a semaphore of `max_concurrent_requests` and a counter for `max_model_requests`, all under the run's cancellation token. Each task owns its request and returns `(pair index, outcome)`. Results go into a vector by pair index, so order is deterministic (R27) regardless of completion order. Parsing and matching run in `spawn_blocking` per file, as document selection does today.

## Cutover

Hard cutover in one change:

- lint's config contract becomes `RulesConfig`; `rule_sets`, `rules`, `overrides`, `ignore_patterns` and `backend` are refused by name with their replacement (R3); `lint --init` writes the new skeleton and the exported schema is the new shape;
- RuleSets lose `rule_sources`, `marker_prefix` and `max_question_words`; `rule_source_assembly`'s lowering to `RuleDefinition`, `rule_source_entry_wire` and the marker printing in `check_rules` are deleted; marker scanning and binding in `markdown_document` stay;
- lint's native-invocation builder (`invocation.rs`) and its stand-in check binding (`config_validation.rs:173–256`) are deleted;
- the four release fixtures under `scripts/verify/sunstack-rule/release-fixtures/` move to the new config shape;
- sunstack PR #171's runner moves from `exec` with RuleSets to `sunstack lint --config` (consumer work, outside this change).

No migration of stored data: nothing is stored.

## Trust and privacy

The decision model receives the standard, the piece's location and its fields, which are repository text the consumer chose to check; that is the existing contract with OpenRouter. The credential path is unchanged and never enters state, reports or diagnostics. Evidence reads stay inside the repository root (R11). The evidence project is consumer-authored YAML parsed by ast-grep's own deserializer; regexes compile with the `regex` crate (linear time).

## Cross-cutting

- **Performance:** each checked file is read and parsed once, and every applicable project rule runs on it once; kinds, `inside`, `enclosing`, `same_file` and the `ast_grep` tool all read the per-file match index instead of re-matching.
- **Determinism (R27):** directory walks are sorted; pieces are ordered by kind name, file and byte start; results by rule order, file and byte start.
- **Observability:** the report's usage summary (requests, tokens, cost) is filled from the existing receipts.

## Proof seams

| Seam | Real | Faked | Observes |
| --- | --- | --- | --- |
| evidence project + pieces on fixture files | ast-grep-config, the standard utils, the builder | nothing | pieces, fields, spans, cut lists; equality with `ast-grep scan` 0.51.0 output |
| `decision_walking` | the walk | answers supplied directly | results, lines, paths taken, which steps asked |
| `rulebook_run` with a loopback decision backend | dispatch, request building, limits | the HTTP endpoint (existing loopback pattern in `sunstack-decisions/tests/http_adapter.rs`) | requests sent (questions byte-equal, one piece), incomplete reasons, order |
| `sunstack lint` binary on fixture repositories | everything | the decision endpoint via `decision_model.endpoint` | exit codes, JSON report, `--check`, `--evidence` |
| hosted run | everything, real OpenRouter | nothing | schema-valid answers, a complete report |

## Trace

| U | R | E | Owner | Interface | Shape and home | State | Failure | Proof |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| U4 | R1 config shape | E1 | lint rules_config | `load_rules_config(path) -> Result<RulesConfig, LintInvalid>` | `RulesConfig`, `config_contracts` | none | invalid | fixture configs through `--check` |
| U4 | R2 path bases | E1 | lint rules_config | path resolution in validation | `ConfigLocation` (existing) + repository root | none | invalid on missing paths | fixture with config in a subfolder |
| U4 | R3 cutover refusals | E1, E3 | lint rules_config; rules definition loading | decoder refusals | `LintInvalidCode` (existing) | none | invalid | old-shape configs and RuleSets |
| U7 | R4 credential, no switch | E11 | lint_run (gate), CLI (resolution) | `Option<ApiKey>` into the run | existing credential path | none | invalid before reads | binary without a key; source has no off switch |
| U4 | R5 markers both ways | E2, E3 | rulebook_assembly | `assemble_rulebook(config) -> RulebookAssembly { rules, skipped }` | `RulebookSentence`, `RulebookRule` | none | skip or invalid | crafted rulebooks |
| U1–U3 | R6 evidence section | E5 | lint rules_config → evidence | `EvidenceSettings` | `evidence_kind_contracts` | none | invalid | `--check` fixtures |
| U4–U6 | R7 rule file shape | E3 | rulebook_entry_wire | strict Serde | `RuleEntryWire` | none | skip | crafted entries |
| U1, U3 | R8 evidence project | E4 | evidence_project_loading | `load_evidence_project(sgconfig) -> Result<EvidenceProject, ProjectRefusal>` | `EvidenceProject` | none | invalid | equality with `ast-grep scan` |
| U3 | R9 standard library | E12 | standard_markdown_utils | `standard_utils() -> &[(UtilId, &str)]` | embedded YAML | none | reserved-ID refusal | each util on fixtures; reuse refusal |
| U2 | R10 pieces | E6 | evidence_piece_building | `build_pieces(project, kinds, files) -> EvidenceCatalog` | `EvidencePiece`, `FileMatchIndex` | none | per-file incomplete | step 3 fixture bytes |
| U2 | R11 field sources | E7 | evidence_piece_building | `FieldSource` evaluation | `FieldValue` | none | empty field; target `exists: false` | each source on fixtures; outside-root target |
| U2 | R12 size cut | E6 | evidence_piece_cutting | `cut_piece(piece, max_chars)` | `EvidencePiece.cut`, `uncut_chars` | whole → cut | none | oversize fixture |
| U2, U10 | R13 `--evidence` | E6, E11 | lint_run + report | flag → evidence listing | `EvidenceListing` in report modules | none | invalid as config | binary run, no key set |
| U10 | R14 one run per piece | E6, E10 | rulebook_run | pair enumeration | `(RuleId, piece index)` | none | none | counts on fixtures |
| U10 | R15 picking | E6, E11 | evidence_piece_picking | `pick_pieces(catalog, scope, changes) -> Picked` | `CapturedChange.after_spans` (existing) | none | full scope when the before side is unknown, as today | temp Git repo fixtures |
| U5 | R16 decision shape | E9 | decision_graph_contracts | Serde | `DecisionGraph`, `Route` | none | skip | crafted graphs |
| U5 | R17 mechanical tools | E9 | decision_walking | `walk_decision(rule, piece, index, answers) -> WalkOutcome` | `MechanicalTool` | step → step | none (refused at admission) | walk tests |
| U5, U6 | R18 ask and combine | E9 | decision_walking | same | `DecisionStep` | step → step / terminal | uncertain bands | walk tests with supplied answers |
| U6 | R19 question schema | E8 | rulebook_admission via `judgment_validation` | validation call | `JudgmentQuestion` (existing) | none | skip | crafted questions |
| U6, U7 | R20 one request, lazy | E8, E10 | rulebook_run | `build_piece_request(rule, piece) -> StructuredJudgmentRequest` | existing request type; state object `{standard, where, <fields>}` | `NeedsAnswers` → settled | incomplete | loopback request capture |
| U2 | R21 request limits, failures | E10 | rulebook_run | dispatcher | `IncompleteReason` | `NeedsAnswers` → incomplete | contained to pair | loopback oversize and error |
| U5 | R22 finding location | E10 | decision_walking | `Route::Finding.at` → field span line | `RuleResult::Finding` | terminal | none | worked examples |
| U8 | R23 uncertain | E10 | decision_walking; lint report | `RuleResult::Uncertain` | report `uncertain` list | terminal | none | worked examples |
| U9 | R24 check | E1–E5, E8, E9 | rulebook_admission + lint check | `check(config) -> CheckResult` | `CheckResult` (existing, new listing) | none | refusals listed | refusal matrix |
| U5, U8, U10 | R25 run and report | E10, E11 | lint_run + report modules | `run(request, key) -> LintReport` | `LintReport` + `uncertain` | none | per R26 | binary JSON report |
| U8 | R26 status and exit | E11 | lint outcome | `LintStatus::exit_code` | `LintStatus::Uncertain` = 5 | precedence table | none | binary exit codes |
| U2, U10 | R27 determinism | E6, E10 | piece building, dispatcher | ordering rules | ordered vectors | none | none | repeat runs, shuffled completion |
| U12 | R28 skips | E3, E11 | rulebook_assembly + lint outcome | `RulebookAssembly.skipped` | `SkippedRule` (existing shape) | none | incomplete: coverage_partial | crafted bad entry |

Deviations and unresolved decisions: none. `decision_model.endpoint` exists for tests and alternative gateways, as lint's `backend.endpoint` does today.
