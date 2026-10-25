# Ordered walkthroughs and targets

A Walkthrough serves guided explanation, not a download link or local disclosure. Keep optional prose in `details` and ordinary page/file links as links. Required `title` and `description` explain the path's purpose; stable `id` makes it addressable. A boolean flag such as `variations` is enabled by presence: remove it to disable it, rather than writing `variations="false"`.

Inspect the existing content before writing a path: island IDs, Markdown headings, diagram-local node names and any containing tab/detail/layer. Content lives there once; presentations only reference it.

Keep `<ex-nav></ex-nav>` on the page even when it has only one path and no collection links. It supplies the visible Walkthroughs launcher; the path declaration below stays hidden. Use the launch control to start the path during verification.

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

Step narration supports focused Markdown: a bold lead, paragraphs, short lists, emphasis, links and inline code. Use an optional curated `icon` to name the step's subject. The player uses regular 18px prose and reserves space for its measured height. Keep document headings, images, tables, diagrams and large code in the target; narration does not create another document hierarchy. Authored HTML remains literal text, and executable link protocols are not enabled.

```html
<ex-step to="composition" icon="path"><script type="text/markdown">
**Why there are two MCP levels**

- The **cloud entry** connects providers and protected services.
- Each **computer service** exposes its configured tools.

Collaboration remains independently governed. Inspect `SessionId` before continuing.
</script></ex-step>
```

Relative narration links resolve from the page that declares the path, including after cross-page continuation. Plain sentences remain valid narration. The player title is larger than its regular narration. Stops position immediately, including cross-page continuation, rather than animating a long scroll from the top.

- Whole island: `architecture`.
- Heading `## Recovery` inside Markdown island `requirements`: `requirements--recovery`.
- Node Registry inside Mermaid island `after`: `after--Registry`.
- Another HTML page: `details.html#implementation`.

Use IDs exactly, including case. Qualify Mermaid nodes when names repeat: a bare `Registry` does not identify which diagram you intend. Mermaid fences inside Markdown island `requirements` get IDs `requirements--diagram-1`, `requirements--diagram-2`, in fence order. Its first fence's Registry node is `requirements--diagram-1--Registry`. Confirm the rendered target and recheck links after inserting or reordering fences. For a durable authored heading slug, give the HTML heading an explicit ID.

The shared resolver reveals containing tabs, details and layers, then waits for Markdown or diagrams to render. Missing destinations produce a visible player error. The player carries the path when navigating between HTML pages, so normal cross-file continuation does not require fetching sibling HTML. Keep ordinary page/fragment links too. Use the current ex-presentation/ex-step contract; do not wrap content or recreate the old ex-route form.

**Walkthrough** is the general name: it may span pages and mix text, diagrams and gallery pictures. **Gallery Journey** describes a picture-only walkthrough over gallery figures, using the same player. It may reference more than one gallery or page; it is not a separate route engine or a restriction on what a general walkthrough may contain.

An indexed collection lists its Walkthroughs and Gallery Journeys on every included page, including a page with no local path. Keep each declaration once on its source page and `ex-nav` on each peer. Load `navigation.md` for the explicit collection inputs and preparation order; return the indexed reader files and a shared path started from a peer without local paths. Without an index, the chooser uses loaded-page declarations and authored presentation links; it does not discover sibling files automatically. Cross-page continuation carries the selected path to its destination.

The Walkthroughs chooser, opened by W or the path button, shows the source page above each title and its authored description below. It uses an image icon when all targets are local pictures/gallery content, and a path icon for mixed, ordinary or cross-page explanations. Keep the title and description concise; no type column is needed. The player has an icon-only close action, Left/Right for steps and Escape to leave.

## Gallery Journeys

Author the picture collection as its own Gallery page: one `ex-gallery` directly inside `ex-document`, optionally after one h1. Put the hidden path declaration beside `ex-document`, targeting its existing figure IDs; keep prose sections on the linking explanation page. A mixed text/diagram path can continue onto the Gallery page, but only picture-only paths are Gallery Journeys. Pages/outline remain independent navigation; the thumbnail hamburger and player close action do not hide them.

Reference gallery figures directly, keeping their original captions and one shared player:

```html
<ex-presentation id="checkout" title="Complete checkout" description="Review the cart and payment captures." variations>
  <ex-step to="cart-mobile">Review the cart before continuing.</ex-step>
  <ex-step to="payment-mobile">Choose a payment method.</ex-step>
</ex-presentation>
```

`data-screen` and `data-state` identify a logical screen; `data-viewport` names its variations. The optional `variations` flag enables a button group and Up/Down switching while Left/Right continues between steps. Omit it for an exact pictured sequence. If the chosen variation is missing or ambiguous, show the gap on that step rather than substituting another picture. The floating player reserves a bottom gutter so it does not cover the picture or caption. Leaving restores the gallery's search, filters, selection, view and zoom.

Complete by opening a cold heading/node fragment, starting the named path, checking every Next/Previous destination and trying Back/Forward. For pictured journeys, check optional variations, missing captures, narrow-screen gutters and restored browse state after closing. For a cross-page stop, verify continuation and the return to its source page using the intended delivery method. A target in the wrong repeated diagram, an unresolved stop or a local-server-only result for a promised double-click artifact is not complete.
