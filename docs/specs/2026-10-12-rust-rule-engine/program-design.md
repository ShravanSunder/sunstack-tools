# Rust rule engine: one core, evidence to decisions

The [Specification](specification.md) fixes behavior; [Requirements](requirements.md) supplies its authority and boundaries. The target is one implementation composed from three cohesive Rust library crates and one thin executable crate. Native consumers choose evidence, decisions or rule orchestration directly; a skill tool or optional TS caller invokes the executable. There is no mandatory Bun runtime, JS addon, daemon or language server.

## The integrated model

![Evidence, decisions, rules and executable owners](assets/program-components.png)

[Diagram source](assets/program-components.mmd)

The owners above are modules grouped by responsibility into the crates below. Rule questions, applicability, thresholds, feedback and eval expectations remain Sunstack content; Tools interprets the bounded declarations. Crate boundaries are reusable domains, not one crate for each helper or provider.

## Composable crate boundaries

| Crate | Owns and reason to change | Public contract and actual consumer | Excludes |
| --- | --- | --- | --- |
| `sunstack-evidence` | Source capture, Git comparison, paragraph resolution, text/AST-grep queries, source spans and completeness; changes when source formats or selector behavior change | Validated snapshots, target/evidence requests, immutable bundles and evidence diagnostics; rules and direct repository tools consume it | Decision HTTP, credentials, model questions, thresholds, lint findings |
| `sunstack-decisions` | Native primitive schemas, request-relative answer validation, backend trait, Jev HTTP adapter, usage and attempt/cancellation semantics; changes when decision contracts or provider protocols change | Structured requests, typed decision outcomes, prepared backend and decision diagnostics; rules and direct decision callers consume it | Git, AST-grep, Markdown paragraph IDs, rule-pack loading, pass/fail policy |
| `sunstack-rules` | JSONL admission, applicability/dependency selection, policy validation/composition, per-rule scheduling and reports; changes when data-rule or finding semantics change | Admitted rule packs, preparation/evaluation operations, findings, selection records and reports; skill tooling/native rule callers consume it | Provider wire decoding, source parsing algorithms, signal handlers and process framing |
| `sunstack-rule-cli` | Invocation JSON framing, environment/configuration wiring, process signals, exit codes and serialization; changes when process UX/protocol changes | The program contract for agents and optional TS callers; delegates to the three libraries | Duplicate selection, response validation, retry or policy implementations |

![Four cohesive crates and their allowed dependency direction](assets/crate-dependencies.png)

[Diagram source](assets/crate-dependencies.mmd)

Arrows show compile-time dependency direction. Evidence and decisions are independent siblings; neither depends on rules or the executable. Rules depends on both, and the executable composes them. No library imports the executable, and no circular dependency or global service locator is permitted.

Git capture, Markdown targets, regex/text matching and AST-grep stay modules inside evidence because they share captured source bytes, spans, limits and query lifecycle. Canonical primitive types, validation and current provider adapter stay modules inside decisions because they implement one decision contract. Future providers start as adapter modules there; a separate provider crate needs an actual independent consumer/distribution reason. No generic `common`, `utils` or all-domain contract crate is introduced.

Each crate owns its domain types and diagnostics. Rules imports evidence's `SourceSpan`, `TargetId`, selectors, bundles and `EvidenceDiagnostic`, and decisions' question/answer/receipt types and `DecisionDiagnostic`; it does not recreate them or pass provider objects into evidence. Converting an evidence bundle into `SharedEvidenceState` is a rule-composition operation, not a model dependency in evidence. Rules' report diagnostic is a tagged union wrapping those typed diagnostics plus its own rule/invocation diagnostics. The executable adds a protocol-error variant at its outer envelope without moving domain validation into the CLI. No lower crate depends on an upper report schema to describe an error.

The cost is three public library surfaces and a workspace dependency graph. The gain is concrete: local evidence tools do not pull in HTTP/provider dependencies, and decision-only consumers do not pull in Git/AST parsers. V1 proves independent consumers; V15 inspects the actual dependency graph. No smaller one-crate-per-module split is needed for these outcomes.

## Foundation and tradeoffs

