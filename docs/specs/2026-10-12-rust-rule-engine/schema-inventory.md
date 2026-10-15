# Domain schema inventory

The authored definition is split into **RuleSet JSON** and **rule-definition JSONL**. Question and answer contracts belong to decisions; source selectors belong to evidence; RuleSet loading, rule identity and result correlation belong to rules. These are the same three library domains in [Program Design](program-design.md#composable-crate-boundaries).

## Definitions authored by a rule author

| Domain and schema | Field | Purpose and why it is separate |
| --- | --- | --- |
| RuleSet — [schema](schemas/rule-set.schema.json) | `schema_version` | Identifies the document format; unsupported versions refuse admission. Not a revision or another RuleSet identity |
| RuleSet | `name` | Human/agent label for choosing a RuleSet. It is not model instructions or an execution condition |
| RuleSet | `description` | Explains what this RuleSet checks. It is not automatically forwarded to the model |
| RuleSet | `rule_paths` | Ordered explicit JSONL files to load, relative to this RuleSet file. Duplicate normalized sources refuse admission |
| RuleSet | `instruction_prefix` | Shared question instructions for this RuleSet. Null means add nothing. Other values retain structured content; no string concatenation or key override |
| Rule — [one-line schema](schemas/rule-file.schema.json) | `rule_id` | Stable check identity within the selected RuleSet. Distinct from a question name and source target ID |
| Rule | `target_id` | Paragraph selected in the bound skill document. This is the initial skill-lint profile, not a restriction on the generic decisions API |
| Rule | `applicability` | Locally evaluable `always` or selector-presence condition for this rule. It does not start a run or subscribe to a lifecycle |
| Rule | `context_selectors` | Declares additional evidence the rule needs, with required/optional meaning and finite limits. It stores requirements, not captured source text |
| Rule | `questions` | Nonempty map of independently answered names to native question variants; all questions in a rule share its prepared evidence |
| Question — [schema](schemas/decision-question.schema.json) | `type` | Closed noul/choice/score discriminator; determines the corresponding criteria/answer shape |
| Question | `instructions` | What this question asks. String/object/array/null; author-controlled JSON labels inside objects have no engine behavior |
| Question | `criteria` | Meaning of choice options, ordered score levels or paired noul descriptions. Required for choice/score; absent or complete true/false pair for noul |

`RuleSet.instruction_prefix` and `question.instructions` serve different scopes. The prefix exists only for authoring reuse. With a null prefix the question value is forwarded unchanged; with a prefix, both values survive under separate `prefix` and `question` labels. If reuse is not useful, use null. No third instruction layer exists on a JSONL file or individual rule.

RuleSet composition groups its referenced definitions, shared instruction prefix and named native results, as defined in [Specification C6](specification.md#c6--native-judgments-and-result-composition). The schema fully expresses that metadata/prefix/file-reference contract. There is no placeholder `composition` accepting arbitrary JSON, no hidden all/any/average operation and no required overall-verdict choice. Consumer code can deliberately combine native results without adding an interpretation layer to every rule.

## Evidence declarations and invocation input

| Owner / schema | Fields | Purpose / invariant |
| --- | --- | --- |
| Skill binding — [schema](schemas/skill-binding.schema.json) | `repository_id`, `skill_path`, `rule_set_paths` | Associates caller-selected RuleSets with one subject document. Subject contents are captured at runtime; they are not copied into definitions |
| Evidence — [schema](schemas/evidence-selector.schema.json) | `kind`, `repository_id`, query-specific path/text/pattern fields | A primitive selector declares what source to select and in which supplied repository. Repository is stated once; there is no second wrapper override |
| Evidence | `selector_id`, `requirement`, `selector`, `max_matches`, `max_bytes` | Attributes each context requirement and its bounded capture. Required failure blocks inference; optional omission remains visible |
| Evidence | `SourceUnitQuery`, `RelationshipQuery`, `ChunkSettings` payloads | Closed syntax-unit/relationship/chunk declarations. Language/kind compatibility, AST catalog membership and resource bounds need semantic admission in addition to JSON Schema |
| Evidence invocation | Repository roots, input paths, `change_scope`, file/discovery limits | Selects which captured material is available. A RuleSet does not secretly override input scope or retrieve outside it |
| Decisions execution | Backend/model, attempt settings, context budget and cancellation | Runtime configuration supplied by the caller, separate from reusable authored questions |
| Private transport | Credentials | Kept outside rule files, RuleSets, ordinary invocations, model state and diagnostics |

`RepositorySelector` references the same evidence-owned primitive selector schema; it is not another schema with its own duplicate repository/path fields. `TargetId` and source selectors are likewise imported by the rule schema, not redefined by rules.

## Returned data

| Owner | Shape / fields | Purpose / invariant |
| --- | --- | --- |
| Decisions — [answer schema](schemas/decision-answer.schema.json) | Noul `yes_probability`; choice `selected_label`, `confidence`, `probabilities`; score `score`, `confidence`, `legend`, `probabilities` | One canonical typed answer definition. Native values/distributions are preserved; no mandatory verdict remapping |
| Decisions | Receipt with resolved model, token usage and independently absent reported cost | Records observations from one actual run. Missing cost is not zero; configured and observed model serve different purposes |
| Decisions | Completed / invalid request / provider failure / invalid response / cancelled | Closed execution outcomes. A failure never becomes an answer or finding |
| Rules | Occurrence, target/source attribution and authored-question-to-native-answer references | Restores source identity after request-local correlation names. References target one canonical execution outcome rather than duplicating answers/receipts |
| Boundary owner | Safe diagnostic origin/code/location and repair advice | Useful machine/agent repair information; no raw errors, response bodies, secret values or arbitrary received fields |
| Rules model state | `related_context`: `none` or `prepared` with required units/relationships/chunks/coverage | Keeps conditional data together as one tagged payload; ordinary target/context fields remain required |
| Evidence/rules | Coverage, omissions and work measurements | Makes bounded search, incomplete evaluation and actual work visible; not a model judgment or calibrated probability |

Public answer field names are the Tools contract: the adapter maps Jev wire keys into these typed fields without changing probabilities, labels, scores or distributions. This is one answer schema, not another verdict or policy layer.

Request-relative validation still checks exact question names/kinds, choice labels/probability keys and score indices/ranges. JSON Schema alone cannot enforce those relationships to a different request document. Strict JSON decoding separately refuses duplicate keys and non-JSON numbers; constructors enforce finite numbers, IDs, paths and cross-field invariants. Deserialization is not semantic admission.

## Excluded redundant or misplaced fields

- No JSONL header, record tag or `definition` wrapper: each line is directly a rule definition.
- No extra pack manifest: a RuleSet selects rule files; a skill binding associates it with a subject.
- No mandatory `policy` mapping an already desired choice label to another label, and no mandatory duplicated feedback map.
- No fixed `guidance`, `purpose`, `inspect`, `focus` or `examples` schema inside question content. Authors may use descriptive keys as ordinary structured data.
- No authored `state`, source bytes, model, provider credentials, retries or usage fields inside a rule definition.
- No implicit trigger/event/lifecycle/watch fields. The caller selects a RuleSet and starts the invocation.
- No local two-level minimum imposed on the native score rubric: the schema requires a nonempty list and retains actual request-relative indices.

The JSON Schemas describe the proposed wire shapes, not a running Rust implementation. Production schemas must be derived from their owning Rust types and checked against this reviewed contract. Schema/example checks do not establish capture, execution, cancellation, hosted inference or semantic accuracy.
