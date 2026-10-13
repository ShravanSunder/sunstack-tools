# Rust rule engine: observable contracts

This document realizes the authorized needs in [Requirements](requirements.md). [Program Design](program-design.md) gives entities and obligations Rust owners, data shapes and proof seams. The first concrete journey is a JSONL rule pack evaluating skill Markdown; the same judgment contract serves native Rust consumers.

## What callers see

![External consumers and the Rust engine boundary](assets/specification-context.png)

[Diagram source](assets/specification-context.mmd)

The engine has no authority to approve tools, accept a skill, merge changes or declare software verified. Production plugin/router integrations and alternative providers are outside initial implementation.

## Entities and identities

IDs E1–E8 retain the decision-domain meanings. E9–E16 make the evidence/lint journey explicit. They are invocation/content identities, not a persisted identity service.

| Entity | Same-instance rule and relationships | Invariants and observable states | Basis |
| --- | --- | --- | --- |
| E1 Named question | Its name within one E4 request; request has 1..n questions | One noul/choice/score, with structured instructions/criteria. Supplied → admitted or refused | U3–U5 |
| E2 Shared state | One evidence value associated with an E4 request; every question reads that value | JSON-compatible text/object/array/null; no silent truncation. Supplied → admitted or refused | U4, U5 |
| E3 Decision backend | One prepared provider/model connection; runs select exactly one connection | Only declared supported semantics can execute. Prepared or preparation refused; actual model is observed separately | U2, U12 |
| E4 Structured request | One nonempty named-question set and shared state; repeated contents do not make two E5 runs the same | Independent questions only; consumer schedules answer-dependent follow-up. Supplied → admitted or refused | U3–U5 |
| E5 Decision run | One invocation selecting E3 and proposing E4; retries remain within it | One terminal outcome: completed, invalid request, provider failure, invalid response or cancelled | U1, U3, U14 |
| E6 Answer | E5 run plus E1 name; a completed run has exactly one per question | Requested kind/names/labels/rubric match; probabilities and fractional scores retained. Untrusted → admitted or refused | U3, U4 |
| E7 Receipt | One completed E5 run's execution metadata | Resolved model and native usage; reported cost if available, absence is not zero. Untrusted → admitted or refused | U6 |
| E8 Finding | Consumer policy evaluation for one rule/target occurrence; belongs to one E16 invocation | pass/fail/review, category or rubric feedback is policy-owned; execution failure is never a finding | U7, U9 |
| E9 Rule definition | Nonempty rule ID within E10 content; changing the definition changes its effective version | A separate target ID locates E12. One named primary question and bounded policy in the initial JSONL contract. Supplied → admitted or refused | U15 |
| E10 Rule pack | Ordered JSONL content and its content identity; contains 1..n E9 with unique rule IDs | Malformed lines, duplicate IDs or policy/question mismatch refuse the pack before inference | U15, U3 |
| E11 Source snapshot | Selected repository/file content, revision identities and captured content identities for one invocation | Diff and source spans refer to these bytes, never later live reads. Captured or unavailable/changed-during-capture | U16 |
| E12 Target paragraph | Snapshot plus source document plus target ID; multiple rules may reference it | Exactly one matching Markdown paragraph; duplicate/missing ID refuses that target. Resolved, missing, ambiguous or deleted | U15 |
| E13 Evidence bundle | Selected target/context occurrences from one snapshot, with their provenance and completeness | Includes source ranges, facts and explicit uncertainty; no invented context. Ready or incomplete/refused | U16, U17 |
| E14 Diagnostic | One safely described issue at a rule/source/request/response/provider boundary | Origin, closed code, safe path/location and repair guidance; never input/body/credential passthrough | U14 |
| E15 Selection record | One E9 occurrence in one E16 invocation | Exactly one of selected, not applicable, outside requested change scope or selection failed, with reason | U16 |
| E16 Lint invocation | One caller-requested rule pack, source scope and configured backend run | Contains one selection record per admitted rule occurrence and selected-rule outcomes. Complete for requested scope, incomplete or cancelled | U1, U15, U16 |

![Rule pack, source snapshot, targets, evidence and outcomes](assets/specification-entities.png)

[Diagram source](assets/specification-entities.mmd)

Target identity never implies symbol/type identity. A syntax match, a relationship hypothesis and a model judgment are different kinds of evidence.

## Obligations and proof coverage

Each P/O cell names a concrete problem and desired observation, rather than adding a new outcome. R identifiers are local to this new design set.

