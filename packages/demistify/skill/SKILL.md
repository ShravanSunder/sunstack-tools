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
		<meta charset="utf-8" />
		<meta name="viewport" content="width=device-width,initial-scale=1" />
		<title>Session architecture</title>
		<link rel="stylesheet" href="demistify/theme.css" />
		<script type="module" src="demistify/runtime.js"></script>
	</head>
	<body>
		<ex-document> …islands… </ex-document>
	</body>
</html>
```

Use the library path you are given; `demistify/` is the default. `ex-document` adds the outline (built from your `h1`–`h3`) and the theme toggle.

## Islands

| Element                                  | Use for                                                           | Content                                                   |
| ---------------------------------------- | ----------------------------------------------------------------- | --------------------------------------------------------- |
| `<ex-markdown>`                          | prose, headings, lists, tables, code                              | Markdown inside `<script type="text/markdown">…</script>` |
| `<ex-mermaid>`                           | flows, sequences, states, dependency graphs                       | Mermaid inside `<script type="text/plain">…</script>`     |
| `<ex-card title="…" tone="…" tag="…">`   | one thing: component, step, option, finding                       | ordinary HTML (`p`, `ul`, `strong`, `code`)               |
| `<ex-callout tone="…" title="…">`        | TL;DR, decision, risk, gotchas, open questions                    | ordinary HTML                                             |
| `<ex-grid cols="3">`                     | side-by-side islands; children may set `span="2"` or `span="all"` | any islands                                               |
| `<ex-tabs>`                              | alternative views of one subject                                  | children with `label="…"`                                 |
| `<ex-arrow from="id" to="id" label="…">` | a relationship between two islands with ids                       | empty                                                     |
| `<ex-file-tree>`                         | files a change touches                                            | plain text tree inside `<script type="text/plain">`       |

Every attribute is optional except `from`/`to`. Islands nest: cards and callouts can contain Markdown, Mermaid or grids. A ` ```mermaid ` fence inside Markdown also renders a diagram.

## Tones

`tone` names a meaning; the library picks the icon and colour. Use only these:

`note` · `tldr` · `decision` · `risk` · `question` · `done` · `planned` · `changed` · `added` · `removed`

Titles say the specific thing ("Retry storms under load"), the tone says what kind of thing it is (`risk`).

## Progressive disclosure

Show the overview first; keep depth one action away.

- Put the summary in the card body and the depth in `<details><summary>Boundary and sources</summary>…</details>`. Cards with `details` get an inspect button that opens them in a sidebar with everything expanded.
- Diagrams get an expand button. Add `zoom="sidebar"` or `zoom="modal"` to any other island to make it inspectable.
- Use `ex-tabs` when the reader picks one view at a time.

## Composition

Arrange islands in any order and any layout: `ex-grid`, or plain `display:grid`/`flex`, or a `position:relative` box with absolutely positioned islands for a whiteboard. Islands never move themselves.

- Give an island an `id` before an arrow points at it. Leave at least 48px between boxes an arrow joins (`ex-grid style="gap:56px"`).
- Prefer Mermaid when there are more than about six connected things; use cards plus arrows for a few richly described ones.
- Headings: one `#` title, `##` for sections, `###` sparingly. Cards and callouts do not need headings.

## Example

```html
<ex-document>
	<ex-markdown
		><script type="text/markdown">
			# Session survival

			Sessions outlive dropped sockets, so a reconnecting client resumes where it left off.
		</script></ex-markdown
	>
	<ex-callout tone="tldr">The registry, not the socket, owns a session.</ex-callout>
	<ex-mermaid
		><script type="text/plain">
			flowchart LR
			  Client --> Socket --> Registry --> Session
		</script></ex-mermaid
	>
	<ex-grid cols="2" style="gap:56px">
		<ex-card id="socket" title="Socket" tone="changed" tag="transport">
			<p>Carries frames; may drop at any time.</p>
		</ex-card>
		<ex-card id="registry" title="Session registry" tone="added" tag="new">
			<p>Maps session ids to live state for 5 minutes after a drop.</p>
			<details>
				<summary>Boundary</summary>
				<p>Does not persist across process restarts.</p>
			</details>
		</ex-card>
	</ex-grid>
	<ex-arrow from="socket" to="registry" label="re-attaches"></ex-arrow>
	<ex-callout tone="risk" title="Gotchas"
		><ul>
			<li>Two tabs can claim one session.</li>
		</ul></ex-callout
	>
</ex-document>
```

## Check

Open the page: an "authoring issues" panel lists unknown elements, unknown tones, broken arrow ids, arrows too short to see and diagrams that failed to render. A correct page shows no panel.
