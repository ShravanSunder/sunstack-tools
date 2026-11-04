# Verification Design: owner assessment

4 Nov 2026. Author: 🦁 Lead. Status: draft, not reviewed. A plan: nothing in it has been observed.

## How we will know it works

| Need | Proven by | Unverified |
| --- | --- | --- |
| U1 one answer per unit | a keyless run over a real-shaped tree counts the model questions and reads each answer in the report | live model accuracy (the skills repository's kind census does that) |
| U2 rules judge only accepted answers | a mechanical and an asking gated rule on a unit outside `accept` run no step and report the reason | — |
| U3 unsure skips are listed | a controlled unsure answer gives no gated results, a listed skip and exit 5; a failed question gives exit 3 | — |
| U4 generic | the diff is read for skill words and caller option names | the existing resolver (out of scope) |

## Observations

- **OA · Library journeys** (`crates/sunstack-rules/tests/integration/owner_assessment.rs`, new; `crates/sunstack-lint/tests/integration/`): the real backend against `ControlledServer` (`controlled_http.rs:18–113`) with scripted Choice answers; requests counted on the server.
- **OB · Process journey** (`crates/sunstack/tests/e2e/`, new case beside `evidence_selection_process.rs`): the real binary on a fixture tree with two units and a reference-only path argument; exit codes and JSON report read.
- **OC · Admission tests** beside `rulebook_admission.rs` and `config_decoding.rs`: each R6 refusal by `--check`.
- **OD · The verify-sunstack-rule journey**: a new `owner-assessment` case in `scripts/verify/sunstack-rule/rulebook/rulebook-lint.ts`, run against the release candidate binary.
- **OE · Source read** of the diff for R7.

## Rows

| U · R | Passes when | Fails for | Observation | Unverified |
| --- | --- | --- | --- | --- |
| U1 · R1 | two gated rules over three files of unit A and one of unit B: exactly 2 questions reach the server; a unit no gated rule reads gets none; `assessments[]` has both answers with option, probability and model | a question per rule or per file; a missing answer | OA, OB | — |
| U1 · R2 | a run whose only path argument is A's reference file sends A's description in the question; a change-scoped run where only A's describing file changed selects A's members | the reference judged with no answer; members not picked | OB, OA | — |
| U2 · R3 | on unit B (answer outside `accept`) a mechanical gated rule and an asking gated rule give `not_applicable`, reason `assessment work_scope: one_app_procedures`, basis judged, and no rule question reaches the server | a mechanical step running; basis mechanical; a rule question sent | OA | — |
| U3 · R4 | a scripted answer below `min_probability` gives no result for gated rules on that unit, one skip per rule and file with the probabilities, exit 5; a server error on the assessment gives exit 3 and an incomplete part naming the assessment and unit; a file in no unit gives `no_owner` skips | a gated result on an unsure unit; exit 0 | OA, OB | — |
| U2, U4 · R5 | the existing rulebook-lint journeys' JSON reports are unchanged with units declared and no gates | any changed result | OB on existing fixtures, OD | — |
| U1, U2 · R6 | each listed refusal appears with its field path from `--check`, which exits 2 | a refusal missing or untagged | OC | — |
| U4 · R7 | the diff adds no `skill`, `SKILL.md` or caller option name to engine code | such a word in engine code | OE | — |
