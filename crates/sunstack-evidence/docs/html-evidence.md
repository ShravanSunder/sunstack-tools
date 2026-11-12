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

For an HTML fragment, the check searches the full acquired target bytes, subject to acquisition budgets, for an element with an exactly matching decoded `id` attribute. It does not use `<a name="...">`. An optional target `anchor` reports `status` as `found`, `missing`, or `unsupported`, alongside `fragment`. A fragment on any non-HTML file is unsupported and makes the check incomplete; Markdown heading anchors are not resolved.

The HTML decoder handles semicolon-terminated named and numeric entities. Legacy shorthand without a semicolon remains raw, so an ID using that shorthand may not compare as a browser would.

## Reverse coverage

Use this decision step to check the identifier against the acquired document:

```json
{ "mechanical_check": { "tool": "contains_field", "field": "verification", "needle": "identifier" }, "yes": "pass", "no": { "finding": "The requirement has no verification coverage." } }
```

`document` resolves its `path` relative to the source piece's file and returns the raw matches of its ast-grep `rule` as a text list with source spans. It uses the shared file acquisition cache and budgets. A missing or unreadable file makes the field unavailable and the rule incomplete; a successfully read file with no matches yields an empty list.

`contains_field` accepts a text or text-list `field` and a single text `needle`. The needle must be nonempty and available, or the result is incomplete. It matches the needle as a whole token: the characters on either side must be absent or neither a Unicode letter, digit, nor `_`. Thus `R1` matches `(R1)` and `R1 / U1`, but not `R10`, `XR1`, or `R1_extra`.

## Language and model views

Evidence supports every language in ast-grep 0.51's `SupportLang`. Serialized evidence language tags use snake_case; existing `markdown` and `json` tags remain readable. JSON, YAML, and HCL parse errors mark the language failed and skip its rules. HTML, Markdown, and the other supported languages retain matches despite parse errors. The `yaml` front-matter field source is admitted only for Markdown kinds.

For asking rules, HTML fields are projected as text content: tags, attributes, and comments are omitted, entities are decoded as above, and whitespace is collapsed. Text inside `script` and `style` remains, as with DOM `textContent`. Other languages' fields remain raw. Mechanical checks use raw source fields, and locations retain raw byte ranges.
