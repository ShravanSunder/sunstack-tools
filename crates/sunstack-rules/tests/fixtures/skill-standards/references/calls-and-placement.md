# Calls and Placement

## Standards

Placement:

- Material only some runs need lives in a reference loaded with `IF <condition>, load`.
- All-run detail moves to a `MUST load` reference only when it forms one coherent procedure, is dense enough to obscure the main path, or changes for its own reason; line count alone never justifies the split.
- Moving all-run procedure behind `MUST load` leaves its obligation, order, decision, required return, invariant and completion visible in `SKILL.md`.
- Long examples, provider mechanics, branch-only rubrics and exceptional procedure live in references, unless the user explicitly asked for one file.
- Each review check lives in its own reference, and the status fields shared by review checks live in one review schema.
- A skill that defines review checks has a schema for the statuses, verdicts, finding fields and reduction its review uses.
- Deterministic executable mechanics live in `scripts/`.
- Term meanings live in the glossary and nowhere else, including the coined terms a skill needs.
- Each defined term is concrete and matches how `SKILL.md` and the references use it.
- Each meaning has exactly one home.
- Every reference exists because a named call site asks for it.
- A reference section that owns a stage says what to inspect, what good and bad look like, and when to stop.<!-- R001 -->
- Every stage or branch a skill promises has a teaching owner: an inline section of `SKILL.md` or a teaching reference.
- A runtime skill loads only the references its own phase needs.
- Shared authoring lessons live in authoring skills or their references, never in runtime workflow skills.
- A skill has no per-skill provenance document unless it needs a task-specific reference for progressive disclosure.

Calls:

- Each load or dispatch call says when to go, where, what to do there, and what to bring back.<!-- K001 -->
- Every call site uses exactly one form: a load, a dispatch or a citation.
- A reference call opens with exactly one literal load form, `MUST load` or `IF <observable condition>, load`.
- A dispatch is only for a prescribed procedure: a command, a watch or a mechanical transform with an observed result; review checks, research and every other judgment are loaded and performed by the current agent.
- A dispatch names its exact steps, its allowed commands or transformation, its execution boundary and the observation to return.
- A dispatched helper reports observations and exceptions, and the calling agent judges them.
- Independent review is not dispatch: a review skill may start reviewer agents with no authoring history to walk its checks, and its lead verifies their findings.
- The caller, not the reference, owns the entry route: the load mode and the condition that selects the reference.
- An opened reference owns its decision or detail, its expected inputs, its local procedure and examples, its result, and a checkable stop when it asks for work.
- A reference holds one coherent kind of detail, with its local rules, examples, caveats and templates kept together.

Citations of principles:

- A principle is only cited, never loaded; a reference or procedure is loaded or dispatched, never cited.
- A citation is one sentence naming an observable moment and the action to take there, followed by a relative link to the principle.
- A citation's action says what to produce or check, in words other than the link text; "apply X", "(see X)", or an action that restates the link text is incomplete.
- A principle is cited only where its judgment changes how this action is done.
- A principle is cited once per decision; when a skill used at that step already cites it for that decision, the citation isn't repeated.
- A citation's path resolves from the file that holds it.
- A citation never replaces a load call for the skill's own procedure.
- A cited principle ships in the same plugin as the file that cites it.
- The citing skill writes no loader line for a principle, because the principle's first body line is its own read rule.

Shared shapes:

- A model-readable output shape gets its own home only when two or more real consumers need the same fields and meanings; a shape with one consumer stays with it.
- A tool shape gets its own home when a tool, test, CI check or runtime validates the structure, and it names that validator; a readable example is not a machine contract.
- A shared shape defines each field, value and ordering constraint once.
- A shared shape leaves the judgment, selection and completion that produce it with the calling workflow; a schema owns fields and labels, not the workflow.
- A reference that only pins an output shape is justified by a named consumer.
- A reference that only pins an output shape never owns a stage.
- Each skill's packet layout, source rules, security context, receipts, reducer rules, check names, route-backs, proof details, statuses and examples stay in that skill's own references; no runtime contract is imported by every skill.
- When a shape or rule changes home, every consumer links to the new home and the old definitions go in the same change, with no forwarding stub or alias.

## Who reads it, and when

Placement starts from who reads the material and when; that answer picks the home and the call form.

