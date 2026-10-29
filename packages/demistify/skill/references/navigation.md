# Pages, headings and command search

The document owns navigation. Pages move between HTML files, the outline jumps to sections, and the command bar searches authored pages/headings or named Walkthroughs. They are usable without starting a guided path.

## Declare the collection with native links

Repeat the same page list on every collection page. Its first link is Home; the matching page URL supplies the current name and breadcrumb.

```html
<ex-nav for="content">
  <nav slot="pages" aria-label="Pages">
    <a href="index.html">Session guide</a>
    <a href="details.html">Recovery details</a>
  </nav>
</ex-nav>
<ex-document id="content">
  <h1>Recovery details</h1>
  <h2 id="identity">Retain the original identity</h2>
  <p>A replacement connection resolves the continuing session.</p>
</ex-document>
```

This example's companion `index.html` must exist and repeat the list. The library creates the top Pages dropdown, breadcrumb and heading outline; hand-authored breadcrumb UI is unnecessary. Home uses the built-in house icon. Real h2–h6 headings supply outline sections, with the current page as its label. Captions stay out of that outline.

Keep explicit `ex-nav` on pages offering a Walkthrough or Gallery Journey, even a single page/path. It creates their visible launcher. The `ex-presentation` declaration is hidden; relative page links alone do not declare a collection or reveal a path launcher. Path construction belongs to `tours.md`.

## Use stable, scoped targets

Give durable HTML headings explicit IDs and linked islands stable IDs. Markdown heading fragments derive from rendered heading text: trim, lowercase, replace each run of non-letter/non-number characters with a hyphen, remove end hyphens; an empty result becomes `section`. Markdown island `requirements` with `## Recovery` yields `requirements--recovery`. Repeated generated targets gain `-2`, `-3` and later suffixes as needed. Unscoped generated document headings use a `heading-` prefix. Prefer explicit HTML IDs when editing the label must not change an external link.

`## Recovery {#identity}` is not an explicit-ID feature here; it remains heading text. For that durable ID, write `<h2 id="identity">Recovery</h2>` or link to the actual generated fragment. Within a Markdown island, a shorthand link such as `[Recovery](#recovery)` resolves to that island's heading.

Diagram targets qualify the node by its island: `architecture--Registry`. Node names are case-sensitive. A Markdown fence gets `requirements--diagram-1` in fence order, so its node becomes `requirements--diagram-1--Registry`. Inspect the actual destination; inserting a fence can change that numbering. A bare repeated node name does not identify which diagram to reveal.

The resolver reveals containing tabs, disclosures and layers and waits for lazy Markdown/diagrams. A deliberately clicked lower heading stays current even when it cannot scroll all the way to the reading line; later deliberate scrolling resumes position tracking.

## Search the collection and keep the index current

Cmd/Ctrl-K or the search button beside the bottom theme control opens pages/headings search. Heading rows show page and ancestor context above the matching title. Walkthrough rows show source-page context, title, a concise authored description and the appropriate path/images icon. Use meaningful labels rather than a separate type column.

The current page's live headings and authored Pages links work without an index. Other pages' headings and descriptions require an embedded index generated from explicit local inputs:

```sh
pnpm dlx https://demistify.getagentstudio.dev/v1/demistify-cli.tgz index index.html details.html
```

The command reads local inputs, including referenced local Markdown and the exact retained source written by `embed`, prepares the collection and writes each listed HTML file **in place**. It does not fetch sibling HTML in the reader's browser. Give searchable Markdown islands stable IDs before running it. It retains explicit IDs and assigns stable native heading IDs. Search matches titles and heading ancestry, not whole-page prose or general fuzzy text.

After changing a heading, source, page path or walkthrough description, regenerate both the source snapshot and search index as appropriate. For direct-file readers, embed each live-source page first, then index the delivered collection. Indexing adds generated data, so a byte comparison with an unindexed fresh embed alone is not a like-for-like freshness check: apply the same indexing transform to the regenerated collection before comparing, or compare the exact source carriers and index destinations separately. Keep the live source pages and files as the authority; do not manually edit generated index data.

## Keyboard ownership

| Key, when its control is available | Action |
| --- | --- |
| Cmd/Ctrl-K | Open pages/headings search |
| P | Open the top Pages dropdown |
| W | Open the command bar's Walkthroughs scope |
| 1–9 in Pages or an empty Walkthroughs search | Immediately open that entry |
| Arrows and ↵ in a chooser | Move and open, including later entries |
| Escape | Close the chooser and restore its trigger |
| Hold Option | Show P/W hints beside their controls |

The bottom button shows ⌘K on Mac and Ctrl-K elsewhere. Editing fields, composing text, open dialogs/popovers and modified browser commands retain their keys. Tabs and Galleries own their local arrow behavior. Do not install a competing document shortcut handler or capture OS/browser commands outside document focus.

Finish with an ordinary page link, a cold scoped heading/node fragment, a contextual search result from another page, and a renamed heading after regeneration. Confirm chooser focus returns, descriptions remain current, and Pages/outline remain available after leaving a guided path. A successful static check does not prove native links exist or search data is fresh.