| R | Observable obligation, including negative case | U / E | Problem → outcome | Contract / proof |
| --- | --- | --- | --- | --- |
| R1 | Rust callers MUST compose the evidence, decision and rule libraries without a JS runtime. Evidence preparation MUST be usable without a decision-provider dependency, and decision execution without repository/AST dependencies. Program callers MUST reach the same behavior and structured outcomes | U1,U8,U18,U19 / E5,E13,E16 | P1 duplicated or entangled runtime → O1 independently reusable responsibilities | C1 / V1,V2 |
| R2 | Invalid JSONL, duplicate rule IDs, invalid questions or inconsistent policy MUST refuse the rule pack before any inference, with line-linked diagnostics | U3,U14,U15 / E9,E10,E14 | P2 bad data can infer → O2 preflight refusal | C2 / V3 |
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
| R13 | A negative judgment MAY yield a failing finding, but invalid input, unavailable evidence, provider failure, invalid response and cancellation MUST remain evaluation outcomes/diagnostics | U7,U14 / E5,E8,E14,E16 | P13 failure becomes pass → O13 policy/execution separation | C5,C6 / V8,V12 |
| R14 | Observed cancellation MUST stop new work/attempts and await owned in-flight task cleanup before returning terminal cancellation; it MUST not claim remote billing was reversed | U1,U3 / E5,E16 | P14 detached work → O14 owned settlement | C5 / V9,V13 |
| R15 | Exactly one controller MUST apply configured finite attempts/timeouts; retries and ambiguous accepted inference MUST be visible as execution behavior, never hidden judgments | U3,U6 / E3,E5,E7 | P15 retry multiplication → O15 bounded attempts | C5 / V9,V13 |
| R16 | Sunstack-defined policies MUST produce source-linked pass/fail/review, category or rubric feedback; independent eval expectations MUST not be generated by the engine | U5,U7,U9,U15 / E6,E8,E9,E16 | P16 duplicated meaning/circular eval → O16 shared policy with independent oracle | C6 / V12 |
| R17 | Structural facts, heuristic relationship/relevance labels and provider probabilities MUST retain distinct attribution; a heuristic score MUST not masquerade as calibrated probability | U4,U17 / E6,E13 | P17 approximate retrieval mistaken for truth → O17 readable uncertainty | C3,C4 / V6,V12 |
| R18 | The engine MUST expose local-work and provider-work measurements sufficient to evaluate filtering efficiency without claiming an unmeasured speed target | U6,U16 / E11,E16 | P18 “fast” unverifiable → O18 declared workload measurements | C7 / V14 |
| R19 | Credential material MUST be kept at the transport boundary; rule packs, source feedback, default logs and serialized outcomes MUST not contain it or raw provider bodies | U2,U10,U14 / E3,E11,E14 | P19 diagnostic data exposure → O19 bounded privacy | C5 / V10 |
| R20 | Initial delivery MUST remain in sunstack-tools and meet language-appropriate checks/proof, including independent crate consumption and acyclic responsibility boundaries; other repositories, real alternate providers, native addons and publication require separate scope | U9,U10,U11,U13,U18,U19 / none: delivery/platform constraint | P20 integration overreach → O20 bounded composable delivery | C1,C7 / V1,V2,V15 |

## C1 — Two entry surfaces, one implementation

The Rust libraries expose independently reusable evidence preparation, prepared-judgment execution and rule orchestration. Their contracts are defined by the domain that owns them; orchestration imports those types rather than recreating their schemas. A thin program accepts bounded structured invocations and returns the same data contracts. Neither initializes hidden model inference or requires a JS runtime. The program is an invocation process, not a required daemon. TS/native-addon/WASM packaging is optional later work.

Dynamic JSONL names/labels are validated against each actual request at runtime. Native callers get typed primitive/result variants; they do not receive a false claim of compile-time knowledge of file-loaded keys. If a TS convenience library is later admitted, literal name/choice inference needs proof through that wrapper's real declarations, not merely generated wire types.

## C2 — JSONL rules and Markdown targets

Each nonblank JSONL line describes one uniquely identified rule: target document and paragraph ID, applicability/evidence declarations, one native primary question, explicit policy and feedback. Pack-level preflight validates the entire file; blank lines are ignored, comments are not JSON, and malformed/duplicate entries refuse the pack atomically before inference. Line/column diagnostics identify the rule-definition repair point.

Rule IDs identify checks; paragraph IDs identify source targets. One paragraph can have several rules. The initial target convention is `U001: paragraph text` at the start of a Markdown paragraph, with an ID consisting of an ASCII letter followed by at least three decimal digits; lowercase such as `u001` is also valid. IDs are scoped to a document, matched case-sensitively, and do not include the colon. Paragraph text includes wrapped continuation lines until the Markdown paragraph ends. A list item's paragraph may qualify; code fences, headings, inline code and quoted examples do not create target declarations. No implicit “first match wins” is allowed.

