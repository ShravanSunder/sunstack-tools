# Changelog

## Unreleased

### Changed

- `within` is now a nonempty list of distinct kind names. Rewrite a single kind
  as a one-item list, for example `"within": ["workflow_section"]`. A match inside
  any listed kind produces one piece, including when containers overlap.

### Fixed

- Config refusals name the full path of the setting (for example `limits.evidence_max_chars`).

- Check refusals for undefined kinds and kind cycles now point at the kind in
  the config. Cycle refusals name a deterministic path and point at its first kind.

## 0.3.0

Release date: 2026-10-30.

- Add noul-based evidence selection, kind containment, kind-based field sources and
  strict-child remainder exclusions. Selection and rules share model-request limits.
- Report every asked candidate and per-file/per-kind counts, including files with
  no candidates. Selected kinds require credentials for `--evidence`; `--check`
  remains credential-free. Uncertain and incomplete selection affect run status.
- Recognize numbered and Step section headings at H2 through H6, excluding H1.
  Configurations using those generic utilities can gain evidence within the new
  sections and changed fields or results that read them.
- Run the fresh-install smoke in TypeScript with a current rulebook fixture.

## 0.2.0

This pre-1.0 minor release cuts over the lint configuration and rulebook format.

- Replace lint's `rule_sets` configuration with marked Markdown `rulebook` pairs,
  checked-file globs, an ast-grep evidence project and `decision_model`. Rule
  entries run explicit mechanical and model decision steps on evidence pieces;
  asking rules always require a configured model and credential.
- Add `sunstack lint --check` for credential-free admission and rule listing,
  and `--evidence [KIND]` for inspecting picked evidence without model requests.
  `--init` writes a generic configuration skeleton.
- Report uncertain decisions with their step, sent state and answers. Exit 5
  means uncertainty when cancellation, incomplete work or failing findings do
  not take precedence.
- Remove native RuleSet `rule_sources`, `marker_prefix` and `max_question_words`.
  Native JSONL `rule_paths` and conversation operations remain supported. Existing
  lint configurations must migrate to the [current configuration](README.md#repository-lint);
  removed keys are refused with replacement guidance.
