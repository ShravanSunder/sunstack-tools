# Sunstack Tools rule-runner requirements

The developer needs one reusable rule-execution library for a linter and a future eval runner. Rule authors supply named questions over shared evidence and compose the answers into findings. Changing the decision-model connection should not require a second implementation of the rule's meaning.

This document owns needs, outcomes and limits. The separate [Specification](specification.md) owns observable contracts; [Program Design](program-design.md) owns their proposed realization.

## People and the missing capability

| Person and job | Current foundation | Desired difference | Basis |
| --- | --- | --- | --- |
| Developer calling rules from Bun | This repository contains no runner, public exports or proof harness | Import a typed library, supply evidence and model configuration, receive validated answers and execution receipts | U1, U3, U8 |
| Rule author maintaining lint policy | Named noul, choice and score questions are the selected vocabulary | Compose several independent questions over shared state; keep applicability and thresholds in reusable rule code | U4, U5, U7 |
| Eval developer measuring rule quality | A future eval consumer is required; its runner is outside this work | Execute the same rule code against independently defined expected outcomes | U7 |
| Owner accepting design and delivery | The handoff contains requirements and proposals, without an admitted implementation plan | Inspect separate design artifacts and evidence before implementation admission; retain material design and merge decisions | U11 |

The target repository was inspected at commit `4e55750644b66de7ca76cd9d25d43877e581e25b`. Its absence of a runner establishes the local capability gap. No measured usability, accuracy, latency or cost improvement is established.

## What each caller does

| Caller | Job sequence | Present gap or preservation need | Desired observation |
| --- | --- | --- | --- |
| Developer, U1/U3/U4/U5 | Define named questions → supply shared state → execute → inspect typed answers → compose findings | A reusable exported runner is absent here | Named answers retain their kind and literal choice labels; execution failures remain visible |
| Eval developer, U7 | Import the same rules → supply a scenario → execute → compare findings with an independent expectation | Future eval execution is separate work; preserving shared rule meaning is required | Eval and lint call the same rule implementation; the runner does not generate the expected outcome |

These are library journeys, with no UI screen. Production consumer behavior outside this repository has not been verified by this design work.

## Authorized needs

All authorized rows below are owner requirements, with priority **unranked by the owner**. They preserve U1–U11 from the handoff; direct corrections in the receiving session govern U8 and the newer rows. The minimum first delivery includes the authorized rows rather than an agent-assigned priority subset.

| ID | Need or outcome | Why it matters | Evidence and authority |
| --- | --- | --- | --- |
| U1 | A reusable programmatic library that runs on Bun | Developers can call it in their existing runtime; CLI/service/daemon interfaces are unnecessary | Authorized: handoff owner direction U1; current owner: “bun is for running” |
| U2 | Supply OpenRouter credentials securely at execution | Rules, documents, fixtures and reports must not carry secrets | Authorized: handoff owner direction U2 and supplied secrets instructions |
| U3 | Constrain requests and results with precise types and runtime validation | Developers can rely on named answers while rejecting malformed external data | Authorized: handoff owner direction U3; current owner: “follow my good ts rules” |
| U4 | Support native noul, choice and score questions, including mixed named items and structured JSON instructions and criteria | Rules need probability, categorical and ordered-rubric judgments with enough context for precise model decisions | Authorized: handoff owner direction U4; current owner: “make sure we support structured advanced” |
| U5 | Ask independent questions over one shared state; compose in code | Shared evidence can serve several questions; an answer-dependent question belongs in a subsequent request | Authorized: handoff owner direction U5 |
| U6 | Preserve usage so batching economics can be measured | Lower cost is a hypothesis requiring measurement, not a guarantee from request count | Authorized: handoff owner direction U6; savings remain unverified |
| U7 | Reuse rule meaning and execution between linting and a future eval consumer | Avoid two implementations of applicability, thresholds and composition | Authorized: handoff owner direction U7 |
| U8 | Bun is the runtime; pnpm may install dependencies; repo-local node_modules is allowed for package development | Ordinary package/type resolution is permitted without changing the runtime | Authorized: current owner: “you can use pnpm to isntall as package maanger”, “bun is for running”, “for local package devleopment node-moduels is fine”, “i meant for runtime”; replaces the handoff's cache-only restriction |
| U9 | Put reusable runtime code in this separate repository | Sunstack retains skills, rule policy and eval definitions | Authorized: handoff owner direction U9 and successor commission |
| U10 | Strict TypeScript, Zod 4, descriptive names and separation by responsibility | Contracts communicate meaning and validate untrusted boundaries without hiding failures in casts | Authorized: handoff owner direction U10; current owner reconfirmation and full TS rules |
| U11 | Separate, understandable design artifacts and a review before planning/implementation admission | A successor can inspect scope, contracts and evidence without treating a folder tree as a plan | Authorized: successor commission; owner explicitly commissions this session as Lead |
| U12 | Abstract decision-model execution generally, allowing Jev through OpenRouter now and later compatible Decisions backends such as OpenAI Decisions or OpenRouter Alpha Decisions | Rule meaning should survive a different supported model connection; v1 only admits Jev with Zod 4 contracts | Authorized: current owner: “i want jev to be generally abstracted so we call via openrotuer or other deiciosn models”; current owner: “later we will support this too as an alternative” |
| U13 | Use current Oxc, Vitest and Vite tooling; use tsdown only if a build is needed | The project follows the owner's selected development stack while retaining Bun compatibility | Authorized: current owner tooling instruction |
| U14 | Forward safe structured schema, model and validation diagnostics so a caller or agent can act on them without exposing secrets, raw SDK errors or unbounded provider payloads | A failure that is only a boolean or opaque reason cannot guide repair or a bounded retry | Authorized: current owner: “schema issues and useful model errors or validation issues need to be passed forward in some way to be useful to models or agents” |

