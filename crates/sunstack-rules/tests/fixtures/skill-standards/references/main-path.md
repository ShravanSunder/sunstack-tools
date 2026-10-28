# Main Path

## Standards

- `SKILL.md` states the mental model or stance before any detail or exception.
- The mental model names the concept, lens or leading word the skill draws on, and the behavior it makes predictable.
- A reader of the mental model alone can predict the shape of the workflow.
- `SKILL.md` shows the all-run spine, from load to completion, in one scan.
- The all-run spine handles every branch the description promises.
- Steps are ordered only where order changes behavior.
- Each meaningful step or reference pass ends by saying what must be true before the work continues.
- A rule every run needs stays inline, beside the decision it governs.
- `SKILL.md` names its completion boundary: the proof, unresolved conditions or blockers that prevent a done claim.
- A branch names an observable condition and the different work it causes.<!-- K001 -->
- Each branch returns a concrete result the main path uses; "more context" is not a result.
- A skill's body takes the form its job needs (steps, a compact route, references, or a mix), and each piece of machinery is there because the job needs it.

## Reading a body

Inside a body, the surfaces' order runs at a finer grain: the lens answers to what the trigger promised, the route to the lens, the call sites to the route, and the wording to the failure it targets. Read the body in that order.

The elements a body can carry: the mental model or stance; the all-run spine; completion checks on its steps; the steering and invariants every run needs; reference calls; dispatch for a prescribed procedure; citations of principles; and the overall completion boundary. Headings and format are the skill's own choice.

Inspection questions:

- Cover everything below the mental model. Can you sketch the workflow from what is left? If not, the lens is missing or doesn't carry the job.
- Can you trace one run from load to completion without opening a reference?
- For each branch the description promises, where does the spine handle it?
- For each step, what would you check to know it is done?
- Where does each rule every run needs sit: beside its decision, or in a reference some runs never open?
- What, by name, stops a done claim?

## The lens

Good: a flaky-test skill opens with "A flaky test is a measurement problem before it is a code problem: the test asserts on something the runtime does not guarantee." `measurement problem` is a concept the model already holds, and a reader can predict the route from it: reproduce, name the missing guarantee, fix the guarantee or the assertion.

Bad: "This skill helps you handle flaky tests effectively." It names no lens, so nothing about the workflow follows from it.

IF the lens rests on a leading word whose strength is in doubt, load `steering.md` and return its leading-word standards.

## The route

The same job can take several forms. Ordered steps fit when doing one thing first changes what the next one finds. A compact route fits a judgment the agent applies in one pass. A thin body that loads references fits a job whose depth only some runs need. Most skills mix them.

IF the size of the body is contested, load `worked-examples.md` and return the shape choice it supports.

## Branches

- `IF the skill needs more depth, load …` is a topic wearing IF clothing: nothing observable selects it, and the work doesn't change when it holds.
- `IF the diff adds a script, load …` is a branch: the condition can be seen in the diff, and the work changes when it holds.
- The near miss: a topic being interesting, provider-specific or detailed is not a branch condition. The condition is whatever marks the runs that need the topic, such as the provider the repository actually uses.
- A result the main path can use is a verdict, filled slots, a proof result, a route decision or an edit boundary.

## When to stop

Stop when each standard above is named as kept or broken for the body, with the line that shows it. IF a step's completion criterion is in doubt, load `steering.md` and return its two tests for a completion criterion.
