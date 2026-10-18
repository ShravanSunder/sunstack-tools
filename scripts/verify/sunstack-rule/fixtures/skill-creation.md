---
name: skill-creation
description: "Use when designing or writing one named skill, or its skill spec, especially when its trigger, main path, references, steering, or proof plan need judgment. Not for judging a finished skill (skill-review) or for choosing which skills should exist (skill-audit)."
---

# Skill Creation

A skill wrangles determinism out of a stochastic system by making the agent follow a predictable process.

Predictable means the same process, not the same output. A brainstorming skill should diverge every run and still take the same route to diverge.

## Stance

Work on exactly one named skill per run. Portfolio inventory, duplicate-surface archaeology, and "which skills should exist" belong to `skill-audit`.

Open the skill's current files before saying anything about them. A claim about text you have not read in this run is not made; "I already know this skill" is not a read.

Whoever writes a change never reviews it. Review runs in agents that did not write it (`skill-review`).

## Great Skill Frame

A skill has four surfaces:

| surface   | lives in         | owns                                 |
| --------- | ---------------- | ------------------------------------ |
| trigger   | YAML frontmatter | when and why the skill loads         |
| main path | `SKILL.md`       | the mental model and the route       |
| depth     | `references/`    | detail the main path calls for       |
| proof     | tests            | that the other three change behavior |

Design them in that order. Each one can only be judged against the one before it: the trigger decides what the path must handle, the path decides what depth is needed, and proof judges the result.

## Invocation

The trigger surface has two capabilities. Each pays a different cost.

**Model-invocable.** The skill keeps a `description`, so the agent loads it on its own and other skills can reach it. It pays **context load**, because the description sits in the window every turn.

**User-invocable.** The human names the skill directly. It pays **cognitive load**, because the human is the index that has to remember it exists.

They are not exclusive: a description adds agent reach without removing the human's. Choose model-invocable when the agent must find the skill unprompted, or when another skill must reach it. A principle, which other skills only cite, stays hidden from model invocation. When user-invocable skills outgrow what a human can hold, a router skill indexes them.

`references/frontmatter-design.md` owns description wording, adjacent-skill boundaries, and the invocation tradeoff. `references/platform-mechanics.md` owns client-specific invocation controls.

## Information Hierarchy

Placement depends on who reads the material and when, and that answer picks the call form.

| home | read by | when |
| --- | --- | --- |
| `SKILL.md` | this agent | every run |
| `MUST load` reference | this agent | on reaching the call |
| `IF ..., load` reference | this agent | when the predicate holds |
| `MUST dispatch` procedure | a helper agent | on reaching the call |
| `IF ..., dispatch` procedure | a helper agent | when the predicate holds |
| cited principle | this agent | at the moment its citing sentence names |

Review checks and research are steps the current agent loads and performs. Dispatch is only for a prescribed procedure: a command, watch, or mechanical transform with an observed result. A judgment check is never a dispatched procedure, even when it could be described as independent. Independent review is not dispatch: a review skill may start reviewer agents with no authoring history to walk its checks, and its lead verifies their findings.

### Call Grammar<!-- K014 -->

A call the agent cannot act on sends it away with nothing to bring back. Each load or dispatch form names when to go, where, what to do there, and what to return. Every call site uses exactly one form: load, dispatch, or citation.

```text
Reference
  MUST load `<reference>` and return `<result>`.
  IF `<predicate>`, load `<reference>` and return `<result>`.
  ...add `to <requested work>` when the result alone does not say what to do there.
```

```text
Prescribed procedure
  MUST dispatch `<procedure>` to a helper agent using `<prescribed steps>`; return `<observed result>`.
  IF `<predicate>`, dispatch `<procedure>` to a helper agent using `<prescribed steps>`; return `<observed result>`.
```

The caller names the exact procedure, allowed commands or transformation, execution boundary, and result to return. The helper reports observations and exceptions; the calling agent judges them. A known check skipped under pressure cannot become a dispatched procedure because it is independent. Load its reference and perform it as a step.

