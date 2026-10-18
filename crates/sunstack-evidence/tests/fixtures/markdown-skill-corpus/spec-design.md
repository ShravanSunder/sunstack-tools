---
name: spec-design
description: "Use when authoring or revising durable Requirements from settled meaning, or a Specification's observable obligations, domain entities, and coverage views. Not for eliciting genuinely unwritten owner meaning (discuss-pathfinding)."
---

# Spec Design

IF no work reference is in context and the task qualifies, open or resume the trace through `practices-show-me-your-work` before phase work.

Requirements, Specification, and Program Design are separate authoritative concepts:

```text
Requirements
  WHY, for whom, and within what boundary?
  Authorized needs, desired outcomes, priorities, and limits.
                    |
                    v
Specification
  WHAT must be observably true?
  Normative observable obligations traced to Requirements.
                    |
                    v
Program Design
  HOW will the internal system satisfy it?
  Structural realization of the fixed observable obligations.
```

A specification is the observable contract for authorized product meaning. It never substitutes for the separately identifiable Requirements source that authorizes it.

The Lead loads this skill and authors the Requirements and Specification itself, including their views, diagrams, organization, settled sections and visual briefs. Bounded helpers may return evidence. A rendering tool may realize pixels, typography, spacing and style from an unchanged Lead-authored brief, but the Lead chooses the reader question, exact labels and relationships, semantic composition, invariants and exclusions and accepts or corrects the candidate. Neither route transfers governing authorship or makes an image the normative home.

It turns evidence and authorized decisions into a contract that another capable agent can realize without inventing product meaning:

```text
consumer and problem
  -> current observable reality
  -> desired outcome
  -> domain entities the obligations are written over
  -> normative requirement
  -> observable contract or constraint
  -> failure expectation
  -> proof obligation
```

Source authority establishes which meaning is legitimate; it supports the contract rather than replacing it as the primary model. Every `MUST` needs a legitimate basis. Every material requirement needs an observable consequence. An obligation is a sentence about nouns, so the Specification defines those nouns (what makes two instances the same, how they relate, which states they pass through) before it writes obligations over them. Unresolved owner meaning routes through `discuss-pathfinding`; internal component structure and the binding of entities to packages, schemas, and wire shapes stay downstream in `program-design`.

## Boundary

This skill admits or materializes durable Requirements from already-settled authoritative meaning and authors the separate Specification:

- Requirements: affected people, authorized needs, desired outcomes, priorities, limits, and non-goals;
- Specification: normative observable obligations traced to the Requirements identity;
- problem, consumers, and current observable behavior;
- domain entities: canonical terms with stable `E` identifiers, identity rules, relationships, invariants, and observable states that obligations are written over;
- source authority and important user decisions;
- normative requirements and traceability;
- public or externally observable UI, API, CLI, data, configuration, and operational contracts;
- externally required failure, compatibility, security, reliability, performance, accessibility, privacy, and operability obligations;
- proof modalities required by material obligations;
- the specification artifact and author self-check.

It does not elicit or confirm genuinely unwritten owner meaning and does not choose components, internal owners, dependency direction, state storage, call graphs, recovery mechanisms, task order, files, or exact validation commands, and it does not bind an entity to a package, schema, type, table, or wire shape. Route unwritten owner meaning to `discuss-pathfinding`, structural How and entity binding to `program-design`, and review-only work to `spec-program-review`.

Routing cases:

```text
true:      "Write a durable Requirements document from these settled, user-confirmed decisions. Do not write the Specification yet."
near miss: "Interview me to discover what users need and which boundary I actually want." -> discuss-pathfinding
near miss: "Clean up this settled Requirements document without changing its meaning." -> docs-maintain
near miss: "Take this through Requirements, Specification, Program Design, and independent three-artifact design review." -> the full design-cycle workflow, not this phase
```

## Terminal Contract

Return exactly one:

```text
locally-ready
decision-needed
evidence-blocked
deferred
```

