# Choosing the Observation

This reference owns choosing, for each obligation, the evidence that can tell a correct build from a wrong one at the obligation's real boundary, and whether a journey already makes that observation. It doesn't choose test files, commands or run order; the plan does.

Expected inputs: each requirement with its obligations, references and target constraints; each obligation's seam from the Program Design trace table; and the journeys and real-path tests step 2 read.

Return: for each obligation, the evidence class, the boundary where its wrong behavior stays real, the target, the journey or test that will observe it (existing, needed with its entry and observation, or none), each journey's lifetime, and any `specification-gap` or `program-design-gap`.

## Evidence classes

```text
automated behavior
manual interaction or visual evidence
API or CLI transcript
state or data inspection
log, trace, or metric observation
security analysis or misuse case
performance measurement
release or runtime evidence
structural check: schema, type, lint, manifest or document
```

## Find the boundary, then the class

Start from the wrong build, not from a test layer. Name the wrong behavior the obligation must rule out, then find the smallest part of the system that has to run for that wrong behavior to show ([keep real the smallest thing responsible for the wrong behavior](../../principle-write-tests-that-can-fail/SKILL.md)). That boundary picks the class.

| Obligation | Wrong build it must catch | Where the wrong behavior stays real | Class |
| --- | --- | --- | --- |
| the layout holds at the supported viewport | the side panel overflows at 1280 px | the rendered screen at that size | visual evidence at that viewport |
| a retried submit is idempotent | the retry creates a second order | the stored orders after a duplicate submit | state inspection, not the returned status |
| a viewer can't delete a project | only the UI hides the button | the API's enforcement point, called by an allowed and a denied actor | API transcript for both actors |
| search answers within 300 ms at p95 | the large index is slow | the named endpoint under the named workload | performance measurement against that threshold |
| an outage pages on-call | the alert rule never fires | the page reaching its channel | metric and alert observation, not a log line |
| line totals round half-up to the cent | 0.125 rounds to 0.12 | the pricing rule alone | focused automated test, no app launch |
| the plugin manifest declares the new skill | the entry is missing | the manifest file | structural check, naming "the skill loads in a session" as unevaluated |

Bad choices:

- "unit tests" for cross-process wiring;
- "manual test" without the behavior and the observation;
- static schema validation for runtime ordering or recovery;
- mocked integration evidence presented as proof of production wiring;
- a success status or a row count for a durability claim;
- a new test file or command prescribed in the row, when the plan chooses those and a row names only an existing test it reuses.

## Journey: existing, needed or none

- **Existing:** a `verify-<app>` journey whose source drives this entry and reads this outcome on the planned target. When it observes the outcome but misses a case the obligation covers, such as the failure path or the denied actor, the row names the case to add.
- **Needed:** no journey reaches the entry or reads the outcome. Write `needed:` with the entry it must drive and what it must read.
- **None:** a pure rule or structural claim needs no journey.

## Lifetime: lasting or temporary

Each row that names a journey marks it lasting or temporary.

- **Lasting:** the outcome will be checked again, such as a user-facing behavior that can regress, a release check, or something other projects touch. The journey runs from a self-describing script in the project's own scripts location, and `verify-<app>` says which script proves it and when and why to run it. An existing journey is lasting. A lasting `needed:` journey is built before any observation that depends on it.
- **Temporary:** a one-time check. The Verifier runs it as a probe in the task's scratch beside its evidence, and deletes the probe once the evidence is kept.

A row checking that a one-time backfill set each old note's created date is temporary: once the release has run the backfill, nothing runs it again. The drafts row below is lasting.

A row with no journey, decided by a focused test, structural evidence or a reused real-path test, carries no lifetime and adds no script or probe.

## Worked rows

The names, paths and requirements below are illustrative.

### Durability across a restart

