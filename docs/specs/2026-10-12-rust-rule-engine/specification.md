# Rust rule engine: observable contracts

This document realizes the authorized needs in [Requirements](requirements.md). [Program Design](program-design.md) gives entities and obligations Rust owners, data shapes and proof seams. The first concrete journey is an explicitly selected RuleSet evaluating skill Markdown; the same judgment contract serves native Rust consumers.

## What callers see

![External consumers and the Rust engine boundary](assets/specification-context.png)

[Diagram source](assets/specification-context.mmd)

The engine has no authority to approve tools, accept a skill, merge changes or declare software verified. Production plugin/router integrations and alternative providers are outside initial implementation.

## Entities and identities

IDs E1–E8 retain the decision-domain meanings. E9–E16 make the evidence/lint journey explicit; E17–E22 define connected change sourcing and bounded model context. E23 defines a physical JSONL rule file; E24 defines the named RuleSet that selects those files. They are invocation/content identities, not a persisted identity service.

| Entity | Same-instance rule and relationships | Invariants and observable states | Basis |
| --- | --- | --- | --- |
| E1 Named question | Its name within one E4 request; request has 1..n questions | One noul/choice/score, with structured instructions/criteria. Supplied → admitted or refused | U3–U5 |
| E2 Shared state | One evidence value associated with an E4 request; every question reads that value | Text or structured JSON object/array, including nested null values; no silent truncation. Supplied → admitted or refused | U4, U5 |
| E3 Decision backend | One prepared provider/model connection; runs select exactly one connection | Only declared supported semantics can execute. Prepared or preparation refused; actual model is observed separately | U2, U12 |
| E4 Structured request | One nonempty named-question set and shared state; repeated contents do not make two E5 runs the same | Independent questions only; consumer schedules answer-dependent follow-up. Supplied → admitted or refused | U3–U5 |
| E5 Decision run | One invocation selecting E3 and proposing E4; retries remain within it | One terminal outcome: completed, invalid request, provider failure, invalid response or cancelled | U1, U3, U14 |
| E6 Answer | E5 run plus E1 name; a completed run has exactly one per question | Requested kind/names/labels/rubric match; probabilities and fractional scores retained. Untrusted → admitted or refused | U3, U4 |
| E7 Receipt | One completed E5 run's execution metadata | Resolved model and native usage; reported cost if available, absence is not zero. Untrusted → admitted or refused | U6 |
| E8 Rule judgment | One E9 occurrence within a selected E24/bound E12 subject in E16 | References completed native answers and source location; consumer interpretations remain separate. Execution failure never becomes a judgment | U7,U9,U14 |
| E9 Rule definition | Nonempty rule ID within E24; effective content includes its own definition plus that RuleSet's instruction prefix | A separate target ID locates E12 in the binding's skill document. Nonempty named independent questions; no mandatory local policy. Supplied → admitted or refused | U3,U4,U15 |
| E10 Skill binding | One repository-qualified skill document plus ordered selected E24 source paths in E16 | One binding per document; duplicate selected RuleSet paths refused. The same rule in different RuleSets remains a distinct occurrence. Supplied → admitted or refused | U15,U3 |
| E11 Source snapshot | Captured content and resolved revisions for one invocation's E17 bindings; each file is identified by binding, revision/content and path | Diff and source spans refer to these bytes, never later live reads. Captured or unavailable/changed-during-capture; no cross-repo atomic-instant claim | U16,U21 |
| E12 Target paragraph | Snapshot plus source document plus target ID; multiple rules may reference it | Exactly one matching Markdown paragraph; duplicate/missing ID refuses that target. Resolved, missing, ambiguous or deleted | U15 |
| E13 Evidence bundle | Selected target/context occurrences from one snapshot, with their provenance and completeness | Includes source ranges, facts and explicit uncertainty; no invented context. Ready or incomplete/refused | U16, U17 |
| E14 Diagnostic | One safely described issue at a rule/source/request/response/provider boundary | Origin, closed code, safe path/location and repair guidance; never input/body/credential passthrough | U14 |
| E15 Selection record | One E9 occurrence in one E16 invocation | Exactly one of selected, not applicable, outside requested change scope or selection failed, with reason | U16 |
| E16 Lint invocation | One caller-requested ordered collection of skill/RuleSet and repository bindings, input-path scopes and backend configuration | One binding per repository-qualified skill document; each admitted RuleSet/rule occurrence has one selection record. Complete for requested scope, incomplete or cancelled | U1,U15,U16,U21 |
| E17 Repository binding | One caller key associated with one canonical worktree/root in the invocation; linked worktrees remain distinct even when sharing Git objects | Declared paths bound discovery. Supplied → admitted or refused; duplicate key/root and undeclared cross-root reads are refused. Canonicalizes caller “repo” without equating two worktrees | U20,U21 |
| E18 Change selection | One E17 binding plus a declared scope and resolved interval/revisions within E11; a union retains each member's provenance | Dirty layers, commit events and net comparisons are distinguishable. Requested → resolved, approximate or unavailable; time metadata is not local-arrival/edit history | U20 |
| E19 Source unit | One E11 source/revision plus syntactic kind and byte range; before/after units are distinct and linked only with evidence | Function, method, class or Markdown paragraph/section candidate. Located or unavailable; source syntax does not imply compiler symbol identity | U22 |
| E20 Relationship candidate | One E11 plus source unit/call site, relation kind and an identified endpoint or unresolved target hint; multiple evidence items may support one candidate | Candidate caller/callee/import relationship with facts and unresolved conditions. Discovered, contradicted or unresolved; absent target identity is never invented; never a runtime stack or calibrated probability | U17,U21,U22 |
| E21 Semantic chunk | One requested evidence plan plus its E19/E20 source membership, selected source sides and content identities | Related excerpts, attribution and omissions retained. Planned → ready or incomplete; splitting cannot silently erase a rule-required relationship | U22,U23 |
| E22 Context budget | One proposed E4 request plus selected backend/model accounting profile and finite caller limits | Includes questions, state and serialization overhead. Assessed within configured budget, exceeded or unverifiable; estimated accounting stays labeled | U12,U23 |
| E23 Rule file | One normalized JSONL source path and captured contents; may be referenced by several E24 | One bare rule definition per nonblank physical line; no header or record wrapper. At least one rule, with source locations. Supplied → admitted or refused | U3,U4,U15 |
| E24 RuleSet | One normalized selected RuleSet JSON source path; captured contents determine its version, while name/description are descriptive metadata | Selects ordered E23 paths and a shared instruction prefix. Rule IDs unique across its files; instruction scope is this RuleSet only. Supplied → admitted or refused | U3,U5,U15 |

