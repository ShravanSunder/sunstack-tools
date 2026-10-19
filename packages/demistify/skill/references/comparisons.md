# Compare corresponding topics

Use `ex-compare` when readers need to judge alternatives on the same topics. Each child section is one alternative and may contain ordinary HTML, Markdown, code, diagrams, Tree, Trace or ordinary pictures. A Gallery owns its own page; link to it instead of embedding its browser in a comparison. The agent chooses the presentation and width; readers choose content.

```html
<ex-compare label="Session lifetime" data-width="canvas">
  <section id="current" label="Current" icon="code">
    <section data-compare-row="owner"><h3>Owner</h3><p>The Transport owns lookup.</p></section>
    <section data-compare-row="disconnect"><h3>Disconnect</h3><ex-code lang="typescript"><script type="text/plain">registry.delete(id);</script></ex-code></section>
  </section>
  <section id="proposed" label="Proposed" icon="git-branch">
    <section data-compare-row="owner"><h3>Owner</h3><p>The Registry retains lookup.</p></section>
    <section data-compare-row="disconnect"><h3>Disconnect</h3><ex-code lang="typescript"><script type="text/plain">transport.close();</script></ex-code></section>
  </section>
</ex-compare>
```

Matching `data-compare-row` keys share CSS Grid tracks, including when one side has more content. Give all topic sections a key and use the same topic order where possible. A missing topic appears as **Not supplied**, rather than letting unrelated content drift into that row. Without keyed topic sections, each alternative keeps its own ordinary flow; no correspondence is inferred from similar text. There is no fixed criteria schema.

Choose columns for two or three short alternatives. Use `view="tabs"` when each alternative needs its full reading width or a long explanation; sections become content tabs with the normal arrow/Home/End keys. A tabbed comparison uses one alternative at a time. `data-width="reading"` and `data-width="canvas"` apply to the whole comparison.

For a larger set, add `choose` to show two content selectors, anchored like gallery Filters. Optional `selected="current proposed"` names the initial child IDs in display order. Selecting the other pane's current alternative swaps the pair; it never duplicates a live island. This is content selection, not a reader-facing layout or column-count setting.

Comparisons grow to fit their content in normal document flow; the page owns vertical scrolling and column headings stay sticky. Nested viewers such as code and pictures keep their own local controls. Narrow layouts stack alternatives and retain their topic order. A heading, fragment or walkthrough target inside a hidden alternative reveals that alternative and updates its selector/tab.

Sidebar inspection is an independent interactive copy; fullscreen uses the live content and returns it. Exercise nested state, choice, deep links, narrow containment and inspection for the actual composition being authored. A source comparison does not establish that either implementation ran.

The existing `slider` form remains for two same-size pictures; see [pictures](pictures.md). It is a different comparison task from aligning rich topic rows.
