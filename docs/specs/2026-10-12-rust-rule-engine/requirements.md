# Rust rule engine: people, purpose and boundaries

The first customer is a Sunstack skill-creation or skill-update agent. It needs to check a Markdown skill against reusable rules, obtain pass/fail, classification or rubric feedback, and locate the paragraph to repair. The expensive model call should receive relevant, source-linked evidence selected by fast local analysis.

The second customer is a Rust application such as codex-router. It needs the same evidence and decision semantics without a Bun process inside its execution path. TypeScript applications may call the Rust program; they do not require a separate implementation of its behavior.

[Specification](specification.md) defines the observable contract. [Program Design](program-design.md) proposes its Rust realization. These three documents form a new design set; the [October 11 proposals](../2026-10-11-rule-runner/requirements.md) remain historical alternatives. No implementation, installed dependency or runtime integration exists in this repository at the design baseline `c3220fc`.

## The two customer journeys

| Customer | Job sequence | Current missing capability and evidence | Desired outcome |
| --- | --- | --- | --- |
| Skill creator/updater and rule author | Write/update skill → load JSONL rules → locate paragraph IDs → collect evidence → evaluate checks → inspect source-linked feedback → repair | Owner supplied the JSONL-to-paragraph use case; repository inspection finds no loader, evidence engine or runner. No measured failure rate or latency baseline exists | Cheap local filtering, precise model inputs, useful findings and a separate explanation when evaluation fails; U4, U7, U14–U17 |
| Rust or TypeScript developer | Supply an invocation → call the reusable engine → handle judgments, findings or diagnostics | Owner asks for Rust reuse and optional TS consumption; only earlier design documents exist | One implementation callable as a Rust library or program; U1, U3, U8, U12, U18 |

There is no new UI screen. Findings and diagnostics are data for an agent, terminal or host application. Integration into the skill-authoring plugin and codex-router remains separate work; this design does not claim either integration is working.

## Authorized needs

IDs U1–U14 retain the prior needs where applicable. Their revised language/platform constraints follow the owner's explicit Rust-first reset and “we have built nothing so we have no priors.” Priorities are unranked by the owner. New needs U15–U18 capture the concrete evidence and consumption use case, rather than treating it as an SDK convenience.

| ID | Need and reason | Evidence and authority |
| --- | --- | --- |
| U1 | One reusable Rust engine callable as a library and a program; avoid imposing a JS runtime on Rust consumers | Authorized: owner asks why TS/Bun is needed and agrees to the Rust-core recommendation with “yes lets make a brand new spec” |
| U2 | Credentials belong at execution configuration, never in rule definitions or published feedback | Authorized: original secrets requirement, unchanged |
| U3 | Precise typed contracts plus semantic validation of untrusted inputs and outputs; types should communicate what callers can rely on | Authorized: original typing requirement and current engineering/Rust rules |
| U4 | Native public noul/choice/score judgments, mixed named questions, probabilities, fractional scores and structured JSON instructions/criteria | Authorized: owner confirms public primitives and explicitly supplies TypeSafe advanced-structure documentation |
| U5 | Independent questions may share evidence; consumer-owned composition and dependent follow-up remain explicit | Authorized: original shared-state/composition direction, unchanged |
| U6 | Preserve observed usage, resolved model and optional billing cost so economics can be measured | Authorized: original usage requirement and source-backed cost correction; savings remain unmeasured |
| U7 | Lint and later evals reuse rule meaning; expectations remain independent of the engine | Authorized: original reuse direction and concrete skill-lint first customer |
| U8 | Rust is the engine runtime. TypeScript/Bun may be a caller, with no second engine or compulsory sidecar | Authorized: owner's Rust discussion, no-priors correction and agreement to the single-core proposal |
| U9 | Reusable mechanisms belong in sunstack-tools; Sunstack owns skill meaning, rule content and expected eval outcomes | Authorized: repository split, continued through the language reset |
| U10 | Rust domain enums, distinct validated IDs, explicit errors, one Tokio runtime and responsibility-based modules | Authorized: supplied AGENTS and fully read Rust rules; Rust selection supersedes a mandatory Zod/Effect/TS core |
| U11 | Separate Requirements, Specification and Program Design; inspect the model and obtain independent design review before planning/implementation | Authorized: original design commission and fresh-spec request |
| U12 | Replaceable decision providers, Jev/OpenRouter first; OpenAI Decisions and OpenRouter Alpha Decisions are future candidates | Authorized: owner explicitly names future Decisions alternatives and limits initial support to Jev |
| U13 | Language-appropriate formatting, linting, real type checking and tests; TS tooling applies only to an admitted TS caller | Authorized: existing craft/proof rules interpreted under the explicit Rust reset |
| U14 | Forward useful schema/model/validation diagnostics to agents, while hiding raw SDK/provider internals and keeping failures out of findings | Authorized: owner's explicit useful-error correction |
| U15 | Load JSONL rule definitions that refer to stable paragraph IDs in skill Markdown, with source-linked lint feedback | Authorized: owner supplies a JSONL rule file linking an ID such as U001 to a paragraph beginning with that ID |
| U16 | Robust, fast filtering and input preparation using Git diffs, text search and AST-grep; supply useful evidence to decision engines | Authorized: owner explicitly moves these capabilities into the core and asks for stronger/faster filtering |
| U17 | Structural facts, relationship hypotheses and model judgments must remain distinguishable; uncertain candidates can still inform an agent | Authorized: owner-supplied AST-grep/evidence-tag research and the request to make those inputs useful; heuristic/calibration details remain proposals |
| U18 | TS can consume Rust through a thin boundary; direct Rust consumers should not pay a mandatory process or JS cost | Authorized: owner asks whether TS can just use Rust and accepts the one-core proposal |