At repository baseline `c3220fc`, tracked content is LICENSE, .gitignore and earlier design/review documents. There is no current production or proof call path, crate, package install or runnable engine to preserve. Every local owner, call and return below is added. Existing provider HTTP endpoints are external constraints; existing Sunstack/router source stays unchanged.

| Structural choice | Gain | Cost, payer and revisit signal |
| --- | --- | --- |
| Selected: cohesive Rust evidence, decisions and rules crates plus program | One implementation with independent library use; Rust hosts avoid process overhead; other hosts use a portable request boundary | Tools maintains three public library surfaces, a native shipping matrix and protocol. Add bindings only when a measured caller need justifies them |
| Rust evidence with TS decision service | Easy JS SDK/Zod/Effect use and arbitrary TS policies | Two runtimes, shared semantics and service lifecycle; credible only if executing arbitrary TS rule code is an authorized requirement |
| Rust core plus native JS bindings immediately | Direct calls for existing JS hosts | Addon packaging, async/cancellation and runtime ABI proof before the first customer needs it; defer |
| Mandatory service or persistent index | Warm cross-process state and possible reuse | Adds lifetime, versioning and invalidation machinery without a measured requirement; defer |

The policy interpreter is a bounded data mechanism, not a workflow language. If the first actual rule needs arbitrary cross-answer composition inside JSONL, the policy contract must be revisited. Direct Rust consumers can already compose answers in code. No execution of JS or externally supplied Rust code is introduced.

## Entity bindings and data homes

Rust domain conventions come from the supplied AGENTS and full Rust rules: validated distinct IDs, Serde-tagged enums, `Result`/`Option`, `thiserror` domain failures, one caller-owned Tokio runtime and no raw IO/provider errors crossing the public boundary. Deserialization admits shape; constructors and request-relative validators enforce meaning. `serde_json::Value` is confined to the untrusted provider-envelope boundary; structured internal values use a recursive JSON domain enum.

| E | Semantic owner | Package/module and schema/type home | Boundary shape | Lifetime / convention |
| --- | --- | --- | --- | --- |
| E1 Named question | Judgment admission | `sunstack-decisions/decision_contracts` (new) | `NamedQuestionMap<QuestionName, JudgmentQuestion>`; noul/choice/score enum with `StructuredJson` instructions/descriptions | Derived immutable; enum and validated names |
| E2 Shared state | Caller supplies, judgment owner preserves | `sunstack-decisions/decision_contracts` (new) | `SharedEvidenceState`: text, JSON object/array or null | Invocation-owned snapshot; recursive JSON enum |
| E3 Backend | Adapter prepares, judgment owner selects | `sunstack-decisions/decision_backends` (new); public trait in `decision_contracts` | `DecisionBackend` capability set, async request operation; `BackendPreparation` ready/refused | Configured in-process; secret private |
| E4 Request | Judgment admission | `sunstack-decisions/decision_contracts` (new) | `StructuredJudgmentRequest { questions, state }`; connection and cancellation passed separately | Derived immutable; fallible constructors |
| E5 Decision run | Judgment owner | `sunstack-decisions/judgment_execution` (new); outcome in `decision_contracts` | `DecisionRunOutcome`: completed/invalid_request/provider_failure/invalid_response/cancelled | Invocation-local; tagged enum |
| E6 Answer | Response admission | `sunstack-decisions/judgment_validation` (new); answer shapes in `decision_contracts` | `JudgmentAnswer`: native noul, choice or score fields; map keyed by requested name | Admitted output; runtime correlation |
| E7 Receipt | Adapter reports, response admission validates | `sunstack-decisions/judgment_validation` (new) | `ExecutionReceipt { resolved_model, usage: { input_tokens, output_tokens, cost_usd? } }` | Derived; finite counts/cost; absence explicit |
| E8 Finding | Policy owner executes Sunstack meaning | `sunstack-rules/rule_policy` (new); shape in `rule_contracts` | `RuleFinding { rule_id, target_id, source_span, policy_result, judgment, feedback }` | Derived per rule/target; tagged policy enum |
| E9 Rule definition | Rule admission | `sunstack-rules/rule_contracts` (new) | `RuleDefinition { rule_id, target, applicability, context_selectors, question, policy, feedback }` | Supplied JSONL then validated immutable data |
| E10 Rule pack | Rule admission | `sunstack-rules/rule_pack_loading` (new); shape in `rule_contracts` | `ValidatedRulePack { content_identity, rules, definition_locations }` | Invocation-local; atomic admission |
| E11 Snapshot | Evidence owner | `sunstack-evidence/source_evidence` (new); shape in `evidence_contracts` | `SourceSnapshot { resolved_comparison, captured_sources, index_identity? }`; source bytes keyed by repository-relative path/content identity | Frozen within invocation; no persistent cache |
| E12 Target | Target resolver | `sunstack-evidence/markdown_targets` (new); shape in `evidence_contracts` | `TargetResolution`: resolved paragraph/span, missing, ambiguous or deleted | Derived from captured bytes; distinct `TargetId` |
| E13 Evidence | Evidence owner | `sunstack-evidence/source_evidence` (new); shape in `evidence_contracts` | `EvidenceBundle { snapshot_identity, target, context, facts, uncertainties, completeness }` | Derived; source-linked facts/closed completeness |
| E14 Diagnostic | Boundary that detects issue; rule report wraps without revalidating | Domain diagnostic modules in `sunstack-evidence`, `sunstack-decisions` and `sunstack-rules` (new); aggregate in rules' `report_contracts` | Typed `EvidenceDiagnostic`, `DecisionDiagnostic`, `RuleDiagnostic` and `InvocationDiagnostic`; rules' `ReportDiagnostic` wraps them with their origin. CLI framing diagnostics belong to the executable only | Derived; each payload has one type owner, closed codes and bounded safe fields |
| E15 Selection | Selection owner | `sunstack-rules/rule_selection` (new); shape in `rule_contracts` | `RuleSelection`: selected/prepared, not_applicable, outside_change_scope, selection_failed | Derived once per rule occurrence |
| E16 Lint invocation | Invocation owner | `sunstack-rules/rule_execution` (new); program envelope in `sunstack-rule-cli/protocol_boundary` | `LintInvocationOutcome`: invalid_invocation, completed_report, incomplete_report or cancelled_report | Ephemeral report; per-rule results preserved |

