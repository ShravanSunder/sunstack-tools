# Verification Design: filters

4 Nov 2026. Author: 🦁 Lead. Status: draft; renamed from the reviewed owner-assessment draft (`9116442`), plus U5 · R8. A plan: nothing in it has been observed.

## How we will know it works

| Need | Proven by | Unverified |
| --- | --- | --- |
| U1 one answer per folder | a keyless run over a real-shaped tree counts the model questions and reads each answer in the report | live model accuracy (the skills repository's kind census does that) |
| U2 rules judge only the answers they name | a mechanical and an asking gated rule on a folder whose answer `only_when` doesn't name run no step and report the reason | — |
| U3 unsure skips are listed | a controlled unsure answer gives no gated results, a listed skip and exit 5; a failed question gives exit 3 | — |
| U4 generic | the diff is read for skill words and caller answer names | the existing resolver (later work, owner decision 9) |
| U5 inline rule | the same rule inline and in a rule file collect the same pieces | — |

## Observations

- **OA · Library journeys** (`crates/sunstack-rules/tests/integration/filters.rs`, new; `crates/sunstack-lint/tests/integration/`): the real backend against `ControlledServer` (`controlled_http.rs:18–113`) with scripted Choice answers; requests counted on the server.
- **OB · Process journey** (`crates/sunstack/tests/e2e/`, new case beside `evidence_selection_process.rs`): the real binary on a fixture tree with two folders and a reference-only path argument; exit codes and JSON report read.
- **OC · Admission tests** beside `rulebook_admission.rs`, `config_decoding.rs` and `evidence_project_loading.rs`: each R6 refusal by `--check`.
- **OD · The verify-sunstack-rule journey**: a new `filters` case in `scripts/verify/sunstack-rule/rulebook/rulebook-lint.ts`, run against the release candidate binary.
- **OE · Source read** of the diff for R7.
- **OF · Evidence integration** (`crates/sunstack-evidence/tests/integration/`, beside `evidence_selection.rs`): two evidence listings of the same fixture files, one kind inline and one naming the same rule from a rule file.

## Rows

| U · R | Passes when | Fails for | Observation | Unverified |
| --- | --- | --- | --- | --- |
| U1 · R1 | two gated rules over three files of folder A and one of folder B: exactly 2 questions reach the server; a folder no gated rule reads gets none; `filter_answers[]` has both answers with answer, probability and model. Then, in the same process, a second library run with the same names and a changed description: a fresh question carries the new description, and the report has the new scripted answer. A filter whose `reads.field` is `folder` sends both values: the captured request state has `where.folder` and the caller's `folder` text | a question per rule or per file; a missing answer; an answer cached across runs; either `folder` value overwritten | OA | — |
| U1 · R1 (activation) | a rulebook of only mechanical gated rules, no selected kinds: with a key, the filter question reaches the server; without a key, the run refuses before acquiring files; `--check` stays keyless | the filter refused for lack of a backend | OB | — |
| U1 · R2 | a run whose only path argument is A's reference file sends A's description in the question; a change-scoped run where only A's `reads` file changed selects A's members, including a member whose evidence kind has `select`, which must reach the selection question (not only the final picker) | the reference judged with no answer; members not picked; a selected-kind member dropped because the dependency was added only at final picking | OB, OA | — |
| U2 · R3 | on folder B (answer not named by `only_when`) a mechanical gated rule and an asking gated rule give `not_applicable`, reason `filter kind: repo_specific`, basis judged, and no rule question reaches the server; on folder A (answer named) the mechanical rule gives its fixture's expected outcome and the asking rule sends its question and gives its result; the unit test beside `rulebook_run.rs` shows excluded pairs are never walked | an always-exclude build; a walk whose result is discarded; basis mechanical | OA; unit test | — |
| U3 · R4 | a scripted response selecting A with `{A: 0.3, B: 0.7}` at `min_probability` 0.7 is unsure; one selecting A at exactly 0.7 is judged; an unsure folder gives no gated results and one skip per rule and file (a file with three section pieces gives one), with probabilities, exit 5; a server error on the filter question gives exit 3 and an incomplete part naming the filter and folder; a file in no folder gives `no_folder` skips and exit 3 | a gated result on an unsure folder; B substituted for A; three skips for one file; exit 0 | OA, OB | — |
| U2, U4 · R5 | the existing rulebook-lint journeys' JSON reports are unchanged with filters declared and no gates, including an ungated choice step routing to `not_applicable` with basis judged; every existing named-rule kind collects as before | any changed result; that result turned mechanical | OB on existing fixtures, OD | — |
| U1, U2, U5 · R6 | each listed refusal appears with its field path from `--check`, which exits 2, including a `per` of `**/{name}`, an `only_when` with two filters, a filter question with a top-level `task` (the native `instructions` form is admitted and reaches the request unchanged), a kind with both `rule` and `inline_rule`, an inline rule declaring `id`, and a malformed inline rule in the second of two kinds, reported at `evidence.kinds.<second>.inline_rule` while a broken named rule file keeps its file and line | a refusal missing or untagged; an inline refusal reported at line 1 of a rule file or at the first kind | OC | — |
| U4 · R7 | the diff adds no `skill`, `SKILL.md` or caller answer name to engine code | such a word in engine code | OE | the existing resolver (later work, owner decision 9) |
| U1, U3 · R9 | on real skills pinned to a commit with controlled responses: a `SKILL.md` without a description, one whose description is a list, and one with malformed frontmatter each give `incomplete` with the matching reason and no request on the server; a description longer than the kind's `max_chars` is sent marked cut and its answer has `input_cut`; a judged answer in the report carries every probability and both model names | a request sent for an absent description; a kind guessed from the folder name; a cut input reported as whole | OA, OC | — |
| U5 · R8 | the inline and named listings are byte-identical in pieces and fields, including a field that reads the inline kind through `enclosing` and a kind `within` it; editing only the inline string changes `EvidenceProject.inputs`' bytes | a piece or field differs; the inline kind unusable as a source; an inline edit invisible to the inputs | OF | — |