The owner resolved “make sure they dont pass through” as: keep typed `noul`/`choice`/`score` contracts public, hide SDK/provider internals, keep failures separate from findings, and forward bounded structured diagnostics so callers and agents can act on failures. Raw credentials, SDK errors, provider bodies and arbitrary metadata remain excluded.

## Boundary

The initial delivery is one reusable rule-runner package in this repository. Sunstack owns skill/SOP meaning, rule questions, evidence selection, applicability, rubric wording, thresholds, composition and eval definitions. Tools owns shared execution, request/answer validation, model transport, cancellation, failure classification and execution receipts.

Permitted design work covers this repository's library and development tooling. Existing Sunstack consumers and runtime infrastructure are protected from modification or migration in this task. Cross-repository consumption must have an explicit package boundary; publishing and its destination are not selected.

Excluded work:

- Renaming ai-tools or migrating its existing tools, hooks, observability, sidecar or production consumers.
- Editing skills or rebuilding the eval system or its expected outcomes.
- A Markdown AST/document-format validator as the center of prose correctness.
- A new question-node language, workflow DAG, mandatory JSON loader or filesystem discovery.
- Speculative CLI, provider-plugin, storage or watchdog packages.
- Automatic retrieval, token estimation, partitioning, truncation or silent rule dropping.
- A universal semantic pass threshold, automated acceptance/merge authority, or claims that model judgments prove delivered software works.
- Package publication, destructive repository operations or changes to live infrastructure.

## Evidence that must close the outcome

The completed library needs current strict type checking, applicable Oxc checks and meaningful behavioral tests. Type-contract evidence must prove named-answer and choice-label inference, including rejected invalid accesses. Real SDK/transport integration must establish payload, cancellation and failure behavior. A genuine Bun → exported library → SDK → OpenRouter journey must receive validated mixed-primitives answers and receipts. A skipped credential-gated journey is an explicit integration gap.

Model accuracy and batching cost comparisons are separate measurements. Expected semantic outcomes must be independent of the runner. A stand-in can establish local interactions but cannot prove hosted-provider behavior. Manual proof fills only behavior not already established by equivalent automated real-path evidence.

## Source basis and unresolved meaning

The source packet is `/Users/shravansunder/dev/ai-tools.sdlc-learning-coverage/tmp/spec-workflows/2026-10-11-sunstack-tools-rule-runner/spec-handoff.md`; its companion successor prompt and the receiving owner's direct corrections establish current authority. The supplied AGENTS instructions and `/Users/shravansunder/dev/devfiles/shared/language-rules/ts-rules.md` govern craft. The packet's folder tree and execution-contract proposals are advisory, not an approved plan.

Owner-controlled meaning still open: the exact additional guarantee intended by “don't pass through”. Library distribution scope, provider capabilities and tool compatibility require explicit structural treatment and evidence; they do not authorize publication or a runtime substitution.
