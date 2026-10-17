# Markdown skill rules: specification

The observable contract for [Requirements](requirements.md) U1–U10. Rows marked *(new)* add to the [engine Specification](../2026-10-12-rust-rule-engine/specification.md). Nothing here changes an existing contract's meaning.

## Entities

| E | Term | Identity | Notes |
| --- | --- | --- | --- |
| E1 | Marker | (source file, marker ID) | The text `<!-- K014 -->` at the very end of a host line |
| E2 | Bound text | (source file, host node byte range) | What a marker binds; becomes the question after stripping |
| E3 | Rules file | path | JSON object keyed by marker ID; one per marked source file. An entry is a judgment rule (`threshold`) or a mechanical rule (`expect` or `ast_grep`). |
| E4 | Rule source | (Markdown path, rules-file path) | Listed in a RuleSet's `rule_sources` |
| E5 | Assembled rule | (RuleSet, `name`) | An in-memory `RuleDefinition`, the same type the JSONL path builds, keyed by the entry's `name`, which is the native and report rule ID. It keeps its source-qualified marker (Markdown path, rules path, marker ID) as provenance. `a.md:K014` and `b.md:K014` are two rules when their names differ. |
| E6 | Skipped rule | (RuleSet, source file, marker ID or rules-file key), or (RuleSet, source file) for a source-level skip | A rule that failed assembly or admission at runtime. Two invalid entries stay distinguishable, even when they share an ID across files. |

## The marker

| R | Serves | Obligation |
| --- | --- | --- |
| R1 | U1 | A marker is exactly `<!-- ` + the RuleSet's `marker_prefix` letter + exactly three digits + ` -->`, and it ends its line, with only trailing whitespace after it. Anything else, including comments with other content, comments mid-line, and comments inside code spans or fenced code, is not a marker and is ignored. |
| R2 | U6 | A marker's host is the outermost allowed node whose line it ends. A list item wins over the paragraph inside it, so `- text<!-- K014 -->` binds the item and its nested items. Allowed hosts: a heading of level 2–6, a paragraph, a list item. A marker in any other host, or inside a forbidden container at any depth (an H1 line, a block quote, a table, an HTML block, frontmatter, a link definition, a footnote, or a node kind not on this list), is a refusal. So `> text<!-- K014 -->` is refused even though it ends a paragraph. |
| R3 | U6 | A heading marker must touch the heading text with no space between (`### Call Grammar<!-- K014 -->`), so the heading's anchor doesn't change. A heading marker with a space before it is a refusal. |
| R4 | U1, U6 | Binding: a heading binds its section, from the heading to the next heading of the same or higher level, nested subsections included. A paragraph binds itself. A list item binds itself and its nested items. Markers may nest, and nested bindings overlap. |
| R5 | U4 | A marker ID appears at most once per source file. A duplicate is a refusal. A marker's identity is (source file, ID): two files may both use `K014`, but each entry's `name` must be unique across the RuleSet. |

## The question