A `locally-ready` result includes distinct Requirements and Specification identities; governing-source identities, authority, current applicability, and coverage; the confirmed goal boundary; the accepted requirements set; load-bearing decisions; requirement/proof inventory; author self-check; required independent local-review coverage; remaining gaps; and an explicit statement that three-artifact design acceptance is not claimed. These are returned workflow state, not narrative sections in the specification.

For an explicitly Requirements-only request, finish the Requirements identity and return `deferred` for Specification. Before that early return, state the Requirements reader question at Requirements altitude (affected people, their job, current pain and desired outcome) and MUST load `../../shared-references/diagram-rendering-and-fallbacks.md` to return the selected medium, semantic-preservation result, actual visual check and exact generated-image/capability gap for that Requirements artifact. This call does not run Specification-only entity, context or coverage predicates and must not invent a Specification, interface or internal component. Record the caller's authority to stop there, the consequence that Specification and Program Design are not ready, the exact visual or capability gap when one remains, and the condition for re-entering Specification work. Do not create a Specification or recommend `program-design`.

Produce terminal labels by observable condition:

- `locally-ready`: every completion blocker is cleared and the complete return above exists.
- `decision-needed`: legitimate sources cannot settle load-bearing product meaning, public behavior, compatibility, policy, or cost/risk tolerance; return the decision owner and evidence, and IF returning `decision-needed`, load `../../shared-references/owner-decision-brief.md` and return the brief.
- `evidence-blocked`: required governing or observational evidence is missing, inaccessible, stale, or contradictory enough that a truthful obligation cannot be derived; return the exact evidence and access/state change needed.
- `deferred`: an authorized caller explicitly postpones scoped work after its consequence is recorded; return completed coverage, deferred scope, authority for deferral, consequence, and re-entry condition.

After producing the terminal result, return exactly one phase-guided route:

```text
locally-ready -> recommend program-design
decision-needed | evidence-blocked | deferred -> stop with the exact reason
```

When unmade owner meaning blocks Requirements or Specification, use `discuss-pathfinding` and resume this phase with its clarified return. Pathfinding is not a separate next phase after `locally-ready`. Before recommending `program-design`, inspect its declared inputs and return a compact pointer-based handoff containing the distinct Requirements and Specification identities, confirmed goal boundary and accepted-requirements status, phase result, exact remaining gaps, and why structural How is next. Do not copy full artifacts, unrelated history, or orchestration counters. A direct spec-design invocation carries no cycle budget.

## Workflow

### 1. Establish authority, audience, and artifact boundary

Accept an optional call-scoped `new artifact home` policy. When the caller supplies `<project-root>/docs/specs/`, create any new file-backed Requirements and Specification as distinct resolvable paths beneath that home and return both exact paths. Preserve authoritative pre-existing artifacts elsewhere; do not relocate or reject them because of the policy. The artifact-home policy applies only to new outputs in the current call and is not persisted as workflow state.

MUST load `../../shared-references/requirements-specification-program-design.md` and return the applicable identity representation, the Requirements/Specification separation check, and the identities downstream phases must consume.

MUST load `references/authority-and-problem-framing.md` and return its first result for stage 1: artifact boundary, consumers, decision authority, governing-source identities/classes and current applicability, confirmed goal boundary, and accepted-requirements recovery. Retain the loaded reference and working state for stage 2 and the terminal return. Surface an exact owner decision immediately when it blocks authoring.

Inventory current sources before treating the draft as truth. External popularity is advisory evidence unless an authorized contract makes it normative.

When implementation or incident evidence names internal mechanisms, classify it as evidence and return every authorized observable outcome before any remaining gap. That evidence does not create an owner choice between retaining the mechanism and turning it into a product contract. Route structural choices to `program-design`; ask only about missing product meaning or an actual external compatibility or operational constraint established by a legitimate source.

IF important user or stakeholder meaning is unwritten, or an authoritative source cannot settle required Requirements or Specification meaning, use `discuss-pathfinding`, record which of those two homes owns the missing meaning, and return the complete record, record identity and rows, goal boundary established by `discuss-pathfinding` and checked against the fields in the already-loaded reference, explicit confirmation or correction by the authorized owner, refusal or fallback result, and exact gaps.

