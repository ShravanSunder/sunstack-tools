# Ordered presentations and targets

Inspect the existing content before writing a path: island IDs, Markdown headings, diagram-local node names and any containing tab/detail/layer. Content lives there once; presentations only reference it.

```html
<ex-presentation id="review" title="Session review"
  description="Requirements, the Registry node and the shared decision.">
  <ex-step to="requirements--recovery">Start with recovery.</ex-step>
  <ex-step to="after--Registry">Inspect this diagram's Registry.</ex-step>
  <ex-step to="lifetime" open="sidebar">Read the decision beside the page.</ex-step>
  <ex-step to="details.html#implementation">Continue on the details page.</ex-step>
</ex-presentation>
```

Each path needs a stable ID, reader-facing title and short description. One step is valid; multiple paths may reuse the same destination. Steps use `open="sidebar"` or `open="modal"` only when inspection helps. Keep narration about the destination, not implementation details.

- Whole island: `architecture`.
- Heading `## Recovery` inside Markdown island `requirements`: `requirements--recovery`.
- Node Registry inside Mermaid island `after`: `after--Registry`.
- Another HTML page: `details.html#implementation`.

Use IDs exactly, including case. Qualify Mermaid nodes when names repeat: a bare `Registry` does not identify which diagram you intend. Mermaid fences inside Markdown island `requirements` get IDs `requirements--diagram-1`, `requirements--diagram-2`, in fence order. Its first fence's Registry node is `requirements--diagram-1--Registry`. Confirm the rendered target and recheck links after inserting or reordering fences. For a durable authored heading slug, give the HTML heading an explicit ID.

The shared resolver reveals containing tabs, details and layers, then waits for Markdown or diagrams to render. Missing destinations produce a visible player error. The player carries the path when navigating between HTML pages, so normal cross-file continuation does not require fetching sibling HTML. Keep ordinary page/fragment links too. Use the current ex-presentation/ex-step contract; do not wrap content or recreate the old ex-route form.

Complete by opening a cold heading/node fragment, starting the named path, checking every Next/Previous destination and trying Back/Forward. For a cross-page stop, verify continuation and the return to its source page using the intended delivery method. A target in the wrong repeated diagram, an unresolved stop or a local-server-only result for a promised double-click artifact is not complete.
