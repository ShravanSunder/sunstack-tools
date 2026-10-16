# Public contracts: exact shapes

This companion to the [Program Design](program-design.md) gives the complete fields of every public input and output: the four program operations, their outcomes, reports, diagnostics and progress. A caller can build each request and decode each result from this page alone. Authored definitions (RuleSets, rule lines, routers, declarations, applicability, invocation input) are in [`schemas/`](schemas/) and the [schema inventory](schema-inventory.md).

**Notation:**
- `name: Type` is a required field.
- `name?: Type` is an independently absent observation (`Option` in Rust). It is never a default and never a variant.
- `[T]` is a list, `map<K, V>` a JSON object.
- `a | b { ... }` is an internally tagged union. The tag is `kind`, except the Jev-facing primitives, which use `type`.
- Each section names its owning crate and module. The executable adds only the envelope.

## Envelope (`sunstack-rule-cli/protocol_boundary`)

| Shape | Fields |
| --- | --- |
| `ProgramRequest` | `protocol_version: 1`, `operation: prepare_evidence \| prepare_rules \| run_judgments \| evaluate_rules`, `input`: the matching input below |
| `ProgramResponse` | `protocol_error { protocol_version: 1, diagnostics: DiagnosticList }` or `operation_result { protocol_version: 1, result: { operation, outcome } }`, where `outcome` matches `operation` |

An unknown version or operation, malformed framing or an over-limit input is a `protocol_error`, never a fabricated known outcome.

## Shared shapes