Consume pathfinding's complete return as input to the recorded Requirements or Specification owner. Do not create a second Requirements record or re-ask an already confirmed decision. If a field required by the already-loaded reference is absent, keep it as an exact gap. Classify the returned record in the governing-source inventory.

If the user declines extraction, reassess the remaining sources:

- continue when an alternate authoritative source exists;
- return `decision-needed` when owner meaning is missing;
- return `evidence-blocked` when necessary evidence is missing;
- keep only non-blocking uncertainty as a visible hypothesis gap.

Decline itself selects no terminal label, and a hypothesis never authorizes a normative requirement or `locally-ready`.

The minimum accepted Requirements source contract is:

```text
source identity
inspectable location when a human must verify the source
affected user and stakeholder classes
stable U identifiers
need or outcome and why it matters
row-level evidence and producer-owned authority state
priority and priority assigner
unresolved hypotheses
```

A row is normative-eligible only when its producer-owned authority state is `authorized`. A pathfinding record or an equivalent source may satisfy this contract; equivalent sources need not copy the pathfinding document shape. Reuse one qualifying, separately identifiable Requirements source. When settled authoritative meaning is distributed across sources but has no qualifying Requirements home, materialize one normalized Requirements artifact without eliciting, confirming, or inventing meaning. Then preserve its stable identities, evidence/authority classifications, priorities, and gaps in Specification traceability.

Before deriving normative requirements, consume the explicitly confirmed goal boundary or apply the already-loaded reference's goal-boundary check to another governing source. Proposed meaning outside that boundary returns `decision-needed`; specification completeness does not authorize it.

IF an external platform, protocol, library, policy, or empirical claim could change product meaning or an observable obligation and current local sources do not establish it, perform a bounded lookup directly or use `practices-research`; return the exact external source identity/version, authority status, transfer assumptions, and remaining evidence gap before deriving the affected obligation.

Completion: the Requirements identity is qualifying, reused or materialized without duplication, and separate from the planned Specification identity; artifact boundary, consumers, decision authority, source classes, confirmed goal boundary, accepted-requirements recovery, and authority conflicts are explicit.

### 2. Model the problem and its domain before proposing obligations

Name what happens today, who bears the cost, where it is observable, what proves it, and what would remain wrong if the requested feature existed only nominally. Keep implementation root cause separate from the observable problem.

Using the already-loaded `references/authority-and-problem-framing.md`, complete problem modeling and return its second result: the authority/problem model with current reality, desired gap, important decisions, conflicts, and evidence gaps. Retain it in working state for the artifact and terminal return.

MUST load `references/domain-entities.md` and return the entity table (stable `E` identifier, canonical term, identity rule, relationships with cardinality, invariants, observable states, the source terms it canonicalizes, and the authorizing U rows when present or otherwise the settling source or decision record) with every slot filled from the sources or marked as an exact gap, plus undefined-noun gaps and any term routed to `discuss-pathfinding` with its exact same-instance question. Define an entity when the settled sources let two readers decide the same-instance question the same way; route a term only when they cannot. Deriving an identity rule or state from settled owner behavior statements is Specification authoring. The inference `references/authority-and-problem-framing.md` forbids is filling in owner meaning the sources leave missing. Two rationalizations leave nouns undefined at this stage and both are rejected: "the data model is design's problem", and "the source did not define enough entity semantics, so I must ask", which is a stop only when two readers would answer differently. An entity definition names no type, schema, table, package, or payload field; that binding is `program-design`'s work.

When revising existing artifacts, classify the requested correction before editing: Requirements, Specification, Program Design, or a named combination. Removing unrelated concurrency, cleanup, reporter, or lifecycle machinery is a Program Design correction unless the authorized owner also changes users, outcomes, requirements, scenarios, defaults, observable obligations, or proof obligations. A Program Design-only correction routes to `program-design` and does not authorize narrowing the governing Requirements or Specification. When upstream meaning and Program Design both change, settle Requirements and Specification first.