## What the Rust reset supersedes

| Earlier proposal | Current boundary | Reason |
| --- | --- | --- |
| Bun must execute the engine; TS public library is the main surface | Rust library plus thin executable; optional TS caller | Explicit owner reset after examining the evidence/filtering use case |
| Zod 4 or Effect Schema owns core validation | Rust-owned deserialization and semantic validation, with equivalent useful diagnostics | JS validation libraries do not require a JS engine; retain outcomes, change mechanism |
| SDK owns retries; Rust talks to a Bun host | Rust adapter owns one bounded HTTP attempt controller | Direct Rust execution calls the documented provider HTTP API |
| Model runner never collects repository evidence | Reusable source capture, diff/search/structural selection is now in core scope | Owner explicitly expands the core's job |
| TypeScript modules are the rule loading format | JSONL data is the first rule-pack surface; native consumers may compose in code | Concrete first customer; no arbitrary JS execution is necessary for data rules |

No shipped behavior or data needs migration. Earlier review receipts do not establish acceptance of this new design.

## Scope and protected systems

This design covers the reusable Rust core, a thin executable boundary and a local skill-lint example/proof fixture. Sunstack supplies rule questions, applicability declarations, evidence requirements, rubrics, thresholds and feedback. Tools implements the reusable mechanisms that interpret those declarations, gather evidence and execute decisions. A native caller may supply prepared evidence and compose answers directly.

Permitted repository: sunstack-tools. Existing skill packages, production hooks, codex-router, infrastructure and eval datasets are read-only consumers/reference sources in this job. Shipping their integration, publishing packages or installing a service is outside the first delivery.

The first data-rule format is deliberately bounded. The proposed policies classify one answer using explicit noul/score bands or a choice-label map. It is not an arbitrary expression language or workflow DAG. Consumers can implement richer composition in Rust code using the same judgment entrypoint. Whether v1 must express richer composition inside JSONL is an owner decision if that restriction prevents a real rule.

Excluded: mandatory Bun service; mandatory native JS addon; LSP/type-checker replacement; automatic edits/renames; dynamic provider discovery; persistent index, daemon or result store; generic workflow language; hidden retrieval/truncation; automatic acceptance/merge authority; new skill/eval authoring inside Tools.

## Evidence needed to close the outcome

The first delivery needs real Rust library and executable paths, strict applicable checks, source/diff/Markdown/query integration, boundary and negative controls, and a genuine mixed-primitives Jev request with validated answers/usage. Controlled HTTP transport proves local wiring; it does not prove hosted behavior or semantic accuracy. A skipped hosted call is an integration gap.

Fast filtering needs measurements over declared workloads: repository size, changed files, rule count, scanned bytes, parsed files, selected/skipped/failed checks, elapsed time and model request count. No numeric performance promise or improvement percentage has been supplied. Accuracy and ranking calibration require independently labeled examples, not self-generated expectations.

## Source basis and remaining choices

Authority is the owner conversation and supplied AGENTS/language rules. Observational basis is the repository at `c3220fc`, current AST-grep source and official Git/Rust/OpenRouter/Decisions documentation cited in Program Design. The linked research snippets are pointers and hypotheses; their unexecuted examples and timings are not runtime proof.

Open material choice: richer JSONL composition if the bounded policy format cannot express the first real rule. Unverified: concrete provider/model availability and credentials, numeric latency targets, supported distribution platforms, optional TS binding and immutable publication destination. These gaps do not block a Rust design draft or authorize extra integrations.
