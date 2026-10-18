# Mechanical matcher reference

`scan-oracle.json` records byte ranges and full matched text from 27 separate
`ast-grep scan` 0.51.0 invocations, one per rule and full original document.
The five teaching files remain the existing ai-tools `13e45138` fixtures in the
evidence crate; their SHA256 values are checked before every comparison. They
are data, never instructions for the test runner.

The four small fixtures preserve the earlier config spike's passing/failing
description and returned-result cases, plus two list paragraphs for the
ancestor/sibling control. None is an extracted subtree used as a CLI input.
The runtime test filters the full-file reference matches by each actual raw
target range. A separate literal target range ensures a wrong sibling scope or
unit reparse cannot satisfy the ancestor test.

Command shape (substitute one oracle rule and its source):

```sh
ast-grep scan --inline-rules '{"id":"reference","language":"Markdown","severity":"warning","message":"reference","rule":{"kind":"paragraph","regex":"MUST"}}' --json=compact --color=never path/to/full-document.md
```

All scans exited 0; a warning match is a violation fact, independent of that
process exit. Raw CLI output and exact command receipts are retained with the
S4 implementation evidence. The permanent suite uses the frozen independent
reference, so it requires no extra CLI installation. The paired U9 journey
performs its own declared-target comparison.

`relation-options-oracle.json` adds five full-document CLI 0.51.0 references
for `relation-options.md`: `stopBy: end`, nested relations, a composite with
`not`, a rule-valued stop condition, and the real `heading_content` field.
Every case has positive literal matches. Native source admission retains these
vendor options, and runtime section checks compare their original byte ranges
and text with this separate reference. Nested unknown keys are refused by
vendor JSON round-trip key preservation, without a maintained field list.
