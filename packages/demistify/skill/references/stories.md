# Stories, sections and the page Map

Use this when a page tells accounts of work: each account a short story with labelled rows and a supporting record, stories grouped into sections, and a Map panel that lets the reader jump between them. Three things share the word map. The page Map is `ex-map`, the reader navigation panel this reference teaches. A Board with `mode="map"` is a spatial ownership drawing (`maps.md`). A Tree with `layout="map"` draws a hierarchy as a map (`trees.md`).

## What you author and what the library derives

You supply the meaning: story prose and rich bodies, IDs and links, which section each story belongs to and whether it is primary or smaller, the order of sections, stories and record entries, each story's subject, time, icon and tone, the record and smaller-fold summary text under the count rule below, and initial disclosure states. The library derives the rest from that structure: each section header's primary and smaller counts, the Map's section totals and the members of every bulk expand/collapse. It also owns the shared side panel, the row labels, icons and colors.

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

## Sections and bulk folding

An `ex-section` groups stories under a sticky category header. It needs a stable `id` and a `title` without a count; `icon`, `tone` and `series` are optional. The header shows the derived split, such as "· 2 + 2 smaller", and its own expand/collapse control.

Smaller stories go in at most one native `details slot="smaller"`, which must be the section's last child. Its summary follows the count rule above, such as "2 smaller stories". Content that isn't a story, such as a short table or two `ex-grid` columns of decisions, is ordinary HTML in the section and outside every count. A section with no stories is valid and counts zero.

Put one `ex-fold-toggle for="<page scope id>"` beside the heading that introduces the sections to expand or collapse every story on the page. A section's header control covers only that section. Bulk actions change story roots, More, records, record entries and smaller folds; an ordinary `details` outside a story keeps its own state.

## The page Map

A page has at most one `ex-map`. Its `for` names the page scope (normally the `ex-document` id), and it contains exactly one `ex-tree` whose Markdown list is the hierarchy. Place it after the sections, at the end of the scope; when the reader opens it, its tree shows in the shared side panel. For every `ex-section` in the scope, give exactly one top-level list item whose own text links to the section with a fragment-only `#id` href, and write no total in that label: the Map appends each section's total from the same membership the header reads. Other branches may group links to stories, and a branch can start closed with `folded:`. IF a Map branch combines `folded:` with a tone or checklist marker, load `trees.md` and return its marker order and literal-marker cases.

The checker reports a missing local target, a duplicated section branch and a typed total on a section label as errors. A missing section branch is an error too, or, when any Map link isn't fragment-only, a warning that the branch can't be established without the deployed address. Either one means the section lacks its branch: give it exactly one top-level branch with a fragment-only `#id` href rather than reading the warning as coverage.

What the reader gets, with no page script, style or overlay from you:

- The Map button sits with the shared reader controls; its tooltip reads "Show or hide Map (M)". Pressing M, upper or lower case, toggles the same panel with the same protections as C for Comments: it ignores the key with Meta, Ctrl or Alt held, a repeat, an event another handler already prevented, composition input, and typing in a field, text area, select or other editable element.
- The Map opens in the one shared, non-modal side panel and replaces Comments there, and opening Comments replaces the Map. Closing returns the panel's contents and the trigger's state.
- A Map link uses ordinary fragment navigation, which reveals a target inside closed disclosures, including on a repeat click after the reader folded it again. Below 900px the panel closes after a local link so the destination is visible; at 760px and below the controls move to the top and the toolbar clears them.

## A complete example

An invented maintenance week on a small CSV library: four sections, primary and smaller stories, a rich showpiece, More, records with linked entries and the Map.

