# Stories, sections and the page Map

Sections are empty declarations. Put authored content in separate `ex-section-item` elements. A Story account is a short story with labelled rows and a supporting record. Group stories in sections and let readers jump between sections using the page Map. Three things share the word map. The page Map is `ex-map`, the reader navigation panel this reference teaches. A Board with `mode="map"` is a spatial ownership drawing (`maps.md`). A Tree with `layout="map"` draws a hierarchy as a map (`trees.md`).

## What you author and what the library derives

You supply the story prose, rich bodies, IDs and links, the order of sections and entries, and each story's subject, time, icon, tone and disclosure states. Use sibling section items to assign stories to a section. A story can be primary or part of a native smaller group inside an item. The library derives the section counts, the Map totals and the members of the bulk expand/collapse controls. It also owns the shared side panel, row labels, icons and colors.

Infer nothing from importance. A tone, status, classification, routine/smaller placement or order comes from the supplied content; a quiet story is not "done" because nothing went wrong in it. Keep the supplied order of sections, stories and record entries; a story the source assigns to the smaller group goes into its section's smaller fold, keeping its order among the smaller stories.

Count rule for record and smaller-fold summaries: when the source supplies no summary, write the count from the members you wrote under it, the record's entries or the smaller fold's stories. When the source supplies a summary with a count, compare that count with those members before you keep it. IF they differ, load `content.md` and return its carried-form rule, then keep or change the label under it; either way, write a brief note beside the summary that names the supplied label and the number of members actually supplied.

A story's rows are always Happened, For you and Next. IF an account's supplied rows carry other labels or meanings, keep those labels as supplied in ordinary HTML or a card instead of mapping them into the three rows; for a card, load `cards-and-callouts.md` and return its named-card form. Ordered steps whose order is the meaning stay a Trace (`traces.md`); a hierarchy that isn't a work account stays a Tree.

## A story

An `ex-story` holds exactly one direct native `details` root with a stable `id` and one `summary`, the story's title. Inside that root, every other child carries a `slot`:

| Slot | How many | Element |
| --- | --- | --- |
| `headline` | exactly one | any block, such as `p` |
| `happened` | exactly one | any block |
| `for-you` | exactly one | any block |
| `next` | exactly one | any block |
| `showpiece` | at most one | any block holding rich content |
| `more` | at most one | native `details` with one `summary` |
| `record` | exactly one | native `details` with one `summary` |

The library adds the "Happened", "For you" and "Next" labels and their icons, so write only the content after them. `subject` and `time` on `ex-story` are supplied text shown beside the title; `icon` and `tone` use the curated values. IF a story or section needs an icon, tone or series, load `cards-and-callouts.md` and return its supported values. Slots accept supported rich HTML and islands: emphasis, inline code, links, native tables, `ex-code`, Mermaid and other views from their own references.

The record's direct `details` children are its entries, each with its supplied `id` and one `summary`. Its summary follows the count rule above, such as "The record: 2 entries"; the library doesn't count the entries and the checker doesn't verify the number. When one entry or story refers to another, link to its `id` instead of repeating its body.

States for a newly composed page: each story root starts `open`; More, the record, every record entry and the smaller fold start closed. IF the source supplied a story's or disclosure's state, or the request asks for one, load `content.md` and return its carried-form and state rule, then keep the supplied state; a change the request or the reader's task calls for gets a brief note beside it. There is no auto-expand option; `open` on the native element is the state.

## Sections

An `ex-section` is an empty declaration. Write its content in separate `ex-section-item` elements outside it.

```html
<ex-section id="decide" title="For you to decide" icon="user" holds="needs-owner"></ex-section>
<ex-section id="know" title="Things to know" icon="file"></ex-section>
<ex-section-item section="know"><p>The alarm follows the existing cadence.</p></ex-section-item>
```