```text
requirement   U2 "my drafts are never lost": R4 a saved draft survives an app restart;
              R5 a reopened draft keeps its title
proven by     Save a draft in the installed app, quit it, reopen it, and see the same title and body.
reference     R4's worked example: title "Q3 plan", a 2,000-character body ending in an emoji
target        the installed release build on macOS, default profile, empty data directory
journey       verify-notes `drafts`: drives save, quit and relaunch; reads the editor's title and body
lifetime      lasting: any later storage change can lose drafts
passes when   after relaunch the editor shows exactly that title and body; a build that keeps drafts
              only in memory, or cuts the body at 1,024 characters, fails
reuses        none: `drafts-store.test.ts` swaps the disk for an in-memory store, so it never crosses a restart
unverified    Windows, which this cycle doesn't ship
```

The weak version of the same row:

```text
proven by     e2e coverage of drafts
passes when   the save request returns 201 and the draft count goes up
reuses        drafts-store.test.ts
```

A build that never writes to disk passes all three. The status and the count are acknowledgements, and the reused test replaces the very boundary R4 is about.

### Routing, with honest reuse

```text
requirement   U5 "text I send reaches the pane I chose": R11 text sent to pane B appears in
              pane B's input and nowhere else
proven by     Send text to pane B from the CLI and see it in pane B, not in the focused pane A.
reference     R11's example: "echo hi" sent to pane B while pane A has focus
target        the installed app at the release candidate, default configuration, the shipped CLI
journey       none added: the reused test drives the same entry
passes when   pane B shows "echo hi" and pane A shows nothing new; a router that writes to the
              focused pane, or replies ok before any pane write, fails
reuses        `pane-routing.e2e.ts`: drives the real CLI, router and panes and asserts both panes'
              contents; applies while it runs against the installed build and default configuration
unverified    remote panes over SSH
```

The row adds no manual journey, because the reused test already keeps the router real and fails for wrong routing. The router's `ok` reply is not the outcome. If the test had asserted only that reply, it would decide nothing here and the row would need the `pane-send` journey.

### A pure rule

```text
requirement   U7 "totals match the invoice": R15 line totals round half-up to the cent
proven by     A focused test checks R15's rounding table against the pricing rule.
reference     R15's table: 0.125 -> 0.13, 0.124 -> 0.12
target        the pricing module at the delivered commit
journey       none: no entry point, screen or configuration changes the rule
passes when   every entry in R15's table matches; a rule that rounds half to even (0.125 -> 0.12) fails
reuses        none; the plan adds the focused test
unverified    none: the totals screen only shows the rule's output, and U8's row observes that screen
```

### One observation, two rows

U3 "search finds my file" (R8 pressing Enter submits once; R9 results come from the real index) and U4 "I can see how many matched" (R10 the panel shows the match count) share the `search` journey: type a query, press Enter once, read the results and the count. Describe the observation once. U3's row names it for R8 and R9, U4's row names it for R10, and each row's passing condition says what it reads in that observation. If the journey runs against a fixture index, R9 is undecided, and U3's row has to show that rather than look covered because its journey exists.

## When a row can't be written

- **No seam to observe through.** R20 says a reminder fires at its due time, but the design leaves nothing a user or caller can see showing when it fired: no notification with its time, no readable record. No observation can tell on-time from late. Record a `program-design-gap`: the obligation, its trace row's seam, and the observable effect it needs. Waiting for a near due time on the planned target is a valid observation, not a gap.
- **No reference to judge against.** R21 says search "feels fast", with no threshold or workload, so no passing condition can fail a slow build. Record a `specification-gap` for the missing threshold and workload.
- **A harness that can't reach the boundary.** When the existing journey can't restart the app, "survives a restart" doesn't become "save returns 201". Keep the obligation, and write the journey as `needed:`.

Complete when every obligation has a class whose observation sits where its wrong behavior is real, a target, and an existing, needed or no journey, with a lifetime for each journey; every pure rule and structural claim has proportional proof; and every row that can't be written carries its gap.