![RuleSet, source snapshot, targets, evidence and outcomes](assets/specification-entities.png)

[Diagram source](assets/specification-entities.mmd)

Target identity never implies symbol/type identity. A syntax match, a relationship hypothesis and a model judgment are different kinds of evidence.

## Obligations and proof coverage

Each P/O cell names a concrete problem and desired observation, rather than adding a new outcome. R identifiers are local to this new design set.

| R | Observable obligation, including negative case | U / E | Problem → outcome | Contract / proof |
| --- | --- | --- | --- | --- |
| R1 | Rust callers MUST compose the evidence, decision and rule libraries without a JS runtime. Evidence preparation MUST be usable without a decision-provider dependency, and decision execution without repository/AST dependencies. Program callers MUST reach the same behavior and structured outcomes | U1,U8,U18,U19 / E5,E13,E16 | P1 duplicated or entangled runtime → O1 independently reusable responsibilities | C1 / V1,V2 |
| R2 | Invalid RuleSet JSON/version, malformed JSONL, duplicate rule IDs within a RuleSet or invalid named questions MUST refuse admission before inference, with source-linked diagnostics | U3,U14,U15 / E9,E10,E14,E23,E24 | P2 bad data can infer → O2 preflight refusal | C2 / V3 |
| R3 | Paragraph IDs MUST resolve to exactly one eligible paragraph in the specified document; missing, duplicate or deleted targets MUST not become passes | U14,U15 / E11,E12,E14 | P3 wrong paragraph judged → O3 source-linked target resolution | C2 / V4 |
| R4 | Requested Git comparison and source evidence MUST refer to resolved/captured content; staged, unstaged, untracked, rename and deletion behavior MUST be explicit | U16 / E11,E13,E16 | P4 mutable/diff ambiguity → O4 traceable comparison | C3 / V5 |
| R5 | Text/structural selectors MUST return source-located facts and candidates without claiming compiler-backed symbol/type identity | U16,U17 / E11,E13 | P5 raw matches mislead → O5 attributed local evidence | C3 / V6 |
| R6 | Every admitted rule occurrence MUST expose a selection disposition and reason; uncertain applicability MUST not be silently treated as not applicable | U14,U16 / E9,E15,E16 | P6 filtering hides checks → O6 visible coverage | C3 / V7 |
| R7 | Required target and declared context MUST be present before inference; missing, over-limit or unresolvable evidence MUST produce diagnostics rather than silent trimming | U5,U14,U16 / E11,E13,E14 | P7 incomplete input false-flags → O7 declared context honored | C3 / V7 |
| R8 | The engine MUST preserve mixed named primitives and structured JSON instructions/criteria over shared state; explicit batching MUST not drop or regroup evidence silently | U3,U4,U5 / E1,E2,E4,E5 | P8 flattening loses meaning → O8 native structured requests | C4 / V8,V9 |
| R9 | A completed run MUST contain exactly requested answer names/kinds/labels/rubric ranges and valid receipts; mismatches MUST be invalid response | U3,U4 / E1,E5,E6,E7 | P9 successful HTTP mistaken for valid judgment → O9 correlated validation | C4 / V8,V9 |
| R10 | Diagnostics MUST retain origin, safe path/location, closed code, bounded actionable text and observed retry advice; raw errors/values MUST not escape | U2,U3,U14 / E14 | P10 opaque/leaky failure → O10 useful safe repair feedback | C5 / V10 |
| R11 | Backend selection MUST be independent of rule meaning; initial execution supports Jev only, with unsupported semantics refused instead of fabricated probabilities | U4,U12 / E1,E3,E5 | P11 provider coupling → O11 replaceable semantic boundary | C4 / V9,V11 |
| R12 | Completed execution MUST preserve observed model, native token usage and supplied billing cost; absence MUST remain unavailable and savings MUST not be asserted without measurement | U6 / E5,E7 | P12 count mistaken for economics → O12 measurable execution | C4 / V9,V11 |
| R13 | An admitted native answer MAY be unfavorable, but invalid input, unavailable evidence, provider failure, invalid response and cancellation MUST remain distinct execution/evaluation outcomes, never completed rule judgments | U7,U14 / E5,E8,E14,E16 | P13 failure becomes pass → O13 answer/execution separation | C5,C6 / V8,V12 |
| R14 | Observed cancellation MUST stop new work/attempts and await owned in-flight task cleanup before returning terminal cancellation; it MUST not claim remote billing was reversed | U1,U3 / E5,E16 | P14 detached work → O14 owned settlement | C5 / V9,V13 |
| R15 | Exactly one controller MUST apply configured finite attempts/timeouts; retries and ambiguous accepted inference MUST be visible as execution behavior, never hidden judgments | U3,U6 / E3,E5,E7 | P15 retry multiplication → O15 bounded attempts | C5 / V9,V13 |
| R16 | Rule execution MUST preserve source-linked native answers so lint and eval consumers can reuse the same questions/criteria; independent expected eval results MUST not be generated by the engine | U5,U7,U9,U15 / E6,E8,E9,E16,E24 | P16 duplicated meaning/circular eval → O16 reusable questions and independently judged outcomes | C6 / V12 |
| R17 | Structural facts, heuristic relationship/relevance labels and provider probabilities MUST retain distinct attribution; a heuristic score MUST not masquerade as calibrated probability | U4,U17 / E6,E13 | P17 approximate retrieval mistaken for truth → O17 readable uncertainty | C3,C4 / V6,V12 |
| R18 | The engine MUST expose local-work and provider-work measurements sufficient to evaluate filtering efficiency without claiming an unmeasured speed target | U6,U16 / E11,E16 | P18 “fast” unverifiable → O18 declared workload measurements | C7 / V14 |
| R19 | Credential material MUST be kept at the transport boundary; RuleSets and rule files, source feedback, default logs and serialized outcomes MUST not contain it or raw provider bodies | U2,U10,U14 / E3,E11,E14 | P19 diagnostic data exposure → O19 bounded privacy | C5 / V10 |
| R20 | Initial delivery MUST remain in sunstack-tools and meet language-appropriate checks/proof, including independent crate consumption and acyclic responsibility boundaries; other repositories, real alternate providers, native addons and publication require separate scope | U9,U10,U11,U13,U18,U19 / none: delivery/platform constraint | P20 integration overreach → O20 bounded composable delivery | C1,C7 / V1,V2,V15 |
| R21 | Change selection MUST expose dirty layers, selected commits/comparisons, resolved time bounds and member provenance; timestamp-based recent dirty selection MUST be labeled approximate and unknown recency MUST be visible | U16,U20 / E11,E17,E18 | P21 time filter misstates history → O21 inspectable selected changes | C3,C8 / V16 |
| R22 | Connected analysis MUST remain inside supplied repository/path scopes and retain repository/worktree/revision attribution for every source and relationship; one unavailable repository MUST remain visible in overall coverage | U16,U21 / E11,E13,E17,E20 | P22 cross-repo confusion → O22 attributed connected evidence | C3,C8 / V17 |
| R23 | Changed source MUST seed bounded syntax-unit and candidate relationship discovery, with limits, contradictions, unresolved links and coverage exposed; no exhaustive call-stack/symbol claim is permitted | U17,U22 / E18,E19,E20 | P23 diff loses related behavior → O23 useful bounded relationships | C8 / V17,V18 |
| R24 | Semantic chunks MUST preserve source sides, membership and declared rule-required relationships; optional omissions MUST be explicit and chunk judgments MUST not be combined by an undeclared policy | U5,U9,U22,U23 / E9,E13,E19,E20,E21 | P24 chunking breaks rule meaning → O24 coherent rule evidence | C8 / V18 |
| R25 | Request budget assessment MUST include the serialized questions, instructions, state and adapter overhead; unsupported or exceeded accounting MUST be explicit, and required context that cannot fit MUST not yield a complete evaluation/pass | U12,U14,U23 / E3,E4,E13,E21,E22 | P25 model limit silently loses context → O25 bounded visible evaluation | C4,C8 / V19 |