For a removed paragraph in change-scoped execution, the before snapshot can identify the rule occurrence, but the after target is deleted and cannot be judged as if present. Duplicate IDs are ambiguous source diagnostics. Unsupported Markdown/source language is explicit, never a fallback to line guessing.

This convention is the proposed narrow first format, rather than a promise to parse every historical skill style. A real required variant reopens this contract instead of adding silent heuristics.

## C3 — Scope, filtering and model-ready evidence

The caller selects full declared scope or change-scoped evaluation and supplies a concrete comparison. Full scope evaluates every applicable rule in that scope. Change scope considers rules whose targets or declared context dependencies changed, including rule-definition changes. Merely changing line numbers is not changing paragraph identity/content. If dependency or applicability analysis is incomplete, the engine conservatively selects the rule or returns selection failure; it never asserts irrelevance from missing evidence.

Repository comparison covers two resolved commits or a resolved base against captured working-tree content. Working-tree scope includes staged and unstaged content, and explicitly requested untracked sources; ignored-file inclusion follows caller scope. Added, modified, deleted and renamed paths are reported. Rename handling may be delete/add; no heuristic rename identity is required to preserve paragraph/content tracking.

Evidence includes target text, declared context, source/content identities, ranges and selector provenance. Text search finds text occurrences; structural search identifies syntax nodes/captures; neither proves type/symbol identity. Target IDs, source existence and mechanically checkable structure are established locally. Models receive contextual evidence rather than being asked to locate IDs that the engine can resolve.

Limits on files, bytes, candidates and model input are explicit invocation settings. Exceeding a limit yields incomplete evidence/selection diagnostics, not silent truncation, dropping rules or a fabricated zero result. Required evidence must be complete for each model call. No persistent index, automatic graph traversal, learned ranker or mandatory model reranking is introduced. A caller can add explicit dependency/context selectors; transitive compiler resolution is outside v1.

Each rule occurrence has a selection record: selected, not applicable, outside requested change scope, or selection failed, with a machine code and safe explanation. A report is complete only for its stated scope, never implicitly for the whole repository.

## C4 — Native decisions and adapters

Noul returns a finite yes probability in `[0,1]`. Choice returns a supplied label, concentration confidence and probabilities for exactly the supplied label set. Score returns a finite probability-weighted position from zero to the final rubric index, confidence and the actual index/legend/probability set. Fractions remain fractions; no automatic conversion to a quality percentage or boolean is applied by transport validation.

Instructions and descriptions accept strings, nested JSON objects/arrays or null. Noul true/false descriptions are optional; choice labels form a nonempty map; score has at least two ordered levels. Numbers nested in JSON are finite. Functions, cycles and unsupported native values are refused.

A request has multiple named independent questions over one shared state. Explicitly requested groups may share a call only with identical evidence and declared independence. Answer-dependent follow-up remains a caller's next invocation. No undocumented exact floating-point probability-sum or legend-text equality constraint is invented.

Jev/OpenRouter is the first actual backend. OpenAI Decisions and OpenRouter Alpha Decisions are future candidates; adapters must independently establish primitive, confidence, structured input and receipt semantics. OpenAI's predicate probability is a candidate noul mapping, but its answer-array and input envelopes are not the public contract. Missing fields cannot be fabricated to make an unsupported model look conforming.

Validate all untrusted provider data against the actual request before completion. Native model/token usage and optional reported USD cost are preserved. Malformed supplied receipt fields refuse completion. Unknown receipt metadata is excluded from the public result, without rejecting otherwise valid additive metadata. Unknown answer names or requested probability/index keys are errors.

## C5 — Diagnostics, attempts and cancellation

Diagnostics identify source (`rule-pack`, `source`, `selection`, `request`, `response`, `provider` or `execution`), a closed code, safe path/location and actionable guidance. Expected kind/range/allowed labels may be described using admitted declaration data; received values and raw bodies are excluded. Messages are constructed by the boundary owner, never copied from SDK/HTTP error text. Retryability is known yes/no or unknown, not an instruction to run another attempt automatically.

Decision execution ends as completed, invalid request, provider failure, invalid response or cancelled. Preparation failures and evidence failures have their own boundary diagnostics; neither creates a fake decision run. Request validation happens before inference. Valid pre-aborted requests cancel without inference; invalid requests report their validation errors. Abort observed before completed admission wins over successful response admission; later abort cannot mutate a returned result.