| R | Serves | Obligation |
| --- | --- | --- |
| R6 | U7 | The question is the bound text as a reader sees it rendered. Kept: heading text, paragraphs, lists, emphasis, inline code, link text, and fenced code and tables as literal text. Stripped: HTML comments (all markers included), raw HTML blocks and inline HTML, link reference definitions, and images (an image's alt text is kept). |
| R7 | U6 | Bound text over the RuleSet's `max_question_words` (default 400) is a refusal that names the marker and suggests a narrower host. |
| R8 | U3 | The same sources always assemble the same question bytes. |

## Rules files and rule sources

| R | Serves | Obligation |
| --- | --- | --- |
| R9 | U2, U3, U10 | A RuleSet may list `rule_sources: [{ "markdown": <path>, "rules": <path> }]` beside or instead of `rule_paths`. Both paths resolve exactly as `rule_paths` entries do today: relative to the RuleSet file's folder, with the existing normalization. A RuleSet needs at least one `rule_paths` or `rule_sources` entry. `marker_prefix` (one uppercase letter, required with `rule_sources`) and `max_question_words` (optional) are RuleSet fields. |
| R10 | U2, U10 | A rules file is one JSON object whose keys are marker IDs. Each entry has `name` (the rule ID used in reports, unique across the RuleSet), `target` (`{ "input": <a RuleSet input name>, "select": <an evidence selection, see R14> }`; the input is always named, with no default even when only one document input exists), and exactly one of: a judgment verdict (`threshold`, optional `block_at_or_below`), an `expect`, or an `ast_grep` rule (R16). It may also have `severity` (`nudge` by default) and `context` (extra sources added to the request as evidence, never followed implicitly). Unknown or duplicate keys are refusals. |
| R11 | U4 | Consistency runs both ways. A marker without a rules entry is a refusal, and so is a rules entry without a marker. |
| R12 | U3 | An assembled rule passes the same admission as a JSONL rule, plus the checks R1–R11. |

## Check, print and runtime

| R | Serves | Obligation |
| --- | --- | --- |
| R13 | U4 | `sunstack-rule` accepts a `check_rules` operation that assembles every rule in the invocation's RuleSets and runs full admission. It judges nothing, sends no model request, and needs no backend or credential. It exits non-zero listing every refusal with file, line and column, or exits zero. With print, it also returns each assembled rule: its name, the assembled question bytes, its target, and its verdict or check. That's for reviewing the question; the full runtime request also adds the RuleSet instructions, the selected evidence and batching, which are inspected through the existing prepared-request output. |
| R14 *(new target)* | U8 | `markdown_units` selects units in a reviewed document. `mode: unit` gives one occurrence per selected section, paragraph or list item; `units` is required and names the kinds. `mode: call_site` gives one occurrence per filter match inside a unit, and requires a non-empty `starts_with` or `pattern`. A regex that can match the empty string is a refusal. Optional `starts_with` or `pattern` (a regex over rendered-visible text, code spans excluded) filters, and each match maps back to its raw source byte span. `frontmatter_field` (with `field`) gives exactly one occurrence per document, for mechanical rules: the field's value, or *absent* when the field is missing, so a missing field is judged rather than skipped as zero matches. A document whose frontmatter is missing or malformed gives *unresolved*. Each occurrence's identity is its source, byte span and, in call-site mode, the match span, so several calls in one list item stay distinct. That identity follows the occurrence through its evidence labels, its questions' owners, request correlation, settlement and the report: two occurrences of one rule, batched together or split by budget, each read their own target. Zero matches are reported as a named result, distinct from a skipped rule, a failed selection and a stopped gate. |
| R15 | U5 | At runtime an invalid rule (any R1–R12 refusal tied to one marker or entry) is logged as a skipped rule, identified by source file and marker or key, with its diagnostics, and every valid rule still runs. An unreadable or malformed rules file or Markdown source skips every rule from that source as one source-level skip. Coverage is reported explicitly: `full` when nothing was skipped; `partial` when some rules were skipped and the rest completed, with exit 0, so the run works as the owner asked and the gap is visible; `none` when no valid rule remained, which is an incomplete report with today's incomplete exit. Skipped-rule counts are separate from today's not-applicable count. Invocation-level and RuleSet-header errors still refuse the whole invocation, as today. |
| R16 *(new verdict)* | U9 | A mechanical rule settles locally, with no model request, per target unit, in one of two forms. **`expect`**, the easy form in skill terms, is exactly one of `matches` (a regex over the unit's rendered-visible text, code spans excluded), `not_matches`, `starts_with`, or `present` (for `frontmatter_field`). **`ast_grep`** is an ast-grep rule object (`kind`, `regex`, `pattern`, `not`, `has`, `inside`, `all`, `any`). It is evaluated on the document's original parse tree, with ancestors visible to `inside`, the way `ast-grep scan` sees the whole file. A match whose range lies within the target unit is a fail. A target unit is never reparsed on its own. On an *absent* frontmatter value, `present` fails and every other `expect` form fails too. Check mode validates every `ast_grep` block with ast-grep's own rule parser and refuses an invalid one with file:line, the same way an unknown key is refused. The result is pass, fail, unresolved (the target unit couldn't be read) or cancelled, and only fail is a failed verdict. The marked text isn't sent anywhere; it's the explanation shown with a failure. The engine builds a backend only when model work is reachable, so an invocation of only mechanical rules, with no model-based gate or Router, needs no backend or credential. Whether a referenced file exists is outside this contract, and stays a CI script. |
| R17 | U5, U8, U9 | The report keeps today's fields and adds coverage and skipped rules (R15), per-occurrence spans and zero-match results (R14), and check results (R16). The report types reject unknown fields today, so these are a versioned report change. The supported consumers (the CLI and the Rust API) move together, and an older strict decoder is not promised compatibility. |

## Worked example

A skill teaching the call standard, as written:

```markdown
### Call Grammar<!-- K014 -->

A call the agent can't act on sends it away with nothing to bring back. Each form names when to go, where, what to do there, and what to return, and every call site uses exactly one of them.

### Steering

**Completion criteria.** Test every one twice: can the agent tell done from not-done, and does it demand the legwork? <!-- K015 -->
```

Its rules file, `rules/SKILL.rules.json`:

```json
{
  "K014": {
    "name": "skill.call.complete",
    "target": { "input": "skill",
                "select": { "kind": "markdown_units", "mode": "call_site",
                            "units": ["paragraph", "list_item"], "pattern": "MUST load|IF .*, load" } },
    "threshold": 0.9,
    "severity": "nudge"
  },
  "K015": {
    "name": "skill.step.completion",
    "target": { "input": "skill",
                "select": { "kind": "markdown_units", "mode": "unit",
                            "units": ["paragraph"], "starts_with": "Completion:" } },
    "threshold": 0.8
  }
}
```

The assembled question for `skill.call.complete` (R6), as the print flag shows it:

```text
Call Grammar

A call the agent can't act on sends it away with nothing to bring back. Each form names when to go, where, what to do there, and what to return, and every call site uses exactly one of them.
```

The heading's binding (R4) runs to the next heading of level 3 or higher, so it stops at `### Steering` and covers only the call paragraph. K015 binds only its own paragraph. On a reviewed skill, K014 gives one occurrence per call that matches its pattern (R14), and K015 gives one per paragraph that starts with "Completion:".

## Proof obligations

| V | Decides pass and fail for | Reference |
| --- | --- | --- |
| V1 | R1–R5: every refusal kind fires on a crafted source and names file:line; every allowed host binds the exact byte range; nested headings bind to the right boundary | the rows above, and CommonMark section semantics |
| V2 | R6–R8: the question bytes equal the bound text with exactly the R6 removals; two runs give identical bytes | R6's keep/strip table |
| V3 | R9–R13: check mode exits non-zero on each refusal kind and zero on a clean set, makes zero model requests and constructs no backend; print shows the R6 question | R13 |
| V4 | R14: on five real skills, the units and calls selected match a reviewed baseline, with no merged call identities | the spike's five-skill baseline (53 sections, 34 calls) |
| V5 | R15: with one broken marker in a source, the run completes, the report lists that skipped rule, and the other rules' verdicts are unchanged | R15 |
| V6 | R16: each `expect` form passes a good unit and fails a bad one; an `ast_grep` rule matches as ast-grep's own `scan` does on the same unit; an invalid `ast_grep` block is refused by check mode; an all-mechanical invocation builds no backend | R16, and `ast-grep scan` as the reference |
| V7 | R17 and the non-goals: existing JSONL RuleSets, `U001:` targets and today's reports behave exactly as before | the existing engine test suite |
