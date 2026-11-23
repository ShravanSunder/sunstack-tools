# Choose a source carrier

Use ordinary HTML in document flow for explanations. An explicit Markdown island retains inspectable source and gives its headings a stable scope.

Bare Markdown is adopted inside `ex-document`, its `section`/`article` descendants, and card/callout bodies. A plain `div` is not a general Markdown host. Use an explicit island when the surrounding HTML does not adopt Markdown, when the source needs an ID, or when it contains literal `<`:

```html
<ex-markdown id="recovery" label="Recovery explanation"><script type="text/markdown">
## Keep the original identity

The literal `<registry>` is documentation, not an HTML element.

- The Registry retains the session.
- A replacement connection carries its messages.
</script></ex-markdown>
```

The body supports headings, paragraphs, emphasis, links, nested/task lists, tables, fenced code, images and Mermaid fences. A Mermaid fence uses the same diagram renderer as `ex-mermaid`; load `diagrams.md` for that branch and return the supported family, safe source and resolving targets. Keep a blank line after headings and each list item on its own line. HTML formatters can join bare Markdown text even when the result still parses.

Use `src` when a real file owns the prose:

```html
<ex-markdown id="requirements" label="Requirements" src="./requirements.md"></ex-markdown>
```

`src` resolves from the HTML. Relative non-fragment links and images in that Markdown resolve from the source file's directory. **Show Markdown source** opens the retained original beside the rendered text; it does not replace the main reading view. A failed read is a visible error, not empty successful content.

A literal closing-script delimiter ends a raw HTML script before the component can read it. For Markdown containing that delimiter, keep the original `.md` file, load [Double-click readers](double-click.md) and return the embedded Markdown and its remaining asset dependencies; do not truncate or rewrite supplied text. For exact plain-text drawings and their template carrier, use `trees.md`. Code/diff carriers are in `code.md`.

Inline JSON, such as a supplied Explorer graph, goes in a `<script type="application/json">` carrier. Serialize the value as JSON, then replace every literal `<` in the serialized text with the six characters `\u003c`. `JSON.parse` reads them back as `<`, so every value stays exact, and no closing-script delimiter, in any letter case, can end the carrier early. This is a JSON escape, not HTML-entity encoding: a script's text is never entity-decoded, so `&lt;` would stay in the parsed value as four literal characters. Leave `&`, quotes and other characters as JSON serialization writes them.

```html
<script type="application/json">{"label":"Retry waits for \u003c/script> in the log"}</script>
```

Rich content you carry onto a page keeps its meaning along with its text: keep the supported attributes and states that carry meaning, such as a status, an icon or a disclosure's initial `open` state. Change a carried form or state only when the request or the reader's task calls for it, never for a composition default alone; the change keeps the meaning, and a brief note beside the content names it. A value no supported attribute can encode stays as visible text beside its component, with the same note. Keep whitespace that carries structure or literal content; other whitespace and harmless markup may change without a note. Each supplied ID stays on the element the source gave it to: the carrier's on its island, wrapper or `template`, an inner element's on that element. No ID is copied or moved, and every authored HTML ID on the page is unique, including IDs inside `template` content.