```html
<ex-document id="maintenance">
  <h1>Tabular reader: maintenance week</h1>
  <p>An invented account of one maintenance week on a small CSV library. The stories are examples, not records of a real project.</p>
  <h2>This week at a glance <ex-fold-toggle for="maintenance"></ex-fold-toggle></h2>

  <ex-section id="decide" title="Waiting on your call" icon="chat-circle" series="6">
    <ex-story subject="Reader options" time="Tuesday" icon="gear" tone="decision">
      <details id="story-encoding" open>
        <summary>Pick the default text encoding</summary>
        <p slot="headline">Files without a declared encoding are still guessed byte by byte.</p>
        <p slot="happened">Two issue reports traced garbled column names to the guess.</p>
        <p slot="for-you">Choose UTF-8 as the default, or keep guessing and document it.</p>
        <p slot="next">The change lands the day after your answer.</p>
        <details slot="record">
          <summary>The record: 2 entries</summary>
          <details id="encoding-utf8"><summary>Option: default to UTF-8</summary><p>Predictable for new files; older Latin-1 exports need an explicit <code>encoding</code> option.</p></details>
          <details id="encoding-guess"><summary>Option: keep guessing</summary><p>Old exports keep working and the garbled-name reports stay open. Compare <a href="#encoding-utf8">the UTF-8 option</a>.</p></details>
        </details>
      </details>
    </ex-story>
  </ex-section>

  <ex-section id="released" title="Released this week" icon="rocket" series="1">
    <ex-story subject="Parser" time="Monday" icon="code" tone="done">
      <details id="story-line-breaks" open>
        <summary>Quoted fields can hold line breaks</summary>
        <p slot="headline">A quoted cell that spans two lines now stays one cell.</p>
        <p slot="happened">The tokenizer keeps a quote open across the end of a line.</p>
        <p slot="for-you">Nothing to change; files that relied on the old split now read like other tools read them.</p>
        <p slot="next">Watch the next release's issues for multi-line reports.</p>
        <div slot="showpiece">
          <table>
            <thead><tr><th>Input</th><th>Before</th><th>After</th></tr></thead>
            <tbody><tr><td>One quoted cell over two lines</td><td>Two rows</td><td>One row</td></tr></tbody>
          </table>
        </div>
        <details slot="more"><summary>More</summary><p>A line break outside quotes still ends the row, as the format defines.</p></details>
        <details slot="record">
          <summary>The record: 2 entries</summary>
          <details id="line-breaks-change"><summary>The tokenizer change</summary><ex-code title="tokenizer.ts" lang="typescript"><script type="text/plain">if (insideQuotes) continue;</script></ex-code></details>
          <details id="line-breaks-test"><summary>The regression case</summary><p>A fixture with a two-line cell covers <a href="#line-breaks-change">the tokenizer change</a>.</p></details>
        </details>
      </details>
    </ex-story>
    <ex-story subject="Reader options" time="Wednesday" icon="database" tone="added">
      <details id="story-row-limit" open>
        <summary>Readers can stop after a row limit</summary>
        <p slot="headline">A new <code>maxRows</code> option ends parsing early.</p>
        <p slot="happened">Previews read large exports whole just to show their first rows.</p>
        <p slot="for-you">Use it for previews; by default every row is still read.</p>
        <p slot="next">Document it beside the streaming reader.</p>
        <details slot="record">
          <summary>The record: 1 entry</summary>
          <details id="row-limit-default"><summary>The option and its default</summary><p><code>maxRows</code> is unset by default, which means no limit.</p></details>
        </details>
      </details>
    </ex-story>
    <details slot="smaller">
      <summary>2 smaller stories</summary>
      <ex-story subject="Docs" time="Thursday" icon="book-open">
        <details id="story-delimiter-typo" open>
          <summary>A typo in the delimiter guide is fixed</summary>
          <p slot="headline">The semicolon example now shows a semicolon.</p>
          <p slot="happened">A reader reported the mismatch.</p>
          <p slot="for-you">No action needed.</p>
          <p slot="next">Nothing planned.</p>
          <details slot="record"><summary>The record: 0 entries</summary><p>The corrected page is the whole change.</p></details>
        </details>
      </ex-story>
      <ex-story subject="Build" time="Thursday" icon="gear">
        <details id="story-runner-pin" open>
          <summary>The CI runner image is pinned</summary>
          <p slot="headline">Builds use one fixed runner image.</p>
          <p slot="happened">A silent image update changed a test's line endings.</p>
          <p slot="for-you">No action needed.</p>
          <p slot="next">Review the pin when the image's support ends.</p>
          <details slot="record">
            <summary>The record: 1 entry</summary>
            <details id="runner-pin-reason"><summary>Why pin it</summary><p>Unpinned images can change between two runs of the same commit.</p></details>
          </details>
        </details>
      </ex-story>
    </details>
  </ex-section>

  <ex-section id="background" title="Background worth keeping" icon="book-open">
    <table>
      <thead><tr><th>Area</th><th>Who looks after it this month</th></tr></thead>
      <tbody><tr><td>Parser</td><td>Rotating maintainer</td></tr><tr><td>Docs</td><td>Volunteer reviewers</td></tr></tbody>
    </table>
  </ex-section>

  <ex-section id="checked" title="Checks and open gaps" icon="shield-check" series="3">
    <ex-story subject="Fuzzing" time="Friday" icon="bug" tone="risk">
      <details id="story-fuzz" open>
        <summary>One fuzz crash is still open</summary>
        <p slot="headline">A malformed header row crashes the reader instead of reporting an error.</p>
        <p slot="happened">The nightly fuzz run found it; the other seeds passed.</p>
        <p slot="for-you">Avoid untrusted header rows until the fix ships.</p>
        <p slot="next">Turn the crash into an error that names the row.</p>
        <details slot="record">
          <summary>The record: 3 entries</summary>
          <details id="fuzz-input"><summary>The crashing input</summary><ex-code title="seed.csv" lang="csv"><script type="text/plain">"id,"name</script></ex-code></details>
          <details id="fuzz-cause"><summary>Where it fails</summary><p>The header parser reads past the end of the row.</p></details>
          <details id="fuzz-plan"><summary>The planned fix</summary><p>Return a parse error for <a href="#fuzz-input">the crashing input</a>.</p></details>
        </details>
      </details>
    </ex-story>
  </ex-section>

  <ex-map for="maintenance">
    <ex-tree id="maintenance-map" label="Week map"><script type="text/markdown">
- [Your call](#decide)
- folded: Stories by area
  - [Reader options](#story-encoding)
  - [Parser](#story-line-breaks)
  - [Fuzzing](#story-fuzz)
- [Released this week](#released)
  - [Quoted line breaks](#story-line-breaks)
  - [Row limit](#story-row-limit)
- [Background worth keeping](#background)
- [Checks and open gaps](#checked)
  - [Open fuzz crash](#story-fuzz)
</script></ex-tree>
  </ex-map>
</ex-document>
```

The "Your call" branch shows that a section's Map label can name what the reader does there, while its href still targets the section. The headers read "· 1", "· 2 + 2 smaller", "· 0" and "· 1", and the Map appends the same totals: 1, 4, 0 and 1.

## When to stop

Complete when every supplied field, ID and link occurs once and each link resolves; sections, stories and record entries keep their supplied order, with assigned smaller stories in their fold; every story has its required slots once and its record entries carry their supplied IDs; each count you wrote matches its members, and each supplied count that differs from its members has a note beside it naming the supplied label and the number of members actually supplied; states follow the defaults above or the carried rule; each section has exactly one fragment-only top-level Map branch with no typed total; and the main guide's single lint pass accepts the page. That pass checks structure and local links, not readability or whether the supplied history is true.
