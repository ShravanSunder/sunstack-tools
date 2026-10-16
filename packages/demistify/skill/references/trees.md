# Trees and literal text drawings

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

```html
<ex-pre id="review-layout" label="Review layout"><script type="text/plain">
┌─ Screenshots ──┬─ Selected picture ─────┐
│ Menu · Mobile │                       │
│ Menu · Laptop │                       │
└───────────────┴───────────────────────┘
</script></ex-pre>
```

The plain-text script preserves whitespace and literal characters. Use the shared zoom/sidebar/fullscreen inspection rather than custom fonts, borders or controls. The author may put any literal text drawing here; HTML inside it stays text.

Complete by folding without navigation, following the trailing go-to to the intended target, and checking a long label with inline code and multiple paragraphs. For ex-pre, compare text and line breaks in main and inspector; clipping or changed characters fail the intended faithful drawing.