## C1 — Two entry surfaces, one implementation

The Rust libraries expose independently reusable evidence preparation, prepared-judgment execution and rule orchestration. Their contracts are defined by the domain that owns them; orchestration imports those types rather than recreating their schemas. A thin program accepts bounded structured invocations and returns the same data contracts. Neither initializes hidden model inference or requires a JS runtime. The program is an invocation process, not a required daemon. TS/native-addon/WASM packaging is optional later work.

Dynamic JSONL names/labels are validated against each actual request at runtime. Native callers get typed primitive/result variants; they do not receive a false claim of compile-time knowledge of file-loaded keys. If a TS convenience library is later admitted, literal name/choice inference needs proof through that wrapper's real declarations, not merely generated wire types.

## C2 — RuleSets, JSONL rules and Markdown targets

![RuleSet and JSONL admission, with refusal before inference](assets/rule-file-admission.png)

[Diagram source](assets/rule-file-admission.mmd)

The caller supplies ordered skill bindings. Each identifies one repository, one skill document and nonempty ordered RuleSet JSON paths. A RuleSet has a name, description, shared instruction prefix and nonempty ordered JSONL rule paths. Name/description are metadata for caller selection, not model instructions or automatic execution conditions. The caller initiates every invocation. No trigger, event subscription, watcher, pack manifest or persistent scheduling state is part of this contract.

