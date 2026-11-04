# Specification: filters

4 Nov 2026. Author: 🦁 Lead. Status: draft; renamed from the reviewed owner-assessment draft (`9116442`) to the owner's final vocabulary, plus R8. Answers to [requirements.md](requirements.md).

The config a caller writes, with the skills repository's filter as the example (the engine knows none of these names):

```json
"filters": {
  "kind": {
    "per": "plugins/*/skills/{name}",
    "reads": { "file": "SKILL.md", "kind": "skill", "field": "description" },
    "question": { "type": "choice", "instructions": { "task": "What is this skill's work for?" },
                  "criteria": { "general": "…", "role": "…", "domain": "…", "tool_manual": "…", "repo_specific": "…" } },
    "min_probability": 0.7
  }
}
```

and on a rule entry, `"only_when": { "kind": ["general", "role", "domain"] }`.

## Entities

### E1: Filter folder

`kind: entity · basis: U1, U4`

| Slot | Meaning |
| --- | --- |
| identity rule | one filter plus the folder path its `per` pattern matched (relative to the repository root). The `{name}` capture is that folder's display name, not its identity: two skills named `review` in different plugins are two folders. Two files are in the same folder when both resolve to the same filter and folder |
| relationships | has the filter's `reads` file (relative to the folder) and, when well formed, exactly one piece of the `reads` kind in that file; has 1..n member files (every checked file under the folder); has at most one answer per run |
| invariants | a file belongs to at most one folder of a given filter, because `per` has a fixed number of segments (literal, `*`, or the one `{name}`; no `**`) |
| observable states | `described` (exactly one piece to read), `undescribed` (none), `ambiguous` (more than one) |

### E2: Filter

`kind: entity · basis: U1`

| Slot | Meaning |
| --- | --- |
| identity rule | one declared filter name |
| relationships | has one `per` pattern and one `reads` (file, evidence kind, field); has one choice question whose 2..n criteria labels are its answers; has one `min_probability`; is answered by the run's decision model (owner decision 8) |
| invariants | its question is a choice question in the same shape rule questions use (`instructions` plus `criteria`; no other top-level key) |

### E3: Filter answer

`kind: entity · basis: U1, U3`