For a semantic correction to an existing Requirements or Specification view, re-open its governing sources, re-run correction classification and the affected view predicate and semantic-field check, update affected trace links, and run artifact self-review. Skip unrelated stages unless the correction invalidates their source or decision result. Pure rendering-format changes route to `docs-maintain`.

Using the comparison taught by `references/authority-and-problem-framing.md`, build the accepted requirements set from the current owner-confirmed requirements record and confirmed goal boundary. If unavailable, recover the last inspectable owner-accepted governing source. If neither exists or the sources conflict, return `decision-needed` with the authority conflict; mutually narrowed current files never establish the accepted starting point by themselves.

Reuse the existing identities and coverage links for:

- affected classes and stable U/E/P/O/R/C/V identities;
- priorities and assigners;
- named variants such as skills or scenarios;
- customer defaults;
- observable contracts, constraints, and proof obligations.

Do not create a separate ledger or duplicate document. Every removed or superseded item needs explicit owner authority.

Produce inspectable per-item coverage that names each accepted identity, authoritative meaning, and specification destination or owner-authorized supersession. A count or an assertion that coverage is intact is not coverage evidence.

Re-anchor before deriving or revising normative requirements: compare the proposed meaning with the confirmed goal, accepted requirements, permitted and protected systems, owner-set package limits, non-goals, and existing foundation from the confirmed goal boundary. Return `aligned` or the exact mismatch. On mismatch, stop normative authoring and return the existing owner decision, or use `discuss-clarify-mental-models` when the shared model itself drifted. Keep the comparison in returned workflow state, not durable specification prose.

Completion: the current/desired gap is inspectable, each causal claim is evidenced or labeled as a hypothesis, every noun the desired outcomes name is a defined entity with an `E` identifier and every slot filled or marked as an exact gap, accepted-requirements coverage is inspectable when applicable, and the Re-anchor comparison is aligned or has returned the exact mismatch and owner route.

### 3. Define outcomes, non-goals, and semantic slices

State the outcomes the change must create and the nearest plausible expansions it will not create. Separate outcomes from requested mechanisms.

When independently governed capabilities, protocols, domains, or consumer journeys are mixed, split those semantic slices before polishing requirements.

Completion: each goal has an observable success condition, each material non-goal blocks a likely scope guess, and every slice has one reason to change.

### 4. Resolve decisions at the correct altitude

Search legitimate sources before asking. Ask only when product meaning, public behavior, irreversible compatibility, policy, or cost/risk tolerance remains undecidable.

Ask one load-bearing decision at a time: load `../../shared-references/owner-decision-brief.md` and return the brief, including foreclosed choices. Delegation, vague assent, silence, or a topic change is not decision authority.

Completion: each load-bearing branch has durable authority evidence, is explicitly deferred with consequence, or produces `decision-needed`.

### 5. Derive normative requirements

MUST load `references/requirements-and-traceability.md` with the stage-2 entity table and undefined-noun gaps to construct requirements over defined entities, repair vague or task-shaped statements, and return requirement-to-entity/problem/outcome/basis coverage plus any entity no requirement uses.

For substantial or uncertain work, stage source notes, alternative wording, prototype views, and temporary U→E→P→O→R→C→V comparisons in private working state, the repository's ignored scratch convention, or `tmp/design-workflows/<date>-<slug>/`. Quick work keeps the comparison in working state. Scratch never owns normative meaning or becomes required reading.

For each requirement, contract, failure obligation, constraint, and proof obligation, name the confirmed need it serves and what becomes observably false or unverifiable if removed. Owner-set repository or package limits may constrain implementation without becoming normative product behavior. Delete an element that changes neither the confirmed outcome nor a necessary truth boundary; a template slot does not authorize adjacent work.

Completion: every goal is covered, every requirement has a basis and observable pass/fail consequence, every normative statement names only defined entities (an obligation over an undefined noun routes back to stage 2, and an entity no requirement uses is pruned or its gap recorded), and two capable implementers would not need to invent different product behavior.

