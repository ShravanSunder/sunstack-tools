# Changelog

## 0.22.0

Release date: 2026-11-14.

- Resolve a path target's `as_kind` at its fragment, including cached target files
  and containing pieces. Missing anchored kinds are unavailable with
  `target_kind_missing`; file openings and no-fragment behavior stay unchanged.
- Add `containing` to `document` and `same_file` fields. A scalar or list-valued
  subject field filters raw matches by the shared whole-token rule. Empty lists
  remain available facts, and unavailable needles propagate. Candidate-path
  resolution, raw spans and old configurations retain their behavior.
- Markdown-island pieces can read enclosing HTML rule/kind context and native
  heading paths, plus HTML section prefixes, using original source containment.
  Other ordinary cross-language directions remain refused.
- Align the SDK and six platform packages to 0.22.0 / contract revision 16,
  regenerating declarations from the Rust field and unavailable-reason contracts.
## 0.21.2

Release date: 2026-11-14.

- Publish the TypeScript SDK and its six native addon packages from the CLI's
  release tag using npm trusted publishing, after all addon builds and candidate
  installs pass. PR CI dry-runs all seven packages and checks their contents and
  platform metadata. Release smoke installs the SDK by version from npm on
  Linux and macOS. First publication requires owner bootstrap and per-package
  trusted-publisher setup before enabling the SDK publishing variable.
- Align the CLI, SDK and six addon packages to 0.21.2; contract revision 15 stays.

## 0.21.1

Release date: 2026-11-14.

- Bind every exact marker in a trailing same-line run to the same existing
  Markdown host and comment-free standard text. Spaces and tabs may separate
  markers; heading adjacency is checked before the first marker. Each marker
  retains its own original source coordinates. Escapes, code exclusions, prefix
  selection and duplicate-ID refusals still apply. Existing single-marker
  bindings and public contract shapes remain unchanged.
- Align the SDK and six platform packages to 0.21.1; contract revision 15 stays.

## 0.21.0

Release date: 2026-11-14.

- Add `word_count` and `piece_count` decision tools with exclusive `at_least`,
  `at_most` and `greater_than` integer bounds. A scalar field name or nonempty
  list of distinct names selects values to sum. Word counts use complete
  captured text content with markup removed, including inline HTML text
  attributes. Counts are independent of asking-view cuts and pending siblings;
  unavailable measured fields stay incomplete. Piece counts count captured list
  entries, including empty text.
- Scoped fields add `section_own`, a nonempty list of configured outline kinds.
  It ends the nearest section at the next configured heading regardless of level,
  including HTML/Markdown boundaries, so parents cannot borrow child pictures.
  Existing section/prefix scopes retain their meanings. Rust `SectionScope`
  adds `Own`; callers use `kinds()` instead of the former scalar `kind()`.
- Findings support `{{word_count:field}}` and `{{piece_count:field}}`, including
  comma-separated field sums. Existing raw fields, spans and result shapes stay
  unchanged. The README adds an executable HTML/Markdown section-picture rule.
- Rust `DecisionMechanicalTool` adds `WordCount` and `PieceCount`, with
  `MeasureFields` and `CountBound` contracts. The matching TypeScript SDK and six
  addons use release 0.21.0 / contract revision 15; earlier saved revisions are
  explicit incompatibilities.
- Manually constructed Rust `DecisionGraph` values supply `html_settings`;
  admitted graphs inherit the project's existing HTML text conventions.

## 0.20.1

Release date: 2026-11-14.

- The 0.20.0 release build failed; republished as 0.20.1. CLI distribution
  builds only its selected package, excluding the SDK's Node cdylib on musl.
  The SDK's independent addon workflow is unchanged.

## 0.20.0

Release date: 2026-11-13.

- `lint --init` writes a complete mechanical starter: config, rulebook, rule
  entry, ast-grep project and sample. Existing config or starter paths are
  refused without overwriting files. The sample runs locally with no model
  requests; the README includes executable picture and requirement-ID examples.
- Missing `decision_model` diagnostics explain its purpose and that a configured
  model name sends no requests for mechanical-only rules. The field remains
  required; asking and selection behavior is unchanged.