No owner writes a run database, modifies source or changes Git state. Rule preparation and decision execution can also be used separately; the orchestration module does not make every consumer adopt the skill-lint report.

## Library and program boundaries

| Operation | Inputs / preconditions | Output / effects |
| --- | --- | --- |
| Backend preparation | Explicit endpoint/model/secret and finite attempt settings | Ready backend or safe configuration diagnostic; no inference |
| Prepare rule inputs | Unknown pack, source comparison, full/changed scope and explicit limits | Admitted pack, snapshot, every selection record and ready evidence or boundary diagnostics; repository reads only |
| Execute judgments | Prepared backend, supplied named request and cancellation context | One closed decision outcome; external inference after validation only |
| Evaluate selected rules | Prepared selections, backend, bounded execution settings and cancellation | Report with findings, failures, skips, coverage and measurements; no source writes |

The program envelope uses `protocol_version: 1` and `operation: prepare_evidence | run_judgments | evaluate_rules`. Unknown version/operation or malformed framing is an invalid invocation. One invocation per process accepts one bounded JSON request from stdin and emits one JSON result to stdout; stderr is bounded operational text, never machine results or input/provider dumps. EOF is a framing boundary. Usage exit codes are 0 for completed execution/report (including policy failures), 2 for invalid invocation, 3 for incomplete execution and 4 for cancellation. Policy pass/fail is in JSON, not inferred from process success.

The serialized payloads use their owning crate's public types, with a thin program envelope. The CLI serializes those types directly rather than maintaining parallel copies:

| Shape | Required fields / variants and optionality |
| --- | --- |
| `InvocationEnvelope` | `protocol_version: 1`, `operation`, matching `input`; no arbitrary top-level metadata |
| `PrepareEvidenceInput` | `source_root`, `rule_pack_path`, `comparison`, `scope`, `limits`; no credentials |
| `RunJudgmentsInput` | `request: { questions, state }`, `backend_selection`, `attempt_settings`; no secret value |
| `EvaluateRulesInput` | PrepareEvidenceInput fields plus `backend_selection`, `attempt_settings`, `max_concurrent_groups`, explicit `batch_groups` (empty means one call per selected rule) |
| `SourceComparison` | tagged `commits { base_revision, head_revision }` or `worktree { base_revision, include_untracked }`; refs resolved before use |
| `SourceSpan` | repository-relative path, snapshot identity, start/end byte offsets (end exclusive), start/end line and column; user-facing lines/columns are 1-based |
| `RuleEvaluationOutcome` | tagged `finding { finding }`, `evaluation_failed { diagnostics }`, `cancelled { diagnostics }` or `not_started { reason }`, all correlated to rule ID and target ID |
| `LintReport` | requested scope/comparison, pack/snapshot identities, ordered selection records and evaluation outcomes, diagnostic list plus omitted count, measurements |
| `ProgramResponse` | `protocol_version: 1`, operation, matching closed outcome; failure/partial/cancelled results never contain a completion-shaped default |

Batch-group admission requires selected rules, unique question names, identical evidence identities and caller-declared independence; invalid groups refuse the invocation before model calls. Full-scope preparation may use the specified comparison only as provenance; change-scope preparation uses it for selection.

Secrets enter through configured in-memory native arguments or the executable's inherited environment, never its request document. A process caller closes/cancels explicitly through OS termination signaling; the executable converts a supported termination signal into cancellation, stops admission, joins owned work, emits its terminal result when stdout remains available and exits. Forced process kill/crash may yield no report; the caller must treat missing/truncated output as execution failure, never completion. No multiplexed daemon protocol is needed for v1.

A native host owns its existing Tokio runtime. The executable creates one process runtime. The library never calls nested `block_on` or creates a second runtime. Async functions remain normal native library calls. A future TS client can serialize these contracts; an N-API wrapper is not part of initial delivery.

## The narrow JSONL policy and target format

Representative proposed rule line, formatted here for readability; the pack stores one object per line:

```json
{
  "rule_id": "skill.trigger.clarity",
  "target": { "source_path": "skills/example/SKILL.md", "target_id": "U001" },
  "applicability": { "kind": "always" },
  "context_selectors": [],
  "question": {
    "type": "choice",
    "instructions": { "question": "How explicit is this paragraph's activation condition?", "inspect": "target.text" },
    "criteria": {
      "missing": { "definition": "No activation condition is stated" },
      "vague": { "definition": "Topic named, but activation condition unclear" },
      "explicit": { "definition": "Reader can identify when to use the skill" }
    }
  },
  "policy": {
    "kind": "choice_disposition",
    "labels": { "missing": "fail", "vague": "review", "explicit": "pass" }
  },
  "feedback": {
    "fail": "State the concrete situation that activates this skill.",
    "review": "Make the activation condition more specific.",
    "pass": "Activation condition is explicit."
  }
}
```

`RuleId` and `TargetId` are distinct validated types. `source_path` is normalized relative to caller-declared source root; resolutions escaping that root are refused rather than silently collecting unrelated files. IDs and offsets are validated before diagnostic locations are exposed. This is scope containment for declared evidence, not a general sandbox claim.

The Markdown target resolver uses AST-grep's Markdown block tree to locate paragraph nodes, then recognizes the exact ID prefix in eligible prose. It retains full wrapped paragraph bytes and ancestor context, excluding fence/heading/blockquote/inline-code examples. Structural matching alone does not define the ID convention. A `TargetResolution` cannot choose the first of multiple declarations.

Policy variants are `noul_bands`, `choice_disposition`, `choice_category` and `score_bands`; limits, inclusivity and domain coverage are checked during pack admission. Probability/score bands use ordered lower-inclusive/upper-exclusive intervals, with the last endpoint inclusive, covering the primitive's domain without gaps/overlap. Choice mappings cover exactly admitted labels. Native composition is a consumer function over validated outcomes; no string expressions/eval or arbitrary rule code is loaded.

## Source capture and reusable selectors

The proposed repository reader uses native `git2`/libgit2 for resolved refs, tree comparisons and combined index/worktree metadata. Git comparison is batched once per invocation, never per rule. Declared untracked inclusion is explicit. The design treats rename as delete/add where needed; it does not depend on similarity-based rename detection.

