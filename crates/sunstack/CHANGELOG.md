# Changelog

## 0.5.0

Release date: 2026-10-31.

### Added

- Add rule-only `next` and `previous` evidence fields with a default `count` of 1
  and a range of 1 through 5. Neighbours share the match's parent and retain
  source order and spans; continuation nodes do not count or stop the walk.
- Let ask and combine steps route uncertain answers to another decision step
  using the same request's answers. Reports name the step that ended the walk.

### Breaking

- Incomplete provider-failure reasons require `failure_class` and carry
  `http_status` when received. Requests refused by the engine use
  `invalid_request`; answers that fail admission use `invalid_response`.
  The default line includes the provider class and any HTTP status.
- Rename selection's `malformed_answer` reason to `invalid_response` and carry
  provider class/status in incomplete selection candidates. Update report
  consumers for this hard cutover.

## 0.4.0

Release date: 2026-10-30.

### Breaking

- Set `within` to a nonempty list of distinct kinds, for example
  `["step", "standards_section"]`. A match counts once inside any listed kind,
  including overlapping containers. Rewrite `"within": "x"` as `"within": ["x"]`.
  A present `null` or any non-list is refused; omit the key for no containment.

### Fixed

- Check refusals for undefined kinds, cycles and `within` entries point at the
  actual kind or field, including names with dots. Cycle refusals name and point
  at the first kind in the cycle.
- Config refusals use the full setting path: `limits.evidence_max_chars`,
  `check.files`, `evidence.kinds`, `evidence.ast_grep`,
  `rulebook.<index>.markdown` and `rulebook.<index>.rules`.

### Internal

- Separate child startup waits from active test deadlines (#113).
- Group Rust crate modules by responsibility (#110).
- Batch repository metadata reads (#112).
- Run CI on PRs and main, with cache writes only from main (#109, #111).

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