- Successful human runs with warnings explain exit 0 and whether an explicit
  `--max-warnings` limit was set. JSON status, report shape and exit policies are
  unchanged. Rust callers can use `render_with_warning_limit` to pass their limit.
- Document fields accept ordered candidate paths, choosing the first existing
  file and recording its path. Missing documents have a `missing_document` state
  with candidate paths; `complete` returns no so a rule can skip or nudge.
  Read, containment and budget failures remain incomplete. Scalar successful
  document fields retain their existing text-list output. Rust `FieldSource`
  uses `DocumentCandidates`; `FieldValue` adds `Document` and `MissingDocument`.
- The TypeScript SDK and its six platform packages align with 0.20.0. Generated
  declarations and exact codec metadata carry both document states and candidate
  paths under SDK contract revision 14. Older saved revisions remain explicit
  incompatibilities; no migration is guessed.

## 0.19.0

Release date: 2026-11-13.

- Add scoped `inside` fields collecting final pieces of a configured kind from
  the subject's nearest containing section or its prefix. Matching crosses
  evidence languages through original spans; fields retain full raw piece texts
  and provenance in source order. Empty results, unavailable context and pending
  selections remain distinct. Existing unscoped fields are unchanged.
- Rust `FieldSource` adds `ScopedInside { kind, scope }`, serialized as
  `from: "inside"` with a required `scope`. Exhaustive consumers must handle it;
  the public `SectionScope` enum selects `section` or `section_prefix`.

## 0.18.0

Release date: 2026-11-13.

- Add opt-in `evidence.html.embedded_markdown` selectors for HTML tags and exact
  attribute values. Markdown AST rules and native units can inspect each selected
  element's inner source, retaining original HTML identity, byte ranges and
  line/column coordinates. Markdown section and sibling fields stay in their
  island; fenced Mermaid diagrams use the existing Markdown fence kind.
- Configured catalogs interleave HTML and Markdown pieces by file and source
  start. Omitted settings preserve existing selection, serialized defaults and
  kind-first ordering. Only closed, outermost selected elements produce islands;
  the engine has no built-in wrapper or MIME-type convention.
- Rust callers constructing `HtmlEvidenceSettings` directly must supply the new
  `embedded_markdown` field or use defaults. JSON configs can omit the field.

## 0.17.0

Release date: 2026-11-13.

- Add opt-in `{"from":"section_prefix","kind":"configured_kind"}` fields:
  exact source bytes before each subject in its nearest configured containing
  section, with original provenance, available empty text, ordinary projection,
  cut accounting and selection dependencies. Existing field modes are unchanged.
  Public Rust `FieldSource` adds `SectionPrefix { kind }`; exhaustive consumers
  must handle it. The field retains its span but does not add an AST-to-field join.
- `lint --report FILE cases ...` now saves the complete existing case report,
  preserving stdout and result exits; a failed save uses ordinary report error
  handling (exit 3). No additional model work is dispatched.

## 0.16.0

Release date: 2026-11-13.

- Selection request candidates and shared views now expose `cut-fields`, using
  the existing rule-request field-name list and omitting it when empty. The
  list comes from the bounded model view; inline cut markers and text stay
  unchanged. Authored marker text does not create cut metadata.
- This changes saved cut-bearing selection requests and their hashes.
  `--selections-from` cannot reuse an older selection answer when the complete
  request hash differs; that selection remains unanswered during preparation.
  Uncut requests with identical hashes remain eligible under the existing
  selection-kind, answer and resolved-model checks. There is no blanket
  rejection based on the source report's version.
- Added metadata counts toward the unchanged character, byte and token limits.
  At an exact-fit boundary it may cause rebatching or `RequestTooLarge`;
  limits are not raised and metadata is not excluded from accounting.
  Public Rust function signatures and the model-view cutter are unchanged.

## 0.15.1

Release date: 2026-11-13.

- Native section fields support `text: "heading"` for their own heading and
  for enclosing native section kinds, preserving the existing context order
  and equal-range containment. HTML heading text is projected once; Markdown
  heading markers are removed while inline source stays literal. Stored fields
  and asking views share the derived text, with original heading spans and
  normal text budgets. Unbounded recovered HTML headings are unavailable.