```text
Principle
  <moment>, <action in plain words> ([<the principle's judgment in plain words>](<relative path to the principle's SKILL.md>)).

  strong  Before relying on a handback, match each claim to the commit, target and output you opened, or name it as a gap ([verify before accepting](../verify-before-accepting/SKILL.md)).
  weak    Before relying on a handback, verify it before you accept it ([verify before accepting](../verify-before-accepting/SKILL.md)).
  load    MUST load `references/handback-checks.md` and return each claim matched to its commit, target and output.
```

A principle is defined in `references/glossary.md`. It ships in the same plugin as the citing file, and its first body line after the title is its read rule: "Read this principle in full before applying it; your own full read earlier this session counts." So the citing skill writes no loader line. A principle is only cited, never loaded; a reference or procedure is loaded or dispatched, never cited. The sentence carries the moment and the action, and the link carries the depth; steps and narrative prose use the same form. The moment is an observable point or condition in the work, held to the branch-predicate test in step 4. The action names what to produce or check at that moment, in words other than the link text. A bare name such as "apply X" or "(see X)" states no action, and neither does an action that restates the link text; either leaves the citation incomplete. Cite a principle only where its judgment changes how this action is done. The path resolves from the file that holds the citation; from `references/`, add one `../`. A citation never replaces a load call for this skill's own procedure depth. Cite a principle once per decision. The skills this one uses at a step are those it names or loads there; when one of them already cites the principle for that decision, rely on that citation and do not repeat it here.

### Progressive Disclosure

Two different reasons move material out of `SKILL.md`.

**Branch.** Only some runs need it. Moving it out keeps every other run from reading it. This is the `IF <predicate>, load` case, and it is an attention decision.

**Module.** Every run needs it, but it is one coherent thing that changes for its own reason. Moving it out keeps the main path scannable and lets that piece be maintained on its own. This is the `MUST load` case, and it is a maintenance decision.

Moving all-run procedure behind `MUST load` does not move the obligation. The obligation, order, decision, required return, invariant, and completion stay visible in `SKILL.md`; the reference owns the detail.

Long examples, provider mechanics, branch-local rubrics, and exceptional procedure always move out; step 5 states the one exception. Four things never do: the mental model, the all-run spine, rules every run needs at the decision they govern, and the completion boundary.

## Leading Words

A leading word is a compact concept the model already holds from pretraining, such as `root cause`, `vertical slice`, `tracer bullet`, `red-green`, or `single source of truth`. Repeated as a token, it anchors a region of behavior in a few characters by recruiting priors the model already has.

It works twice. In the body it anchors execution: the agent reaches for the same behavior every time the word appears. In the description it anchors invocation: when the same word lives in the user's prompts, docs, and code, the agent links that language to the skill and loads it more reliably.

Prefer a word the model already has. A coined term recruits nothing, so you pay in definition tokens what a pretrained word gives free. When a skill needs coined terms anyway, `references/glossary.md` owns them.

A leading word too weak to beat the model's default changes nothing. `be thorough` is not a leading word when the agent is already thorough-ish.

IF a term is unclear, load `references/glossary.md` and return the applicable definition.

## Steering

Steering is the wording that changes what the agent does. Three moves carry most of it.

**Completion criteria.** Test every one twice: can the agent tell done from not-done, and does it demand the legwork? `Understanding reached` fails the first, because there is nothing to check. `Produce a change list` passes the first and fails the second, because a list of nothing satisfies it. `Name every caller of the changed function and say which ones break` passes both, because it cannot be written without opening the callers. The second test is the one that gets skipped, and it is the one that buys depth. <!-- K015 -->