Captured file bytes, not subsequent worktree reads, feed parsing/search and model state. Commit-to-commit sources use immutable blob IDs. For working-tree capture, index metadata and selected-file content identities are checked during admission; changing relevant content refuses/retries capture at the caller boundary. Diff regions for captured text are derived from the same before/after bytes. This is a consistent captured input set for the run, not a repository-wide transactional snapshot guarantee.

The evidence owner reads a relevant file once and shares its immutable bytes and parsed tree across rule selectors. Regex/text search and AST-grep matches produce source ranges/captures; a local cache lives only for that invocation and is keyed by content identity and language. Query plans are compiled once per distinct selector. No subprocess-per-rule search and no persistent invalidation service are introduced.

Applicability is a bounded declaration: always, file-path match, or a declared text/structural presence test. Context selectors name source paths/target IDs or text/AST matches inside declared scope. `git_changed` is observed comparison evidence, not an untrusted rule assertion. Selection decisions carry their reason. For unknown query/language/scope state, selection fails or conservatively selects; a zero candidate count under a failed scan is never a proof of absence.

Change scope checks target content, declared context content and rule-definition content identities. Before/after target maps handle deleted/moved paragraphs without equating line movement to behavior change. A missing required after target produces a target diagnostic even if a before paragraph exists. Required evidence overflow refuses that rule before inference. Optional context may be omitted only when its optionality and omission are represented in the bundle. No model reranker is on the essential local path.

## Judgment admission, transport and useful diagnostics

`StructuredJson` represents finite JSON values; questions preserve structured instructions, choice descriptions, score levels and noul true/false descriptions. `QuestionName`, `ChoiceLabel` and `ModelSelection` are distinct validated values. The native request map is dynamic where loaded from JSONL; response correlation checks exact names, kinds, labels/probability keys, rubric/index sets, numeric ranges and receipt fields.

The adapter trait exposes supported primitive semantics and async execution, not filesystem or rule policy. Its received body is an explicitly untrusted adapter-envelope value consumed by the judgment owner; it is never a public completed outcome. Unknown provider data is decoded into canonical primitive structures and validated relative to the immutable request. Alternate/injected adapter envelopes are admitted before branching; unexpected adapter faults become safe `unexpected_provider_error` outcomes.

The first Jev adapter calls OpenRouter System One over HTTP directly using a reusable `reqwest` client. It preserves the native questions/state/model protocol and Authorization header. Client errors, body text, response headers and parse trees never leave the adapter raw. No SDK is required. Future Alpha Decisions and OpenAI Decisions adapters can map their own wire contracts; confidence, structured data, usage and cancellation semantics require new conformance evidence before claiming support.

The public decision outcome variants and their fields are:

| kind | Fields / invariants |
| --- | --- |
| `completed` | Required validated answers and receipt; no failure diagnostics masquerading as an answer |
| `invalid_request` | Required nonempty safe diagnostics; no inference or answers |
| `provider_failure` | Required closed failure class and diagnostics; optional admitted HTTP status; no answers |
| `invalid_response` | Required nonempty safe diagnostics; no partial answer map |
| `cancelled` | Required caller-aborted reason and safe cancellation diagnostic; no completed answers |

Each domain diagnostic is validated where its issue is detected. Origin is rule_pack/source/selection/request/response/provider/execution. The closed code vocabulary across those domain enums is malformed_json, duplicate_rule_id, invalid_policy, missing_target, duplicate_target, deleted_target, unsupported_language, invalid_selector, source_changed, evidence_limit, missing_context, invalid_question, unsupported_primitive, missing_answer, unexpected_answer, wrong_answer_kind, invalid_choice, invalid_number, invalid_rubric, invalid_receipt, authentication, rate_limit, timeout, connection, http_error, unexpected_provider_error or caller_aborted. A crate owns only the codes relevant to its boundary: evidence owns source/target/query issues; decisions owns request/response/provider issues; rules owns pack/policy/selection/scheduling issues. Rules' `ReportDiagnostic` is a tagged union retaining the already admitted domain payloads, not a second universal parser or schema. Paths have at most 32 safe segments; messages have at most 512 characters; issue lists have at most 100 entries and explicitly report omitted count. These are proposed v1 bounds, not measured latency limits.