- Admission refuses heading context on non-section kinds, `same_file`, and
  kinds nested through a target's `as_kind`. Existing field modes retain
  their behavior. Public Rust enums add `SectionText::Heading` and
  `ContextText::Heading`; exhaustive consumers must handle the new variants.

## 0.15.0

Release date: 2026-11-12.

- Project-specific HTML outline exclusions and text attributes are configured
  under `evidence.html`; both default to empty. Standard `alt`, `aria-label`
  and `<template>` outline behavior remains built in.
- Markdown heading fragments require an explicit named scheme. Without one,
  unresolved fragments are `unsupported` with reason `heading_scheme_not_configured`.
  Explicit HTML ids and anchor names inside Markdown remain available.
- Breaking: `TargetAnchor::Unsupported` requires a typed `reason`;
  `load_evidence_project` takes a final `&DocumentEvidenceSettings` argument,
  and `select_document` takes it after the selection and before the match limit.
  The same settings are retained on `EvidenceProject`.
- Known limit: `EvidencePlanInput` and `sunstack exec` use standard defaults.
  Call the direct project/select APIs when document conventions are required.

To restore the 0.14.1 conventions, add these members to the existing `evidence`
object alongside `ast_grep` and `kinds`:

```json
"html": {
  "skip_tags": ["ex-markdown"],
  "text_attributes": {"ex-mermaid": ["label"]}
},
"markdown": {"heading_anchors": "github"}
```

## 0.14.1

Release date: 2026-11-12.

- HTML asking text keeps plain-text parser-error spans. Error spans containing
  ambiguous markup and unfinished tags mark their field incomplete instead of
  silently losing words. Structural tag-only slice errors retain spacing;
  raw evidence, selectors, mechanical inputs and section outlines stay unchanged.

## 0.14.0

Release date: 2026-11-12.

- Markdown fragment targets resolve GitHub-style heading slugs (github-slugger
  2.0.0 removal rules), duplicate suffixes and explicit HTML id/a-name anchors.
  Heading text keeps inline code/link text and surrounding spaces, excludes
  image alt text, and ignores headings in code, comments and front matter. Markdown URLs now percent-decode UTF-8 paths and
  fragments once, so file_exists also handles `%20` paths. HTML target behavior
  stays unchanged.
- `path_or_skill` treats `foo.md#x` as a file path with a fragment; earlier
  versions treated the whole fragment-bearing string as a skill name.
- Rulebook kinds accept exclusive `markdown_units` unit selectors. Section
  fields expose full, own and enclosing own source text, retaining exact spans
  when own text has multiple segments. Existing selector ranges remain: only
  list items clamp headings; quoted headings do not close outer sections and
  are not clamped to quote boundaries. This differs from HTML lexical outlines.
- Rust callers handle the new `KindMatcher::MarkdownUnits` variant. The existing
  Markdown selection validator is public; selection/filter types now implement
  equality. No saved report shape or low-level CallSite behavior changes.
- Known limit: recent Unicode lowercase mappings can differ between Rust std
  and GitHub's unobservable renderer version. Real-link and rendered-page
  comparisons remain the deciding compatibility evidence; no runtime case table
  is substituted. Demistify render-time page-island IDs remain out of scope.

## 0.13.0

Release date: 2026-11-12.

- HTML heading-outline units cover each h1–h6 through the next same-or-higher
  heading, across flat or wrapped markup. Generic evidence and rulebook kinds
  share the collector; section fields expose full, own and enclosing own text.
  Template, script, comment and Markdown-island source headings are excluded.
  Opening tags retain their sections when the parser recovers unclosed headings.
- HTML asking text retains alt, ex-mermaid label and aria-label attributes and
  separates adjacent elements. Rules, selection/shared views and filters project
  before applying cut budgets; raw mechanical evidence and reads hashes remain.
- Rust callers replace `KindMatcher::rule_id` with `ast_rule_id`, returning an
  optional AST identity. `ask_filters` now borrows `FilterEvidence` containing
  pieces, kinds, project and matches; `project_kind_model_view` owns shared
  asking-text projection. Native outline kinds never alias AST rule IDs. `sunstack_lint::run` is unchanged.

## 0.12.1

Release date: 2026-11-12.

- Clean change scopes and empty unnamed target sets finish without a credential
  when no model can be asked. Any changed path still requires a credential for
  model work; named empty selections remain incomplete.