Each nonblank JSONL line is one bare rule definition: rule ID, target paragraph ID, applicability, evidence declarations and nonempty named questions. There is no file header, record tag, definition wrapper or shared instruction layer in JSONL. One rule may contain multiple independent questions; all read the same prepared evidence. JSONL files can be reused across RuleSets. Duplicate IDs across the files of one RuleSet refuse admission; the same ID in different selected RuleSets is a distinct occurrence.

The RuleSet instruction prefix applies to every question in its referenced files, never to another RuleSet. A null prefix adds nothing. A supplied prefix and each question's own instructions must both survive as separately labeled structured values, without merging keys, flattening data or substituting text. Criteria remain unchanged, and captured evidence is supplied separately as state. The final serialized content, including repeated prefixes, counts toward model budgets. Changing the prefix changes every contained rule's effective question content; name/description alone do not change the model request.

RuleSet paths resolve from the invocation working directory. Referenced JSONL paths resolve from their RuleSet file's directory. Skill paths resolve within the explicitly bound repository. Reading a RuleSet authorizes its explicitly named rule files, not a directory scan or access to evidence outside caller scope. All selected RuleSets and referenced JSONL files are admitted before inference. Bad JSON, unsupported version, duplicate IDs/keys, empty collections, unknown contract fields or invalid questions refuse admission. JSON locations include file, line/column and a bounded safe containing-field path. Arbitrary keys inside structured instructions/criteria remain data. [Program Design](program-design.md#rulesets-and-rule-definitions) and the [schema inventory](schema-inventory.md) distinguish the domains and exact shapes.

