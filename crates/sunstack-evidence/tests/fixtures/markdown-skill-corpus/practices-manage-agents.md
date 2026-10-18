---
name: practices-manage-agents
description: "Use when delegating to subagents, agent teams or parallel agents: the Lead or an authorized helper chooses an Advisor, Implementer, Verifier, Reviewer, Researcher or Operator, its valid Worker or Sidekick form, and model; or spawns, commissions, continues, steers, waits for, monitors, verifies or assesses that work, including a fan-out across many units. Not for performing the assigned engineering phase itself."
---

# Practices: Manage Agents

Collaborate with other agents through a fixed structure: each job is classified, each agent has a role and a model, each handoff is a packet, and each result is checked against its evidence.

The 🦁 Lead is the agent the owner talks to. It designs, plans, cuts the work, and accepts results. The owner decides tradeoffs, merges, and can override any pick. Other agents take jobs from the Lead, or from a Sidekick whose role skill and commission let it start helpers, and return evidence, findings, or changes; they never write the design or the plan.

## Classify

### Work levels

Work nests like a project plan. At every level the agent works in a loop: do one piece, check it against that level's check, then do the next piece or hand back. An agent can drive only as far as there is a check to verify against; if the Lead cannot name the check for a level, the job is cut smaller.

| Level | Made of | Checked against |
|---|---|---|
| Project | milestones | the owner's definition of done |
| Milestone, such as one PR | tasks | the plan's proof, then review |
| Task, such as one slice | subtasks | its acceptance check |
| Subtask, one step or one written procedure with no judgment between its steps | edits or commands | its expected result |

### Horizon

The highest work level the agent closes before handing back. Name the check that ends each job.

| Answer | The agent closes | Before handing back at |
|---|---|---|
| Subtask | one step, or one written procedure | its expected result |
| Task | one result made of subtasks | its acceptance check |
| Milestone | one deliverable made of tasks | the plan's proof |

### Direction

How much of the job the brief decides.

| Answer | The brief | Test | Then |
|---|---|---|---|
| Exact | is a procedure; nothing is left to judge | could a script run it? | only an Exact job can go to a 🔧 Operator |
| Complete | names the approach; the agent decides only details nothing else depends on | can you name the approach in one sentence? | ready to hand off |
| Partial | gives only the outcome, so the agent would choose the approach | does the job hide a design choice? | the Lead writes any design choice into the plan, since a design choice never goes to another agent; what stays Partial after that takes a Partial row in the catalog |

Missing intent or a missing plan is missing input, not Partial.

### Span

What the agent must hold in its head to get the job right. Size and module count don't change it.

| Answer | The agent must hold | Test | Then |
|---|---|---|---|
| Local | one owner's code and the contracts it uses | is reading that owner and its contracts enough? | ready to hand off |
| Cross-domain | two or more owners in one system whose shared contract must stay in step | would the change break another owner's code in this system? | write the shared contract into the plan |
| Cross-system | a contract another system consumes: an API, a wire format, stored data another system reads | does another system read what changes? | split at the system boundary if you can; once the contract is written into the plan, each side built against it is Local |

### How they fit

Horizon caps the role, direction decides whether an Operator can take the job, and direction and span pick the catalog row.

## Roles

| Role | Takes | Returns | Form | Commissioned by | Role skill |
|---|---|---|---|---|---|
| 🦁 Lead | the Project | governing design and plan, integration, acceptance | the owner's main conversation | never commissioned | `role-lead` |
| 🦉 Advisor | an independent second look at high-risk points it picks, the owner's questions, unblocking the Lead's stalled work within its grant | judgments with what was checked, concluded, recommended and kept out | Sidekick | the Lead when the owner asks, or the owner directly | `role-advisor` |
| 🐒 Implementer | one planned workstream, a Milestone or Task: changes, product tests, proof and corrections | the change and its proof | Sidekick | the Lead, one per workstream | `role-implementer` |
| 🔬 Verifier | the paired Implementer's workstream: a proof-feasibility challenge, building or updating `verify-<app>`, observing the delivered target and checking the offered tests | observations and gaps | Sidekick | the Lead, as an Implementer's pair | `role-verifier` |
| 🔎 Reviewer | an independent review of a declared artifact and its corrections | anchored findings | Sidekick with no author history | the Lead, for a review lead; that review lead, for its stage reviewers | `role-reviewer` |
| 📚 Researcher | one bounded question, or a continuing investigation | a checked answer with its sources | Worker; Sidekick for continuing work or when its model needs Router | the Lead, or a Sidekick allowed to start helpers | `role-researcher` |
| 🔧 Operator | an Exact Subtask: a written procedure, a watch, a mechanical transform or a guided collection | what it observed | Worker; Sidekick when the job needs its own session | the Lead, or a Sidekick allowed to start helpers | `role-operator` |

A **Worker** is a native subagent its parent spawns for one bounded job and manages through that job's corrections. A **Sidekick** is a separately managed session, reused by address across related work.

An Operator makes no judgment calls: it never decides what to run, what a result means, or how to fix it. It runs the suite and reports failures, watches CI until it finishes, pushes and publishes a verified file, applies a codemod, or collects logs; it does not fix a failing test or decide whether a failure matters. Judgment it meets goes back to whoever assigned it. Tests and proof for an agent's own change stay with that agent; an Operator takes only a procedure assigned on its own, such as a suite run or a CI watch.

- **Implementer and Verifier:** create both sessions without starting work, fill each packet with the other's address, then dispatch both. A collection subtask inside a workstream gets no pair.
- **Reviewer:** never fork or resume the author's conversation; the packet carries the sources.

Delegate only independent work that costs less to hand off than to do. Work that splits into independent units (an audit, a migration, a review across files) fans out: one helper per unit, one table at the end.