- Config open failures name the selected file and reason. Named paths report
  full scope even on early errors; invalid and cancelled summaries count problems.
- Prepared summaries name saved request counts and the destination file. Lint help
  describes config discovery and exit codes; saved reports and JSON stay unchanged.

## 0.12.0

Release date: 2026-11-12.

- Change-scoped lint screens paths with raw Git blob identities and acquires only
  changed sources. Unchanged files are read when lint needs them, rather than
  frozen for context. Connected-evidence callers retain full-context capture.
- Hashing reads still count toward the existing byte budget; `max_files` counts
  acquisitions. Encoding refusals, symlink payloads and mutation checks remain.

## 0.11.1

Release date: 2026-11-12.

- `lint show` reads completed cases reports, accepts exact case IDs, and names
  every candidate when a selector matches multiple cases or numeric results.
  Prepared case ordinals and saved detail shapes stay unchanged.
- Change-scope refusals retain the offending path in diagnostics and displayed
  locations. Limit stops name the configured setting and value; existing
  invalid and incomplete exit meanings stay unchanged.

## 0.11.0

Release date: 2026-11-11.

- Evidence projects support every built-in ast-grep language, with HTML text
  projected for asking rules while mechanical checks and locations retain source.
- Cross-document checks add `document` fields, whole-token `contains_field`, and
  `anchor_exists` for HTML IDs. Referenced files share acquisition budgets.
- `lint --root` checks project files with an external configuration; named paths
  outside the root are invalid, and an empty named selection is incomplete.

## 0.10.1

Release date: 2026-11-11.

- `lint --since` skips unchanged binary files before decoding textual change
  witnesses. Changed non-UTF-8 content still refuses capture; file and byte
  limits and mutable-source verification still include unchanged binary reads.
- Change capture compares worktree symlinks by their Git link payload without
  following the target, and verifies both payload and file type.
- Known limit: on a large repository with an all-files binding, `--since` can
  still stop at the change-discovery limit (exit 3). Change discovery is
  addressed in a later release.

## 0.10.0

Release date: 2026-11-08.

- Default human lint output groups unsure results into one summary per rule,
  showing the count and a saved-report readback example. Findings keep their
  individual locations and messages. JSON, GitHub, SARIF, saved reports and
  `lint show` retain every result; run status, counts and exit codes are unchanged.

## 0.9.0

Release date: 2026-11-07.

- `lint --prepare FILE` and `lint cases --prepare FILE` save exact model requests
  and evidence without credentials or provider dispatch, with explicit gates and
  unanswered selection dependencies, including composed-field/shared-projection
  uncertainty. Clean preparation exits 6; failed admission and replayed selections
  keep ordinary request-allowance semantics.
- Ordinary saved reports and JSON output retain selection requests and answers.
  `--prepare --selections-from REPORT` continues from identical recorded selection
  inputs with an exact resolved-model pin and explicit source provenance.
- `lint show` reads prepared requests, saved selections and prepared case ordinals
  using only the saved report. Existing ordinary human output and result ids stay
  unchanged.

## 0.8.3

Release date: 2026-11-07.

- Case snapshots replace the evidence-rule directories declared by today's
  ast-grep project with today's contents, removing obsolete historical rules.
  Other rulebook inputs retain their individual overlay; historical context
  outside those directories stays intact.
- Case-run overlays refuse rule directories that cover the repository root or
  checked source, and retain the snapshot's symlink boundary.

## 0.8.2

Release date: 2026-11-07.

- Rule cases match exact first and last piece lines, including filter-skip locations.
  Nested sections keep distinct identities; partial or spanning ranges are unmatched.
- Case labels accept `owner-ruling` for an explicit owner gold-label decision,
  separately from `owner-fix` for a real text fix and `blind-agents`.
- Finding routes can share one locator name when its field, task and probability
  bar are identical. The request offers it once; conflicting definitions and
  question-name collisions remain refused.

## 0.8.1

Release date: 2026-11-06.

- Filters accept list-shaped text fields, including `same_file` openings, alongside
  scalar text. Requests preserve bounded lists and cut metadata; input hashes and
  saved reports retain the raw declared values.

## 0.8.0

Release date: 2026-11-06.