Rule IDs identify checks; paragraph IDs identify source targets. One paragraph can have several rules. The initial target convention is `U001: paragraph text` at the start of a Markdown paragraph, with an ID consisting of an ASCII letter followed by at least three decimal digits; lowercase such as `u001` is also valid. IDs are scoped to a document, matched case-sensitively, and do not include the colon. Paragraph text includes wrapped continuation lines until the Markdown paragraph ends. A list item's paragraph may qualify; code fences, headings, inline code and quoted examples do not create target declarations. No implicit “first match wins” is allowed.

For a removed paragraph in change-scoped execution, the before snapshot can identify the rule occurrence, but the after target is deleted and cannot be judged as if present. Duplicate IDs are ambiguous source diagnostics. Unsupported Markdown/source language is explicit, never a fallback to line guessing.

This convention is the proposed narrow first format, rather than a promise to parse every historical skill style. A real required variant reopens this contract instead of adding silent heuristics.

## C3 — Scope, filtering and model-ready evidence

The caller declares repository-qualified input paths and selects full or change-scoped evaluation. Full scope needs no Git repository or comparison and evaluates applicable rules in that scope. Change scope resolves the declared comparison or change-event selection (C8), considering target, context, applicability and rule-definition changes on available before/after sides. Merely changing line numbers is not changing paragraph identity/content. Missing baseline or incomplete dependency analysis conservatively selects; unresolved current applicability produces selection failure. Missing evidence never proves irrelevance.

Repository comparison covers two resolved commits or a resolved base against captured working-tree content. Working-tree scope includes staged and unstaged content, and explicitly requested untracked sources; ignored-file inclusion follows caller scope. Added, modified, deleted and renamed paths are reported. Rename handling may be delete/add; no heuristic rename identity is required to preserve paragraph/content tracking.

Evidence includes target text, declared context, source/content identities, ranges and selector provenance. The model-facing state has a defined target, attributed context entries and explicit optional-context omissions; rule instructions can rely on those fields. Text search finds text occurrences; structural search identifies syntax nodes/captures; neither proves type/symbol identity. Target IDs, source existence and mechanically checkable structure are established locally. Models receive contextual evidence rather than being asked to locate IDs that the engine can resolve.

Limits on files, bytes, candidates and model input are explicit invocation settings. Exceeding a limit yields incomplete evidence/selection diagnostics, not silent truncation, dropping rules or a fabricated zero result. Required evidence must be complete for each model call. Explicit bounded relationship discovery and semantic chunking are covered by C8. No persistent index, learned ranker or mandatory model reranking is introduced; transitive compiler resolution remains outside v1.

