# Markdown skill rules: program design

How the [Specification](specification.md) lands in the existing crates. Anchors are at engine main `9a83303`. The plug-in points come from the spike's source map (`tmp/markdown-skill-rules/plugin-points/report.md` in the `spike/markdown-skill-rules` worktree), and its prototypes are evidence only.

## Owners

| Crate | Owns, new | Existing types and functions it extends |
| --- | --- | --- |
| `sunstack-evidence` | One Markdown syntax-tree module: parse, find markers, resolve hosts and bindings, select units, call sites and frontmatter fields, render-visible text (R6) | `EvidenceSelection`, `SelectionContent`, `TargetLocation`, `document_selection::select_document_bounded`, the private `source_units::markdown_units` |
| `sunstack-rules` | Rule-source assembly (rules file + bound text → `RuleDefinition`), consistency (R11), lenient admission (R15), occurrence expansion (R14), local settlement of mechanical rules (R16: `expect` predicates, plus `ast_grep` through ast-grep's rule-config library), report additions (R17) | `RuleSetDefinition`, `definition_admission`, `invocation_admission::DefinitionLoader`, `definition_topology`, `request_state::resolve_target`, `verdict_evaluation`, `report_contracts` |
| `sunstack-rule-cli` | The check and print request (R13), with no backend construction for it | `protocol_boundary` (operation enum and decoding), `operation_execution::execute_operation` |

The engine stays generic (U10): it knows markers, hosts and rules files, never which skills or standards exist.

## Decisions

| D | Decision | Why |
| --- | --- | --- |
| D1 | **One Markdown parser: tree-sitter-md through ast-grep** (block and inline grammars), for markers, bindings, units and render-visible text. It offers two views: raw source spans and rendered-visible text. The existing `U001:` paragraph resolution moves onto the same module with identical behavior: it returns its old raw text, keeps its block-quote exclusion and tight-list ranges, and gets no R6 stripping. `tests/document_selection.rs` exact-range cases guard it. | Owner: "rust tool has astgrep"; the spike measured exact spans on five real skills; one parser means one span semantics. Cost: the `U001:` path is migrated, and V7 guards it. |
| D2 | **Assembly is a front end, not a format.** `rule_sources` produce the same `RuleDefinition` values the JSONL loader produces, and everything after the loader is shared. | No second rule model (U3); admission, topology and reports are reused. |
| D3 | **Admission has two modes.** Strict mode (check, and every `rule_paths` set as today) refuses on any error. Lenient mode (runtime `rule_sources`) turns per-rule errors into skipped rules, and a bad source file into one source-level skip. It also sets coverage to full, partial or none (R15); completeness no longer passes on an empty occurrence list (today's `report_contracts::is_complete`). RuleSet-header and invocation errors refuse in both modes. | Owner: compile-check flag plus "log ... and ignore rules that fail"; existing JSONL behavior is unchanged (V7). |
| D4 | **Target identity reaches the request, not only the report.** `RuleOccurrenceKey` gains a target discriminator: the source identity and byte span of the unit, plus the match span in call-site mode. The discriminator is carried through `plan_contracts::unit_label` (today `rule:<id>`), the evidence labels in `request_state` (which today drops a duplicate label), `QuestionOwner`, request/report correlation and settlement. Equal evidence content is deduplicated separately from its attributions. | The spike found today's key {rule set, group, rule} would collapse per-unit occurrences; one list item can hold three calls. |
| D5 | **Expansion happens after gates.** Units are selected only once a rule's applicability gates allow evidence acquisition, which keeps today's lazy-read behavior for stopped gates. Batching and whole-occurrence budget splitting stay as they are, and one request can carry many occurrences. | Spike: eager expansion would change stopped-gate behavior and cost. |
| D6 | **Mechanical rules settle locally.** `VerdictDeclaration::Mechanical { expect | ast_grep }` gets its own settlement path (`Completed { passed, observations } | Unresolved | Cancelled`), keyed by occurrence, beside request settlement. Its results project into the report and its completeness. `expect` is a few regex and prefix predicates over a unit's visible text. `ast_grep` delegates to ast-grep's rule-config library: one added dependency from the project the engine already links, at the same version. No rule language of our own, no fabricated answer, no receipt and no confidence. | Owner: "i dont want to build rules engine", "B ... then actual ast grep with validation". The spike confirms the rule-config library's parse and match API before implementation. |
| D7 | **A backend is built only when model work is reachable.** Check and print never build one. A runtime invocation builds one only if a judgment, a model-based gate or a Router is reachable. Today the CLI builds the backend before either rules operation (`operation_execution.rs:11–26`). | R13; today's CLI builds the backend first. |
| D8 | **File existence stays outside the engine.** "A referenced file exists" is a small CI script, not an engine rule. | It needs filesystem policy (scopes, symlinks, escapes) that neither `expect` nor ast-grep should carry; owner: no rules engine. |

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
