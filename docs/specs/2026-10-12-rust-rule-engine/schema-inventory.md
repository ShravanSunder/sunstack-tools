# Schema inventory

Every field a rule author or a caller writes, and what the engine returns. Each schema has one owning crate, named in [Program Design](program-design.md#crates-and-dependency-direction). The JSON Schemas in [`schemas/`](schemas/) are design contracts. Production schemas are generated from the owning Rust types and checked against them.

**Shared conventions:**
- Every union is internally tagged. The tag is `type` for the Jev-facing primitives, and `kind` everywhere else.
- Unknown fields and duplicate keys are refused.
- Free JSON is allowed only inside instructions, criteria and payload values.

**Tag conventions are per domain, and deliberately so.** Decision primitives use `type` because that is the native noul, choice and score contract. Tools-owned contracts use `kind`. Payload values are owned by their publishers and keep their own field case; `AgentJobPacket` is camelCase.

## Authored by a rule author

| Schema | Field | Purpose, and why it is separate |
| --- | --- | --- |
| [RuleSet](schemas/rule-set.schema.json) | `schema_version` | Format version. Unsupported versions are refused |
| RuleSet | `name`, `description` | For people and agents choosing RuleSets. Never sent to the model, never a condition |
| RuleSet | `inputs` | Named inputs, each `document`, `conversation`, `change_set` or `payload { schema }`. Rules reference names; invocations bind them |
| RuleSet | `applicability` | Ordered conditions and decisions. Empty means always |
| RuleSet | `common_ruleset_instructions` | Shared instructions for this RuleSet's rule questions and noul verdicts. Null adds nothing. Never sent with routers or decisions |
| RuleSet | `rule_paths` | Ordered JSONL files, relative to the RuleSet |
| [Rule line](schemas/rule-file.schema.json) | `rule_id` | Unique within a RuleSet. The same ID under another RuleSet is a different occurrence |
| Rule line | `evidence` | Declarations over named inputs. Each one is labeled with this rule and its ID in the request |
| Rule line | `target` | Which declaration the result points at: a paragraph, conversation records, a payload field or code units |
| Rule line | `applicability` | Rule-level conditions and decisions, after the RuleSet's |
| Rule line | `questions` | Informational native questions. May be empty |
| Marked entry | `mechanical_pass`, `narrowing`, `gates`, `questions` | Optional native decision-model fields; assembly keeps the marked verdict sentence and sets conformance meaning; entries cannot override `pass_means` |
| Rule line | `verdict` | `noul`, `composite` or local `mechanical`; judged verdicts accept `pass_means: conforms \| finding` (default `finding`); mechanical verdicts mean conformance. Required on every rule |
| Rule line | `severity` | `error` or `warn`. Omitted means warn. Never sent to the model |
| [Router](schemas/router.schema.json) | `inputs`, `evidence`, `question` | Its own named inputs and declarations, and one `choice` question |
| Router | `routes` | Label to RuleSets, each with input wiring from RuleSet input name to router input name |
| Router | `confidence_floor` | Compared with the Choice answer's native confidence |
| [Evidence declaration](schemas/evidence-declaration.schema.json) | `id`, `input`, `requirement`, `select`, `max_items`, `max_bytes` | What to read from one input. `requirement` omitted means required |
| Evidence declaration | `select.kind` | Document: `paragraph`, `whole_document`, `literal_text`, `regex_text`, `ast_pattern`. Conversation: `conversation`. Payload: `whole_payload`, `payload_field`. Change set (milestone 2): `change_set_literal_text`, `change_set_regex_text`, `change_set_ast_pattern`, `changed_units`, `related_units` |
| Conversation selection | `turns`, `user_messages`, `assistant_messages`, `tool_use`, `hook_injected` | One generic selector with no presets. `turns` is `all`, `last { count }` or `since_cursor`. `tool_use` is `excluded` or `included { tools, results }` |
| [Applicability](schemas/applicability.schema.json) | `condition { test }` | `payload_field_equals`, `payload_field_in`, `payload_field_present`, `origin_field_equals`, `pattern_present` |
| Applicability | `decision { question, evidence, threshold }` | A noul over its own evidence. Passes at yes probability ≥ threshold |
| [Questions](schemas/decision-question.schema.json) | `type`, `instructions`, `criteria` | Native noul, choice and score. Noul criteria may be omitted, or given as both `true` and `false` |

**What JSON Schema cannot check, and admission does:**
- references across files
- input kinds against selections
- weights summing to 1
- a band below its threshold, and only on an error rule
- composite dimensions naming this rule's noul or score questions
- route labels inside the choice criteria
- complete, kind-compatible wiring
- a conversation selection that selects at least one record type
- unique rule IDs per RuleSet

## Supplied by a caller

| Schema | Field | Purpose |
| --- | --- | --- |
| [Definition check](schemas/check-rules-input.schema.json) | `definition_root`, `definitions`, `payload_schemas`, `limits`, `output` | `CheckDefinitionsInput`: path-only RuleSets or Router; compile declared payload schemas; required positive `max_file_bytes`. No target bindings, repositories, discovery or backend |
| [Invocation](schemas/rule-invocation.schema.json) | `definition_root` | Where definition, rule and payload-schema paths resolve from |
| Invocation | `definitions` | `router { path, binding_groups }` or `rule_sets { rule_sets: [{ path, binding_groups }] }` |
| Invocation | `binding_groups[].bindings` | Each input bound once: `document { repository, path }`, `conversation { transcript_path, format, final_assistant_text? }`, `payload { value }`, `change_set { repository }` |
| Invocation | `repositories` | Repository bindings: the only authority for roots, allowed paths and change scope. `ChangeScope` is `full`, `uncommitted`, `revision_diff`, `commit_set`, `recent_commits`, `recent_uncommitted`, or a `union` of those (no nested union) |
| Invocation | `payload_schemas` | `{ name, path }` catalog entries. Remote references are never fetched |
| Invocation | `origin` | `hook { host, event, session?, turn?, agent?, observed_at? }` or `direct`. An omitted hook field is reported as unavailable. Provenance only |
| Invocation | `cursors` | One per transcript source, as returned by an earlier report |
| Invocation | `execution` | `speculation: off \| on` for gates over different inputs, and `max_concurrent_requests` |
| Invocation | `limits`, `backend`, `attempts`, `request_budget` | Cumulative evidence limits; the Jev endpoint, model and optional caller-declared `context_ceiling_tokens`; attempt control; the request budget. Credentials are never here |
| Invocation (milestone 2) | `discovery`, `package_bindings` | Optional `none \| bounded { limits: DiscoveryLimits }` and package-source mappings, passed unchanged into the invocation's evidence session. Absent means `none` and no mappings. The [invocation schema](schemas/rule-invocation.schema.json) carries them, with the change scopes, from milestone 2 |
| [Native selectors](schemas/evidence-selector.schema.json) | Repository-qualified selectors, `RelationshipQuery`, `ChunkSettings`, `QueryPath` | For native and `prepare_evidence` callers. `QueryPath` is now tagged `exact` or `glob` |

## Returned

| Owner | Shape | Purpose |
| --- | --- | --- |
| Decisions ([answer](schemas/decision-answer.schema.json)) | Noul `yes_probability`; choice `selected_label`, `confidence`, `probabilities`; score `score`, `confidence`, `legend`, `probabilities` | Native answers, unchanged |
| Decisions | `DecisionRunOutcome`, receipt, attempt records | One per actual request |
| Rules | `RuleInvocationOutcome` | `invalid`, `preparation_failed`, `complete_report`, `incomplete_report`, `cancelled { progress }` |
| Rules | `RuleOccurrenceResult` | `judged`, `not_applicable`, `outside_change_scope`, `selection_failed`, `evaluation_failed`, `cancelled`, `not_started`. Milestone 2: selected variants carry `selected_by: [SelectedReason]` |
| Rules | `PreparedOccurrence` | `planned { selection }`, `not_applicable`, `outside_change_scope` (milestone 2), `selection_failed` |
| Rules (milestone 2) | `OccurrenceSelection` | `selected { selected_by }` or `comparison_pending` |
| Rules (milestone 2) | `SelectedReason` | Closed and ordered: `full_scope`, `target_changed`, `context_changed`, `rule_changed`, `new_rule`, `applicability_changed`, `baseline_unavailable`, `before_analysis_unknown` |
| Evidence (milestone 2, native only) | `ChangeComparison` | `unchanged`, `changed` (content level), `path_changed` (path level), `added`, `deleted`, `before_unknown`, `no_before_side`, `refused`, from `EvidenceSession::compare_change_sides` over a declaration, a whole input or a definition file, at `path` or `content` level |
| Rules | `VerdictOutcome` | Basis (noul answer, or composite dimensions with normalized values and weights), value, threshold, `pass` or `fail { activation: warn \| error \| warn_above_band }` |
| Rules | `RouterOutcome` | `routed`, `uncertain_intent { confidence, floor, distribution }`, `no_route`, `router_failed`, `not_evaluated` |
| Rules | `RequestRecord` | Mode, question bindings, outcome, discarded answers with their cost |
| Evidence | Captures, coverage, cursors | Tail state, gaps, omissions, bytes read and parsed |
| Each boundary owner | Diagnostics | Origin, closed code, safe location, guidance, retry advice. Never received values |

## Removed from the earlier design

| Removed | Replaced by |
| --- | --- |
| `instruction_prefix` | `common_ruleset_instructions` |
| `target_id` on a rule | A `target` declaration ID, of any targetable kind |
| `context_selectors` and `selector_id` | `evidence` declarations with `id` |
| `applicability: { kind: always \| presence }` | An ordered list of conditions and decisions |
| `SkillRuleBinding` | Named inputs and binding groups |
| `batch_groups` and per-rule calls | One request per RuleSet × group, planned by the engine |
| An untagged `QueryPath` string | Tagged `{ kind: exact, path }` |
| A rule-authored `related_units` with its own seeds and repositories | A seed-free declaration; seeds come from the bound change-set input |

## Checked

`validate_design_examples.py` validates every example against these schemas and runs 24 refusal controls. It also checks each schema against the Draft 2020-12 metaschema and refuses duplicate keys. The script lives in the Lead's job scratch.

Last run, with jsonschema 4.26.0, exit 0:
- 14 valid cases, including a seed-free `related_units` declaration
- 24 refused controls, including a `related_units` that names its own seed

This proves shape only. It does not prove admission semantics, capture, execution or model behavior.
