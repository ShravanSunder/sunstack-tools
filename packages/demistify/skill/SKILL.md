---
name: demistify-explainer
description: Use when explaining engineering work (architecture, a plan, a change, a review, a domain model, a status report) as a visual HTML page. Write content and HTML layout; the Demistify library renders Markdown, Mermaid, cards, trees, code notes, arrows, maps, guided routes, navigation, inspection and light/dark themes.
---

# Demistify explainer

Write one HTML file. You supply meaning, content and layout; the library supplies styling, icons, rendering, outline, theme toggle and route player. Never write colours, borders, fonts or shadows; layout CSS (grid, flex, gaps) is fine.

```html
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Session architecture</title>
<link rel="stylesheet" href="demistify/theme.css">
<script type="module" src="demistify/runtime.js"></script>
</head>
<body>
<ex-document>
…
</ex-document>
</body>
</html>
```

Use the library path you are given (default `demistify/`).

## Markdown first

Write prose as bare Markdown directly inside `ex-document`, tab sections, cards, callouts and their `<details>`. Use section headings (`##` in Markdown or `<h2>` in HTML) to build useful outline navigation. Bare Markdown must not contain `<`; for that, use `<ex-markdown><script type="text/markdown">…</script></ex-markdown>`.

## Islands

| Element | For |
|---|---|
| `<ex-card id title tone icon tag pick>` | one thing (component, entity, step, option, finding); body in Markdown; depth in `<details><summary>…</summary>…</details>`; `pick` marks the chosen option |
| `<ex-callout tone title>` | TL;DR, decision, risk, gotchas, open questions |
| `<ex-stat value label note tone>` | one number: `value="143 KiB" label="First load" note="was 443 KiB" tone="done"` |
| `<ex-mermaid>` | Mermaid in `<script type="text/plain">` |
| `<ex-code title lang start>` | code or a diff in `<script type="text/plain">`, then `<ex-note line="42" tone="risk">…</ex-note>` |
| `<ex-tree title>` | a Markdown list as a tree: status reports, checklists (`- [x]`), breakdowns; `- risk: text` gives a node a tone |

Tones (only these): `note` `tldr` `decision` `risk` `question` `done` `planned` `changed` `added` `removed`. Every card and region takes a tone: its icon and colour. Title = the specific thing; tone = its status or kind; `tag` = timing, location or tradeoff (`tone="planned" tag="after implementation"`, not a bold status sentence). Review severity: blocking `risk`, nit `question`, nice `note`.

`icon` = what the thing is: database hard-drives cloud globe cpu stack plug code terminal git-branch package gear bug test-tube user users robot lock key shield-check file folder book-open clock envelope chat-circle bell shopping-cart credit-card truck chart-line rocket.

Mermaid: `flowchart`, `sequenceDiagram`, `stateDiagram-v2`, `classDiagram`, `erDiagram`, `xychart-beta` (`x-axis ["A", "B"]`, `bar "Before" [1, 2]`). A node whose id equals a card `id` takes its tone and opens it, so keep node labels short.

## Layout: rows, not a stack

| Element | Does |
|---|---|
| `<ex-grid cols="3">` | side by side; child `span="2"` or `span="all"`; `sticky` keeps a side column in view; `swipe` makes a swipe row on phones |
| `<ex-tabs>` | child `<section label>` views; `columns` shows them side by side when they fit |

Patterns: a four-column summary row (`<ex-grid cols="4">` with a `tldr` callout `span="2"` + two stats); a diagram `span="2"` beside a `sticky` column of cards; options as cards in `<ex-grid swipe>` with `pick`; a chart beside the callout that states its conclusion. Phones get one column automatically.

## Routes

```html
<ex-route label="60-second tour">
<ex-step to="verdict">Start here.</ex-step>
<ex-step to="f1" open="sidebar">The blocking bug.</ex-step>
</ex-route>
```

Steps target any island, region or diagram node id. Add one or two routes.

## When you need more

Read the matching file in `references/` beside this guide only when the page needs it:

- `references/maps.md`: many things at once (a domain, a system) as a pannable board, arrows between boxes, hand-placed layouts, layers readers switch on and off.
- `references/code.md`: diffs across several files, line rules for notes.
- `references/trees.md`: trees that link to islands, mind maps.
- `references/pictures.md`: images, galleries, before and after.
- `references/tours.md`: tours across pages, walking a diagram step by step.

IF the page uses a supplied explorer graph document, load `references/explorers.md` and return a page that references it without recreating its data or application code.

## Check

Run the checker you are given (`demistify check page.html`) and fix every line until it reports 0 errors and 0 warnings; `--fix` repairs the mechanical ones.
