# Pages, headings and command search

The document owns navigation. Pages move between HTML files, the outline jumps to sections, and the command bar searches authored pages/headings or named Walkthroughs. They are usable without starting a guided path. IF a page needs the reader Map panel, load `stories.md` and return its scope binding and linked section and story destinations.

## Share collection navigation through JSON

For a collection, author Pages, Map and complete Walkthroughs once. Each page loads the same JSON:

```html
<ex-nav for="content" src="./navigation.json"></ex-nav>
<ex-document id="content">
  <h1>How work runs</h1>
  <!-- Ordinary explanations, diagrams and stable target IDs live here. -->
</ex-document>
```

```json
{
  "version": 1,
  "pages": [
    { "id": "work", "title": "How work runs", "href": "how-work-runs.html", "icon": "path" },
    { "id": "state", "title": "Where state lives", "href": "state.html", "icon": "database" }
  ],
  "map": [
    { "title": "Reliable delivery", "description": "Follow custody across disconnects.", "icon": "shield-check", "children": [
      { "title": "Keep delivery through disconnects", "description": "User need · Preserve work until delivery resumes.", "icon": "user", "children": [
        { "title": "Stored custody", "description": "State owner · Inspect the durable delivery record.", "icon": "database", "target": { "page": "state", "fragment": "stored-custody" } },
        { "title": "Send, sleep, then handle the answer", "description": "Execution flow · Trace the gap between send and answer.", "icon": "envelope", "target": { "page": "work", "fragment": "seq-mail" } }
      ] }
    ] }
  ],
  "walkthroughs": [
    { "id": "delivery", "title": "How does delivery survive a nap?", "description": "Follow the real content in explanatory order.", "steps": [
      { "target": { "page": "work", "fragment": "seq-mail" }, "narration": "Find the point where the sending event ends.", "icon": "envelope" },
      { "target": { "page": "state", "fragment": "stored-custody" }, "narration": "Which facts survive the next object construction?", "icon": "database" }
    ] }
  ]
}
```

`version` is exactly `1`. `pages` is nonempty and ordered; its first page is Home. Each page requires unique `id`, `title`, `href` and a curated subject `icon`. Page IDs and Walkthrough IDs start with an ASCII letter and contain only letters, digits and hyphens. All page paths resolve beside the shared JSON source, including when a referring HTML page is nested elsewhere.

`map` and `walkthroughs` are arrays and may be omitted when absent. Each Map node requires `title`, a short navigational `description`, and a curated `icon`; it can have a `target`, `children`, or both. A target names a page ID and optional raw HTML/scoped island `fragment`, without `#`. References reuse the page registry, so changing one page address updates its Map and Walkthrough links. Unknown page IDs, duplicate page/route IDs, unknown fields/icons and unsafe link protocols are errors.

Each Walkthrough requires unique `id`, `title`, `description`, and nonempty ordered `steps`. Each step has a target, focused `narration`, and optional curated icon. Its explanations stay at their targets; narration frames what to inspect. Every peer page can list and start the route. Cross-page continuation reads the same shared definitions and uses the existing target resolver and player.

The right-hand Map stays fully expanded, with meaningful icons and whole-row links. Its search matches group and item titles and descriptions, retaining ancestor context. Opening Map focuses Search; Up/Down moves between rows, Enter follows a destination, and Escape closes Map and returns focus to its trigger. Left/Right moves to a parent/child without collapsing the Map. Descriptions explain what a destination offers; governing facts stay in its HTML content. It is independent of the normal left heading outline. Do not recreate it with a spatial Tree diagram or an author script. A page using `src` does not also author its own Pages/presentation lists, Map or Walkthrough declarations.

Ordinary document shortcuts do not fire while Map, Comments or an editable field has focus. Typing W, P, C or M in Map Search filters its contents; it does not open another tool. Cmd/Ctrl-K can explicitly open collection search from Map Search. Option/Alt page digits are navigation commands and also work from navigation searches, while content editors retain their text keys.

This mode needs HTTP serving for the shared JSON fetch. A missing or invalid source produces a visible navigation error while ordinary page content and its heading outline remain usable. The matching CLI checks local JSON and local destination IDs; remote sources are validated at runtime. Preparing direct-file copies and compiling standalone artifacts are separate delivery work, not implied by this fetch-based mode.

## Existing page-local navigation

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

The top bar orders the contents menu when needed, Pages, Search, theme, Comments, Walkthrough, Map, then breadcrumbs. Pages and Search share a pill surface. Breadcrumbs are hidden on small screens; Walkthrough (Road Horizon) sits immediately before Map. Cmd/Ctrl-K opens three search sections: Pages and headings, Walkthroughs, and Map. Each result puts its title first and breadcrumb context below. Map results include real groups and requirements: choosing a group reveals its expanded Map context; choosing a leaf opens its content. Walkthroughs use Road Horizon; picture journeys retain Images.

The current page's live headings and authored Pages links work without an index. Other pages' headings and descriptions require an embedded index generated from explicit local inputs:

```sh
pnpm dlx https://demistify.getagentstudio.dev/v1/demistify-cli.tgz index index.html details.html
```

The command reads local inputs, including referenced local Markdown and the exact retained source written by `embed`, prepares the collection and writes each listed HTML file **in place**. It does not fetch sibling HTML in the reader's browser. Give searchable Markdown islands stable IDs before running it. It retains explicit IDs and assigns stable native heading IDs. Search matches titles and heading ancestry, not whole-page prose or general fuzzy text.

After changing a heading, source, page path or walkthrough description, regenerate both the source snapshot and search index as appropriate. For direct-file readers, embed each live-source page first, then index the delivered collection. Indexing adds generated data, so a byte comparison with an unindexed fresh embed alone is not a like-for-like freshness check: apply the same indexing transform to the regenerated collection before comparing, or compare the exact source carriers and index destinations separately. Keep the live source pages and files as the authority; do not manually edit generated index data.

## Keyboard ownership

| Key, when its control is available | Action |
| --- | --- |
| Cmd/Ctrl-K | Search Pages and headings, Walkthroughs, and Map |
| Option/Alt + 1–9 | Open that numbered page in registry order |
| P | Open the top Pages dropdown |
| W | Open the command bar's Walkthroughs scope |
| 1–9 in Pages or an empty Walkthroughs search | Immediately open that entry |
| Arrows and ↵ in a chooser | Move and open, including later entries |
| Escape | Close the chooser and restore prior focus; pointer-opened search returns to its button |
| Hold Option | Show P/W hints beside their controls |

The Search button shows ⌘K on Mac and Ctrl-K elsewhere. Keyboard-opened search returns focus to the previous reading location when dismissed. Editing fields, composing text, open dialogs/popovers and modified browser commands retain their keys. Tabs and Galleries own their local arrow behavior. Do not install a competing document shortcut handler or capture OS/browser commands outside document focus.

When the user requests verification or a concrete navigation defect is being diagnosed, finish with an ordinary page link, a cold scoped heading/node fragment, a contextual search result from another page, and a renamed heading after regeneration, and confirm that chooser focus returns, descriptions remain current, and Pages/outline remain available after leaving a guided path. A successful static check does not prove native links exist or search data is fresh.
