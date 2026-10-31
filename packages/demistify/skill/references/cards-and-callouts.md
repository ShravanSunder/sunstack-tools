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

`ex-card` accepts `title`, `tone`, `icon`, `tag`, optional `series` (1–6), bare `pick` and an optional inspection `label`. A missing card tone produces a checker warning; choose the meaning deliberately. `ex-callout` accepts `title`, `tone`, `icon` and `label`; its default tone is `note`. `ex-stat` requires `value` and `label`; `note` and `tone` are optional. Its attributes are text, not a second rich-content body.

| Meaning | Tone choices |
| --- | --- |
| Ordinary information or short summary | `note`, `tldr` |
| A decision or unresolved question | `decision`, `question` |
| Risk or a known limitation | `risk` |
| Completion or future work | `done`, `planned` |
| A supplied change | `changed`, `added`, `removed` |

A tone supplies a default icon and semantic color on unclassified surfaces. A curated subject `icon` overrides the pictured subject while retaining the tone. Use short explicit wording such as **Proposed**, **Recommended**, **Owner selected**, **Supplied source**, **Observed** or **Not executed** when the distinction matters. A colored dot alone does not communicate those facts.

Use `pick` only after the owner selects an option. It adds a separate Chosen label without changing the option's semantic tone. `tag` carries a short location, timing or tradeoff. Neither is a paragraph, a general inline badge or a substitute for evidence. A recommendation does not acquire `pick` merely because its author prefers it.

## Compose one useful unit

Cards use 8px corners, one outer shadow and a darker header band. Body content has one shared 20 px outer inset, retaining normal paragraph spacing. A direct native `<footer>` adds an optional neutral lower area with 8 px vertical padding and no divider; leave it out when the card needs no footer. Keep body content directly inside the card so existing review targets retain their ancestry.

Use `series="1"` through `series="6"` for an explicit category shared across cards and regions. These reuse the six Mermaid chart palette slots: one hue supplies the filled heading band, perimeter edges and direct heading icon. The heading uses a 10% category tint darkened 12% black; body and footer stay neutral. The top edge is 2 px; other edges are 1 px. Omit `series` for a neutral surface. Invalid values are checker errors. Keep a visible category label (`tag` or body text); color alone is insufficient. Keep `tone` and explicit status text or badges; on categorized cards the category supplies the border and heading-icon color. Unclassified cards retain their existing tone paint.

```html
<ex-card title="Registry" tone="planned" series="2" tag="Store" icon="database">
  <p>Retains lookup across reconnects.</p>
  <footer>Store · proposed; expiry policy still needs review</footer>
</ex-card>
```

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

Plain cards do not get inspection buttons automatically. Bare `zoom` requests them; `zoom="none"` removes them. Native disclosures remain their own local control. When inspection is part of the composition, load `inspection.md` and return the supported opening choice and the intended reader behavior.

## Subject icons and chips

The implemented subject names are:

`database`, `hard-drives`, `cloud`, `globe`, `cpu`, `stack`, `plug`, `code`, `terminal`, `git-branch`, `path`, `package`, `gear`, `bug`, `test-tube`, `user`, `users`, `robot`, `lock`, `key`, `shield-check`, `file`, `folder`, `book-open`, `clock`, `envelope`, `chat-circle`, `bell`, `shopping-cart`, `credit-card`, `truck`, `chart-line`, `rocket`.

Choose a subject from that set. Tone names such as `question` are not subject icons. Avoid per-page emoji/icon systems when the shared subject names express the meaning.

The existing shared `.chip` class is separate from `tag` and `pick`:

```html
<span class="chip">Source supplied</span>
```

It keeps its monospace text, leading dot and pill shape. IF a fact needs a normal-text status or category label, load `small-parts.md` and return supported badge markup for that supplied fact. Do not add `tone` to a native `span` or use a chip as a general semantic badge.

Give every enclosure a useful unit name, distinguish title from body, and keep recommendation/chosen/source/observation labels faithful to the supplied facts. A missing tone, misleading Chosen label or repeated outer frame is a correction even when the markup parses.
