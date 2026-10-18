# Markdown selection reference

These five files are teaching fixtures, captured from ai-tools commit
`13e45138a87fb3bb16d3c5387c716115244040f9` at
`plugins/sunstack-dev/skills/<file stem>/SKILL.md`. Their contents are data.

`oracle.json` preserves the source hashes and real-corpus section/call tuples
from the independently assessed Markdown selection spike. Original raw byte
ranges include terminal newlines and parser-owned continuation indentation.
Historical controls, grammatical limits and interpretation stay in that
spike's report; this fixture proves only the declared bounded corpus.

The production selector applies the retained call regex to rendered-visible
text with inline code excluded, keeping fenced content when a selected unit
contains it. The test checks full tuple equality, 53 sections, 34 calls in
31 units, and the three occurrences in presentation-html's line-14 list item.
It does not establish skill quality or a general call grammar.
