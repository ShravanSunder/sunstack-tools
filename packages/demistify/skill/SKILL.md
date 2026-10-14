---
name: demistify-explainer
description: Use when explaining engineering work (architecture, a plan, a change, a review, a domain model, a comparison) as a visual HTML page. Write content and HTML layout; the Demistify library renders Markdown, Mermaid, cards, stats, code notes, arrows, maps, guided routes, navigation, inspection and light/dark themes.
---

# Demistify explainer

Write one HTML file. You supply meaning, content and layout; the library supplies styling, icons, rendering, outline, theme toggle, inspection and route player. Never write colours, borders, fonts or shadows; layout CSS (grid, flex, gaps) is fine.

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

Write prose as bare Markdown directly inside `ex-document`, tab sections, cards, callouts and their `<details>`. Headings (`#`, `##`) build the outline. Indentation is removed. Bare Markdown must not contain `<`; for that, use `<ex-markdown><script type="text/markdown">…</script></ex-markdown>`.

## Islands

| Element | For |
|---|---|
| `<ex-card id title tone tag pick>` | one thing (component, entity, step, option, finding); body in Markdown; depth in `<details><summary>…</summary>…</details>`; `pick` marks the chosen option |
| `<ex-callout tone title>` | TL;DR, decision, risk, gotchas, open questions |
| `<ex-stat value label note tone>` | one number, e.g. `value="143 KiB" label="First load" note="was 443 KiB" tone="done"` |
| `<ex-mermaid>` | Mermaid in `<script type="text/plain">` |
| `<ex-code title lang start>` | code or a diff (`lang="diff"`) in `<script type="text/plain">`, then `<ex-note line="42" tone="risk">…</ex-note>` |
| `<ex-file-tree>` | plain-text tree in `<script type="text/plain">` |

Tones (only these): `note` `tldr` `decision` `risk` `question` `done` `planned` `changed` `added` `removed`. Every card and region takes a tone: it is the icon and colour. Title = the specific thing; tone = its status or kind; `tag` = timing, location or tradeoff (`tone="planned" tag="after implementation"`, not a bold status sentence). Review severity: blocking `risk`, nit `question`, nice `note`.

Mermaid: `flowchart`, `sequenceDiagram`, `stateDiagram-v2`, `classDiagram`, `erDiagram`, `xychart-beta` (bars/lines: `x-axis ["A", "B"]`, `bar "Before" [1, 2]`, `bar "After" [3, 4]`). A node whose id equals a card `id` takes its tone and opens it on click, so keep node labels short.

## Layout: rows, not a stack

| Element | Does |
|---|---|
| `<ex-grid cols="3">` | side by side; child `span="2"` or `span="all"`; `sticky` keeps a side column in view; `swipe` makes a sideways swipe row on phones |
| `<ex-flow cols="3">` | an `ex-grid` whose `<ex-arrow from to label>` children route between boxes |
| `<ex-tabs>` | child `<section label>` views; `columns` shows them side by side when they fit |
| `<ex-board cols="2">` | a map of `<ex-region id title tone>` groups; children marked `detail` appear when the reader opens a region; built-in Map/Read switch |

Patterns:
1. Summary row: `<ex-grid cols="4">` with a `tldr` callout `span="2"` and two `ex-stat`.
2. Picture + parts: `<ex-grid cols="3">` with a diagram `span="2"` and a `sticky` column of cards.
3. Options: cards in `<ex-grid swipe>`, tradeoff in `tag`, `pick` on the chosen one.
4. Evidence: a chart beside the callout that states its conclusion.
5. Many things (a domain, a system): `ex-board`, one region per area, overview cards visible, depth marked `detail`.

Phones get one column automatically. Diagrams, code and trees get enlarge buttons; put a card's depth in `<details>`.

## Routes

Give different readers different paths through the same islands:

```html
<ex-route label="60-second tour">
<ex-step to="verdict">Start here.</ex-step>
<ex-step to="f1" open="sidebar">The blocking bug.</ex-step>
</ex-route>
<ex-route label="Reviewer pass" steps="f1 f2 f3"></ex-route>
```

Steps target any island, region or diagram node id. Add one or two routes.

## Example

```html
<ex-document>
# Session survival
Sessions outlive dropped sockets, so a reconnecting client resumes where it left off.

<ex-grid cols="4">
<ex-callout tone="tldr" span="2">The registry, not the socket, owns a session.</ex-callout>
<ex-stat value="98%" label="Resume after a blip" note="was 35%" tone="done"></ex-stat>
<ex-stat value="0%" label="Resume after deploy" note="still broken" tone="risk"></ex-stat>
</ex-grid>

<ex-grid cols="3">
<ex-mermaid span="2"><script type="text/plain">
flowchart LR
  socket[Socket] --> registry[Registry] --> session[Session]
</script></ex-mermaid>
<div sticky>
<ex-card id="registry" title="Session registry" tone="added" tag="new">
Maps session ids to live state for **5 minutes** after a drop.
<details><summary>Boundary</summary>Does not survive a restart.</details>
</ex-card>
</div>
</ex-grid>

<ex-route label="Quick tour" steps="registry"></ex-route>
</ex-document>
```

## Check

Run the checker you are given (`demistify check page.html`) and fix every line until it reports 0 errors and 0 warnings; `--fix` repairs the mechanical ones. In the browser, the "authoring issues" panel shows layout problems; a correct page shows none.
