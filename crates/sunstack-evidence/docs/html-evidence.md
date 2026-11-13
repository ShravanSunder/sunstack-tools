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

## Language and model views

Evidence supports every language in ast-grep 0.51's `SupportLang`. Serialized evidence language tags use snake_case; existing `markdown` and `json` tags remain readable. JSON, YAML, and HCL parse errors mark the language failed and skip its rules. HTML, Markdown, and the other supported languages retain matches despite parse errors. The `yaml` front-matter field source is admitted only for Markdown kinds.

For asking rules, HTML fields are projected as text content: tags and comments are omitted, entities are decoded as above, and whitespace is collapsed. Text inside `script` and `style` remains, as with DOM `textContent`. Other languages' fields remain raw. Mechanical checks use raw source fields, and locations retain raw byte ranges.

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
