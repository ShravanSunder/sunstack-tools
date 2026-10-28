# Steering

## Standards

- A leading word is a concept the model already holds from pretraining, and it is preferred over a coined term.
- A leading word is strong enough to beat the model's default behavior; one that isn't changes nothing.
- Each completion criterion lets the agent tell done from not done, and demands the legwork the step needs.<!-- K001 -->
- Every sentence changes what the agent would do; a sentence whose deletion would change nothing is padding, however true.
- The deletion test is applied sentence by sentence, in context, never to whole sections at once.
- Guidance says what to do and what good looks like, not only what to avoid.<!-- K002 -->
- A prohibition appears only where a known failure needs a bright line.
- A wording change names the failure or success gap it addresses, without overstating its evidence.
- A skill is created or changed only where its wording teaches behavior a capable model would otherwise shortcut.

Wording repairs, each matched to the failure a run shows:

- A rule's strength matches how the agent fails: a rule it already follows needs no wording, and a rule it skips under pressure gets a bright line with one unambiguous condition.
- A bright-line rule names, beside it, the rationalization it blocks, in the agent's own words.
- A wrong output shape gets a positive shape or template.
- An omitted element gets a required slot beside the output.
- A conditional mistake gets an observable condition and the action it causes.
- Shallow legwork gets a stronger completion criterion.
- Wrong invocation gets a sharper description or a user-invocable route.
- A reference that isn't reached gets a stronger pointer or inline material.

## Leading words

`root cause`, `vertical slice`, `tracer bullet`, `red-green` and `single source of truth` are leading words: repeated as a token, each anchors a region of behavior in a few characters by recruiting priors the model already has.

A leading word works twice. In the body it anchors execution, because the agent reaches for the same behavior every time the word appears. In the description it anchors invocation, because when the same word lives in the user's prompts, docs and code, the agent links that language to the skill and loads it more reliably.

A coined term recruits nothing, so a skill pays in definition tokens for what a pretrained word gives free. `be thorough` is too weak to be a leading word when the agent is already thorough-ish: the run looks the same with it or without it.

## Completion criteria, tested twice

Ask two things of every criterion: can the agent tell done from not done, and does it demand the legwork?

- `Understanding reached` fails the first test: there is nothing to check.
- `Produce a change list` passes the first and fails the second, because a list of nothing satisfies it.
- `Name every caller of the changed function and say which ones break` passes both, because it cannot be written without opening the callers.

The second test is the one that gets skipped, and it is the one that buys depth.

## Bright lines and their rationalizations

A bright line leaves no judgment call at the moment of temptation. Written in the agent's own words, the rationalization lets the agent recognize its excuse at the moment it reaches for it:

```text
rule:            Open the skill's current files before saying anything about them.
rationalization: "I already know this skill."
why it fails:    knowing an earlier revision is not a read of this one.
```

Common ones are "I already know this", "I'll verify later", "this is obvious" and "the user probably meant...". They belong in the skill, not in the postmortem.

In a flaky-test skill, the bright line "Never quarantine by deleting the assertion or adding a retry" sits beside the step that says what to do instead: fix the guarantee or the assertion, or skip the test with a ticket that names the missing guarantee.

## The deletion test, in practice

Read the sentence in place, then ask what the agent would do if it were gone.

- "It is important to be careful when editing files." The agent edits the same way without it, so it goes.
- "Open every caller before renaming the function." Without it, some runs rename blind, so it stays.

A whole section can look necessary while half its sentences change nothing, which is why the test runs on sentences.

## The positive shape

- Weak: "Don't skip tests."
- Strong: "Run the tests for each changed module and report each result with its command."

The weak line names a failure and leaves the target to the agent. The strong line names the target, so the failure has nowhere to sit.

## Using the repairs as a lookup

Name the failure a run showed before choosing any wording, then take the matching repair from the list above; the list is the lookup.

## When a change is worth making

A sentence that tells a capable model to "write clear code" teaches what it already does; a skill built on it changes nothing. A sentence that tells it never to mask a flaky test with a retry teaches what it would otherwise shortcut under deadline, and earns its place.

## When to stop

Complete when every changed sentence has passed the deletion test in place, every completion criterion passes both tests, and every repair names the failure it answers. Return each standard above that applies as kept or broken, with the line that shows it.