- Filters now use named questions and the shared decision-tree walker, with
  declared label endings or unsure. The old single-question filter shape is refused.
- Rule and filter reference material reaches request state unchanged. Questions
  name their input fields; location is sent only when a rule question reads it.
  Reserved state names and missing reference paths are refused at load.
- The mechanical `complete` check guards cut or unresolved text. Skill-name loads
  include their opening and cut metadata, as path loads do; unavailable inputs
  remain explicit and a question requiring them ends incomplete.
- Human lint output uses short numbered warnings, unsure/gap lines and a filter
  summary. `--report FILE` retains exact requests, all answers, model receipts,
  source contents and hashes; `lint show FILE ID` explains one saved result.
- `lint cases FILE [--set SET]` checks revision-bound rule and filter cases through
  ordinary lint on whole-repository copies with today's rulebook overlaid. It
  reports per-owner counts, stale/unmatched/excluded cases and unassessed bars,
  without changing files used by ordinary lint. Selected historical symlink paths
  are listed as not judged, without following them into a different lint scope.

## 0.7.1

Release date: 2026-11-06.

- Refuse `limits.max_locate_lines` above the provider's 255-option Choice cap
  during config checking. Filter declarations above that cap are refused at
  `filters.<name>.question.criteria`, with their option count and limit.
  Native rule, filter, direct decision and exec requests refuse oversized
  Choice questions locally before any HTTP call; filter results retain the
  count and limit. Unreached locators preserve mechanical outcomes.
- Findings and result rows carry optional `locator_omitted` evidence when a
  reached locator exceeds the configured line limit: locator name, field,
  reason, candidate-line count and limit. The established default line and
  anchor remain intact; default output explains that fallback.

## 0.7.0

Release date: 2026-11-05.

### Added

- Declare `filters` to group checked files by folder and ask one native Choice
  question about a configured text field per folder per run. Context files use
  the checked source and limits; changes to them select affected members,
  including evidence that needs selection.
- Rule entries accept `only_when` to run only for named filter answers. Excluded
  rules report a judged `not_applicable` reason before any decision step runs.
  Unsure, incomplete and folderless checks are listed as skips; unsure runs exit
  5, while failed filter requests and folderless checks exit 3.
- Evidence kinds accept either a named `rule` or an `inline_rule` YAML string.
  Inline matchers use the kind name as their rule ID, retain native matching and
  field behavior, and locate refusals at the owning configuration key.
- JSON reports expose `filter_answers`, `filter_skips` and result reasons.
  Answers retain their probabilities, requested and resolved model names and
  input-cut provenance. Default output lists answers and skips; SARIF continues
  to represent findings rather than skips.

### Breaking

- The report schema requires the filter answer and skip collections. Older
  reports do not validate against the 0.7.0 schema.
- Native evidence kind definitions now carry `KindMatcher`; project loading
  takes inline rules and their configuration source. Native rule admission,
  assembly and shared-dispatch execution receive their filter context.

## 0.6.0

Release date: 2026-10-31.

### Added

- Findings carry related places, beginning with the rule's standard, and a
  mechanical or judged basis. JSON, SARIF, default and GitHub output retain the
  files and lines of the evidence the run read.
- Finding routes can declare a `locate` Choice question. A selection meeting its
  probability threshold resolves the line inside its field and piece; `span`
  stays intact. `max_locate_lines` defaults to 80, and locators fit within the
  existing request budgets while preserving every authored question.
- Every result retains its walked `path` and the model `answers` it read,
  including passes, gate exclusions and incomplete walks. Completed results
  carry `basis`; cut model fields appear in row `cut` and state `cut-fields`.
- Target fields accept rule-only `exclude` ranges. Enclosing and same-file
  fields accept `text: "full"`, and path targets can collect `as_kind` fields.
  Shortened target openings carry `opening_cut`.
- Evidence projects admit JSON rules and utils alongside Markdown. JSON uses
  the existing field sources except `yaml`; field references stay within their
  language, while `as_kind` can select either grammar. A malformed JSON file is
  incomplete without preventing valid files from being checked.

### Breaking

- The report schema requires `related`, `anchor` and `basis` on findings and
  `path` on result rows. Readers that ignore new fields retain the meaning of
  existing fields; older reports do not validate against the 0.6.0 schema.

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
