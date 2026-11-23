# Trees and literal text drawings

`ex-tree` accepts `title`, optional `tone`/subject `icon`, `label` and `layout="outline|map"`; outline is the default. It is a hierarchy view, not successive pages or a Walkthrough. Bare `zoom` requests whole-tree inspection. `ex-file-tree` accepts `label` and plain-text path source; use `ex-pre` when exact drawing/whitespace is the content. Its `label` names the view and bare `zoom` makes inspection explicit when needed. Preserve the literal template example below and its formatter-ignore comment.

Choose from the input: hierarchical Markdown becomes `ex-tree`; an exact monospace/TUI drawing becomes `ex-pre`. Use `ex-file-tree` only for a plain repository-path illustration. A literal drawing needs no terminal emulator or per-document styling. IF a Tree is the page's reader Map rather than a `layout="map"` drawing, load `stories.md` and return its `ex-map` container, top-level section branches and scope binding.

## Mind map from shared navigation

In a typed design milestone, draw a need and its rules with `<ex-tree data-design-need="U1"></ex-tree>`, as [typed-design.md](typed-design.md#place-records-on-pages) teaches.

If the reader Map and the in-page mind map should use the same categories, put the topic hierarchy in [navigation.json](navigation.md#share-collection-navigation-through-json). Add a Tree inside the `ex-document` targeted by the page's `ex-nav src`. Choose `navigation="page"` for the current document's destinations and the ancestors that explain them. Choose `navigation="collection"` for a collection overview that can take the reader across documents. The reader Map and search still offer the full collection.

```html
<ex-nav for="content" src="./navigation.json"></ex-nav>
<ex-document id="content">
  <h1>Where state lives</h1>
  <ex-tree navigation="page" layout="map" label="Delivery topics">
    <a slot="details" data-for="#stored-custody" href="#custody-example" hidden>Delivery example</a>
  </ex-tree>
  <h2 id="stored-custody">Stored custody</h2>
  <p>The delivery record outlives the worker that processed it.</p>
  <section id="custody-example" label="Delivery example">
    <h3>What survives a disconnect</h3>
    <p>The retry reads the existing send identity from the durable record.</p>
  </section>
</ex-document>
```

The Tree uses node titles, icons, descriptions and links from the existing version-1 JSON. The title is the link and descriptions are plain text below it. Branches stay open. The optional Details link names a same-page destination in `data-for` and a separate same-page section in `href`. The icon-only Details button sits at the top trailing edge of the topic. It opens the supporting passage fullscreen and keeps the Tree in place underneath. Close or Escape returns focus to the Details button. Omit the declaration when the title and its destination give enough context. For a pannable canvas use an [existing Board](maps.md). The Tree can also sit in ordinary reading flow.

Use this for requirements, specification, and design pages that share a topic hierarchy. For example, a collection mind map can put the retry requirement and the retry sequence diagram under “Reliable delivery.” The Requirements page's Tree shows the relevant requirements and their context. Keep clauses, rationale, recommendations and decisions in the linked HTML. For a hierarchy unique to one diagram, use a Markdown Tree as below. For an ordered explanation across pages, use a Walkthrough.

## Independent trees

An `ex-file-tree` shows its plain-text source as written, so put the paths, indented as the tree should read, in a direct `text/plain` script:

```html
<ex-file-tree label="Changed files"><script type="text/plain">
src/
  session/
    registry.ts
    connection.ts
</script></ex-file-tree>
```

```html
<ex-tree title="Delivery work" icon="rocket"><script type="text/markdown">
- [Session ownership](#requirements--ownership-table)
  - done: Read the contract
  - [Registry in the new diagram](#after--Registry)
  - question: Confirm expiry semantics
- [ ] Verify recovery
</script></ex-tree>
```

A leading tone (`done:`, `risk:`) supplies the item's meaning/icon; `[x]`/`[ ]` supply checklist states. An unformatted leading `folded:` starts a branch closed: `- folded: risk: Branch`, followed by indented children. A native checklist prefix comes first: `- [ ] folded: Branch`. Unmarked branches start open. A marker on a leaf, inside inline code or emphasis stays literal; `▸` and `›` are literal characters, and a leading `>` keeps its Markdown blockquote meaning. Labels wrap beneath the text with the icon on its first line; no trailing +N badge is needed.

A Markdown link creates a separate trailing go-to icon. Same-document links with a nonempty fragment use an in-page jump icon; another page or external URL retains the outbound arrow, even when it has a fragment. Fragment-only, relative and absolute URLs are compared against the actual page address. The label or whole row does not become a navigation link. `layout="map"` shows the hierarchy left to right, with an outline on narrow screens.

Inspect every authored link's real destination. Use a stable island ID, scoped Markdown heading or scoped Mermaid node as taught by the main guide. Folding a branch and following its link are independent actions.

Icons are consistent across the entire tree. If any item supplies a tone/checklist or the root supplies an icon, unmarked branches/leaves receive neutral folder/file icons. An entirely plain tree omits the icon column. Neutral icons express structure, never an invented completion state. Authors do not insert spacing or fake status to align rows.

```html
<ex-pre id="review-layout" label="Review layout"><script type="text/plain">
┌─ Screenshots ──┬─ Selected picture ─────┐
│ Menu · Mobile │                       │
│ Menu · Laptop │                       │
└───────────────┴───────────────────────┘
</script></ex-pre>
```

The plain-text script preserves whitespace and literal characters except an HTML closing-script delimiter, which terminates the carrier before Demistify reads it. For that input, use the existing template carrier and HTML-escape its text:

```html
<!-- prettier-ignore -->
<ex-pre label="Literal markup"><template>first line
&lt;/script&gt;
&lt;widget&gt;&amp; value
last line</template></ex-pre>
```

Keep the formatter-ignore comment with this literal island: Oxfmt otherwise collapses bare-template line breaks. Preserve hierarchical Markdown in its `text/markdown` script. After formatting, compare decoded literal characters/newlines and Markdown hierarchy with the original. Escape `&` as `&amp;` and `<` as `&lt;` inside this template; the component reads its decoded text. Use the shared zoom/sidebar/fullscreen inspection rather than custom fonts, borders or controls. Verify the literal delimiter and line breaks in the result.

Keep the intended target IDs and exact text/line breaks. Use the main guide's lint pass; folding, navigation and inspector regression journeys belong to library development.
