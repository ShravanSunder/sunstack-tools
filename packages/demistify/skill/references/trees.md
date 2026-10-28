# Trees and literal text drawings

`ex-tree` accepts `title`, optional `tone`/subject `icon`, `label` and `layout="outline|map"`; outline is the default. It is a hierarchy view, not successive pages or a Walkthrough. Bare `zoom` requests whole-tree inspection. `ex-file-tree` accepts `label` and plain-text path source; use `ex-pre` when exact drawing/whitespace is the content. Its `label` names the view and bare `zoom` makes inspection explicit when needed. Preserve the literal template example below and its formatter-ignore comment.

Choose from the input: hierarchical Markdown becomes `ex-tree`; an exact monospace/TUI drawing becomes `ex-pre`. Use `ex-file-tree` only for a plain repository-path illustration. A literal drawing needs no terminal emulator or per-document styling.

```html
<ex-tree title="Delivery work" icon="rocket"><script type="text/markdown">
- [Session ownership](#requirements--ownership-table)
  - done: Read the contract
  - [Registry in the new diagram](#after--Registry)
  - question: Confirm expiry semantics
- [ ] Verify recovery
</script></ex-tree>
```

A leading tone (`done:`, `risk:`) supplies the item's meaning/icon; `[x]`/`[ ]` supply checklist states. A leading chevron folds a branch. Labels remain normal text, wrapping beneath the label with the icon on its first line; no trailing +N badge is needed. A Markdown link creates a separate trailing go-to icon. The label or whole row does not become a navigation link. `layout="map"` shows the hierarchy left to right, with an outline on narrow screens.

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
