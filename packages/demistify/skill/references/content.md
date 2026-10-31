# Content, exact sources and local detail

Use ordinary document flow for the explanation. `ex-markdown` gives Markdown an explicit source boundary, inspectable original text and stable heading scope. It is not an application or an automatic file browser.

## Choose a source carrier

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

A literal closing-script delimiter ends a raw HTML script before the component can read it. For Markdown containing that delimiter, keep the original `.md` file and use the documented embedding transform; do not truncate or rewrite supplied text. For exact plain-text drawings and their template carrier, use `trees.md`. Code/diff carriers are in `code.md`.

## Prepare a double-click reader

Browsers block sibling-file fetches from `file:` pages. Embed local Markdown while authoring, retaining the HTTPS library links:

```sh
pnpm dlx https://demistify.getagentstudio.dev/v1/demistify-cli.tgz embed /path/index.source.html /path/index.html
pnpm dlx https://demistify.getagentstudio.dev/v1/demistify-cli.tgz check /path/index.html
```

Run with Node 26+; the CLI tarball includes its dependencies. Keep output beside input or preserve relative page/image paths. The transform safely carries exact source, including a script terminator; it does not inline the runtime, fonts, pictures or arbitrary JSON. The reader runs no command. Later source edits require another embed.

After editing a known live source, regenerate its reader and collection index when present. Routine delivery needs no second temporary copy or byte-comparison pass. Use a comparison only to investigate a stale-output defect or an explicit freshness-proof request.

Ordinary relative HTML links, fragments and picture paths work when their files travel together. Inline supplied Explorer JSON or use an allowed HTTPS data URL for direct-file delivery; embedding Markdown does not prepare Explorer data. HTTPS assets need a network. Truly offline delivery needs the complete matching local distribution or the source API's standalone bundle, not just `runtime.js` and `theme.css`.

## Tables use the shared surface

Markdown tables receive their scroll wrapper automatically. An authored HTML table uses that wrapper explicitly:

```html
<div class="ex-table-scroll" data-width="canvas">
  <table>
    <thead><tr><th>Owner</th><th>Responsibility</th></tr></thead>
    <tbody>
      <tr><td>Registry</td><td>Retain the session</td></tr>
      <tr><td>Connection</td><td>Carry messages</td></tr>
    </tbody>
  </table>
</div>
```

Both native tables and wrappers use a neutral frame with shared 8px corners, a gray header and quiet stripes. Use the existing wrapper when wide columns need horizontal scrolling; a narrow native table needs no wrapper for its corners. Native corners are CSS-only, preserving saved comment target ancestry. Preserve a real `thead` or a first header-only row. Do not add status colors or per-page border/radius overrides to reproduce the shared treatment. `layouts.md` owns Reading/Canvas and horizontal containment.

## Optional detail stays local

A disclosure is a concise summary followed by useful optional depth:

```html
<details>
  <summary>Recovery evidence and its limits</summary>
  <p>The supplied trace covers lookup recovery. It does not observe a provider reconnect.</p>
  <ex-code title="Lookup guard" lang="typescript"><script type="text/plain">if (!session) return { kind: 'missing' };</script></ex-code>
</details>
```

It can contain ordinary HTML, Markdown islands, code, diagrams, lists, tables and other useful views. Use an explicit Markdown island inside a plain disclosure if writing Markdown. Keep its opening compact; an immediate document-sized heading and large section gap make optional explanation look like an unrelated page. `open` starts it expanded. A disclosure does not create guided stops or download behavior. Gallery browsing belongs on a separate linked page; `traces.md` adds an ordered evidence sequence when that ordering matters.

Preserve complete supplied text/data and source-relative paths. Use the main guide's lint pass; opening tables, links and disclosures is not a separate authoring gate.