A record item uses `record="D5"`. Demistify places it using the checked decision or assumption state. Author one item per record. Within each section, items keep their source order. Declare each reachable `holds` value once, using the [routing table for typed records](typed-design.md#place-records-on-pages). For ordinary prose or a story, use `section="know"` to target a section without `holds`. Each item takes one destination. The header and Map share the count: each record or prose item counts once, and a Story group counts its stories.

Give a section a semantic `tone` to set its default icon and header tint. Use `icon` to choose a different subject icon or `series` to choose a different palette tint. For authored items using `section`, a toned section supplies the item surface, tinted rail, and spacing. The Section supplies its own `h3`. Start each authored item with an `h4` so it appears beneath that section in the outline. Put the summary, links, and evidence directly inside the item. Use a [badge row](small-parts.md#short-status-labels) under the heading when an item has status chips. Place a [Canvas table wrapper](tables.md) directly in the item for a full-width check table. The wrapper uses the standard text gap when it immediately follows the heading.

Place a Flow directly in the section item when cards and arrows explain the change or dependency. The section keeps its rail and spacing. The Flow’s cards supply the surfaces.

```html
<ex-section id="next" title="What’s done / what’s next" tone="note"></ex-section>
<ex-section-item section="next">
  <h4>Import checks</h4>
  <div class="ex-badge-row">
    <ex-badge tone="note">Local fixture passed</ex-badge>
  </div>
  <p>A run against the deployed service is next.</p>
</ex-section-item>
```

For story controls, `<ex-fold-toggle for="work"></ex-fold-toggle>` targets the element with `id="work"`, such as the document or a section. The generated section header also provides a control when the section has stories. Story bulk actions include a direct `<details slot="smaller">` inside a placed section item. Ordinary attachment disclosures stay independent.

```html
<ex-section id="released" title="Released this week"></ex-section>
<ex-section-item section="released">
  <ex-story subject="Parser" icon="file">
    <details id="story-row-limit" open>
      <summary>Long rows get a clear error</summary>
      <p slot="headline">The parser reports the row number when a row exceeds the limit.</p>
      <p slot="happened">The documented limit is enforced on input.</p>
      <p slot="for-you">Use the row number to find the rejected input.</p>
      <p slot="next">Check the location shown for the longest accepted row.</p>
      <details slot="record"><summary>The record: 0 entries</summary></details>
    </details>
  </ex-story>
</ex-section-item>
```

## The page Map

A page has at most one `ex-map`. Its `for` names the page scope (normally the `ex-document` id), and it contains exactly one `ex-tree` whose Markdown list is the hierarchy. Place it after the sections, at the end of the scope; when the reader opens it, its tree shows in the shared side panel. For every `ex-section` in the scope, give exactly one top-level list item whose own text links to the section with a fragment-only `#id` href, and write no total in that label: the Map appends each section's total from the same membership the header reads. Other branches may group links to stories, and a branch can start closed with `folded:`. IF a Map branch combines `folded:` with a tone or checklist marker, load `trees.md` and return its marker order and literal-marker cases.

The checker reports a missing local target, a duplicated section branch and a typed total on a section label as errors. A missing section branch is an error too, or, when any Map link isn't fragment-only, a warning that the branch can't be established without the deployed address. Either one means the section lacks its branch: give it exactly one top-level branch with a fragment-only `#id` href rather than reading the warning as coverage.

What the reader gets, with no page script, style or overlay from you:

- The Map button sits with the shared reader controls; its tooltip reads "Show or hide Map (M)". Pressing M, upper or lower case, toggles the same panel with the same protections as C for Comments: it ignores the key with Meta, Ctrl or Alt held, a repeat, an event another handler already prevented, composition input, and typing in a field, text area, select or other editable element.
- The Map opens in the one shared, non-modal side panel and replaces Comments there, and opening Comments replaces the Map. Closing returns the panel's contents and the trigger's state.
- A Map link uses ordinary fragment navigation, which reveals a target inside closed disclosures, including on a repeat click after the reader folded it again. Below 900px the panel closes after a local link so the destination is visible; at 760px and below the controls move to the top and the toolbar clears them.

## An example with both destinations

```html
<ex-section id="open" title="For you to decide" holds="needs-owner"></ex-section>
<ex-section id="settled" title="You decided" holds="settled"></ex-section>
<ex-section id="agent" title="Decided without you" holds="decided-by-agent"></ex-section>
<ex-section-item record="D5">
  <ex-choice>
    <ex-option data-option="keep"><template slot="summary">Keep the current value.</template></ex-option>
    <ex-option data-option="change"><template slot="summary">Store the replacement.</template></ex-option>
  </ex-choice>
</ex-section-item>
```

The decision record supplies the question, option IDs and titles. Write the record ID once, on `ex-section-item`. Put the explanation, option summary templates and optional Details templates inside its one native `ex-choice`. Keep all options after the decision, using the same IDs. A ruling or assumption uses the same section item with its authored explanation as direct content. Demistify supplies its title, chips and card surface, and shows the date from settled decision authority.

```html
<ex-section id="assumed" title="Assumed" holds="assumed"></ex-section>
<ex-section-item record="A1"><p>The planned check uses a restart fixture.</p></ex-section-item>
```

Run the typed design checker after editing the page or the record.

## When to stop

Run the matching page or initiative check. Confirm that supplied IDs, links, order and native disclosure states survived. Each section has one declaration and, when the page has a Map, one top-level fragment link. Confirm that each story has its required slots and that each record item has the reachable section declarations.
