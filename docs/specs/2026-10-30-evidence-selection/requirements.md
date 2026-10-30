# Evidence selection: requirements

Skill rules judge evidence, and today every kind of evidence is cut by an ast-grep rule alone. For steps, that means a heading's *name* decides whether a numbered list is a workflow: plain numbered items count only under an H2 that starts with "Workflow", "Steps" or "Main path", and numbered stage headings count only at `###`. Measured on 30 Oct 2026 across all 74 skills on sunstack main `690dc5a7`, only 198 of 342 numbered steps became step evidence, and 13 skills got none at all, including `implement-plan`, `plan-implementation`, `orchestrator-implementation-goal`, `implementation-review` and `spec-program-review`. The completion rule never ran on those skills, and their reports looked clean. A longer name list wouldn't fix it: about 116 of the 144 missed items sit under workflow headings with other names ("Core Flow", "Plan the Change", "Hand off"), while 28 are numbered lists of rules or checks ("Core rules", "Structural Principles") that a broader "every numbered list" rule would wrongly take as steps. Telling the two apart needs a judgment about what a section is for.

This change lets the evidence system ask that question: ast-grep cuts candidates mechanically, the decision model chooses which candidates become evidence, and everything after works as it does today. It amends the [skill rules on evidence and decisions](../2026-10-29-skill-rules-evidence-and-decisions/requirements.md) design (U1–U12 stay in force). The [Specification](specification.md) defines the contract, the [Program Design](program-design.md) places it in the crates, and the [Verification Design](verification-design.md) plans its proof. The measurement is in the owner page *Skill-rules evidence coverage* (30 Oct) and its source report.

## Who it serves