### 6. Specify observable contracts and negative space

MUST load `references/observable-contracts.md` to inspect each load-bearing external surface and return its applicable contract, failure, compatibility, example, and undefined-behavior slots.

Completion: each consumer can predict normal, boundary, failure, partial-success, and compatibility behavior without knowing the internal architecture.

### 7. State cross-cutting obligations without designing their realization

For security, privacy, reliability, performance, accessibility, observability, data lifecycle, compliance, and platform compatibility, state the externally required guarantee or mark it not applicable with a reason.

Name assets, actors, prohibited outcomes, and security non-goals when sensitive behavior is in scope. Leave trust-boundary realization and enforcement calls to `program-design`.

Completion: applicable qualities are observable obligations or constraints, not adjectives or hidden implementation choices.

### 8. Define proof obligations

MUST load `references/proof-obligations.md` to map each material requirement to an evidence class and return requirement-to-modality coverage plus proof gaps.

Do not choose test files, commands, mocks, task order, or capture procedure. Those belong to program-design seams and planning mechanics.

Completion: every material obligation names evidence that could prove it, and no proof gap is hidden behind “test later.”

### 9. Author the smallest coherent artifact

Apply the Required Why/What Views predicates. For each selected view, state the reader question it answers: whose job and pain must be understood, who interacts with the opaque system and across which observable surfaces, which nouns the obligations are about and what makes two instances the same, or which need-to-proof link is missing. IF one or more predicates fire, or the substantial per-document visual obligation applies even when no precise-view predicate fires, load `../../shared-references/diagram-rendering-and-fallbacks.md` before the local artifact reference. Supply the selected precise views plus the per-document explanatory-visual obligation when applicable, and return the selected medium, fallback decision, semantic-preservation result, visual-check result, and exact capability or preview gap.

Each newly authored or substantively revised substantial Requirements and Specification document needs a meaningful visual beside the explanation it supports; reuse a current correct embedded visual when it still explains current meaning. Each accepted requirement that changes a screen, page, or other UI surface is a user-visible moment: Requirements shows that moment with Image Gen grounded in the current app, or records `no current UI`. A people/job/pain picture does not cover a changed screen. Non-UI clauses stay sentences. Specification may show the same screens only to pin what the user observes; the product stays one opaque system, with no internal components, stores, or enforcement. Context, entity, and other topology views render as Mermaid when that destination renders Mermaid. An exact user format takes precedence, except a text fence for a picture-type view when Mermaid can render. No decorative filler satisfies the requirement. The shared rendering owner returns the medium, and a text fence is not a pass for a picture-type view when Mermaid can render. A cache path, prompt, chat-only image, or broken embed is not completion. Generated imagery supplements normative prose and tables. A typo-only edit does not regenerate images, and a companion document's image does not satisfy this document's reader question.

Rejecting a requested all-in-one or internally focused diagram does not complete the selected views and does not require owner approval of their presentation. Render the clearest valid replacements that preserve the required meaning. In a read-only or chat-only run, show those views in the response; describing the diagrams without producing and checking them leaves the view work incomplete.

MUST load `references/artifact-and-self-review.md` with the Required Why/What Views decisions and rendering results to choose the artifact structure, preserve the specification spine, apply view discrimination and pruning, and return the structure decision, traceability/navigation result, view-verification result, pruned elements, artifact identity, and exact view gaps.

Apply any caller-supplied artifact-home policy first. Otherwise follow the repository's documented spec location, or use `docs/specs/` for substantial file-backed work. Create a Specification artifact distinct from the admitted Requirements source. A combined `Requirements/spec` artifact or a Requirements document containing the observable contract does not satisfy this boundary. Keep research ledgers and review reports out of the normative design artifact.

Author top-down: begin with the smallest Why/What map a human needs to confirm the problem and intended outcome, then reveal normative requirements, observable contracts, failures, constraints, and proof. Link every normative-eligible U row to the obligation it authorizes. Expose one compact `Requirements -> Specification -> Program Design` path with distinct identities instead of repeating their roles or contents.