Title a session `<emoji> <Role> Sidekick · <purpose>`, such as `🔬 Verifier Sidekick · packet schema`, and label a native helper by its role, such as `📚 Researcher Worker · retry policy`.

## Model catalog

| Tier | Definition |
|---|---|
| Workhorse | procedures, repeatable work, guided execution |
| Daily driver | execution or synthesis that needs judgment |
| Frontier | demanding judgment, design, or review |

A Daily-driver row needs one recorded reason: the owner recommends it, the commissioner says what the Workhorse lacks, or the Workhorse failed after corrections. Rows marked `User must authorize` are used only when the owner names them. Thinking Effort lists the allowed range for the row. A job that matches no row is not ready: cut it until it matches one, or bring it to the owner, who can name a model. An executor the owner assigned keeps its model and effort.

Exact id: the host's row in `~/.config/agent-context/model-map.md`; otherwise the host's current-generation id for the catalog row, never an older generation or a `-fast` variant; otherwise report the gap.

### 🔧 Operator

| Tier | Model | Thinking Effort | Direction | Span |
|---|---|---|---|---|
| Workhorse | OpenAI Luna | medium | Exact | any |
| Workhorse | OpenAI Luna | high | Complete, guided collection under supplied criteria | Local |

### 📚 Researcher

| Tier | Form | Model | Thinking Effort | Direction | Span | Use |
|---|---|---|---|---|---|---|
| Workhorse | Worker, or a Sidekick when the model needs Router | OpenAI Luna | high | Complete | Local | |
| Daily driver | Worker | OpenAI Sol | medium | Complete | Cross-domain | |
| Daily driver | Sidekick, continuing investigation | OpenAI Sol | high to xhigh | Complete, Partial | Local, Cross-domain, Cross-system | |
| Daily driver | Worker or Sidekick | Claude Opus | high to xhigh | Complete, Partial | Local, Cross-domain, Cross-system | User must authorize |

A continuing Researcher Sidekick may use Luna Operators or Researcher Workers for collection.

### 🐒 Implementer Sidekick

| Tier | Model | Thinking Effort | Direction | Span |
|---|---|---|---|---|
| Workhorse | OpenAI Luna | xhigh | Exact, Complete | Local |
| Workhorse | OpenAI Luna | high | Exact, Complete | Local |
| Daily driver | OpenAI Sol | medium | Complete | Cross-domain |
| Daily driver | Claude Opus | medium | Complete | Cross-domain |
| Daily driver | OpenAI Sol | high to xhigh | Partial | Cross-domain, Cross-system |
| Daily driver | Claude Opus | high to xhigh | Partial | Cross-domain, Cross-system |

### 🔬 Verifier Sidekick

| Tier | Model | Thinking Effort |
|---|---|---|
| Daily driver | OpenAI Sol | high to xhigh |
| Daily driver | Claude Opus | high to xhigh |

Choose a different lineage from the paired Implementer when one is available. A missing eligible route is a named gap, never a reason to drop the Verifier.

### 🔎 Reviewer Sidekick

Reviewer Sidekicks review each Project and Milestone, never a single Task or Subtask. Design and proposal reviews happen where their owning phase says and use the same reviewers. Every review has at least one reviewer from a different lineage (OpenAI, Claude, xAI) than the Lead or author.

| Tier | Model | Thinking Effort | Use |
|---|---|---|---|
| Daily driver | xAI Grok | high | |
| Daily driver | OpenAI Sol | xhigh | |
| Daily driver | Claude Opus | xhigh | |
| Frontier | OpenAI Astra | high to xhigh | User must authorize |

Beyond the rows above, another reviewer joins only when the owner asks for one and names its model.

### 🦉 Advisor Sidekick

| Tier | Model | Thinking Effort | Use |
|---|---|---|---|
| Daily driver | OpenAI Sol | xhigh | |
| Daily driver | Claude Opus | xhigh | |
| Frontier | OpenAI Astra | xhigh | User must authorize |
| Frontier | Claude Fable | high | User must authorize |

### 🦁 Lead

The owner chooses the Lead's model. Nothing here escalates it.

## Work with an agent

Classify the job (the Examples show how), pick its role and model rows, then launch, hand off, wait, and check the result's evidence against its claim. The Lead gives the overall verdict.

### Launch

MUST load `references/execution-forms.md` and return the route, lifecycle and history boundary for the chosen form.

### Hand off

Whoever assigns the job writes its prompt.

1. MUST load `schemas/assignment-fields.ts` and the role's `schemas/role-contracts/<role>-contract.ts` and return the packet, filled from each field's description, with the role skill in `requiredReads.charter`.
2. MUST load `practices-preflight` for the assignment's context and return `ready` or the gap list. A preflight that already covered the same context counts. Carry the checked list in `sources`, and hold back any work an open gap blocks.
3. When two jobs would write the same file, branch or service state, write the isolation or the ordering into each packet's `limits`.
4. Send this prompt, then the packet: "You are the <emoji> <Role> <Form>. Load `role-<name>` and read it in full, then the reads listed below; a `principle-<name>` read is the file `principle-<name>/SKILL.md` beside your role skill's folder. If you can't load `role-<name>`, return that gap before starting work."

When an agent drifts, send it a correction; don't restart it.

### Check the result

Before using a handback, match each claim to the commit, file or output you opened; return anything unmatched as a gap. Only the Lead accepts delivery.

## Examples

MUST load `references/delegation-examples.md` and return the six steps (direction, span, horizon, cut, role and form, catalog row) written for this job.

## Completion

Done when the helper's role, form and model rows are chosen with any reason their rows require, its packet carried the preflight result and went to a real address, and its handback was matched to evidence.
