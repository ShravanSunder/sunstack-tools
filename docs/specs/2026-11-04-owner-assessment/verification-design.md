# Verification Design: owner assessment

4 Nov 2026. Author: 🦁 Lead. Status: draft, not reviewed. A plan: nothing in it has been observed.

## How we will know it works

| Need | Proven by | Unverified |
| --- | --- | --- |
| U1 one answer per unit | a keyless run over a real-shaped tree counts the model questions and reads each answer in the report | live model accuracy (the skills repository's kind census does that) |
| U2 rules judge only accepted answers | a mechanical and an asking gated rule on a unit outside `accept` run no step and report the reason | — |
| U3 unsure skips are listed | a controlled unsure answer gives no gated results, a listed skip and exit 5; a failed question gives exit 3 | — |
| U4 generic | the diff is read for skill words and caller option names | the existing resolver, pending the owner decision |

## Observations

- **OA · Library journeys** (`crates/sunstack-rules/tests/integration/owner_assessment.rs`, new; `crates/sunstack-lint/tests/integration/`): the real backend against `ControlledServer` (`controlled_http.rs:18–113`) with scripted Choice answers; requests counted on the server.
- **OB · Process journey** (`crates/sunstack/tests/e2e/`, new case beside `evidence_selection_process.rs`): the real binary on a fixture tree with two units and a reference-only path argument; exit codes and JSON report read.
- **OC · Admission tests** beside `rulebook_admission.rs` and `config_decoding.rs`: each R6 refusal by `--check`.
- **OD · The verify-sunstack-rule journey**: a new `owner-assessment` case in `scripts/verify/sunstack-rule/rulebook/rulebook-lint.ts`, run against the release candidate binary.
- **OE · Source read** of the diff for R7.

## Rows

| U · R | Passes when | Fails for | Observation | Unverified |
| --- | --- | --- | --- | --- |
| U1 · R1 | two gated rules over three files of unit A and one of unit B: exactly 2 questions reach the server; a unit no gated rule reads gets none; `assessments[]` has both answers with option, probability and model. Then, in the same process, a second library run with the same names and a changed description: a fresh question carries the new description, and the report has the new scripted answer | a question per rule or per file; a missing answer; an answer cached across runs | OA | which models assessments use, until the owner settles it |
| U1 · R1 (activation) | a rulebook of only mechanical gated rules, no selected kinds: with a key, the assessment question reaches the server; without a key, the run refuses before acquiring files; `--check` stays keyless | the assessment refused for lack of a backend | OB | — |
| U1 · R2 | a run whose only path argument is A's reference file sends A's description in the question; a change-scoped run where only A's describing file changed selects A's members, including a member whose evidence kind has `select`, which must reach the selection question (not only the final picker) | the reference judged with no answer; members not picked; a selected-kind member dropped because the dependency was added only at final picking | OB, OA | — |
| U2 · R3 | on unit B (answer outside `accept`) a mechanical gated rule and an asking gated rule give `not_applicable`, reason `assessment work_scope: one_app_procedures`, basis judged, and no rule question reaches the server; on unit A (answer inside `accept`) the mechanical rule gives its fixture's expected outcome and the asking rule sends its question and gives its result; the unit test beside `rulebook_run.rs` shows excluded pairs are never walked | an always-exclude build; a walk whose result is discarded; basis mechanical | OA; unit test | — |
| U3 · R4 | a scripted response selecting A with `{A: 0.3, B: 0.7}` at `min_probability` 0.7 is unsure; one selecting A at exactly 0.7 is judged; an unsure unit gives no gated results and one skip per rule and file (a file with three section pieces gives one), with probabilities, exit 5; a server error on the assessment gives exit 3 and an incomplete part naming the assessment and unit; a file in no unit gives `no_owner` skips | a gated result on an unsure unit; B substituted for A; three skips for one file; exit 0 | OA, OB | — |
| U2, U4 · R5 | the existing rulebook-lint journeys' JSON reports are unchanged with units declared and no gates, including an ungated choice step routing to `not_applicable` with basis judged | any changed result; that result turned mechanical | OB on existing fixtures, OD | — |
| U1, U2 · R6 | each listed refusal appears with its field path from `--check`, which exits 2, including a root `**/{owner}` and a unit with no describing file | a refusal missing or untagged | OC | — |
| U4 · R7 | the diff adds no `skill`, `SKILL.md` or caller option name to engine code | such a word in engine code | OE | the existing resolver, until the owner settles it |
