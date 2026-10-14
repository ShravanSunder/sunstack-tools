---
name: demistify-explainer
description: Use when explaining engineering work (architecture, a plan, a change, a review, a comparison) as a visual HTML page. Write content and HTML layout; the Demistify library renders Markdown, Mermaid, cards, callouts, arrows, navigation, inspection and light/dark themes.
---

# Demistify explainer

Write one HTML file. You supply meaning, content and layout; the library supplies styling, icons, rendering, outline, theme toggle and inspection. Never write colours, borders, fonts or shadows; layout CSS (grid, flex, absolute positions, gaps) is fine.

## Page

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
…islands…
</ex-document>
</body>
</html>
```

Use the library path you are given (default `demistify/`). `ex-document` adds the outline (from `h1`–`h3`) and theme toggle.

## Islands

| Element | Use for | Content |
|---|---|---|
| `<ex-markdown>` | prose, headings, lists, tables, code | Markdown inside `<script type="text/markdown">` |
| `<ex-mermaid>` | flowcharts, sequences, states, classes, ER, bar/line charts | Mermaid inside `<script type="text/plain">` |
| `<ex-card id title tone tag pick>` | one thing: component, step, option, finding | HTML (`p`, `ul`, `strong`, `code`) |
| `<ex-callout tone title>` | TL;DR, decision, risk, gotchas, open questions | HTML |
| `<ex-grid>` | islands side by side; columns fit the width (`cols="3"` fixes them; child `span="2"`) | islands |
| `<ex-tabs>` | alternative views of one subject | child `section`s, each with `label="…"` |
| `<ex-flow>` | an `ex-grid` whose arrows route through the gaps, never across a box | islands with ids, then `ex-arrow`s |
| `<ex-arrow from to label>` | a relationship between two islands with ids | empty |
| `<ex-code title lang start>` | code or a unified diff (`lang="diff"`) with margin notes | code inside `<script type="text/plain">`, then `<ex-note line="42" tone="risk">…</ex-note>` |
| `<ex-file-tree>` | files a change touches | plain-text tree inside `<script type="text/plain">` |

Attributes are optional except an arrow's `from`/`to` and a note's `line` (counted from `start`, default 1; `@@` lines are not counted). Indentation inside `script` is removed for you. Islands nest. A `mermaid` fence inside Markdown also renders a diagram.

## Tones

`tone` names a meaning; the library picks icon and colour. Only these:

`note` · `tldr` · `decision` · `risk` · `question` · `done` · `planned` · `changed` · `added` · `removed`

Title = the specific thing ("Retry storms under load"); tone = what kind (`risk`). Omit for neutral. Review severity: blocking `risk`, nit `question`, nice `note`.

## Diagrams

Supported: `flowchart`, `sequenceDiagram`, `stateDiagram-v2`, `classDiagram`, `erDiagram`, `xychart-beta` (not pie, gantt, timeline, mindmap). A chart:

```text
xychart-beta
  title "Transferred KiB"
  x-axis ["No diagram", "With diagram"]
  y-axis "KiB" 0 --> 1000
  bar "Before" [443, 949]
  bar "After" [143, 650]
```

## Progressive disclosure

Overview first; depth one action away.

- Card body = summary; depth in `<details><summary>Boundary and sources</summary>…</details>`.
- Every island has sidebar and expand buttons on hover; both show everything expanded. `zoom="none"` removes them.
- A Mermaid node whose id equals a card `id` takes the card's tone colour and opens the card on click. Keep node labels short; put status and detail in the card.
- `ex-tabs` when the reader needs one view at a time.
- Options: cards side by side, `tag` names the tradeoff ("durable, adds a dependency"), `pick` marks the chosen one.

## Composition

Any order and layout: `ex-grid`, plain `display:grid`/`flex`, or a `position:relative` whiteboard with absolutely positioned islands.

- Connected cards go in `ex-flow` with their `ex-arrow`s inside it. An arrow outside a flow is drawn straight and reported if it crosses a box.
- More than about six connected things: Mermaid linked to cards. A few richly described things: cards in an `ex-flow`.
- One `#` title, `##` sections, `###` sparingly; cards need no headings.

## Example

```html
<ex-document>
<ex-markdown><script type="text/markdown">
# Session survival
Sessions outlive dropped sockets, so a reconnecting client resumes where it left off.
</script></ex-markdown>
<ex-callout tone="tldr">The registry, not the socket, owns a session.</ex-callout>
<ex-mermaid><script type="text/plain">
flowchart LR
  client[Client] --> socket[Socket] --> registry[Registry] --> session[Session]
</script></ex-mermaid>
<ex-flow>
<ex-card id="socket" title="Socket" tone="changed" tag="transport">
<p>Carries frames; may drop at any time.</p>
</ex-card>
<ex-card id="registry" title="Session registry" tone="added" tag="new">
<p>Maps session ids to live state for 5 minutes after a drop.</p>
<details><summary>Boundary</summary><p>Does not survive a process restart.</p></details>
</ex-card>
<ex-arrow from="socket" to="registry" label="re-attaches"></ex-arrow>
</ex-flow>
<ex-callout tone="risk" title="Gotchas"><ul><li>Two tabs can claim one session.</li></ul></ex-callout>
</ex-document>
```

## Check

An "authoring issues" panel on the page lists mistakes (unknown elements or tones, bad arrows or note lines, duplicate ids, failed diagrams). A correct page shows no panel.