Completion: the Requirements and Specification homes are separately identifiable and non-identical; every normative claim has one home; a stranger can navigate from user need through problem, obligation, observable behavior, and proof; and every accepted identity has an inspectable destination or owner-authorized supersession.

### 10. Run the author self-check

Using the Author Self-Check procedure in the already-loaded `references/artifact-and-self-review.md`, re-read the whole artifact. Check authority conflicts, accepted-requirements coverage, traceability, vague obligations, missing contract/failure behavior, hidden How, contradictory goals/non-goals, proof gaps, assumptions disguised as decisions, process residue, obscure headings, duplicated companion narration, and reader-facing elements that fail the human deletion test. For each generated visual also consume the actual project asset, relative embed, pixel inspection and destination-preview result from the shared rendering owner, then return the author self-check with exact gaps.

Completion: the current artifact has a self-check result with exact passes and gaps. Self-check is never independent review.

### 11. Obtain fresh local review when required

Call `spec-program-review` using its `classify-review-requirement` operation with: requested future mode `specification-only`; the distinct Requirements and Specification identities; scope and claimed semantic effect; governing-source coverage; matched material-risk predicates; and `caller requirement: required | none` (default `none`). Consume the `review-required | non-substantial` result, decision branch, basis, source coverage, caller requirement, and preserved identities.

When `review-required`, invoke `spec-program-review` separately in `specification-only` mode with fresh context and read-only authority, carrying the distinct Requirements and Specification identities, governing sources, confirmed goal boundary, accepted requirements set, constraints, non-goals, and proof claims or gaps. Route accepted Why/What findings back here and follow `spec-program-review`'s bounded review-and-correction-round policy for its convergence rule, parent verification, and any later review.

Completion: the permitted independent review coverage and parent-verified corrections are ready under `spec-program-review`'s policy, or the exact `non-substantial` basis or block is recorded.

### 12. Return the local result

Return the distinct Requirements and Specification identities; governing-source identities, authority, current applicability, and coverage; confirmed goal boundary; accepted requirements set; decision inventory; requirement/proof inventory; self-check; independent local-review coverage; gaps; and non-acceptance boundary.

IF returning a substantial specification in chat and problem, consumer, authority, journey, or requirement-to-proof relationships are non-obvious, use the `presentation-*` skill matching the current surface to render only those Why/What relationships; honor an exact user-requested format instead. Keep normative meaning in the artifact and route internal component, call, state, or failure-mechanism views to `program-design`.

Completion: the caller can invoke `program-design`, compose the result into caller-owned workflow state, or provide the exact missing decision/evidence without reinterpreting the artifact.

## Required Why/What Views

Use a view only when it makes an important Why/What relationship easier for a human to understand, confirm, or correct. This table is the sole owner of view predicates, cardinality, and required semantic fields:

| View | Use when | Must expose |
| --- | --- | --- |
| journey map | per normative-eligible load-bearing direct-user class when the job has a material sequence or pain relationship and the view makes that relationship easier to confirm or correct | one distinct view for that class exposing user-worded steps, observed pain and evidence, desired observable difference, and cited U rows; never chain separate direct-user classes into one journey; when pain or evidence is not established, mark that field unknown rather than inventing it, and when the source says the current outcome is already the desired one, show preservation of that settled outcome rather than fabricating a gap; reuse or link a current requirements-level sequence when it already exposes these fields, and keep the user-requirements source as the normative home |
| context diagram | two or more external consumers or observable surfaces exist | consumers and stakeholders, observable surfaces/contracts, relevant negative space, and the system as one opaque node |
| entity map | the Specification defines one or more entities | entities with `E` identifiers, identity rules, domain relationships with cardinality (belongs-to, references; never component owners), and observable states; the entity table remains the normative home and the map shows no package, schema, type, or store |
| requirement coverage table | multiple or interacting material requirements exist, or their U/E/P/O/R/C/V trace is non-obvious | U when present, E, P, O, R, C, and V links plus gaps; keep one simple trace inline instead of creating a table |