| home | read by | when |
| --- | --- | --- |
| `SKILL.md` | this agent | every run |
| `MUST load` reference | this agent | on reaching the call |
| `IF ..., load` reference | this agent | when the condition holds |
| `MUST dispatch` procedure | a helper agent | on reaching the call |
| `IF ..., dispatch` procedure | a helper agent | when the condition holds |
| cited principle | this agent | at the moment its citing sentence names |

Two different reasons move material out of `SKILL.md`. A branch is material only some runs need: moving it keeps every other run from reading it, so it is an attention decision. A module is material every run needs that forms one coherent thing changing for its own reason: moving it keeps the main path scannable and lets the piece be maintained on its own, so it is a maintenance decision.

The near miss for the one-file exception: a draft shown in the conversation is not a request for one file, and neither is asking for the `SKILL.md` draft. Only an explicit request for one file keeps long examples and provider mechanics in `SKILL.md`.

## The placement test

```text
all-run obligation, order, decision, required return, invariant, or completion -> SKILL.md
all-run coherent detailed procedure                      -> MUST load references/<step>.md
branch-only detail                                       -> IF <predicate>, load references/<branch>.md
one ordered review check                                 -> references/checks/<check>.md
shared review status fields                              -> references/checks/review-schema.md
shared model-readable output                             -> references/<name>-output-schema.md
machine-validated structure                              -> schemas/<name>.schema.json
                                                            or references/<name>-tool-schema.md
deterministic executable mechanic                        -> scripts/
judgment a principle teaches                             -> cite the principle at the decision
term meaning only                                        -> glossary.md
no behavior change                                       -> prune
```

## Call forms

A call the agent cannot act on sends it away with nothing to bring back. The forms:

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

```text
Principle
  <moment>, <action in plain words> ([<the principle's judgment in plain words>](<relative path to the principle's SKILL.md>)).

  strong  Before relying on a handback, match each claim to the commit, target and output you opened, or name it as a gap ([verify before accepting](../verify-before-accepting/SKILL.md)).
  weak    Before relying on a handback, verify it before you accept it ([verify before accepting](../verify-before-accepting/SKILL.md)).
  load    MUST load `references/handback-checks.md` and return each claim matched to its commit, target and output.
```

The weak citation's action repeats its link text, so it states no action. The load line shows the form for depth that is the skill's own procedure. The sentence carries the moment and the action, and the link carries the depth; steps and narrative prose use the same form. A citation from `references/` adds one `../` to its path.

IF a citation's moment is in doubt, load `main-path.md` and return its test for a branch condition, which a citation's moment must also pass.

The near miss for dispatch: a known check skipped under pressure is still a check. It is loaded and performed as a step, never turned into a dispatch on the grounds that a helper would be "independent".

## The caller and the opened reference

What the caller writes at the call:

```text
load mode: MUST | IF <observable predicate>
destination: exact reference path
requested work: decision, inspection, or procedure performed there
needed result: concrete result consumed by the continuing main path
```

What the opened reference holds, starting from the work its caller already selected:

```text
owned decision or detail
expected inputs from the caller
local judgment, procedure, examples, caveats, or templates
detailed returned result
checkable stop or completion condition
```

An opened reference may use local conditional procedure and call deeper references. A useful opening shape for a reference:

```text
this reference owns:
expected inputs:
return:
complete when:
```

Reference fit, as questions:

- Does it hold one kind of detail?
- Does it change behavior without repeating the main path?
- Does it return a result the caller can use?
- Where it asks for work, can the agent tell when that work is done?
- Does it leave the entry route and the parent's completion with the caller?

## Shared shapes

When a shape earns its own home, return its consumers, the reason they need the same shape, the authoritative home, and the proof that the current caller can load or validate it.

The near miss: a long provider-specific procedure moved into its own file is still an ordinary reference. With no second consumer of its fields and no validator reading it, it isn't a schema, however structured it looks.

## Pruning pass

For each body or reference section, ask:

```text
what decision or detail does this own:
who consumes it:
what behavior changes:
what result returns:
what would break if deleted:
```

Where those answers are weak, inline the detail with its actual owner, merge overlapping references, or delete the no-op prose.

## When to stop

Complete when each meaning has one owner, every caller is complete, and every opened reference can finish its local work without claiming its own entry route. Return each standard above that applies as kept or broken, with the line that shows it.
