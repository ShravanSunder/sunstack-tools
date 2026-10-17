# Markdown skill rules: specification

The observable contract for [Requirements](requirements.md) U1–U10. Rows marked *(new)* add to the [engine Specification](../2026-10-12-rust-rule-engine/specification.md). Nothing here changes an existing contract's meaning.

## Entities

| E | Term | Identity | Notes |
| --- | --- | --- | --- |
| E1 | Marker | (source file, marker ID) | The text `<!-- K014 -->` at the very end of a host line |
| E2 | Bound text | (source file, host node byte range) | What a marker binds; becomes the question after stripping |
| E3 | Rules file | path | JSON object keyed by marker ID; one per marked source file |
| E4 | Rule source | (Markdown path, rules-file path) | Listed in a RuleSet's `rule_sources` |
| E5 | Assembled rule | (RuleSet, marker ID) | An in-memory `RuleDefinition`, the same type the JSONL path builds |
| E6 | Skipped rule | (RuleSet, marker ID or rules-file key) | A rule that failed assembly or admission at runtime |

## The marker

| R | Obligation |
| --- | --- |
| R1 | A marker is exactly `<!-- ` + the RuleSet's `marker_prefix` letter + three or more digits + ` -->`, and it ends its line, with only trailing whitespace after it. Anything else, including comments with other content, comments mid-line, and comments inside code spans or fenced code, is not a marker and is ignored. |
| R2 | A marker's host is the syntax-tree node whose line it ends. Allowed hosts: a heading of level 2–6, a paragraph, a list item. Any other host (an H1, a block quote, a table cell, an HTML block, frontmatter, a link definition, a footnote, or a node kind not on this list) is a refusal. |
| R3 | A heading marker must touch the heading text with no space between (`### Call Grammar<!-- K014 -->`), so the heading's anchor doesn't change. A heading marker with a space before it is a refusal. |
| R4 | Binding: a heading binds its section, from the heading to the next heading of the same or higher level, nested subsections included. A paragraph binds itself. A list item binds itself and its nested items. Markers may nest, and nested bindings overlap. |
| R5 | A marker ID appears at most once per source file. A duplicate is a refusal. |

## The question

| R | Obligation |
| --- | --- |
| R6 | The question is the bound text as a reader sees it rendered. Kept: heading text, paragraphs, lists, emphasis, inline code, link text, and fenced code and tables as literal text. Stripped: HTML comments (all markers included), raw HTML blocks and inline HTML, link reference definitions, and images (an image's alt text is kept). |
| R7 | Bound text over the RuleSet's `max_question_words` (default 400) is a refusal that names the marker and suggests a narrower host. |
| R8 | The same sources always assemble the same question bytes. |

## Rules files and rule sources

| R | Obligation |
| --- | --- |
| R9 | A RuleSet may list `rule_sources: [{ "markdown": <path>, "rules": <path> }]` beside or instead of `rule_paths`. Paths resolve from `definition_root` under the existing path rules (no `..`). `marker_prefix` (one uppercase letter, required with `rule_sources`) and `max_question_words` (optional) are RuleSet fields. |
| R10 | A rules file is one JSON object whose keys are marker IDs. Each entry has `name` (the rule ID used in reports, unique across the RuleSet), `target` (an evidence selection, see R14), and either a judgment verdict (`threshold`, optional `block_at_or_below`) or a `check` (R16). It may also have `severity` (`nudge` by default) and `context` (extra sources added to the request as evidence, never followed implicitly). Unknown or duplicate keys are refusals. |
| R11 | Consistency runs both ways. A marker without a rules entry is a refusal, and so is a rules entry without a marker. |
| R12 | An assembled rule passes the same admission as a JSONL rule, plus the checks R1–R11. |

## Check, print and runtime

| R | Obligation |
| --- | --- |
| R13 | `sunstack-rule` accepts a check request (an operation or flag on rule preparation) that assembles every rule in the invocation's RuleSets and runs full admission. It judges nothing, sends no model request, and needs no backend or credential. It exits non-zero listing every refusal with file, line and column, or exits zero. With print, it also returns each assembled rule (name, question bytes, target, verdict or check), so the exact model input can be reviewed. |
| R14 *(new target)* | `markdown_units` selects units in a reviewed document. `mode: unit` gives one occurrence per selected section, paragraph or list item, and `units` names the kinds. `mode: call_site` gives one occurrence per filter match inside a unit. Optional `starts_with` or `pattern` (a regex over rendered-visible text, code spans excluded) filters. Each occurrence's identity includes its source, byte span and, in call-site mode, the match span, so several calls in one list item stay distinct. Zero matches are reported as zero, not as an empty silence. |
| R15 | At runtime an invalid rule (any R1–R12 refusal tied to one marker or entry) is logged as a skipped rule in the report, with its diagnostics, and every valid rule still runs. Invocation-level and RuleSet-header errors still refuse the whole invocation, as today. |
| R16 *(new verdict, provisional per U9)* | A `check` settles locally with no model request: `referenced_path_exists` (each inline-code path in the unit, resolved from the document's folder inside the bound repository scope), `pattern_present` / `pattern_absent` (regex over rendered-visible text), `frontmatter_field_present`. The result is pass, fail, or unresolved; unreadable, out-of-scope and escaping paths are unresolved, never fail. An invocation whose rules are all checks constructs no backend and needs no credential. |
| R17 | The report keeps today's fields and adds the skipped rules (R15), per-occurrence spans (R14) and check results (R16). Existing consumers of the report keep working. |

## Proof obligations

| V | Decides pass and fail for | Reference |
| --- | --- | --- |
| V1 | R1–R5: every refusal kind fires on a crafted source and names file:line; every allowed host binds the exact byte range; nested headings bind to the right boundary | the rows above, and CommonMark section semantics |
| V2 | R6–R8: the question bytes equal the bound text with exactly the R6 removals; two runs give identical bytes | R6's keep/strip table |
| V3 | R9–R13: check mode exits non-zero on each refusal kind and zero on a clean set, makes zero model requests and constructs no backend; print shows the R6 question | R13 |
| V4 | R14: on five real skills, the units and calls selected match a reviewed baseline, with no merged call identities | the spike's five-skill baseline (53 sections, 34 calls) |
| V5 | R15: with one broken marker in a source, the run completes, the report lists that skipped rule, and the other rules' verdicts are unchanged | R15 |
| V6 | R16: existing, missing, out-of-scope and escaping paths settle as pass, fail, unresolved and unresolved | R16 |
| V7 | R17 and the non-goals: existing JSONL RuleSets, `U001:` targets and today's reports behave exactly as before | the existing engine test suite |
