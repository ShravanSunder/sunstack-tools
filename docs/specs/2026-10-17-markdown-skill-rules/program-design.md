# Markdown skill rules: program design

How the [Specification](specification.md) lands in the existing crates. Anchors are at engine main `9a83303`. The plug-in points come from the spike's source map (`tmp/markdown-skill-rules/plugin-points/report.md` in the `spike/markdown-skill-rules` worktree), and its prototypes are evidence only.

## Owners

| Crate | Owns, new | Existing types and functions it extends |
| --- | --- | --- |
| `sunstack-evidence` | One Markdown module: parse, find markers, resolve hosts and bindings, select units, call sites and frontmatter fields (tree-sitter-md), and render-visible text (R6, pulldown-cmark offset events; D1) | `EvidenceSelection`, `SelectionContent`, `TargetLocation`, `document_selection::select_document_bounded`, the private `source_units::markdown_units` |
| `sunstack-rules` | Rule-source assembly (rules file + bound text → `RuleDefinition`), consistency (R11), lenient admission (R15), occurrence expansion (R14), local settlement of mechanical rules (R16: `expect` predicates, plus `ast_grep` through ast-grep's rule-config library), report additions (R17) | `RuleSetDefinition`, `definition_admission`, `invocation_admission::DefinitionLoader`, `definition_topology`, `request_state::resolve_target`, `verdict_evaluation`, `report_contracts` |
| `sunstack-rule-cli` | The check and print request (R13), with no backend construction for it | `protocol_boundary` (operation enum and decoding), `operation_execution::execute_operation` |

The engine stays generic (U10): it knows markers, hosts and rules files, never which skills or standards exist.

## Decisions

| D | Decision | Why |
| --- | --- | --- |
| D1 | **Structure from tree-sitter-md through ast-grep; rendered text from pulldown-cmark.** tree-sitter-md (block and inline grammars) owns markers, hosts, bindings, raw spans, units, frontmatter and ast_grep matching. pulldown-cmark owns the rendered-visible text (R6): both projections come from its whole-document offset events that lie inside the tree-sitter host range, and dropping its `Html`/`InlineHtml` events is the hidden-content rule (amended 2026-10-17 after S1 review F1, F3–F9). Rebuilding rendering on the syntax grammar meant re-implementing CommonMark rule by rule, and the review found eight such rules wrong. The spike passed every existing exact-string test, 22 of the 23 review cases, and showed zero differences over 262 bindings on five real skills, for +23 net lines. pulldown-cmark parses a copy of the document with line endings normalized to LF, and each tree-sitter host range is mapped into that copy, so a CRLF document and its LF twin give identical question bytes. This also covers pulldown-cmark 0.13.4 turning one CRLF inside a code span into two spaces (S1 review DQ1). Table text is rebuilt from the table's events (rows, cells, alignment), never copied from raw source between events, so cells the renderer drops (beyond the header width) and their hidden content never reach the question (S1 review F11). **The existing `U001:` paragraph resolution stays on its current pulldown-cmark path, unchanged** (amended earlier the same day, S1 finding F1): migrating it changed legacy behavior, because tree-sitter recognizes GFM tables and frontmatter that the legacy parser deliberately doesn't. Legacy `U001:` ranges are never reused for units (see Boundary contracts). | Owner: "rust tool has astgrep". The non-goal "no change to `U001:` meaning" outranks a single parser for the legacy path. A renderer-grade parser for the reader-visible text serves U7 ("for safety"). Both libraries were already dependencies; `html-escape` is dropped. Cost: two parsers that could disagree on a block boundary. The spike found 236 such records, all whitespace or tight-list paragraph elision, none moving a host or range, because hosts and ranges come only from tree-sitter. |
| D2 | **Assembly is a front end, not a format.** `rule_sources` produce the same `RuleDefinition` values the JSONL loader produces, and everything after the loader is shared. | No second rule model (U3); admission, topology and reports are reused. |
| D3 | **Admission has two modes.** Strict mode (check, and every `rule_paths` set as today) refuses on any error. Lenient mode (runtime `rule_sources`) turns per-rule errors into skipped rules, and a bad source file into one source-level skip. It also sets coverage to full, partial or none (R15); completeness no longer passes on an empty occurrence list (today's `report_contracts::is_complete`). RuleSet-header and invocation errors refuse in both modes. | Owner: compile-check flag plus "log ... and ignore rules that fail"; existing JSONL behavior is unchanged (V7). |
| D4 | **Target identity reaches the request, not only the report.** `RuleOccurrenceKey` gains a target discriminator: the source identity and byte span of the unit, plus the match span in call-site mode. The discriminator is carried through `plan_contracts::unit_label` (today `rule:<id>`), the evidence labels in `request_state` (which today drops a duplicate label), `QuestionOwner`, request/report correlation and settlement. Equal evidence content is deduplicated separately from its attributions. | The spike found today's key {rule set, group, rule} would collapse per-unit occurrences; one list item can hold three calls. |
| D5 | **Expansion happens when acquisition is allowed.** Units are discovered when the existing acquisition frontier allows it. They're deferred behind staged (different-input) prerequisites. Today's permitted speculative acquisition for same-input carriers is preserved (engine R28, R38, C13), and the gates still govern the final outcomes. Batching and whole-occurrence budget splitting stay as they are, and one request can carry many occurrences. Deferral governs canonical target selection and acquisition, unit expansion, occurrence creation and requests; the existing change-scope repository capture keeps its current semantics, even when its frozen snapshot already holds the reviewed file's bytes, so a held-gate witness counts target selection and acquisition, not snapshot capture (a full-scope run shows zero target reads while held). New selector content fingerprints follow the existing content-comparison semantics, per occurrence, conservative when unavailable (S3 Implementer question). The state and interfaces are in Deferred expansion below. | Spike: eager expansion would change stopped-gate behavior and cost. |
| D6 | **Mechanical rules settle locally.** `VerdictDeclaration::Mechanical { expect | ast_grep }` gets its own settlement path (`Completed { passed, observations } | Unresolved | Cancelled`), keyed by occurrence, beside request settlement. Its results project into the report and its completeness. `expect` is a few regex and prefix predicates over a unit's visible text. `ast_grep` delegates to ast-grep's rule-config library: one added dependency from the project the engine already links, at the same version. No rule language of our own, no fabricated answer, no receipt and no confidence. | Owner: "i dont want to build rules engine", "B ... then actual ast grep with validation". Confirmed by spike on 2026-10-17 (report kept in memory-logs `skills/authoring/2026-10-17-skill-rules-and-evals/ast-grep-config-spike.md`): `ast-grep-config` 0.51.0 parses the JSON rule object (`SerializableRule` → `DeserializeEnv::<SupportLang>::deserialize_rule`). It matched the CLI in 24 of 24 comparisons, on whole documents and on selected subtrees. It adds 15 registry packages with no version conflict. An unknown key reports line and column; a bad regex reports only a cause chain, so check mode attributes it to the `ast_grep` block's own location in the rules file. ast-grep stays pinned to an exact version (`=0.51.0` today, as in `crates/*/Cargo.toml`). Upgrading it is a deliberate change that reruns check over every rule source: `expect` rules are insulated by our layer, while `ast_grep` escape-hatch rules may need edits, which check mode and V6 surface (owner: "with ast grep version updates ours might change but i guess that's ok"). |
| D7 | **A backend is built only when model work is reachable.** Check and print never build one. A runtime invocation builds one only if a judgment, a model-based gate or a Router is reachable. Today the CLI builds the backend before either rules operation (`operation_execution.rs:11–26`). | R13; today's CLI builds the backend first. |
| D9 | **A rule from `rule_sources` has two definition files.** The located definition carries both the Markdown path and the rules path into the existing definition-dependency comparison (`invocation_admission.rs` located source; `change_selection.rs` dependency selection). An edit to either file selects the rule. An unchanged tracked pair can stay outside change scope. When comparison isn't available, selection stays conservative, as engine C3 requires. | Review F4: one stored `source` would let a Markdown-only or config-only edit be pruned. |
| D8 | **File existence stays outside the engine.** "A referenced file exists" is a small CI script, not an engine rule. | It needs filesystem policy (scopes, symlinks, escapes) that neither `expect` nor ast-grep should carry; owner: no rules engine. |

## Entity bindings

| E | Realized as | Home | Lifetime |
| --- | --- | --- | --- |
| E1 Marker | `MarkerId` (one uppercase letter + exactly three digits, its own validator; not `TargetId`, which is looser) plus its `MarkdownTextSpan` | `sunstack-evidence` Markdown module | per assembly |
| E2 Bound text | the host node's raw range as a `MarkdownTextSpan` (byte range plus 1-based line and character column, computed exactly as `source_span` computes them, with no source identity), plus two projections: question text (R6 keeps inline code) and filter text (R14/R16 exclude code spans). A definition file is not an evidence source and has no `SourceRef`: the loader pairs the span with the Markdown path, and diagnostics use the existing `DiagnosticLocation::Definition { file, line, column, field_path }`, as JSONL definitions already do (the `CapturedChange` path-plus-span shape is the precedent). Reviewed-document units (R14) keep `SourceSpan` with their real `SourceRef::Document` | `sunstack-evidence` | per assembly |
| E3 Rules file | strict JSON object, decoded with the existing duplicate-key-rejecting decoder (`strict_decoding.rs`), keyed by `MarkerId`. Each entry decodes into exactly one of `Judgment { threshold, block_at_or_below }`, `Expect { expect }` or `AstGrep { ast_grep }`; zero or several alternatives are a refusal | `sunstack-rules` | per assembly |
| E4 Rule source | `RuleSourcePair { markdown, rules }` in `RuleSetDefinition.rule_sources`, beside `rule_paths`, with `marker_prefix: Option<MarkerPrefix>` and `max_question_words` (default 400) | `definition_contracts.rs` | the definition file |
| E5 Assembled rule | a native `RuleDefinition` (`rule_id` = entry `name`; the target lowers to `EvidenceDeclaration { id: "target", input, select }` with `"target"` reserved; `context` entries are native `EvidenceDeclaration`s) plus `RuleSourceProvenance { markdown, rules, marker_id, marker_span: MarkdownTextSpan, entry_location }`. Both files are definition dependencies (D9) | `sunstack-rules` loader | per invocation |
| E6 Skipped rule | `SkippedRule::Entry { rule_set, markdown, rules, key, rule_id: Option<RuleId>, diagnostics }` or `SkippedRule::Source { rule_set, source_file, markdown, rules, diagnostics }`. `key` is the raw key, so a malformed one still reports | report | per report |

## Boundary contracts

New structs and enums follow the engine's conventions: strict `deny_unknown_fields`, `kind`-tagged snake_case enums, `JsonSchema`, and constructor-validated IDs. The research input, with every source anchor, is in memory-logs at `skills/authoring/2026-10-17-skill-rules-and-evals/rules-boundary-shapes.md`.

```text
Selection (evidence)
  EvidenceSelection::MarkdownUnits  { mode: unit { units, filter } | call_site { units, filter: StartsWith | Regex } }
  EvidenceSelection::FrontmatterField { field }
  MarkdownUnitKind = section | paragraph | list_item
  SelectionContent::MarkdownUnits { units: [SelectedMarkdownUnit { kind, span, text, visible_text, occurrence }] }
  a unit's span is the block node's raw tree-sitter range, including its terminal newline (as ast-grep reports matches),
  and ast_grep containment uses those raw ranges; the legacy U001 trimmed paragraph range is unchanged and is never reused for units
  unit mode selects every block of a requested kind (and, with a filter, every such block whose visible text matches); requested kinds may overlap,
  so a section and a paragraph inside it are two occurrences when both kinds are requested
  call_site mode gives each filter match exactly one owner: the innermost block of a requested kind that contains it, where a list item
  owns the paragraphs inside it (as R2 does for markers); this is the spike's rule behind the 34-calls-in-31-units baseline (S3 question 1)
  "contains" is decided in visible text, not raw ranges: the owner is the innermost requested unit whose own visible text holds the
  match's visible characters, so a match made only of generated text between child units (a joining separator) belongs to the
  enclosing requested unit; raw ranges are only the repair location (S3 review F1)
  SelectionContent::FrontmatterField = Present { source, field, value, span } | Absent { source, field, frontmatter_span }
                                     | BlockUnavailable { source, field, reason: missing | malformed, diagnostics }
  frontmatter is a YAML `---` block at the very start of the document; `field` names a top-level mapping key; the block is decoded with a
  maintained YAML decoder (not hand-written scalar parsing) and `value` is the field's decoded value as JSON; any other start of document
  (including a `+++` block) is `missing`; text matching over the value belongs to S4's expect forms (S3 question 2)
  expect over a frontmatter value: a JSON string matches as its text; any other value (number, boolean, null, array, object) matches
  against its canonical serde_json text; `present` is true for any present field, null included; an absent field fails every form
  a missing or malformed block is SelectionReadiness::Ready with BlockUnavailable, NOT Refused, so it never projects as SelectionFailed
  TargetLocation::MarkdownUnit { occurrence } | FrontmatterField { source, field, Present{span} | Absent{frontmatter_span} | BlockUnavailable{reason} }
  BlockUnavailable carries no field or header span (none exists), and its occurrence key is still FrontmatterField{source, field}

Identity (rules)
  RuleOccurrenceKey { rule_set, group, rule_id, target: Legacy | MarkdownUnit{source, byte range} | CallSite{source, unit raw range, match raw range, match visible range} | FrontmatterField{source, field} }
  RuleEvaluationKey { rule_set, group, rule_id }       one per rule before its targets are known
  the call-site key carries the match's visible-text range beside its raw range: one HTML entity can decode to several visible
  characters, so two distinct visible matches can map to the same raw span; the raw range stays the repair location (S3 finding)
  a match's raw range is the smallest original byte range covering the authored lexemes behind its visible characters (a whole CRLF
  line ending, a whole entity or escape); generated text (joining separators, list and table framing) has no authored bytes, so a
  match made only of generated text gets a zero-width raw range at the adjacent authored boundary, and its visible range keeps it
  distinct (S3 Verifier prebuild and Implementer convention)
  QuestionOwner::Verdict / ::Informational carry the full occurrence key; q000001 names stay transport-only

Mechanical verdict (rules)
  VerdictDeclaration::Mechanical { explanation, check: Expect{ Matches | NotMatches | StartsWith | Present } | AstGrep{ validated rule } }
  LocalSettlement = Completed { result, observations } | Unresolved { diagnostics } | Cancelled
  RuleOccurrenceResult::Checked { key, selected_by, target, declared_severity, explanation, outcome }
  projected after routing/applicability/evidence and before request_for_unit; excluded from questions and budget;
  a BlockUnavailable target settles as LocalSettlement::Unresolved { diagnostics } for that occurrence (report: Checked, outcome Unresolved, incomplete);
  frontmatter_field is a mechanical-only target: admission refuses it on a judgment rule;
  a mechanical occurrence with no settlement is reported as unresolved, never a pass

Check operation (CLI + rules library)
  operation "check_rules", input CheckRulesInput { definition_root, definitions, repositories, payload_schemas, limits, discovery, output: diagnostics | assembled_rules }
  no backend, attempts or request budget; reviewed documents are not read
  discovery mirrors the invocation's own setting, so check admits exactly the rules the runtime would (a ChangeSet rule checks clean under bounded discovery and refuses without it); check admits under it but never runs discovery or acquisition (S2 review F2)
  CheckRulesOutcome = Checked { checked_rule_count, printed_rules } | Refused { diagnostics, checked_rule_count, refused_rule_count }
                    | CheckFailed { diagnostics, measurements } | Cancelled { diagnostics, measurements }
  PrintedRule { rule_set, source: Jsonl{file,line} | Markdown{provenance}, definition }   printed only after a fully clean check
  exits: Checked 0, Refused 2, CheckFailed 3, Cancelled 4

Report (rules)
  rule_coverage: full | partial | none        from assembly and admission: no skips = full; skips with ≥1 valid rule = partial; no valid rule = none
  skipped_rules: [SkippedRule]
  rule_evaluations: [Expanded{key, occurrences} | ZeroMatches{key, input, source} | NotApplicable{key, stopped_at, basis}
                     | OutsideChangeScope{key, reason} | SelectionFailed{key, diagnostics} | EvaluationFailed{key, cause} | Cancelled{key}]
  counters: invalid_rule_entries and skipped_rule_sources, separate from today's rules_skipped (not applicable / outside scope)
  is_complete = coverage ≠ none ∧ every rule evaluation terminal and complete ∧ every occurrence complete ∧ today's router condition
  (a valid NoRoute, UncertainIntent or zero-match run can still be complete)
  exits: complete (any coverage but none) 0; coverage none or incomplete 3; cancelled 4
```

## Deferred expansion

A rule whose target count isn't known until its document is read starts as a `PlannedRuleEvaluation { key: RuleEvaluationKey, rule, group, entries, scope, authored_order }`. That's an explicit record, not a placeholder occurrence. Rules with a fixed target stay immediately concrete, as today.

| Piece | Design |
| --- | --- |
| State | `ExpansionState = Waiting | Expanded { children } | ZeroMatches { source } | SelectionFailed { diagnostics } | Stopped { entry } | Failed { cause } | Cancelled`, owned by `ExecutionState` for one invocation |
| Graph | the initial requests and read sets use the record's native declarations, so gate scope still sees the undiscovered work. `entry_outside_scope`, the group leading scan and `rule_pruned` account for records, and the graph is never rebuilt once a request is in flight |
| Scheduler | `discover_ready_rules(plan, state, session, cancellation)` runs after `advance_entries` and before model dispatch and budget admission. It uses the same acquisition readiness as the record's carrier request, plus known-stop checks. `install_discovered_occurrences(plan, state, id, selection)` is the scheduler's synchronous mutation: it rechecks cancellation, then appends children without renumbering any existing index. Each child joins its group, its scope and the still-waiting carrier request, inherits the record's applicability entries and routing, and leaves every gate control in place. The existing budget splitter then treats each child as one unsplittable rule block |
| Carriers | a same-input gate may ride with discovered children (engine R38) and its result later governs them. A different-input staged gate blocks discovery until its frontier passes. When discovery gives zero children, only mechanical children, or a failure, the existing gates stay as a gate-only request with real questions; no empty judgment request is ever sent |
| Identity and order | child keys come from source identity plus the raw unit or match span (frontmatter: source + field). Children sort by unit range, then match range, inside the authored rule-source and binding-group order. Projection and budgeting use that order, never raw append indices |
| Prepare output | `PreparedRules.rule_evaluations` adds `WaitingForExpansion { key, on }`. While a staged gate holds, it shows the record and the real gate request, with no invented spans, counts or questions |
| Termination | the scheduler's progress check includes pending expansions and local settlements. A zero-target discovery is a real transition. A waiting expansion with nothing runnable or in flight is an internal failure, never a success or an endless wait |

| Event | Result |
| --- | --- |
| Different-input gate held | one `WaitingForExpansion` on the exact entry keys; no target read, no child keys; unrelated completed peers stay |
| Gate passes, three calls found | exactly three keys with distinct match spans, installed once, each owner and evidence label naming its own target |
| Zero matches | `ZeroMatches`, no target occurrences, complete if everything else is |
| Read or selection fails | `SelectionFailed`, no invented children, incomplete (exit 3) |
| Gate stops | `NotApplicable` with its entry key and basis, no reads, complete if peers are |
| Gate fails | `EvaluationFailed`, incomplete |
| Cancel before or after discovery | discovered keys and completed results kept, the rest cancelled, exit 4 |
| Same-input carrier says "doesn't apply" after speculative children | the children aren't applicable; their model answers are discarded, as today, and local observations never turn into failures |

## R realization

| R | Realized by |
| --- | --- |
| R1–R5 | evidence Markdown module: `MarkerId`, host resolution with outermost-allowed precedence and forbidden ancestors, section binding |
| R6–R8 | evidence: question-text and filter-text projections; the size cap at assembly |
| R9–R12 | rules: header fields, `RuleSourcePair` loading, the strict E3 decoder, both-way consistency, lowering to `RuleDefinition`, shared admission |
| R13 | CLI `check_rules` plus the rules-library check entrypoint (no backend) |
| R14 | evidence selections and content; rules occurrence keys, `RuleEvaluationKey` and deferred expansion |
| R15 | lenient admission, `SkippedRule`, coverage and the new counters |
| R16 | `VerdictDeclaration::Mechanical`, `LocalSettlement`, `RuleOccurrenceResult::Checked`, the ast-grep-config matcher on the original tree |
| R17 | report additions, `is_complete` and exits, versioned with the CLI and Rust API |

## Flow

```text
invocation (RuleSet with rule_sources)
  └─ load RuleSet header ─── refuse on header error (both modes)
      └─ for each rule source:
           parse Markdown (D1) → markers → hosts (R2–R3) → bindings (R4) → question text (R6–R7)
           read rules file → consistency both ways (R11)
           build RuleDefinition (D2) → admission
             strict: any error → refuse with file:line:col           (check mode, R13)
             lenient: per-rule error → skipped rule, keep going      (runtime, R15)
  └─ check mode: return refusals, or OK plus printed rules → exit (no backend, D7)
  └─ runtime: topology → gates → expand units (D5, R14) → requests (judgments) | local settle (checks, D6)
      └─ report: occurrences with spans, verdicts, check results, skipped rules (R17)
```

## Contracts that change

| Contract | Change | Compatibility |
| --- | --- | --- |
| RuleSet JSON | adds `rule_sources`, `marker_prefix`, `max_question_words` | additive; `rule_paths` unchanged |
| Rules file | new JSON shape (R10) | new |
| Evidence selection | adds `markdown_units` (R14) | additive |
| Verdict declaration | adds mechanical rules: `expect` and `ast_grep` (R16) | additive |
| Report | adds skipped rules, unit and match spans, and check results; widens the occurrence key | additive fields; the key widening is invisible to consumers that read occurrences by position |
| CLI | adds the check request (R13) | additive operation |

The generated and reviewed schemas, examples and CLI docs follow their owning Rust contracts in the same change.

## Proof seams

| Seam | Where the R rows can be observed |
| --- | --- |
| Markdown module | pure functions over bytes: markers, hosts, bindings, questions, units (V1, V2, V4) |
| Loader | `prepare_rules` with `rule_sources` in strict and lenient modes (V3, V5) |
| CLI | the real process with no credential in its environment (V3, D7) |
| Mechanical settlement | `expect` and `ast_grep` over fixed Markdown units, with `ast-grep scan` as the reference (V6) |
| Regression | the existing engine suite and the `verify-sunstack-rule` journeys (V7) |