Expected shape/range/allowed-label details are constructed from validated declarations, not received provider values. Credential locations use fixed field names; secret values never enter diagnostics. Provider diagnostic paths are empty or validated canonical answer paths; arbitrary unknown provider keys/errors are not echoed. Source spans are permitted caller-scoped repository locations. Retry advice is a tagged enum: retry_after with bounded milliseconds, repair_request, repair_source, do_not_retry or unknown. It reports advice, never starts a new caller retry automatically.

Receipt admission preserves nonempty resolved model, nonnegative integer native token counts and optional finite reported USD cost. Adapter maps native `cost` to explicit `cost_usd`; it never estimates prices or invents a zero. Known malformed fields refuse completion; unknown receipt metadata is stripped, while answer maps/index sets remain exact.

## Representative entry-to-effect path

This is a proposed-only path; no predecessor exists. Every local call/return is added. Library callers enter the same invocation owner after the program framing edge.

![Entry-to-effect sequence with invalid input, execution failure and cancellation paths](assets/execution-sequence.png)

[Diagram source](assets/execution-sequence.mmd)

Shapes on the new boundaries are the binding table's invocation, validated pack, frozen evidence, structured request, closed decision outcome and lint report. HTTP sends only the admitted native model/questions/state; model-request body logs are disabled. Source capture is read-only; policy/report generation writes only the returned value.

## Failure, overlap and runtime ownership

| State / owner | Admission or transition | Failure / cleanup |
| --- | --- | --- |
| Preflight / rule admission | Pack, configuration and limits are valid | Whole invocation refused; zero provider calls |
| Preparing / evidence owner | Refs/bytes captured, targets and queries resolved | Per-rule selection/evidence failure; independently ready rules remain usable |
| Executing / invocation and judgment owners | Explicit group has complete evidence, supported questions, available capacity | Finite attempts; provider/response failure never invokes success policy |
| Composing / policy owner | Completed correlated judgment and validated matching policy | Deterministic finding; impossible policy mismatch becomes execution diagnostic, no guessed disposition |
| Cancelling / invocation owner | Caller cancellation observed before terminal admission | Stop new scans/calls/attempts, propagate tokens, join owned work; completed peer outcomes retained |
| Terminal / invocation owner | All owned work settled; report coverage calculated | Immutable complete/incomplete/cancelled result; no background follow-up |

The native caller supplies a cancellation token; the executable owns its signal-to-token mapping. Invocation tasks are owned in a join set/tracker and drained. Synchronous Git/tree parsing/search work runs in bounded blocking tasks, checking cancellation between bounded units; cancellation cannot forcibly preempt arbitrary blocking code. No “cancelled” terminal claim is made until those owned tasks actually settle. Async HTTP futures are not detached; cancellation/timeout drops the attempt future under the adapter and awaits task settlement before retry/return.

One adapter attempt controller governs the reusable HTTP client. Defaults proposed for initial proof are a 10,000 ms attempt timeout, zero automatic retries, caller-selected finite retry count, and capped backoff/Retry-After. When retries are enabled, only declared transient connection/timeout/rate-limit/server classes qualify; invalid request/response/authentication never retries automatically. Underlying HTTP automatic retry is disabled so another controller cannot multiply attempts. A transport loss may follow accepted inference, so retries can incur extra billing. Usage from a completed response does not establish total billed cost of lost attempts.

Concurrency capacity is explicit invocation configuration; order in the returned report follows rule-pack order, not completion order. Shared immutable bytes/trees and a reusable client do not share mutable per-run answer or cancellation state. No global scheduler/limiter, persistent cache, inference idempotency or crash recovery store is added. A crash leaves invocation completion unknown to the process caller until a complete result is observed.

## Proof paths and real dependencies

![Real local components, controlled HTTP proof and separate hosted provider proof](assets/proof-boundaries.png)

[Diagram source](assets/proof-boundaries.mmd)

Local library/selector dependencies are real in evidence integration. An HTTP fixture substitutes only the hosted boundary in V9/V13; it must observe actual request bodies/attempts, not mock the adapter being proved. V11 closes the hosted gap through the public Rust API. V12 uses one fixture rule definition in lint and eval drivers with independently written expectations; it does not migrate or certify the production plugin. V14 profiles cold/warm runs and counts scans/parses/requests before adopting a persistent cache or native addon.