Each rule occurrence has a selection record: selected, not applicable, outside requested change scope, or selection failed, with a machine code and safe explanation. A report is complete only for its stated scope, never implicitly for the whole repository.

## C4 — Native decisions and adapters

Noul returns a finite yes probability in `[0,1]`. Choice returns a supplied label, concentration confidence and probabilities for exactly the supplied label set. Score returns a finite probability-weighted position from zero to the final rubric index, confidence and the actual index/legend/probability set. Fractions remain fractions; no automatic conversion to a quality percentage or boolean is applied by transport validation.

Instructions for every primitive, choice option descriptions, score level descriptions and both noul descriptions accept the documented EntryType roots: string, object, array or null. Objects/arrays may contain arbitrary recursive JSON values, including finite numbers, booleans and null. Keys inside this content are author-chosen labels; no fixed task/inspect/examples vocabulary or executable path language is imposed. JSON null is literal content, distinct from an omitted field. Shared state separately retains its text/object/array root contract with nested null permitted. Noul criteria may be omitted; when supplied both true and false descriptions are required, including when a description is null. Choice labels form a nonempty map. Score criteria are a nonempty ordered rubric; the contract adds no two-level minimum beyond the published provider shape. Unsupported roots and native values are refused before inference; adapters never flatten or manufacture meaning to gain acceptance. [TypeSafe structured primitives](https://docs.typesafe.ai/primitives/advanced)

A request has multiple named independent questions over one shared state. Explicitly requested groups may share a call only with identical evidence and declared independence. Answer-dependent follow-up remains a caller's next invocation. No undocumented exact floating-point probability-sum or legend-text equality constraint is invented.

Jev/OpenRouter is the first actual backend. OpenAI Decisions and OpenRouter Alpha Decisions are future candidates; adapters must independently establish primitive, confidence, structured input and receipt semantics. OpenAI's predicate probability is a candidate noul mapping, but its answer-array and input envelopes are not the public contract. Missing fields cannot be fabricated to make an unsupported model look conforming.

Validate all untrusted provider data against the actual request before completion. Native model/token usage and optional reported USD cost are preserved. Malformed supplied receipt fields refuse completion. Unknown receipt metadata is excluded from the public result, without rejecting otherwise valid additive metadata. Unknown answer names or requested probability/index keys are errors.

## C5 — Diagnostics, attempts and cancellation

Diagnostics identify source (`rule_set`, `rule_file`, `source`, `selection`, `request`, `response`, `provider`, `execution`, `invocation` or `protocol`), a closed code, safe path/location and actionable guidance. Expected kind/range/allowed labels may be described using admitted declaration data; received values and raw bodies are excluded. Messages are constructed by the boundary owner, never copied from SDK/HTTP error text. Retryability is known yes/no or unknown, not an instruction to run another attempt automatically.

Decision execution ends as completed, invalid request, provider failure, invalid response or cancelled. Preparation failures and evidence failures have their own boundary diagnostics; neither creates a fake decision run. Request validation happens before inference. Valid pre-aborted requests cancel without inference; invalid requests report their validation errors. Abort observed before completed admission wins over successful response admission; later abort cannot mutate a returned result.

Attempt timeout and retry count are finite configuration values. The provider adapter owns one controller. Attempts stop on cancellation; failures retain class/status and safe retry guidance. Each attempt records whether it was not dispatched, produced an admitted completion or ended with an unknown remote outcome. A failed, malformed or cancelled local result cannot assert that inference never happened. Requests already accepted remotely may have cost even when their response is lost. There is no inference idempotency or refund promise.

Per-rule execution failures remain visible while independent rules may finish. Invalid RuleSet/configuration stops the invocation before calls. Invocation cancellation stops further scheduling, waits for owned work, and reports completed versus cancelled/not-started rule occurrences distinctly.

## C6 — Native judgments and result composition

Every completed rule exposes all its named native answers, correlated to that rule occurrence and source subject. Choice labels can directly be pass/fail/review or domain categories. The model supplies probabilities, selected labels and rubric positions; transport validation preserves these semantics rather than inventing another verdict. There is no mandatory per-rule policy, feedback map, threshold conversion or prescribed prompt-content key vocabulary.

Consumers may explicitly apply their own probability/score thresholds and repair text, while reusing the same question definitions in lint and eval paths. Those decisions must not be mistaken for native provider outputs. Rule execution failures, missing evidence, invalid responses and cancellation stay separate; none becomes an admitted answer or a clean pass. Report completeness describes whether the requested questions were evaluated, not whether their answers are favorable.

RuleSet composition means selecting its referenced rule definitions, applying shared instructions and returning their named results together. Individual answers cannot be discarded or silently reduced to an overall verdict. The RuleSet schema is the complete contract for that grouping: name/description, prefix and referenced rule paths, with its format version. It has no all/any/average operator, arbitrary expression evaluator or workflow DAG. Consumers can deliberately compose results in their own code. A future requirement for an overall result would need its own explicit contract; it is not a missing field or admission gate for this RuleSet format.

## C7 — Efficiency, delivery and proof

Local evidence preparation must be usable without a provider call. Report files considered/read/parsed, bytes scanned/included, rules selected/skipped/failed, elapsed local/provider time and model request/attempt counts. These are measured facts, not predicted cost savings. Repository-wide rediscovery per rule is unnecessary work to challenge in profiling; a numeric latency SLA remains unset.

Proof is appropriate to the boundary:

| V | Required observation | Limit |
| --- | --- | --- |
| V1 | Independent native consumers use evidence without provider dependencies, decisions without Git/AST dependencies, and rules through composition; no JS runtime or hidden startup inference | Static source inspection is not execution |
| V2 | Real executable consumes structured input and emits typed outcomes; a process caller observes stdout/stderr/exit and cancellation behavior | Does not prove a native JS addon or production integration |
| V3 | RuleSet JSON/version and JSONL preflight: strict fields/keys, bare rule lines, plural question maps, duplicate bindings/paths/IDs within a RuleSet, reused IDs in different RuleSets and zero inference on refusal | Does not prove skill semantics |
| V4 | Real Markdown parsing: wrapped paragraph, list paragraph, example/fence exclusion, missing/duplicate/deleted IDs and source ranges | AST parsing is not semantic correctness |
| V5 | Real repository fixtures: commits, staged/unstaged/untracked inputs, additions/deletions/renames and content mutation during capture | Not every Git layout/platform is thereby supported |
| V6 | Actual text/AST-grep matches, range/capture integrity and false semantic-inference controls | Syntax discovery is not compiler-backed reference resolution |
| V7 | Selection/evidence controls: both-side context/applicability changes, changed target/rule/RuleSet prefix, unchanged neighboring RuleSet, absent/external baseline and required/optional limits | Coverage is relative to requested scope |
| V8 | Primitive and request-relative tests plus native static contract checks: names/kinds/labels/rubrics, rich JSON and null in every supported instruction/description position, rejected numeric/boolean entry roots, partial noul criteria, malformed numbers and diagnostics | Does not establish model accuracy or future TS literal inference |
| V9 | Real Rust HTTP adapter under controlled transport: preserved prefix/question structure, null-prefix passthrough, no cross-RuleSet leakage, named-question correlation, bounded errors/retry/abort and receipts | Controlled server is a hosted-service stand-in |
| V10 | Useful status-specific diagnostics, unknown-field locations and remote uncertainty retained; sentinel credentials, values, arbitrary provider paths/error text and bodies excluded | Does not authenticate source contents |
| V11 | Genuine Rust public API → Jev/OpenRouter mixed-primitives request, validated structured answers, model and usage | Credential-gated; skipped means unverified; alternate providers remain unproved |
| V12 | Same RuleSet/rule definitions in lint and eval fixtures, source-linked native judgments and independently supplied expectations; all failure-to-pass controls | Not production skill-plugin integration or semantic accuracy |
| V13 | Controlled scheduling/cancellation joins owned tasks and original requests; no new attempt after cancellation | Cannot undo remote inference/billing |
| V14 | Declared workload measurements of cold/warm local runs and request reduction, with selection correctness held fixed | No improvement percentage/SLA without data |
| V15 | Applicable Rust formatting/lint/type/test gates, independent crate consumption, acyclic package dependency checks and library/executable artifact integrity on selected platforms | Release/publication proof only if separately requested |
| V16 | Real Git/filesystem cases for dirty layers, explicit commits, root/merge commits, time windows/unions, nonmonotonic commit dates, recent files with older hunks, deleted/unknown timestamps and reverted net content | Approximate dirty recency is not exact edit history |
| V17 | Connected supplied repositories/worktrees: unchanged consumer caller, import alias/shadow contradiction, unresolved package/re-export, duplicate identity, outside-scope refusal, cycles, partial source failure and stable source attribution | Candidate relationships do not establish exhaustive compiler resolution |
| V18 | Real AST ranges mapped from before/after diff spans into coherent units/chunks, including deletions, giant units and caller/callee co-preservation; limits and omissions visible; independent expectations | Local semantic grouping is not proof of model accuracy |
| V19 | Actual serialized backend requests measured/admitted or refused under exact/estimated/unverifiable budget profiles, including question/escaping overhead, shared-state deduplication and required-context overflow; no call/pass on refusal | Tokenizer/hosted-limit agreement needs real backend evidence |

No V row has been executed by authoring this Specification.

## C8 — Connected changes, relationships and context budgets

![Caller-visible change selection, connected evidence and bounded judgments](assets/specification-change-journey.png)

[Diagram source](assets/specification-change-journey.mmd)

Each supplied repository binding has its own allowed input paths and change scope. Full scope remains available without Git. Change scopes include staged/unstaged/untracked layers, explicit commit sets with stated merge-parent policy, before/after revisions, reachable commit-time windows, approximate recent dirty files, and explicit unions. A relative interval is anchored once for the invocation and returned as UTC bounds; commit timestamps do not establish local arrival time. Dirty layers remain separately attributed even when their net content cancels. Historical event selection and endpoint net comparison are separate views. Missing/shallow history or traversal limits are coverage diagnostics, not proof of no changes.

Recent dirty selection intersects Git dirty candidates with file modification timestamps. It returns the current selected diff, including older hunks in a recently touched file. Deleted paths, unavailable timestamps, staging-only changes and clock skew retain unknown-recency attribution. The caller must explicitly choose whether unknown-recency dirty candidates are included with a diagnostic or excluded with a visible approximate scope omission. Omitted recency policy refuses admission; there is no engine default. No snapshot history or watcher is introduced.

Changed before/after spans seed source units. Unchanged callers/callees in any supplied binding may contribute context even when their files are outside the change seed set; they must still lie inside the declared discovery paths. Imports, aliases, declarations, tests and lexical scope provide attributed evidence. Dynamic dispatch, macros, shadowing and unresolved package/re-export paths are uncertainty, not asserted symbol identity. Traversal has finite depth, fan-out, file/node/byte and time limits; all cutoffs are recorded. No bound is claimed to find “all callers.”

The consumer declares the context needed by its rule. Chunks retain changed units, relevant caller/callee excerpts, required imports/signatures, source sides and relationship links; optional excerpts may be omitted only with reasons. Required context overflow refuses that evaluation. Multiple chunks may be judged independently only when the rule declares that independence and its composition policy. A whole-call-path rule cannot be implemented by arbitrary averaging of isolated chunk scores.

Backend-specific budget assessment applies to the complete serialized request before inference. Limits include configured input tokens, request bytes, question count and request count. A known provider context ceiling further constrains the caller limit. Token accounting states exact, estimated-with-margin or unavailable; estimated admission requires the caller's explicit allowance, and does not claim exact provider agreement. An unavailable counter/limit cannot manufacture a within-budget assessment. Oversized required units, unrepresentable backend semantics or budget failure produce useful diagnostics; they never silently trim input or yield a clean rule pass. OpenAI and OpenRouter envelopes may need different accounting/translation; hosted compatibility is independently proved, not inferred from an endpoint name.