The first internal component, owner, dependency edge, state store, or enforcement point inside the system crosses into `program-design`. Diagrams may explain relationships but may not be the only home of normative meaning.

## Bounded Evidence Help

IF one bounded evidence, observable-surface, product-intent, or contract question benefits from independent work, use `practices-manage-agents` to select the agent pattern and runtime.

The packet names the exact evidence question, sources, maximum authority, non-goals, and expected anchors. Delegation is parallel-safe only after the source inventory and semantic boundary exist. Instance authority is equal to or narrower than the packet and never includes Requirements or Specification meaning, prose, organization, views, diagrams, or integration. Return a `complete | partial | blocked` evidence receipt; the Lead opens the decisive sources, verifies the result, and authors the artifact itself.

## Completion Blockers

Do not return `locally-ready` while any of these hold:

- substantial file-backed work lacks a present, resolvable Requirements identity and a different present, resolvable Specification identity, or either identity cannot be inspected;
- Requirements and Specification are collapsed into one artifact, including a combined `Requirements/spec` label or a Requirements artifact used as the Specification;
- settled authoritative Requirements are duplicated instead of reused, or a normalized Requirements artifact invents or confirms missing owner meaning instead of routing it through `discuss-pathfinding`;
- a normative claim lacks authority or an explicit decision gap;
- a normative requirement names an entity the Specification does not define; an entity lacks an identity rule that decides the hardest case the source raises, or leaves a relationship, invariant, or observable-state slot neither filled nor marked as an exact gap; or an entity is defined in implementation terms such as a type, schema, table, package, or payload field;
- a user-facing normative requirement is based on a row whose authority state is not `authorized`, or normative-eligible user-requirements rows cannot be traced by stable U identifier;
- the goal boundary lacks explicit confirmation or correction by the authorized owner, acceptable outcome-level evidence, or the specification expands its goal, affected classes, missing outcomes, permitted/protected systems, owner-set package limits, non-goals, or acceptable complexity without a new owner decision;
- the accepted requirements set cannot be recovered from the current owner-confirmed source or last inspectable owner-accepted baseline, conflicts with those sources, lacks inspectable per-item coverage, or loses an item without owner-authorized supersession;
- the problem, entity, outcome, requirement, contract/failure, and proof chain cannot be traced;
- unresolved product meaning is disguised as an assumption;
- a returned gap omits an authorized observable outcome, or `decision-needed` is based only on whether implementation or incident mechanisms should be retained or made normative;
- a material non-goal is omitted, or an applicable cross-cutting quality lacks an observable obligation or constraint or a reasoned not-applicable result;
- internal architecture or implementation tasks are presented as requirements without an externally authoritative constraint;
- an applicable Required Why/What View lacks the required cardinality, semantic fields, passed rendering result, or separate normative home;
- an accepted requirement changes a UI surface and Requirements has neither a current-app-grounded Image Gen result for that moment nor an explicit `no current UI` gap, without an exact user-format override that is not a text fence, or an explicit owner-authorized deferral;
- a picture-type view is passed as fenced plain text while the destination renders Mermaid;
- delegated evidence was copied as governing prose or a helper originated, organized, rewrote, or chose the expression of Requirements, Specification, a settled section, or a required view;
- a required independent review is missing, stale, partial, silent, or blocked;
- governing-source coverage, self-check, readiness, review state, or acceptance/PR narration appears as specification prose instead of returned workflow state;
- either Requirements or Specification identity, source coverage, or non-acceptance boundary is missing.
- a `locally-ready` result omits `program-design`, recommends more than one next skill, or routes to planning, implementation, or pathfinding;
- a non-ready terminal invents a continuation instead of returning its exact decision, evidence, or deferral stop;
- the continuation handoff omits the destination's required boundary and authority state or copies full artifacts and unrelated history.

IF a trace is open, at phase completion record the outcome, evidence, and next owner or return token as a checkpoint through `practices-show-me-your-work`.