Attempt timeout and retry count are finite configuration values. The provider adapter owns one controller. Attempts stop on cancellation; failures retain class/status and safe retry guidance. Requests already accepted remotely may have cost or side effects even when their response is lost. There is no inference idempotency or refund promise.

Per-rule execution failures remain visible while independent rules may finish. Invalid pack/configuration stops the invocation before calls. Invocation cancellation stops further scheduling, waits for owned work, and reports completed versus cancelled/not-started rule occurrences distinctly.

## C6 — Rule policy and lint feedback

The first JSONL policy evaluates one primary answer: noul probability bands, choice label mapping, or score bands. Thresholds, pass/fail/review labels and repair messages are rule data supplied by Sunstack. Bands must cover the valid result domain without overlap/gaps; labels must exactly cover choice criteria. A category policy may return the category without asserting pass/fail. Score output retains raw value, rubric endpoints and distribution; normalized display is explicit consumer policy, never a claim that the skill is a percentage correct.

Tools provides the reusable interpreter; Sunstack authors its meaning. Native consumers can compose several answers in code through the lower-level judgment API. No arbitrary expression evaluator, TS rule loader or workflow language is implied by JSONL support.

Findings include rule ID, target ID, source location, declared policy disposition/category, raw judgment reference and rule-authored feedback. Applicability skips and failed evaluations are separate. Reports expose incomplete coverage and cannot aggregate failure, cancellation or missing target into a lint pass. A clean changed-scope report does not certify unselected full-scope rules.

## C7 — Efficiency, delivery and proof

Local evidence preparation must be usable without a provider call. Report files considered/read/parsed, bytes scanned/included, rules selected/skipped/failed, elapsed local/provider time and model request/attempt counts. These are measured facts, not predicted cost savings. Repository-wide rediscovery per rule is unnecessary work to challenge in profiling; a numeric latency SLA remains unset.

Proof is appropriate to the boundary:

| V | Required observation | Limit |
| --- | --- | --- |
| V1 | Independent native consumers use evidence without provider dependencies, decisions without Git/AST dependencies, and rules through composition; no JS runtime or hidden startup inference | Static source inspection is not execution |
| V2 | Real executable consumes structured input and emits typed outcomes; a process caller observes stdout/stderr/exit and cancellation behavior | Does not prove a native JS addon or production integration |
| V3 | Pack preflight: valid/malformed lines, duplicate IDs, policy/question mismatch, no inference on refused pack | Does not prove skill semantics |
| V4 | Real Markdown parsing: wrapped paragraph, list paragraph, example/fence exclusion, missing/duplicate/deleted IDs and source ranges | AST parsing is not semantic correctness |
| V5 | Real repository fixtures: commits, staged/unstaged/untracked inputs, additions/deletions/renames and content mutation during capture | Not every Git layout/platform is thereby supported |
| V6 | Actual text/AST-grep matches, range/capture integrity and false semantic-inference controls | Syntax discovery is not compiler-backed reference resolution |
| V7 | Selection/evidence controls: changed target, changed dependency/rule, unaffected rule, unknown applicability, missing context and explicit limit refusal | Coverage is relative to requested scope |
| V8 | Primitive and request-relative tests plus native static contract checks: names/kinds/labels/rubrics, nested JSON, malformed numbers and diagnostics | Does not establish model accuracy or future TS literal inference |
| V9 | Real Rust HTTP adapter under controlled transport: payload, bounded errors, retry, abort and receipt preservation | Controlled server is a hosted-service stand-in |
| V10 | Useful diagnostics retained; sentinel credentials, values, arbitrary paths/error text and bodies excluded | Does not authenticate source contents |
| V11 | Genuine Rust public API → Jev/OpenRouter mixed-primitives request, validated structured answers, model and usage | Credential-gated; skipped means unverified; alternate providers remain unproved |
| V12 | Same rule definitions/policies through lint and eval fixtures against independent expected findings; all failure-to-pass controls | Not production skill-plugin integration or semantic accuracy |
| V13 | Controlled scheduling/cancellation joins owned tasks and original requests; no new attempt after cancellation | Cannot undo remote inference/billing |
| V14 | Declared workload measurements of cold/warm local runs and request reduction, with selection correctness held fixed | No improvement percentage/SLA without data |
| V15 | Applicable Rust formatting/lint/type/test gates, independent crate consumption, acyclic package dependency checks and library/executable artifact integrity on selected platforms | Release/publication proof only if separately requested |

No V row has been executed by authoring this Specification.