Invalid ID states are rejected by constructors; primitive/result kinds are enum-constrained; JSONL/policy inconsistencies are rejected at pack admission; source/selection incompleteness prevents inference for that rule; wrong answer correlation/unsafe diagnostics are rejected at judgment admission; execution-failure findings are unrepresentable in the report's distinct result variants. Static checks and behavioral negative controls reinforce these boundaries rather than replacing them.

## Obligation realization and proof seams

| U | R and scenario | E | Owner | Interface | Shape and home | State | Failure | Proof seam |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| U1,U8,U18,U19 | R1 independent libraries and program reuse | E5,E13,E16 | Domain owners and invocation owner | Evidence/decision/rule entries; program facade | Owned public types in sunstack-evidence/decisions/rules; envelope in sunstack-rule-cli | Independent entry → terminal | No reverse dependencies or required JS | V1 independent native consumers; V2 real process |
| U3,U14,U15 | R2 pack preflight | E9,E10,E14 | Rule admission | Pack loading | ValidatedRulePack, rule_contracts | Supplied → admitted/refused | Atomic no-inference refusal | V3 real JSONL and call counter |
| U14,U15 | R3 target identity | E11,E12,E14 | Target resolver | Resolve paragraph | TargetResolution, evidence_contracts | Captured → resolved/refused | Missing/duplicate/deleted diagnostic | V4 real Markdown spans |
| U16 | R4 Git/content consistency | E11,E13,E16 | Evidence owner | Capture comparison | SourceSnapshot, evidence_contracts | Read → freeze | Source changed/ref unavailable | V5 repository content mutations |
| U16,U17 | R5 structural candidates | E11,E13 | Evidence owner | Search selector | SourceFact and uncertainty, evidence_contracts | Frozen → matches | Syntax-only attribution | V6 actual AST/text results |
| U14,U16 | R6 selection accounting | E9,E15,E16 | Selection owner | Select rules | RuleSelection, rule_contracts | Rule → one disposition | Unknown cannot silently skip | V7 selected/skipped/failed controls |
| U5,U14,U16 | R7 required evidence | E11,E13,E14 | Evidence owner | Prepare bundle | EvidenceBundle, evidence_contracts | Selected → ready/refused | Limit/missing context diagnostic | V7 no inference on incomplete bundle |
| U3,U4,U5 | R8 structured mixed batch | E1,E2,E4,E5 | Judgment owner | Request admission/execution | JudgmentQuestion and StructuredJson, decision_contracts | Request → execute | No flatten/drop/regroup | V8 domain cases; V9 actual HTTP body |
| U3,U4 | R9 correlated completion | E1,E5,E6,E7 | Response admission | Validate body/request | JudgmentAnswer and Receipt, judgment_validation | Untrusted → complete/refuse | No partial success | V8 mismatch controls; V9 adapter |
| U2,U3,U14 | R10 useful safe diagnostics | E14 | Detecting boundary owner; rules report wraps | Map issue/failure once | Domain diagnostics in evidence/decisions/rules; ReportDiagnostic in sunstack-rules/report_contracts | Fault → typed diagnostic → report | No duplicate schema/raw values; bounded omission count | V10 useful/context/exposure controls |
| U4,U12 | R11 backend substitution | E1,E3,E5 | Judgment owner and Jev adapter | Backend trait/capabilities | DecisionBackend, decision_contracts | Prepare → select/refuse | Unsupported primitive | V9 conformance; V11 Jev only |
| U6 | R12 native usage/model/cost | E5,E7 | Response admission | Receipt admission | ExecutionReceipt, judgment_validation | Complete | Missing/malformed known fields | V9 supplied/absent cost; V11 hosted |
| U7,U14 | R13 failure vs finding | E5,E8,E14,E16 | Invocation and policy owners | Compose/report | Distinct result variants, rule_contracts | Judgment → finding or failure | No failure-to-pass aggregation | V8/V12 independent negative controls |
| U1,U3 | R14 cancellation settlement | E5,E16 | Invocation owner | Cancellation/join | Cancellation token and outcome, rule_execution | Active → cancelling → terminal | No detached owned task | V9/V13 held work and cleanup |
| U3,U6 | R15 bounded one-controller attempts | E3,E5,E7 | Jev adapter | HTTP attempt loop | Attempt settings, decision_backends | Attempt → settle/retry/stop | Finite limits; remote ambiguity | V9/V13 observed attempt count |
| U5,U7,U9,U15 | R16 source-linked policy | E6,E8,E9,E16 | Policy owner | Apply declared policy | RulePolicy/RuleFinding, rule_policy | Valid answer → finding | Refused policy cannot compose | V12 shared rules/independent expectation |
| U4,U17 | R17 facts vs inference | E6,E13 | Evidence and judgment owners | Evidence attribution | Fact/uncertainty/judgment variants | Discover → present | No calibrated heuristic claim | V6/V12 contradiction cases |
| U6,U16 | R18 measured filtering | E11,E16 | Invocation owner | Report measurements | InvocationMeasurements, rule_execution | Run → measured report | No predicted SLA/savings | V14 declared workload profile |
| U2,U10,U14 | R19 credential/body isolation | E3,E11,E14 | Adapter and diagnostic owners | Secret config/safe mapping | Private secret in sunstack-decisions; owned domain diagnostic payloads | Memory-only config | No raw/default body logs | V10 sentinel output capture |
| U9,U10,U11,U13,U18,U19 | R20 bounded composable delivery/quality | none: platform constraint | Repository and program owners | Build/check/public surfaces | Four-crate workspace dependency graph and owned contracts | Development → verified artifacts | Cycles/reverse dependencies and other-repo edits excluded | V1/V2 independent consumers; V15 real gates/dependency graph |

