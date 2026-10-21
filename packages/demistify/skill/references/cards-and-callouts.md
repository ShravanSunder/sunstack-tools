# Cards, callouts, numbers and labels

A card encloses one named thing. A callout draws attention to a consequence that changes the reader's understanding or decision. An `ex-stat` gives one reported number its unit and context. Ordinary explanation stays in document flow.

## Give a surface a meaning

```html
<ex-card id="registry-option" title="Registry owns lifetime" tone="planned" icon="database" tag="Recommended">
  <p>Proposed: retain lookup independently of a temporary connection.</p>
  <details><summary>Tradeoff</summary><p>Expiry and ownership now need an explicit policy.</p></details>
</ex-card>
<ex-callout title="Decision still open" tone="question" icon="clock">
  <p>The owner has not selected an expiry policy. The proposal does not settle it.</p>
</ex-callout>
<ex-stat value="2" label="Proposed ownership options" note="Source inventory; no execution observed"></ex-stat>
```

`ex-card` accepts `title`, `tone`, `icon`, `tag`, bare `pick` and an optional inspection `label`. A missing card tone produces a checker warning; choose the meaning deliberately. `ex-callout` accepts `title`, `tone`, `icon` and `label`; its default tone is `note`. `ex-stat` requires `value` and `label`; `note` and `tone` are optional. Its attributes are text, not a second rich-content body.

| Meaning | Tone choices |
| --- | --- |
| Ordinary information or short summary | `note`, `tldr` |
| A decision or unresolved question | `decision`, `question` |
| Risk or a known limitation | `risk` |
| Completion or future work | `done`, `planned` |
| A supplied change | `changed`, `added`, `removed` |

A tone supplies semantic color and a default icon. A curated subject `icon` overrides the pictured subject while retaining the tone. Use short explicit wording such as **Proposed**, **Recommended**, **Owner selected**, **Supplied source**, **Observed** or **Not executed** when the distinction matters. A colored dot alone does not communicate those facts.

Use `pick` only after the owner selects an option. It adds a separate Chosen label without changing the option's semantic tone. `tag` carries a short location, timing or tradeoff. Neither is a paragraph, a general inline badge or a substitute for evidence. A recommendation does not acquire `pick` merely because its author prefers it.

## Compose one useful unit

A card/callout body can contain paragraphs, lists, bare Markdown, ordinary HTML, short code, figures, disclosures and rich child islands. Direct card/callout disclosures also adopt bare Markdown. Nested cards are technically possible; use them only when both boundaries name different useful units. For source carriers and literal markup, apply `content.md`.

A framed option is useful when it is one independently named item:

```html
<ex-card title="Keep the existing lookup" tone="planned" icon="database">
  <p>Proposed: let the Registry retain the identity across reconnects.</p>
  <p><strong>Cost:</strong> define explicit expiry rather than relying on disconnect.</p>
</ex-card>
```

When a tab or comparison already groups the option, use explanation and its Flow directly. The Flow's cards are the named objects; another enclosing card usually adds a redundant frame:

```html
<h3>Separate lifetime from transport</h3>
<p>Proposed: the Registry owns lookup and the connection carries messages.</p>
<ex-flow cols="2">
  <ex-card id="registry" title="Registry" tone="note" icon="database"><p>Retains lookup.</p></ex-card>
  <ex-card id="connection" title="Connection" tone="note" icon="plug"><p>Carries messages.</p></ex-card>
  <ex-arrow from="connection" to="registry" label="resolve session"></ex-arrow>
</ex-flow>
```

Keep long explanations and Gain/Cost sections in ordinary flow, with normal spacing after the view. Use an ordinary picture or link to a dedicated Gallery instead of making a card into a screenshot-browsing application. Reading/Canvas applies to the whole chosen surface; its internal text follows that surface's component layout.

Plain cards do not get inspection buttons automatically. Bare `zoom` requests them; `zoom="none"` removes them. Native disclosures remain their own local control. Apply `inspection.md` when inspection is part of the composition.

## Subject icons and chips

The implemented subject names are:

`database`, `hard-drives`, `cloud`, `globe`, `cpu`, `stack`, `plug`, `code`, `terminal`, `git-branch`, `path`, `package`, `gear`, `bug`, `test-tube`, `user`, `users`, `robot`, `lock`, `key`, `shield-check`, `file`, `folder`, `book-open`, `clock`, `envelope`, `chat-circle`, `bell`, `shopping-cart`, `credit-card`, `truck`, `chart-line`, `rocket`.

Choose a subject from that set. Tone names such as `question` are not subject icons. Avoid per-page emoji/icon systems when the shared subject names express the meaning.

The existing shared `.chip` class is separate from `tag` and `pick`:

```html
<span class="chip">Source supplied</span>
```

It uses monospace text, a leading dot and a pill shape. There is no dedicated `ex-chip` or general regular-text semantic badge element in this binding. Adding `tone` to a native `span` does not create one. Do not turn a screenshot's green success pill into an invented API; a new status-badge primitive is a separate product proposal.

Finish by checking that every enclosure names a useful unit, titles and body have a clear hierarchy, optional detail folds independently, and recommendation/chosen/source/observation labels match the supplied facts. A missing tone, misleading Chosen label or repeated outer frame is a correction even when the markup parses.