| Slot | Meaning |
| --- | --- |
| identity rule | one filter, one folder, one run |
| relationships | read by every gated rule on that folder's files in that run |
| invariants | asked at most once per run; never reused from another run, even in the same process with the same filter and folder |
| observable states | `judged` (the response's selected answer has probability at or above `min_probability`, equality included), `unsure` (the selected answer's probability is below it, whatever the other answers' probabilities), `incomplete` (the question failed, or the folder is undescribed or ambiguous) |

### E4: Gate

`kind: entity · basis: U2`

| Slot | Meaning |
| --- | --- |
| identity rule | one rule entry's `only_when` |
| relationships | names exactly one filter and 1..n of its answers |

### E5: Filter skip

`kind: entity · basis: U3`

| Slot | Meaning |
| --- | --- |
| identity rule | one gated rule, one file, one run |
| observable states | `unsure`, `incomplete`, `no_folder` |

### E6: Evidence kind's rule

`kind: entity · basis: U5`

| Slot | Meaning |
| --- | --- |
| identity rule | one evidence kind's matcher |
| relationships | is exactly one of: a named rule in the evidence project (`rule`), or an inline rule (`inline_rule`, one YAML string in the JSON config) |
| invariants | an inline rule matches exactly as the same rule written in a rule file would; it is part of the evidence project's inputs, so a change to it changes them |

## Obligations

### R1: One question per folder per run

`kind: obligation · links: U1, E1, E2, E3, V1`

For each filter, a lint run asks its question once for each folder that has at least one checked file a gated rule reads, using the folder's `reads` piece, and asks it for no other folder. The report lists every answer with its folder, filter, state, chosen answer and probability when judged, and the model.

### R2: A member reaches its folder's answer

`kind: obligation · links: U1, E1, E3, V2`

When the checked files include a folder's member but not its `reads` file, the run still reads that file, through the same source and limits as checked files, and asks about the folder; the `reads` file gives no rule results unless it is itself checked. A change to the `reads` file brings the folder's members into a change-scoped run, including members of selected kinds.

### R3: The gate decides before any step

`kind: obligation · links: U2, E4, E3, V3`

Before a gated rule's first step, mechanical or asking, it reads its file's folder answer. Answer judged and named in `only_when`: the rule runs as today. Answer judged and not named: the result is `not_applicable` with the reason `filter <name>: <answer>`, its basis is judged, and no step runs.

### R4: Unsure, incomplete and folderless files are skipped and listed

`kind: obligation · links: U3, E5, V4`

If a gated rule's file has an `unsure` or `incomplete` answer, or lies in no folder of the gate's filter, then the rule gives no result for that file, and the report lists a skip with the rule, file, filter, folder when there is one, the reason, and the answers' probabilities when there are any. An `unsure` answer makes the run uncertain (exit 5) unless it is already failed, incomplete or cancelled. An `incomplete` answer makes it incomplete (exit 3).

### R5: Ungated rules are unchanged

`kind: obligation · links: U2, U4, V5`

A rule without `only_when` behaves exactly as before on every file, whether or not filters are declared. An evidence kind with a named `rule` behaves exactly as before.

### R6: Bad declarations are refused

`kind: obligation · links: U1, U2, U5, V6`

`--check` and a real run refuse, with a path-tagged reason: a `per` pattern without exactly one `{name}` segment, or with `**`; a `reads` with no file, or naming an evidence kind or field that doesn't exist; a question that isn't a choice with at least two criteria; `min_probability` outside (0, 1]; an `only_when` with no filter or more than one, naming an unknown filter, or whose answers are empty, repeat, or name an answer the filter's question doesn't have; an evidence kind with both `rule` and `inline_rule`, or neither; an `inline_rule` that doesn't parse as one ast-grep rule, or that declares its own `id`. A refused rule entry is skipped as today, which makes `--check` invalid.

### R7: Nothing skill-specific in the engine

`kind: obligation · links: U4, V7 · status: authorized; scope set by owner decision 9`

The change adds no reference to skills, `SKILL.md`, or any caller's answer names to engine code, and the schema names only generic terms. The existing `Skill`/`PathOrSkill` target resolver is outside this release by the owner's decision 9 ("Later work, logged"), which supersedes that part of the accepted R5.

### R8: An inline rule matches like a named one

`kind: obligation · links: U5, E6, V8`

An evidence kind may give `inline_rule`, a one-line YAML string holding one ast-grep rule (language, rule, and optional constraints or utilities, as in a rule file, without `id`). The kind collects exactly the pieces it would collect if the same rule sat in the evidence project under the kind's name, and field sources and `within` treat that kind as they treat any other.

## Proof obligations

| V | For | What decides pass and fail | Reference |
| --- | --- | --- | --- |
| V1 | R1 | two gated rules over three files of one folder cause one question; two folders cause two; a folder no gated rule reads causes none; the report lists each answer; a second run in the same process with a changed description sends a fresh question and reports the new answer | R1; E3 |
| V2 | R2 | a run over only a member file asks about its folder from the `reads` file; a change to the `reads` file selects the members | R2 |
| V3 | R3 | a mechanical gated rule whose folder answer `only_when` doesn't name runs no step and reports the reason with basis judged; with a named answer a mechanical rule gives its normal outcome and an asking rule sends its question and gives its result | R3 |
| V4 | R4 | an unsure answer gives no gated results, one listed skip, exit 5; a failed question gives exit 3; a file in no folder gives a `no_folder` skip | R4 |
| V5 | R5 | an existing rulebook without gates gives byte-identical results with and without filters declared, including an ungated choice step that routes to `not_applicable` with its answers (basis judged) | the existing journeys' outputs |
| V6 | R6 | each refusal in R6 is produced, path-tagged, by `--check` | R6 |
| V7 | R7 | the diff adds no skill word or caller answer name to engine code | the diff |
| V8 | R8 | a kind with an inline rule and the same kind pointing to the same rule in a rule file collect byte-identical pieces and fields on the same fixture files | the named-rule run |