| Shape (owner) | Fields |
| --- | --- |
| `Diagnostic` (each crate) | `origin: DiagnosticOrigin`, `code`: a closed code from [Diagnostics](program-design.md#diagnostics), `location: DiagnosticLocation`, `guidance`: text of at most 512 characters, `retry_advice: RetryAdvice` |
| `DiagnosticOrigin` | `rule_set`, `rule_file`, `router`, `input_binding`, `source`, `transcript`, `selection`, `applicability`, `request`, `response`, `provider`, `execution`, `invocation`, `protocol` |
| `DiagnosticLocation` | `none`; `definition { file, line, column, field_path }`; `field { field_path }`; `source { source: SourceRef, span?: SourceSpan }`; `record { source: SourceRef, record_id }`; `answer { request_index, question_name }`. A `field_path` has at most 32 safe segments, and arbitrary author keys are not copied |
| `RetryAdvice` | `retry_after { milliseconds }`, `repair_request`, `repair_source`, `capture_again`, `repair_configuration`, `restore_provider_access`, `do_not_retry`, `unknown` |
| `DiagnosticList` | `items: [Diagnostic]` (at most 100), `omitted_count` |
| `SourceRef` (evidence) | `document { repository, path, content_identity }`, `transcript { path, format, committed_bytes }`, `payload { group_position, input }`, `repository_file { repository, revision, path, content_identity }` (milestone 2) |
| `SourceSpan` (evidence) | `source: SourceRef`, `byte_start`, `byte_end` (exclusive), `start_line`, `start_column`, `end_line`, `end_column` (1-based) |
| `CaptureCoverage` (evidence) | `source: SourceRef`, `cursor_basis: no_cursor_supplied \| continued_from_cursor`, `tail: complete \| incomplete { bytes }`, `gaps: [CoverageGap]`, `bytes_read`, `bytes_parsed` |
| `CoverageGap` | `unsupported_record { position }`, `malformed_record { position }`, `orphan_result { call_id, position }`, `ambiguous_turn { position, reason: consecutive_user_messages \| unresolved_branch }`, `unknown_injection { record_id }` |
| `ReturnedCursor` (evidence) | `transcript_path`, `format`, `cursor`: an opaque string |
| `InvocationMeasurements` (rules; evidence counts are its own) | `files_considered`, `files_read`, `files_parsed`, `transcript_bytes_read`, `transcript_bytes_parsed`, `bytes_scanned`, `bytes_included`, `rules_selected`, `rules_skipped`, `rules_failed`, `applicability_entries_evaluated`, `local_elapsed_ms`, `provider_elapsed_ms`, `model_requests`, `inference_attempts` |

## `prepare_evidence` (`sunstack-evidence`)

**Input `EvidencePlanInput`:**
- `repositories: [RepositoryBinding]`
- `sources: map<SourceName, SourceSpec>`: evidence-owned, so a direct caller needs no rules types
- `payload_schemas: [{ name, path }]`
- `cursors: [{ transcript_path, format, cursor }]`
- `declarations: [EvidenceDeclaration]`: over `sources`, the same shape rules use
- `native_queries: [{ id, selector: EvidenceSelector }]`: repository-qualified, from [the native selector schema](schemas/evidence-selector.schema.json). This is the only place a caller names seeds directly
- `package_bindings: [PackageSourceBinding]`
- `limits: InvocationLimits`
- `discovery: none | bounded { limits: DiscoveryLimits }`. Admission refuses a native query or change-set source that needs discovery when `discovery` is `none`

No backend and no credentials.

**`SourceSpec`** (`sunstack-evidence/evidence_contracts`):
- `document { repository, path }`
- `conversation { transcript_path, format, final_assistant_text? }`
- `payload { schema, value }`
- `change_set { repository }`

Rules never passes its own types down. After admitting an invocation, `sunstack-rules/input_binding` maps each admitted declaration and binding to a `SourceSpec`; a payload takes its schema from the input declaration. `SourceName` is the evidence-owned name type; rules uses it for input names. `EvidenceDeclaration` is also evidence-owned, and rules imports it.

**Outcome `EvidencePlanOutcome`:**

| `kind` | Fields |
| --- | --- |
| `prepared` | `selections: [SelectionResult]`, `captures: [CaptureCoverage]`, `cursors: [ReturnedCursor]`, `measurements` |
| `incomplete` | the same fields plus `diagnostics: DiagnosticList`. Used only when a `required` declaration is refused. A refused `optional` declaration leaves the plan `prepared`: its `SelectionResult` is `refused { diagnostics }`, and rules records it in request state as `Omission { reason: optional_unavailable }` |
| `refused` | `diagnostics` |
| `cancelled` | `progress: before_plan_admission \| plan_admitted { partial: { selections, captures, cursors } }`, `diagnostics`, `measurements` |

**`SelectionResult`:**
- `id`
- `readiness`: either `ready { content: SelectionContent, omissions: [Omission], coverage }` or `refused { diagnostics }`

**`SelectionContent` variants:**
- `paragraph { span, text }`
- `document_text { span, text }`
- `matches { matches: [{ span, text, captures: map<String, SourceSpan> }] }`
- `conversation_records { source, records: [ConversationRecord] }`
- `payload_value { input, pointer, value }`
- `changed_units { units: [SourceUnit] }`
- `related_group { group: ContextGroupOutcome }`

**`Omission`:** `{ reason: window | projection | limit | optional_unavailable, count }`.

Conversation records use the E31–E33 shapes in the main design's binding table.

## `prepare_rules` (`sunstack-rules`)

**Input:** `RuleInvocationInput` ([schema](schemas/rule-invocation.schema.json)). Its backend and budget settings are used to encode and assess requests; nothing is dispatched.

**Outcome `RulePreparationOutcome`:**

| `kind` | Fields |
| --- | --- |
| `prepared` | `preparation: PreparedRules` |
| `prepared_with_failures` | `preparation: PreparedRules`, `diagnostics` |
| `preparation_failed` | `diagnostics`, `measurements` |
| `invalid` | `diagnostics` |
| `cancelled` | `progress: before_preparation_admission \| preparation_admitted { partial: PreparedRules }`, `diagnostics`, `measurements` |

**`PreparedRules`:**
- `origin: ReportedOrigin`
- `routing`: `rule_sets`, or `router { status: not_evaluated, candidate_rule_sets: [path] }`
- `groups: [PreparedGroup]`
- `occurrences: [PreparedOccurrence]`
- `planned_requests: [PlannedRequestRecord]`
- `captures`, `cursors`, `measurements`

| Shape | Fields |
| --- | --- |
| `PreparedGroup` | `rule_set`, `position`, `binding: bound \| routed { router_group }`, `leading_entries: [EvaluatedEntry]`, `status: planned \| not_applicable { entry, basis } \| applicability_failed { entry, cause }` |
| `PreparedOccurrence` | `key: RuleOccurrenceKey`, `status: planned \| not_applicable { entry, basis } \| selection_failed { diagnostics }` |
| `PlannedRequestRecord` | `index`, `owner: RequestOwner`, `mode: RequestMode`, `questions: [QuestionBinding]`, `prerequisites: [EntryKey]`, `budget: within_configured_budget { counts, basis } \| exceeded { diagnostics } \| unverifiable { diagnostics }`, `evidence: [SelectionResult]` |

`prepared_with_failures` means a selection, evidence or budget failure, or an unresolved leading condition, is recorded somewhere in `PreparedRules`.

## `run_judgments` (`sunstack-decisions`)

**Input `RunJudgmentsInput`:**
- `request: { questions: map<QuestionName, JudgmentQuestion>, state: SharedEvidenceState }`
- `backend: BackendSelection`
- `attempts: AttemptSettings`
- `request_budget: RequestBudgetSettings`

**Outcome `DecisionRunOutcome`:**

| `kind` | Fields | Exit |
| --- | --- | --- |
| `completed` | `answers: map<QuestionName, JudgmentAnswer>`, `receipt: ExecutionReceipt`, `attempts: [InferenceAttemptRecord]` | 0 |
| `invalid_request` | `diagnostics`. Includes budget refusal (`context_budget_exceeded`, `budget_unverifiable`). There are no attempts | 2 |
| `provider_failure` | `failure_class: ProviderFailureClass`, `http_status?`, `diagnostics`, `attempts` | 3 |
| `invalid_response` | `diagnostics`, `attempts` | 3 |
| `cancelled` | `diagnostics`, `attempts`. Remote inference may still have happened | 4 |

| Shape | Fields |
| --- | --- |
| `JudgmentAnswer` | `noul { yes_probability }`; `choice { selected_label, confidence, probabilities: map<label, number> }`; `score { score, confidence, legend: map<index, StructuredEntry>, probabilities: map<index, number> }` ([schema](schemas/decision-answer.schema.json)) |
| `ExecutionReceipt` | `resolved_model`, `usage: { input_tokens, output_tokens, cost_usd? }` |
| `InferenceAttemptRecord` | `ordinal`, `evidence: not_dispatched { failure_class: AttemptFailureClass } \| completed { http_status } \| remote_outcome_unknown { failure_class: AttemptFailureClass, http_status? }` |
| `ProviderFailureClass` | Why a whole run ended as `provider_failure`: `provider_rejected_request`, `authentication`, `authorization`, `insufficient_credits`, `provider_resource_unavailable`, `request_too_large`, `rate_limit`, `provider_unavailable`, `timeout`, `connection`, `http_error`, `unexpected_provider_error` |
| `AttemptFailureClass` | Why one attempt ended without a completion: every `ProviderFailureClass` value, plus `invalid_response` (the body failed request-relative admission) and `caller_aborted` (the caller cancelled; after dispatch the attempt is `remote_outcome_unknown`, because cancellation never proves remote inference or billing stopped) |

## `evaluate_rules` (`sunstack-rules`)

**Input:** `RuleInvocationInput`.

**Outcome `RuleInvocationOutcome`:**

| `kind` | Fields | Exit |
| --- | --- | --- |
| `invalid` | `diagnostics` | 2 |
| `preparation_failed` | `diagnostics`, `measurements` | 3 |
| `complete_report` | `report: RuleInvocationReport` | 0 |
| `incomplete_report` | `report: RuleInvocationReport` | 3 |
| `cancelled` | `progress: before_report_admission \| report_admitted { partial_report: RuleInvocationReport }`, `diagnostics`, `measurements` | 4 |

**`RuleInvocationReport`:**
- `origin: ReportedOrigin`
- `routing`: `rule_sets`, or `router { outcome: RouterOutcome }`
- `groups: [GroupResult]`
- `occurrences: [RuleOccurrenceResult]`
- `requests: [RequestRecord]`
- `captures: [CaptureCoverage]`
- `cursors: [ReturnedCursor]`
- `diagnostics: DiagnosticList`
- `measurements: InvocationMeasurements`

| Shape | Fields |
| --- | --- |
| `ReportedOrigin` | `direct`, or `hook { host, event, session: ObservedField, turn: ObservedField, agent: main \| subagent { id, type: ObservedField } \| unknown \| unavailable, observed_at: ObservedField }`. `ObservedField` is `observed { value } \| unavailable` |
| `EntryKey` | `owner: router \| rule_set { path, group } \| rule { path, group, rule_id }`, `position` |
| `RuleOccurrenceKey` | `rule_set`, `group`, `rule_id` |
| `GroupResult` | `rule_set`, `position`, `binding: bound \| routed { router_group }`, `applicability: ApplicabilityOutcome`, `entries: [EvaluatedEntry]` |
| `ApplicabilityOutcome` | `applicable`; `not_applicable { entry: EntryKey, basis }`; `applicability_failed { entry: EntryKey, cause: Cause }`; `not_evaluated { reason: cancelled \| invocation_refused \| earlier_entry_stopped }` |
| `EvaluatedEntry` | `key: EntryKey`, `result`: `condition { outcome: passed \| did_not_pass \| unresolved { diagnostic }, basis }`, `decision { threshold, outcome: passed { answer: JudgmentReference } \| did_not_pass { answer: JudgmentReference } \| failed { cause: Cause } \| cancelled }`, or `not_evaluated { reason }`. `basis` describes the declared test, never a received payload value |
| `RuleOccurrenceResult` | Every variant carries `key: RuleOccurrenceKey`: `judged { judgment: RuleJudgment }`, `not_applicable { stopped_at: EntryKey, basis }`, `outside_change_scope { reason }`, `selection_failed { diagnostics }`, `evaluation_failed { cause: Cause }`, `cancelled`, `not_started { reason: caller_cancelled \| request_limit }` |
| `Cause` | `request { request_index }`, `evidence { diagnostics }`, `budget { diagnostics }`, `gate { entry: EntryKey }`, `condition { entry: EntryKey, diagnostic }` |
| `RuleJudgment` | `key`, `verdict: VerdictOutcome`, `declared_severity: block \| nudge`, `answers: map<QuestionName, JudgmentReference>`, `target: TargetLocation` |
| `VerdictOutcome` | `basis: noul { answer: JudgmentReference } \| composite { dimensions: [{ question, weight, normalized_value, answer: JudgmentReference }] }`, `value`, `threshold`, `result: pass \| fail { activation: nudge \| block \| nudge_above_band { band } }` |
| `JudgmentReference` | `request_index`, `question_name` (the request-local name, such as `q000003`) |
| `RouterOutcome` | `routed { label, confidence, answer }`, `uncertain_intent { confidence, floor, distribution: map<label, number>, answer }`, `no_route { label, confidence, answer }`, `router_failed { cause: Cause }`, `not_evaluated { reason: cancelled }` |
| `RequestRecord` | `index`, `owner: RequestOwner`, `mode: RequestMode`, `questions: [QuestionBinding]`, `prerequisites: [EntryKey]`, `status: sent { outcome: DecisionRunOutcome } \| not_sent { reason: gate_stopped { entry } \| cancelled \| budget_refused { diagnostics } \| request_limit }`, `discarded: [{ question_name, reason: owner_not_applicable { entry } \| not_routed }]` |
| `RequestOwner` | `router`, `rule_set_group { rule_set, group }` |
| `RequestMode` | `rules`, `speculative`, `staged_gate`, `split { part, of }` |
| `QuestionBinding` | `request_name`, `owner: router \| rule_set_decision { entry } \| rule_decision { rule_id, entry } \| verdict { rule_id } \| informational { rule_id, name }` |
| `TargetLocation` | `paragraph { span, target_id }`, `conversation_records { source, record_ids }`, `payload_field { input, pointer }`, `code_units { units: [SourceUnitKey] }` |

A report is complete when every occurrence is `judged`, `not_applicable` or `outside_change_scope`, and the router (if any) is `routed`, `uncertain_intent` or `no_route`.
