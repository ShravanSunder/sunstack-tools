# Specification: owner assessment

4 Nov 2026. Author: 🦁 Lead. Status: draft, not reviewed. Answers to [requirements.md](requirements.md).

## Entities

### E1: Owner unit

`kind: entity · basis: U1, U4`

| Slot | Meaning |
| --- | --- |
| identity rule | one declared unit kind plus the folder path its root pattern matched (relative to the repository root). The `{owner}` capture is that folder's display name, not its identity: two skills named `review` in different plugins are two units. Two files are in the same unit when both resolve to the same kind and folder |
| relationships | has one declared describing file (relative to its folder) and, when well formed, exactly one describing piece of the declared evidence kind and field in that file; has 1..n member files; has at most one answer per assessment per run |
| invariants | a file belongs to at most one unit of a given unit kind, because a root pattern has a fixed number of segments (literal, `*`, or the one `{owner}`; no `**`) |
| observable states | `described` (exactly one describing piece), `undescribed` (none), `ambiguous` (more than one) |

### E2: Assessment

`kind: entity · basis: U1`

| Slot | Meaning |
| --- | --- |
| identity rule | one declared assessment name |
| relationships | asks about exactly one unit kind; has 2..n options; has one `min_probability`; names its decision model, or uses the run's (open for the owner: see Requirements) |
| invariants | its question is a choice question in the same shape rule questions use |

### E3: Owner answer

`kind: entity · basis: U1, U3`

| Slot | Meaning |
| --- | --- |
| identity rule | one assessment, one unit, one run |
| relationships | read by every gated rule on that unit's files in that run |
| invariants | asked at most once per run; never reused from another run, even in the same process with the same assessment and folder |
| observable states | `judged` (the response's selected option has probability at or above `min_probability`, equality included), `unsure` (the selected option's probability is below it, whatever the other options' probabilities), `incomplete` (the question failed, or the unit is undescribed or ambiguous) |

### E4: Gate

`kind: entity · basis: U2`

| Slot | Meaning |
| --- | --- |
| identity rule | one rule entry's `requires` |
| relationships | names exactly one assessment and 1..n of its options |

### E5: Assessment skip

`kind: entity · basis: U3`

| Slot | Meaning |
| --- | --- |
| identity rule | one gated rule, one file, one run |
| observable states | `unsure`, `incomplete`, `no_owner` |

## Obligations

### R1: One question per unit per run

`kind: obligation · links: U1, E1, E2, E3, V1`

For each assessment, a lint run asks its question once for each unit that has at least one checked file a gated rule reads, using the unit's describing piece, and asks it for no other unit. The report lists every answer with its unit, assessment, state, chosen option and probability when judged, and the model.

### R2: A member reaches its unit's answer

`kind: obligation · links: U1, E1, E3, V2`

When the checked files include a unit's member but not its describing file, the run still reads the describing file, through the same source and limits as checked files, and asks about the unit; the describing file gives no rule results unless it is itself checked. A change to the describing file brings the unit's members into a change-scoped run, including members of selected kinds.

### R3: The gate decides before any step

`kind: obligation · links: U2, E4, E3, V3`

Before a gated rule's first step, mechanical or asking, it reads its file's unit answer. Answer judged and in `accept`: the rule runs as today. Answer judged and outside `accept`: the result is `not_applicable` with the reason `assessment <name>: <option>`, its basis is judged, and no step runs.

### R4: Unsure, incomplete and ownerless files are skipped and listed

`kind: obligation · links: U3, E5, V4`

If a gated rule's file has an `unsure` or `incomplete` answer, or belongs to no unit of the assessment's kind, then the rule gives no result for that file, and the report lists a skip with the rule, file, assessment, unit when there is one, the reason, and the answers when there are any. An `unsure` answer makes the run uncertain (exit 5) unless it is already failed, incomplete or cancelled. An `incomplete` answer makes it incomplete (exit 3).

### R5: Ungated rules are unchanged

`kind: obligation · links: U2, U4, V5`

A rule without `requires` behaves exactly as before on every file, whether or not units and assessments are declared.

### R6: Bad declarations are refused

`kind: obligation · links: U1, U2, V6`

`--check` and a real run refuse, with a path-tagged reason: a unit root pattern without exactly one `{owner}` segment, or with `**`; a unit with no describing file; a unit with no members; a describing kind or field that doesn't exist; an assessment naming an unknown unit; an assessment question that isn't a choice with at least two options; `min_probability` outside (0, 1]; a gate naming an unknown assessment; a gate whose `accept` is empty, repeats an option, or names an option the assessment doesn't have. A refused rule entry is skipped as today, which makes `--check` invalid.

### R7: Nothing skill-specific in the engine

`kind: obligation · links: U4, V7 · status: authorized as the accepted R5 states it; its scope over the existing resolver is open for the owner`

The change adds no reference to skills, `SKILL.md`, or any caller's option names to engine code, and the schema names only generic terms. The accepted skills design (R5 at `2a94295a`) also asks that the release carry nothing skill-specific, which covers the existing `Skill`/`PathOrSkill` target resolver. Until the owner decides whether this release makes that resolver caller-declared or leaves it as later work (Requirements, open item 2), this obligation stands as accepted, and the plan cannot mark it met while the resolver remains.

## Proof obligations

| V | For | What decides pass and fail | Reference |
| --- | --- | --- | --- |
| V1 | R1 | two gated rules over three files of one unit cause one question; two units cause two; a unit no gated rule reads causes none; the report lists each answer; a second run in the same process with a changed description sends a fresh question and reports the new answer | R1; E3 |
| V2 | R2 | a run over only a member file asks about its unit from the describing file; a change to the describing file selects the members | R2 |
| V3 | R3 | a mechanical gated rule outside `accept` runs no step and reports the reason with basis judged; inside `accept` a mechanical rule gives its normal outcome and an asking rule sends its question and gives its result | R3 |
| V4 | R4 | an unsure answer gives no gated results, one listed skip, exit 5; a failed question gives exit 3; a file in no unit gives a `no_owner` skip | R4 |
| V5 | R5 | an existing rulebook without gates gives byte-identical results with and without unit declarations, including an ungated choice step that routes to `not_applicable` with its answers (basis judged) | the existing journeys' outputs |
| V6 | R6 | each refusal in R6 is produced, path-tagged, by `--check` | R6 |
| V7 | R7 | the diff adds no skill word or caller option name to engine code; and, unless the owner accepts the resolver as later work, the release has no skill-specific resolver | the diff; the owner decision |