**Rule strength.** Match the rule to how the agent fails. A rule it already follows needs no wording at all. A rule it skips under pressure needs a bright line: one unambiguous condition that leaves no judgment call at the moment of temptation. Name the rationalization beside it in the agent's own words. `I already know this` and `I'll verify later` belong in the skill, not in the postmortem.

**The deletion test.** Would the agent act differently if this sentence disappeared? Apply it sentence by sentence inside its surrounding context, never to whole sections at once. Rationale that changes no behavior is padding however true it is.

Lead with the positive shape: tell the agent what action to take, what result to produce, and what good judgment looks like. Add a prohibition only when a known failure still needs a bright-line boundary, and pair it with the positive target.

# Creating a Skill

## What Belongs in SKILL.md

Include every applicable element below. Choose headings and a format that fit the skill:

- **Mental model or stance:** the lens or domain model that improves judgment.
- **All-run spine:** the work from load to completion in one scan; order steps only when order changes behavior.
- **Completion checks:** each meaningful step or reference pass says what must be true before continuing.
- **Always-needed steering and invariants:** keep rules every run needs inline and near the decision they govern.
- **Reference calls:** name the load mode, exact destination, and concrete result the main path consumes, plus the requested work where the result alone does not say what to do there.
- **Dispatch, for a prescribed procedure:** fill the procedure form in the Call Grammar above.
- **Citations, for a principle:** at the decision the principle informs, fill the citation form in the Call Grammar above.
- **Overall completion boundary:** name the proof, unresolved conditions, or blockers that prevent a done claim.

## Workflow

Steps 1 to 6 produce the skill spec: the proposal a reviewer can judge before any skill file changes. Step 7 writes the files inside the accepted spec. Proof and independent implementation review come after this skill: proof follows `references/proof-and-claims.md`, and `skill-review` reviews the changed files.

A run implementing one slice of an accepted multi-run skill spec reads that spec doc and takes the returns of steps 1 to 6 from it, quoting the slice's success definition, authoring basis, surface allocation, proof posture, and the decision rows it must honor, and checks the doc's coordination slot before editing. Before editing, it also confirms the spec's acceptance still covers the spec's current meaning, under the acceptance binding in `../skill-review/references/spec-review.md`: formatting, typo, link, process-only changes and verified remediations keep the acceptance; any other change since acceptance, or one of uncertain effect, goes back to spec review before any skill file is edited. The doc is the commission for that slice, and each slice still names exactly one skill target.

### 1. Name the promise and success

Classify the run as `create` or `update`. IF the request is to judge an existing skill or a draft rather than change it, use `skill-review` and return its verdict instead of continuing here. Search the owning plugin or skill folder for an existing skill or reference that already owns the named behavior and return the matching paths or `none`. Name the reusable behavior in one sentence: "This skill helps agents reliably do X when Y happens." Before behavior-changing authoring, state a concise, human-readable success definition that names the observable behavior and situation that matter. Ask the user when missing meaning would materially change the intended behavior; do not derive the need from current skill wording alone.

An invitation like "just quickly fix it" that names neither a success definition nor an authoring basis is not a commission for an `update` run.

Completion: classification, owner, reusable behavior, baseline (the current skill files and behavior the change starts from, or `none`), success definition, and the surface allocation (which of the four surfaces carries each part of the change) are named.

### 2. Choose the authoring basis and proof posture

IF the work the skill teaches lives in someone's head and is not yet understood, interview the user until you can restate it and they confirm the restatement; return the confirmed restatement. IF it lives in artifacts the run has not read, read them and return what each establishes with its path. `Build the main path`, `Place the depth`, and `Implement` consume these returns.

Then classify the change, and classify why you are making it.

| change class        | qualifies when                                                                                                                                                                                                                      | consequence                                              |
| ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| `mechanical`        | typos, formatting, version-only, or metadata-only edits with no behavior claim                                                                                                                                                       | static-only; skips both reviews                           |
| `behavior-changing` | the diff alters trigger or invocation, mental model, main path, reference/check/schema allocation, steering, completion, proof, security, or platform contract                                                                        | both review stages plus the proof posture below           |
| `scoped` (behavior-changing) | behavior-changing, and the entire diff is wording inside one owned home (`SKILL.md` prose or a single reference) and touches no trigger, call site, branch predicate, check selection, schema, label set, security surface, or ownership boundary | each review stage narrows to its scoped form; proof rules unchanged |

Every classification is a claim the next reader can check, and this session makes the call about its own work: `mechanical` names the surfaces the change touches and shows none is in the behavior-changing row; `behavior-changing` names which listed surface the diff alters; `scoped` names its one home and shows each excluded surface is untouched. Edits to `SKILL.md` prose, a reference's rules, or the description are behavior-changing whatever their size; size is not one of the listed surfaces.

Behavior-changing work is either `observed failure` or `user-directed intent`. IF the change is behavior-changing, load `references/proof-and-claims.md` and return the proof shape for the skill type; for `observed failure` only, attempt faithful reproduction there, as runs that follow its Running section, and also return the reproduction result, representative-hypothesis boundary, and available next decisions.

- **`user-directed intent`** may draft from an approved success definition without RED.
- **`observed failure`** attempts faithful reproduction before any causal fix is claimed. Reproduced means a targeted RED. Any other result means showing the gap and asking the user to supply evidence and retry, approve a representative hypothesis, author from the success definition with a named proof gap, or defer.

Never manufacture RED, and never let a passing control automatically forbid authoring. "I already know the wording problem" is not a skip.

Completion: authoring basis, reproduction result when applicable, user decision, and strongest honest proof posture are explicit, and each sourcing return that ran exists. A `mechanical` classification names the surfaces it touched; a `scoped` classification names its one home and shows each excluded surface is untouched.

### 3. Design the trigger

Choose the invocation capabilities, then write the YAML description as a trigger-only context pointer for that choice. MUST load `references/frontmatter-design.md` and return the trigger and invocation decision; that reference owns description wording, the description pattern, adjacent-skill boundaries, and the shapes to avoid. IF client-specific invocation controls are requested, load `references/platform-mechanics.md` and return the platform encoding. Completion: invocation capabilities are named, and description or platform policy matches them without summarizing the workflow.

### 4. Build the main path

Judge each part against the one before it: the lens against what the trigger promised, the route against the lens, the call sites against the route, the wording against the failure it targets.

#### The lens

Decide what concept, lens, or leading word the skill should pull into the model's latent space. State the behavior the skill stabilizes and the judgment it should improve. A reader of the mental model alone should be able to predict the shape of the workflow.

#### The route

Express the body in the form this skill needs: steps, a compact route, references, or a mix. IF the classification is `create` or the main-path shape is contested, load `references/worked-examples.md` to anchor the small end of the range and return the shape choice for this skill. Keep the all-run spine, always-needed steering, and the overall completion boundary visible. Put ordered steps in `SKILL.md` only when order changes behavior. Name the workflow from load to completion.

Add a branch only when an observable condition changes the work; a topic being interesting, provider-specific, or detailed is not enough. `IF the skill needs more depth, load ...` is a topic wearing IF clothing; `IF the diff adds a script, load references/security-gate.md` is a branch, because the condition is observable and the work changes. Each branch names its predicate, action or destination, and concrete return. A result of only "more context" is incomplete.

#### The call sites

Write every call site here, in the grammar above. A `load` site names its mode, path, and needed result, plus the requested work where the result alone does not say what to do there. A `dispatch` site names a prescribed procedure, its steps and boundary, and the observed result. A citation names an observable moment and the action, and links its principle relatively. Review checks and research use `load` sites.

#### The wording

Match the guidance form to the observed failure, representative hypothesis, or user-approved success gap:

| observed failure                  | guidance form                               |
| --------------------------------- | ------------------------------------------- |
| known rule skipped under pressure | bright-line rule + rationalization table    |
| wrong output shape                | positive output shape or template           |
| omitted element                   | required slot next to the output            |
| conditional behavior mistake      | observable predicate + action               |
| shallow legwork                   | stronger completion criterion               |
| wrong invocation                  | sharper description or user-invocable route |
| reference retrieval gap           | stronger context pointer or inline material |

Strengthen predicates, returns, and completion criteria when the agent would guess or stop early.

Completion: the mental model is stated before details or exceptions; one all-run spine is visible in one scan and handles every branch the description promises; every branch changes the work and returns something the main path can use; every call site is complete under the grammar; and wording changes cite the failure or success gap they address without overstating its evidence source.

### 5. Place the depth

Keep all-run obligations, decisions, invariants, required returns, and completion in the body while allowing coherent detailed procedure to have its own owner. MUST load `references/reference-design.md` and return the placement decision plus the ordinary caller/callee contract. A draft shown in the conversation still shows each file it would create under its own path: provider mechanics and long examples that Progressive Disclosure moves out appear as their own `references/<name>.md`, called with the load mode the placement decision chose: `IF ..., load` for branch-only detail such as alternative providers, and `MUST load` for an all-run module. Showing the draft in chat is not a request to keep that depth inline; only an explicit request for one file keeps it in `SKILL.md`. Asking for the `SKILL.md` draft is not a one-file request. Depth must teach: a reference owning a promised stage carries what to inspect, what good and bad look like, and when to stop, written from the step-2 sourcing records or a named source; a reference that only pins output shape (schemas, label sets, packet forms) is ceremony, justified by a named consumer and never a stage's owner. IF several consumers need one stable output shape or a tool validates the structure, load `references/shared-shape-design.md` and return the output or tool shape owner and its consumers. Completion: nothing sits in two homes, every reference exists because a named call site asked for it, every promised stage names its teaching owner (an inline body section or a teaching reference), and advanced shape guidance remains discoverable.

### 6. Write the skill spec and get it reviewed

MUST load `references/skill-spec.md` and return the skill spec, in the conversation or as a doc as that reference decides. IF the change is behavior-changing, before any skill file is edited and unless the user explicitly says no review is needed, use `skill-review` at its spec stage and return its implementation decision. A request explicitly skips review only when it names the spec review and declines it ("skip the spec review"). Wanting speed, "just implement it", or "no extra review ceremony unless the skill requires it" is not a skip: this skill requires the review, so say so and run it. Accepted findings inside the settled design return to the step that owns them; a finding that changes design meaning, scope, contract, or a user decision goes to the user first. Completion: the spec is written, and its review decision is `accepted-to-implement`, explicitly skipped by the user, or not applicable because the change is mechanical.

### 7. Implement

IF any surface on the sensitive-surface list in `references/security-gate.md` is in scope, load `references/security-gate.md` before outlining or writing the surface and return its allowed, disallowed, blocked, or deferred decision; a `disallowed` or `blocked` decision stops the write. Then edit the skill surface inside the accepted boundary, and update the spec's run status to `implemented`. Completion: compare the implemented diff against the accepted spec boundary; name a deviation when one exists, and name the proof and implementation review still owed. Return them with the step-2 proof posture and, when the security gate ran, its decision.

## Completion Blockers

The run is not done while any of these hold:

- `SKILL.md` lacks a mental model or main path;
- behavior-changing authoring lacks a human-readable success definition or authoring basis;
- an observed-failure path hides a failed, missing, or inconclusive reproduction result instead of returning the user decision;
- the workflow has branches without observable predicates or return shapes;
- a promised stage or branch has no teaching owner, meaning no inline body section or reference that teaches it; a shape-only reference never owns a stage and separately requires a named consumer;
- a dispatch site is judgment work rather than a prescribed procedure, or omits its steps, boundary, or observed result;
- a behavior-changing change reached implementation without an accepted spec review or an explicit user skip;
- implementation completed without comparing the diff to the accepted spec boundary;
- a change was classified `mechanical` without naming the surfaces it touched, or `scoped` without showing each excluded surface is untouched;
- a sensitive surface was written without an allowed/disallowed/blocked/deferred decision recorded before that surface was outlined or written;
- static validation is claimed as behavior proof.