All E1–E16 are bound in the entity table. C1–C7 correspond to the interfaces, source/selection, judgments, diagnostic/lifecycle, policy and performance sections above. V1–V15 name future observations, not completed tests. No visual or static design check substitutes for their execution.

## External source basis and compatibility limits

- [AST-grep core at commit 0be5673](https://github.com/ast-grep/ast-grep/blob/0be5673107200ee28be83517756e2fd122611fbc/crates/core/src/lib.rs) provides parse/traverse/search/rewrite library operations. The inspected workspace declares version 0.50.0; no exact dependency is installed here.
- [Markdown language support](https://github.com/ast-grep/ast-grep/blob/0be5673107200ee28be83517756e2fd122611fbc/crates/language/src/lib.rs) and [Markdown tests](https://github.com/ast-grep/ast-grep/blob/0be5673107200ee28be83517756e2fd122611fbc/crates/language/src/markdown.rs) establish syntax support. [Outline heading rules](https://github.com/ast-grep/ast-grep/blob/0be5673107200ee28be83517756e2fd122611fbc/crates/outline/src/default_rules/markdown.yml) do not implement paragraph ID meaning or semantic references; those remain our resolver contract/proof.
- [git2 comparisons](https://docs.rs/git2/latest/git2/struct.Repository.html) and [DiffOptions](https://docs.rs/git2/latest/git2/struct.DiffOptions.html) expose tree/index/worktree and untracked controls. Captured-content agreement and changing-worktree behavior remain our V5 obligation.
- [TypeSafe structured primitives](https://docs.typesafe.ai/primitives/advanced) and [OpenRouter Jev HTTP surfaces](https://openrouter.ai/docs/guides/community/jev) ground v1 native request semantics. [System One response schema](https://openrouter.ai/docs/api/api-reference/systemone/submit-a-system-one-request) documents usage and optional cost.
- [OpenAI Decisions](https://developers.openai.com/api/docs/guides/decisions) and [OpenRouter Alpha Decisions](https://openrouter.ai/docs/client-sdks/python/sdks/decisions/README#alpha-decisions) inform future adapter shape only; neither is v1 support.
- [Tokio owned task tracking](https://docs.rs/tokio-util/latest/tokio_util/task/struct.TaskTracker.html) and [reqwest client configuration](https://docs.rs/reqwest/latest/reqwest/struct.ClientBuilder.html) inform cleanup/timeout/retry mechanisms; physical cancellation and exact library versions require runtime proof.

Remaining structural choices are the narrow rule-policy/paragraph conventions if a real first rule cannot use them, native shipping platforms, and an optional TS integration method. No numeric latency budget, calibrated ranking, immutable distribution destination or alternate provider conformance is inferred from these sources.
