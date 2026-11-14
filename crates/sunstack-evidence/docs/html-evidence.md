# HTML evidence and cross-document checks

An ast-grep project can select HTML elements as evidence. For example, put
these rules in a directory listed in `evidence/sgconfig.yml` under `ruleDirs`:

```yaml
id: requirement-row
language: Html
rule:
  kind: element
  regex: '^<tr\s+id="R[0-9]+"'
---
id: whole-document
language: Html
rule: {kind: document}
```

The row selector expects a `tr` whose first attribute is a double-quoted `id` such as `R1`. Adjust it to the markup being checked. In `evidence.kinds`, define the fields used by the checks:

```json
{
  "requirement": {
    "rule": "requirement-row",
    "fields": {
      "identifier": { "from": "capture", "regex": "id=\"(R[0-9]+)\"" },
      "body": { "from": "remainder" },
      "links": { "from": "target", "regex": "href=\"([^\"]+)\"", "resolve": "path" },
      "verification": { "from": "document", "path": "verification-design.html", "rule": "whole-document" }
    }
  }
}
```

The `target` regex captures link text in its first group; this example handles nonempty double-quoted `href` values. To capture `href=""` too, use `*` in place of `+` in the group. An empty captured target is unavailable (`empty_target`) and makes the rule incomplete. See the [CLI README](../../sunstack/README.md#html-evidence) for the same kind in context.

## Forward links

Use this as a rulebook decision step for the `links` field:

```json
{ "mechanical_check": { "tool": "anchor_exists", "field": "links" }, "yes": "pass", "no": { "finding": "The link target or fragment is missing." } }
```

The checked root controls containment for `target` and `document` paths. Absolute paths inside it work as written; outside it, the field is unavailable (`outside_root`) and the rule is incomplete. The [CLI README](../../sunstack/README.md#repository-lint) explains how to choose a checked root.

Scope a forward-link kind to actual `href` attributes (and to fragment links when that is the check). `anchor_exists` requires a nonempty target list; a piece with no matching link is not a positive link witness.

A `target` field with `resolve: "path"` separates a `#fragment` from the physical file path. `anchor_exists` checks every target in the field. A missing file inside the checked root makes the check false; a target without a fragment behaves like `file_exists`. A same-document fragment such as `#R1` resolves against the piece's own file.

For an HTML fragment, the check searches the full acquired target bytes, subject to acquisition budgets, for an element with an exactly matching decoded `id` attribute. It does not use `<a name="...">`. An optional target `anchor` reports `status` as `found`, `missing`, or `unsupported`, alongside `fragment`. Unsupported anchors include a typed `reason` and make the check incomplete.

Markdown targets recognize explicit inline-HTML `id` values and anchor `name` values. Heading fragments require `evidence.markdown.heading_anchors` to select a named scheme. The supported scheme is `"github"`. With no scheme and no matching explicit anchor, the status is `unsupported` with reason `heading_scheme_not_configured`. Other document formats use reason `unsupported_document`.

For an HTML `href`, the captured value is decoded for semicolon-terminated named and numeric HTML entities, then the physical path and fragment are percent-decoded as UTF-8 before resolution and comparison with the decoded `id`. For example, `#A%20B` matches `id="A B"`. The captured name, source text, and spans remain raw for mechanical evidence. Legacy entity shorthand without a semicolon remains raw.

HTML target classification follows ast-grep's supported file types, including `.html`, `.htm`, and `.xhtml`.

With `as_kind`, a resolved fragment selects the narrowest requested-kind piece
containing the anchored element or heading, including when that file is already
in the catalog. No piece at the anchor, or an unresolved fragment, makes the
target field unavailable with `target_kind_missing`; it never borrows the first
piece. Without a fragment, first-piece behavior is unchanged. `opening` remains
the acquired file's opening, independently of the anchor-relative loaded fields.

## Reverse coverage

Use this decision step to check the identifier against the acquired document:

```json
{ "mechanical_check": { "tool": "contains_field", "field": "verification", "needle": "identifier" }, "yes": "pass", "no": { "finding": "The requirement has no verification coverage." } }
```

`document` resolves its `path` relative to the source piece's file and returns
the raw matches of its ast-grep `rule` with source spans. A scalar path keeps the
existing text-list output for a successful read. An ordered, nonempty path list
selects the first existing file and records `state: "document"`, its `path`,
`values` and `spans`, including when the match list is empty:

```json
{"from":"document","path":["verification.html","verification-design.html"],"rule":"whole-document"}
```

If every candidate is missing, raw evidence records `state: "missing_document"`
and the resolved candidate paths. A `complete` check on that field returns no,
so its decision can skip or find a missing page. Other checks requiring its text
remain incomplete until the rule routes around the missing input. For example:

```json
{"mechanical_check":{"tool":"complete","field":"verification"},"yes":"covered","no":"not_applicable"}
```

Read errors, outside-root paths and acquisition/scan limits remain unavailable
and incomplete; they do not fall through to a later candidate. No glob expansion
is performed. The shared cache and budgets still apply. Existing rules reading a
present scalar-path document retain their behavior; missing scalar documents now
support the same `complete` routing as candidate lists. This route operates per
subject; choose `not_applicable` to skip per-requirement warnings when an entire
page is absent. It does not add automatic finding deduplication.

Document fields are acquired after evidence selection. A selection question cannot name one, and a target's `as_kind` cannot name a kind containing document fields. Those configurations are refused during admission; a selected kind may still declare a document field that its selection question does not reference.

`contains_field` accepts a text or text-list `field` and a single text `needle`. The needle must be nonempty and available, or the result is incomplete. It matches the needle as a whole token: the characters on either side must be absent or neither a Unicode letter, digit, nor `_`. Thus `R1` matches `(R1)` and `R1 / U1`, but not `R10`, `XR1`, or `R1_extra`.

Every occurrence is checked, including overlaps: `A-A` matches the final occurrence in `XA-A-A` even though the earlier occurrence touches `X`.

Add `containing: "subject_field"` to `document` or `same_file` to retain only
raw matches mentioning that field's scalar value or any of its text-list values
under the same token rule. Filtering precedes a same-file first-line projection.
An empty list yields an available empty result; missing/empty scalar or unavailable
needles yield unavailable fields. Original spans, ordering and document candidate
resolution are preserved. Undefined/non-text references and cycles are refused.
See the CLI's [linked-row example](../../sunstack/README.md#collect-rows-linked-to-the-subject).

## Native heading fields

Native `html_units` sections and `markdown_units` kinds selecting exactly
`["section"]` can expose a heading without the following source line:

```json
{
  "heading": {"from": "section", "text": "heading"},
  "heading_path": {"from": "enclosing", "kind": "outline", "text": "heading"}
}
```

The referenced `outline` must itself be a native section kind. Enclosing entries
keep their existing containment and selection, outermost first, including an
equal-range section of the referenced kind. No enclosing match gives an empty
list. `same_file` and AST rule references do not support heading text.

HTML headings contain the h1–h6 element's asking text: tags are removed, entities
decoded once, and whitespace folded. Markdown headings keep their inline source,
including emphasis, links and entities, with only ATX heading markers or the
setext underline removed. Both the raw field and asking view contain this derived
heading text; spans still point to the original heading element or Markdown
heading block. Ordinary text budgets and cut markers apply. An unclosed HTML
heading with no bounded heading text is unavailable with reason `unreadable`.

A target's `as_kind` cannot name a kind containing heading fields. These cases
produce admission refusals at the owning field. Existing `first_line`, `full`,
`own` and `enclosing_own` fields retain their source semantics.

## Section prefixes

A subject can expose the preceding content of a configured containing kind:

```json
{
  "outline": {
    "html_units": {"mode": "unit", "units": ["section"]},
    "fields": {}
  },
  "subject": {
    "rule": "your-element-rule",
    "fields": {
      "preceding": {"from": "section_prefix", "kind": "outline"},
      "section_status": {"from": "enclosing", "kind": "outline", "text": "full"}
    }
  }
}
```

This fragment belongs in `evidence.kinds`; `your-element-rule` must exist in the
ast-grep project. `kind` names the caller's configured containing-section kind,
not a fixed built-in outline. Native HTML/Markdown sections or AST-backed kinds
can supply its ranges. Kind dependencies, selection and language checks apply.
Use a filter on that kind when a broader section should govern the subject.

For section `[S0,S1)` containing subject `[E0,E1)`, the scalar raw field is exactly
`[S0,E0)`: heading/opening included, subject and later bytes excluded. The nearest
configured containing range wins, independently of DOM wrappers. Each subject
has its own endpoint. A subject starting at `S0` returns available empty text
with a zero-width span. Source identity, content digest, byte offsets and
line/column coordinates still refer to the acquired original document.

Missing sections, source-version mismatches, crossing ambiguous containers or
scan-limited candidate sets yield `unavailable` (`unreadable`), with no whole-page
fallback. An unanswered containing-kind selection yields `selection_pending`.
The operation reuses acquired bytes and needs no additional read or credential.
Ordinary field budgets and cuts apply; `complete` cannot pass an unavailable or
cut prefix. HTML asking text uses configured text attributes and the same
projection as other fields, without adding closing tags or later content.

The prefix and full/status fields can appear separately in the same saved
prepare report. Field names do not isolate questions within a shared request or
guarantee relevance. Mechanical text checks read the raw prefix. Existing AST
checks still scope matches to their original selected subject, and cannot name
this field as an AST target. Use section-scoped containment to collect parsed
pieces from the prefix. A regex over raw tags is not parsed-element evidence.

## Section-scoped containment

An `inside` field can collect final pieces of a configured kind from the subject's
nearest configured containing section, including pieces in another language:

```json
{
  "pictures_before": {
    "from": "inside",
    "kind": "picture",
    "scope": {"section_prefix": "outline"}
  },
  "pictures_in_section": {
    "from": "inside",
    "kind": "picture",
    "scope": {"section": "outline"}
  }
}
```

This fragment belongs in a kind's `fields`. `picture` and `outline` name caller
kinds. Native sections or AST-backed kinds can supply the containing ranges, as
with `section_prefix`. Both kinds are dependencies; undefined kinds and cycles
are refused. A scope has exactly one of `section` or `section_prefix`; scoped
`inside` requires `kind` and refuses `rule`. Omitting `scope` keeps ordinary
subject-span containment unchanged.

For section `[S0,S1)` and subject `[E0,E1)`, the scope is `[S0,S1)` for `section`
or `[S0,E0)` for `section_prefix`. Only complete piece spans contained in that
range are returned. The nearest containing section wins; pieces under an earlier
heading outside it are excluded. Matching uses the original file and content
identity, so HTML subjects can collect Markdown fences inside configured islands
without an HTML/Markdown syntax-tree join.

The field is a text list of full raw piece texts and their original spans, ordered
by starting and ending byte. Raw syntax is retained in asking views too, including
HTML tags, so cross-language pieces are not reinterpreted as the subject's
language. Normal text budgets and cuts still apply. Scoped context does not
subtract from the subject's `remainder`.

An available section with no matching pieces yields an empty list. Missing,
crossing, stale or scan-limited section evidence, or scan-limited collected-kind
evidence, yields `unavailable` (`unreadable`). Unanswered section or contained-kind
selection yields `selection_pending` when it can affect the result. There is no
whole-document fallback. A mechanical `regex` check with `pattern: "(?s).*"` is false
for an empty list and true when any piece is present, including empty text; unavailable inputs remain
incomplete. `complete` checks availability/cuts, not nonemptiness.

For a section's own content, ending at the next configured heading of any level,
use `scope: {"section_own":["html_outline","markdown_outline"]}`. The nonempty
list names distinct configured outline kinds; both languages can contribute
boundaries in an HTML file. The nearest containing section in their union supplies
the start and maximum end; the next section start clips that end. A parent's own
content therefore cannot borrow a picture under a child heading. Only configured
kinds supply boundaries, and all participate in dependency, selection and match
budget checks. Missing, crossing or limited context remains unavailable; unresolved
boundaries remain selection-pending. Existing `section` and `section_prefix`
retain their whole-section and before-subject meanings.

`word_count` measures complete captured text content, including configured inline
HTML attributes, and `piece_count` counts captured text-list entries. Asking-view
cuts or pending sibling fields do not block an available field's count. Both
support integer bounds and field sums; findings
can interpolate their numbers. See the runnable
[section-picture example](../../sunstack/README.md#mechanical-example-substantial-sections-need-pictures).

## Language and model views

Evidence supports every language in ast-grep 0.51's `SupportLang`. Serialized evidence language tags use snake_case; existing `markdown` and `json` tags remain readable. JSON, YAML, and HCL parse errors mark the language failed and skip its rules. HTML, Markdown, and the other supported languages retain matches despite parse errors. The `yaml` front-matter field source is admitted only for Markdown kinds.

For asking rules, HTML fields other than scoped containment lists are projected as text content: tags and comments are omitted, entities are decoded as above, and whitespace is collapsed. Text inside `script` and `style` remains, as with DOM `textContent`. Other languages' fields remain raw. Mechanical checks use the stored field values: heading fields contain derived text as described above, while ordinary source fields remain raw. Locations retain raw byte ranges.

`alt` and `aria-label` attributes contribute text on every element. Project-specific attributes and outline exclusions belong in the project's evidence config:

```json
{
  "html": {
    "skip_tags": ["example-source"],
    "text_attributes": {"example-diagram": ["caption"]}
  },
  "markdown": {"heading_anchors": "github"}
}
```

Add these members to the existing `evidence` object alongside `ast_grep` and `kinds`. Both HTML collections default to empty. `skip_tags` suppresses outline headings inside the named tags; standard `<template>` contents are always excluded from the outline. It does not suppress asking text. Listed attributes contribute `[caption: value]` tokens in source order, with decoded values. Tag and attribute matching is case-insensitive; configuring a standard attribute does not duplicate it. Empty names, invalid shapes and unknown heading schemes are config refusals.

The low-level evidence API uses the same `DocumentEvidenceSettings` through `load_evidence_project` and `select_document`. The `sunstack exec` evidence plan does not carry these settings and uses standard defaults.

## Embedded Markdown

Configure the HTML elements whose inner source is Markdown:

```json
{
  "html": {
    "embedded_markdown": [
      {"tag": "script", "attributes": {"type": "text/markdown"}},
      {"tag": "custom-markdown", "attributes": {"data-format": "markdown"}}
    ]
  }
}
```

This fragment belongs in `evidence`, alongside `ast_grep` and `kinds`.
Each selector requires `tag`; `attributes` defaults to empty and requires every
listed attribute to have its exact decoded value. Tag and attribute names are
case-insensitive; values are case-sensitive. Unknown selector fields and empty
names are refused. No element or MIME type is selected implicitly.

For `<custom-wrapper><script type="text/markdown">…</script></custom-wrapper>`,
select the `script` element. Its inner source is parsed with the existing
Markdown grammar. Markdown ast-grep kinds such as `paragraph`, `list`,
`pipe_table` and `fenced_code_block`, and the existing `markdown_units` section,
paragraph and list-item selectors, become available in the HTML file. A fenced
`mermaid` diagram is a Markdown `fenced_code_block`; an HTML `<pre class="mermaid">`
remains selectable by an HTML ast-grep rule.

Each island is parsed independently. Inner bytes are neither entity-decoded nor
dedented; delimiters and surrounding HTML are excluded from the Markdown parse.
Raw fields, captures and spans retain original HTML bytes, file identity, digest
and line/column coordinates, including CRLF and Unicode. Markdown section and
sibling fields stay within their island. Ordinary match budgets cover both HTML
and Markdown results. The direct `select_document` API applies the same setting
to Markdown units and Markdown AST patterns in an HTML document.

Only elements with bounded opening and closing tags yield islands. Comments,
self-closing and unclosed elements do not. Repeated selectors produce one parse;
when selected elements nest, the outermost selected element owns that source.
HTML rules still match their original tree, including selected wrappers, so
callers choose which overlapping kinds to request. There is no mixed-language
parent or sibling syntax tree.

Markdown island pieces can read enclosing HTML rules or kinds with `enclosing`
(`first_line`, `full`, or `heading` for a native section kind), and an HTML outline
kind with `section_prefix`. These operations use original-source containment;
raw HTML context stays raw in the Markdown asking view. They require configured
islands and do not admit other ordinary cross-language directions or JSON joins.

With `embedded_markdown` configured, the catalog's pieces are ordered by file,
original starting byte and kind (for equal starts), interleaving surrounding HTML
with Markdown. Without the setting, existing selection, serialization and
kind-first catalog ordering remain unchanged. Markdown rule file globs continue
to apply to the original HTML path, so a rule restricted to `*.md` must include
the intended HTML paths to match an island.