| Who | Job | What's missing today |
| --- | --- | --- |
| The agent running the check (skill creation's self-check, skill review, skill audit) | Act on what the rules found | A skill whose workflow sits under an unrecognised heading produces no finding, which looks the same as a skill that passed |
| Rulebook author (the owner, or an Opus Worker writing the kit) | Say "the skill's workflow" once, however a skill names or levels it | The only tool is a heading-name list in an ast-grep rule, which misses every new name and can't tell rules from steps |
| The rules runner in sunstack `tests/skills` | Show which files each rule actually saw | The report has no per-file counts, so a silent skill is invisible |
| The owner | Trust that "no finding" means "checked" | Nothing distinguishes "checked and fine" from "never looked at" |

## The owner's words (30 Oct 2026)

- On the step-selector page, review comment: "pyou mean pre fitle rmechicnall headers -> use jev to find the header then do teh rest / this is abosltuely crucioual"
- "we alrey establsiehd after a reduciotn in ingest we can use jev for any filtering. if none are found, isnt that a issue we always want workflow secion? is that a core reule? why why not is the qeutiosn at taht stage right? jev is file for evidece candidates for requiremed headines and feautres. but i think we need to build out engien capabilties as we go thorugh and We need to go through and figure out what engine capabilities were missing as we make the creation of skills better, and then we need to validate the creation of skills itself"
- "so the way skilsl are evidece colelciotn seems not great? for skill rkules make me that report please"
- On the request shape: "oen request per rule right? not a jev proimivea s we hsoudl be usign https://docs.typesafe.ai/primitives/advanced"
- Accepting the work order whose step 4 is this design (evidence engine capabilities, the section selector, the spine rule, the request shape and a rule-validity plan, then an engine PR, a kit PR, a release and a rerun): "yes lets merge and finish 1 and 2 / tell me when 4 is done or get sun to ping me"

## Authorized needs

| ID | Need | Authority |
| --- | --- | --- |
| U13 | Evidence can be chosen by the decision model from candidates cut mechanically. ast-grep cuts the candidates with no AI; the decision model decides which candidates become evidence; fields, picking and rules then treat that evidence like any other. | "pre filter mechanically headers -> use jev to find the header then do the rest", "absolutely crucial"; "we can use jev for any filtering"; "jev is fine for evidence candidates for required headings and features" |
| U14 | Evidence can be defined relative to chosen evidence. A kind's matches can count only where they lie within another kind's evidence (steps are the numbered items inside the sections chosen as the workflow, whatever the heading says and at whatever level), and a piece's fields can draw on another kind's evidence in its file (such as which sections were chosen). | "then do the rest"; the measurement above (a name list misses 116 workflow items; "every numbered list" takes 28 rule items) |
| U15 | No evidence disappears silently. Every candidate the model was asked about is reported with its outcome and its heading, and the report counts, for each checked file and kind, the candidates and the evidence, so a file with none is visible. | "if none are found, isn't that an issue"; "the way skills' evidence collection seems not great?"; the step-selector page the owner reviewed, which proposed reporting non-workflow sections and per-file counts |
| U16 | Choosing evidence follows the rules that judging it does. Candidates reach the model in bounded pieces under size limits; the answer goes through thresholds; an answer between them is uncertain and is handed back, never treated as chosen or not chosen; a provider failure is incomplete; nothing switches it off. | Carries U2, U5, U7 and U8 to selection |
| U17 | The standard util library recognises numbered stage headings and "Step" headings at every section level, not only `###`. | U3 ("widely common" pieces ship with the CLI); the measurement (nine numbered `##` stages in `spec-program-review` were missed) |

## What carries over

| Earlier row | Here |
| --- | --- |
| 2026-10-29 U1–U12 | in force; U2's size limits, U5's thresholds, U7's "no switch" and U8's uncertain hand-back extend to selection through U16 |
| 2026-10-29 U3 (common pieces in the standard library) | extended by U17 |
| 2026-10-29 U6 (a rule's questions about one piece go in one request) | unchanged. Sending every rule's questions about one piece in one request, as the decision model's documentation recommends, is not part of this change: it changes what the model sees for rules already calibrated on labelled cases, so it needs a measured answer-stability check first (see Non-goals) |

## Non-goals

- **The kit's content.** Which candidates are workflow sections, the question's wording, the step, branch, sentence and completion conventions, and a rule for the all-run spine (`main-path.md`: "`SKILL.md` shows the all-run spine, from load to completion, in one scan") belong to sunstack's kit change, which consumes this.
- **One request per piece for every rule on it.** A follow-up, after an experiment on the labelled rule cases shows whether answers stay the same.
- **Choice or score questions for selection.** A skill can have several workflow sections, so selection asks one yes/no question per candidate.
- **Validating the creation rules** (labels, held-out cases, spot-checks). That is the next step of the owner's work order, in sunstack.
- **HTML evidence, the session pipeline, and an eval harness in Rust.** Unchanged from 2026-10-29.
- **Releasing or tagging the CLI.** The owner gives the go for a release.

## Goal boundary

- **Goal:** `sunstack lint` can choose evidence with the decision model from mechanical candidates and define evidence relative to chosen evidence, with every candidate's outcome and per-file counts visible, so the kit can find steps by asking "is this the workflow?" instead of listing heading names.
- **Reuse:** everything the 2026-10-29 design built: the evidence project and standard util library, field sources, model views and cuts, the decision model's question schema and thresholds, credential resolution, change scopes, the check and the report.
- **May change:** the sunstack-tools crates `sunstack`, `sunstack-lint`, `sunstack-rules`, `sunstack-evidence` and `sunstack-decisions`, and the standard util library's data files.
- **Protected:** a config that uses none of the new settings keeps 0.2.0's pieces, requests and results, except where its evidence project matches numbered or "Step" headings at a level other than `###` (U17); the `exec` protocol's JSONL and conversation rules; credential storage; Demistify; the session-pipeline branches.
- **Complexity:** no new rule language; selection reuses the question and threshold contract; no cache or store of selections across runs.
- **Evidence of done:** on a local build, a kit config that chooses workflow sections and their steps runs over every skill with each candidate's outcome reported, the 13 step-silent skills get steps, the check validates selection settings, and a config without the new settings gives the same results as on 0.2.0 apart from the U17 headings.
