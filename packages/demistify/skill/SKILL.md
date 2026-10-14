---
name: demistify-explainer
description: Use when explaining engineering work (architecture, a plan, a change, a review, a comparison) as a visual HTML page. Write content and HTML layout; the Demistify library renders Markdown, Mermaid, cards, callouts, arrows, navigation, inspection and light/dark themes.
---

# Demistify explainer

Write one HTML file. You supply meaning, content and layout; the library supplies all styling, icons, rendering, outline navigation, a theme toggle and click-to-inspect. Never write colours, borders, fonts or shadows. Layout CSS (grid, flex, absolute positions, widths, gaps) is allowed.

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

Use the library path you are given; `demistify/` is the default. `ex-document` adds the outline (built from your `h1`–`h3`) and the theme toggle.

## Islands

| Element | Use for | Content |
|---|---|---|
| `<ex-markdown>` | prose, headings, lists, tables, code | Markdown inside `<script type="text/markdown">` |
| `<ex-mermaid>` | flows, sequences, states, dependency graphs | Mermaid inside `<script type="text/plain">` |
| `<ex-card id title tone tag>` | one thing: component, step, option, finding | ordinary HTML (`p`, `ul`, `strong`, `code`) |
| `<ex-callout tone title>` | TL;DR, decision, risk, gotchas, open questions | ordinary HTML |
| `<ex-grid>` | islands side by side; columns fit the width (`cols="3"` fixes them; children may set `span="2"`) | any islands |
| `<ex-tabs>` | alternative views of one subject | child `section`s, each with `label="…"` |
| `<ex-arrow from to label>` | a relationship between two islands that have ids | empty |
| `<ex-file-tree>` | files a change touches | plain-text tree inside `<script type="text/plain">` |

All attributes are optional except an arrow's `from`/`to`. Indentation inside `script` is removed for you. Islands nest: cards and callouts can hold Markdown, Mermaid or grids. A `mermaid` code fence inside Markdown also renders a diagram.

## Tones

`tone` names a meaning; the library picks the icon and colour. Use only these:

`note` · `tldr` · `decision` · `risk` · `question` · `done` · `planned` · `changed` · `added` · `removed`

The title says the specific thing ("Retry storms under load"); the tone says what kind of thing it is (`risk`).

## Progressive disclosure

Overview first; depth one action away.

- Card body = the summary. Depth goes in `<details><summary>Boundary and sources</summary>…</details>`. Cards with `details` get an inspect button that opens a sidebar with everything expanded.
- Link a diagram to its cards: when a Mermaid node id equals a card `id`, the node takes the card's tone colour and clicking it opens the card. So keep node labels short (just the name) and put status and detail in the card.
- Diagrams get an expand button. `zoom="sidebar"` or `zoom="modal"` makes any other island inspectable.
- `ex-tabs` when the reader needs one view at a time.

## Composition

Arrange islands in any order and layout: `ex-grid`, plain `display:grid`/`flex`, or a `position:relative` box with absolutely positioned islands for a whiteboard. Islands never move themselves.

- An arrow needs ids on both islands and at least 48px between them (`<ex-grid style="gap:56px">`).
- More than about six connected things: use Mermaid (linked to cards). A few richly described things: cards plus arrows.
- Headings: one `#` title, `##` sections, `###` sparingly. Cards and callouts need no headings.

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
<ex-grid style="gap:56px">
<ex-card id="socket" title="Socket" tone="changed" tag="transport">
<p>Carries frames; may drop at any time.</p>
</ex-card>
<ex-card id="registry" title="Session registry" tone="added" tag="new">
<p>Maps session ids to live state for 5 minutes after a drop.</p>
<details><summary>Boundary</summary><p>Does not survive a process restart.</p></details>
</ex-card>
</ex-grid>
<ex-arrow from="socket" to="registry" label="re-attaches"></ex-arrow>
<ex-callout tone="risk" title="Gotchas"><ul><li>Two tabs can claim one session.</li></ul></ex-callout>
</ex-document>
```

## Check

Open the page: an "authoring issues" panel lists unknown elements or tones, broken or too-short arrows, duplicate ids and diagrams that failed to render. A correct page shows no panel.
