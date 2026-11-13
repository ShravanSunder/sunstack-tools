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

## Reverse coverage

Use this decision step to check the identifier against the acquired document:

```json
{ "mechanical_check": { "tool": "contains_field", "field": "verification", "needle": "identifier" }, "yes": "pass", "no": { "finding": "The requirement has no verification coverage." } }
```

`document` resolves its `path` relative to the source piece's file and returns the raw matches of its ast-grep `rule` as a text list with source spans. It uses the shared file acquisition cache and budgets. A missing or unreadable file makes the field unavailable and the rule incomplete; a successfully read file with no matches yields an empty list.

Document fields are acquired after evidence selection. A selection question cannot name one, and a target's `as_kind` cannot name a kind containing document fields. Those configurations are refused during admission; a selected kind may still declare a document field that its selection question does not reference.

`contains_field` accepts a text or text-list `field` and a single text `needle`. The needle must be nonempty and available, or the result is incomplete. It matches the needle as a whole token: the characters on either side must be absent or neither a Unicode letter, digit, nor `_`. Thus `R1` matches `(R1)` and `R1 / U1`, but not `R10`, `XR1`, or `R1_extra`.

Every occurrence is checked, including overlaps: `A-A` matches the final occurrence in `XA-A-A` even though the earlier occurrence touches `X`.

## Exact decoded text comparisons

Use `html_text` for a matched HTML element's decoded child text, or
`html_attribute` for one named attribute:

```json
{
  "excerpt": {"from": "html_text"},
  "source_name": {"from": "html_attribute", "name": "data-source"},
  "records": {"from": "document", "path": "messages.jsonl", "rule": "message-text", "text": "json_string"}
}
```

These are mechanical field values. HTML entities decode once, child text joins
in source order, and whitespace, punctuation and Unicode characters stay as
written. No spaces or attribute text are inserted into `html_text`. Attribute
names match case-insensitively; a missing attribute is absent, while a present
empty attribute is available empty text. Ambiguous malformed markup is unavailable.
Spans and content digests still identify the original element/attribute bytes.
The asking view does not decode these fields again; ordinary view cuts apply.

`document.text` defaults to `raw`, preserving existing raw text-list values.
Opt into `html_text` for HTML matches, or `json_string` for JSON string matches.
Each selected JSON span is parsed independently as one string, decoding escapes
such as `\"`, `\n` and `\u2019`. An object, number or malformed string is unavailable,
not converted to text. The selector language must match the projection. For a
JSON-lines file with a `text` string in each object, the caller can select values
without selecting the property names:

```yaml
id: message-text
language: Json
rule:
  kind: string
  inside:
    kind: pair
    field: value
    has: {field: key, regex: '^"text"$'}
```

Record selection remains caller-owned. Every output item retains its own raw
span and source digest; records are never concatenated. Acquisition, containment,
missing files and scan limits use the existing `document` path and shared budgets.

For exact contiguous membership, use:

```json
{ "mechanical_check": { "tool": "exact_contains_field", "field": "records", "needle": "excerpt" }, "yes": "pass", "no": { "finding": "No exact source span." } }
```

This check allows a substring inside a longer token, unlike `contains_field`.
It compares the stored strings without normalizing whitespace, punctuation,
Unicode or JSON escape syntax. Each text-list item is checked separately. A
nonempty available uncut scalar needle and complete source are required; empty
source lists cannot pass, and unavailable or cut fields end incomplete. A
successful membership check establishes text membership only, not authorship,
acceptance or relevance. Existing `contains_field` token boundaries are unchanged.

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
this field as an AST target. Its source span is retained for consumers to compare
against configured AST match spans; there is no config-level AST-selector-to-field
join yet. A regex over raw tags is not parsed-element evidence.

## Language and model views

Evidence supports every language in ast-grep 0.51's `SupportLang`. Serialized evidence language tags use snake_case; existing `markdown` and `json` tags remain readable. JSON, YAML, and HCL parse errors mark the language failed and skip its rules. HTML, Markdown, and the other supported languages retain matches despite parse errors. The `yaml` front-matter field source is admitted only for Markdown kinds.

For asking rules, HTML fields are projected as text content: tags and comments are omitted, entities are decoded as above, and whitespace is collapsed. Text inside `script` and `style` remains, as with DOM `textContent`. Other languages' fields remain raw. Mechanical checks use the stored field values: heading fields contain derived text as described above, while ordinary source fields remain raw. Locations retain raw byte ranges.

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
