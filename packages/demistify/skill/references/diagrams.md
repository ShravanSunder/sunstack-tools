# Mermaid views and their source

Choose a diagram for a relationship the reader needs to see. Use Flow for responsive card-to-card dependencies, Board for authored ownership regions, and Mermaid for a supported family that expresses the question more directly. Keep a small diagram about one question; put the detailed explanation in the document or a named linked card.

| Family | Use it for |
| --- | --- |
| `flowchart` | Directed dependencies or decisions |
| `sequenceDiagram` | Ordered messages between participants |
| `stateDiagram-v2` | States and transitions |
| `classDiagram` | Type relationships |
| `erDiagram` | Entity relationships |
| `xychart-beta` | A supported numeric chart |

These are the binding's renderer families, not a promise that all upstream Mermaid grammar/configuration works. If a needed feature is unsupported, simplify or choose another supported view; do not invent a component attribute or quietly drop the information.

## Shared color defaults

Flowchart nodes and groups, sequence participants, states, classes and entities automatically use the shared six-color categorical palette with soft fills, clear borders and theme-aware text. Repeated identities in one drawing keep their color; start/end markers and relationship lines stay neutral. Chart series use the same palette. Names, arrows and relationship labels still carry meaning: a category color does not mean success, failure or a supplied owner decision.

Prepared SVGs and live-rendered diagrams use the same defaults. Pages that load the current hosted runtime and theme adopt them on reload; a standalone file containing an older embedded library must be rebuilt with the new binding.

Slots cycle after six identities. Assignment follows the drawing's identity order, so editing its structure can change colors; these are visual distinctions, not persistent domain identities. Do not use color alone to identify an entity across separate diagrams. Existing linked-node tones and authored styles retain their precedence.

For a flowchart whose colors need to express supplied categories, pin those categories with the built-in word-only classes `slot1` through `slot6`:

```text
flowchart LR
  reader[Reader] --> worker[Worker]
  worker --> store[Store]
  class reader slot1
  class worker slot2
  class store slot3
```

These use the same blue, amber, teal, violet, rose and green slots as the shared theme. Name the categories in adjacent prose or a key, and reuse the same explicit slot for the same category. No `classDef`, raw hex color or page CSS is needed. When authoring a custom class, use word-only names; the pinned renderer splits property lists on commas, so comma-containing `rgba()`/`color-mix()` values are unsupported there. Use the shared `--chart-N` and `--chart-N-soft` variables instead.

Entity fields keep separate type/name columns in the bundled monospace font, at the sizes used to lay out the SVG. Quoted field notes are retained as tooltips; they are not visible paragraphs. Keep long rules and explanations in a native table beside the diagram when readers must see them without hovering.

## Name numeric series and their meaning

Demistify's chart adapter accepts a quoted name after each `bar` or `line` keyword and retains it in the rendered legend. Name each series, supply actual category labels and identify the axis units; generic labels such as “Bar 1” do not identify evidence. This is the bound adapter's supported source form, not a promise about all upstream chart grammar.

```text
xychart-beta
  title "Requests per batch"
  x-axis ["First", "Second", "Third"]
  y-axis "Requests" 0 --> 8
  bar "Accepted" [3, 5, 7]
  line "Rejected" [1, 2, 1]
```

These illustrative counts describe three ordered batches, with one value per category in each named series. Keep the exact supplied values available in adjacent text or a table when readers need them. If the supported axes, legend or interpolation would misstate the supplied meaning, use a table rather than editing the generated SVG or guessing another renderer feature.

## Preserve source and name the targets

```html
<ex-mermaid id="architecture" label="Session lookup"><script type="text/plain">
flowchart LR
  Client[Client] --> Registry[Registry]
  Registry --> Session[Session]
</script></ex-mermaid>
<ex-card id="Registry" title="Registry ownership" tone="note" icon="database">
  <p>The Registry retains the lookup independently of the connection.</p>
</ex-card>
<p><a href="#architecture--Registry">Locate the Registry node</a></p>
```

`id` makes the whole island linkable; `label` names it for inspection. Put source in the plain-text script, not parsed child HTML. A Mermaid fence inside `ex-markdown` renders through the same component; its generated island ID follows fence order as `navigation.md` describes. Literal script delimiters need the safe source-carrier treatment from `content.md`/`trees.md` rather than a broken raw script.

A diagram node whose original ID matches a real page element takes its tone and opens that element. In this example, clicking the diagram's Registry node opens the Registry card. Islands open beside the diagram; ordinary linked elements scroll into view. This does not establish a runtime relationship beyond the supplied source. Keep global HTML element IDs unique and qualify diagram-node links when diagrams repeat node names.

Use an actual supported inline source carrier. `src` is intended for source preparation/packaging; the runtime does not automatically fetch a sibling Mermaid file for an unprepared direct-file page. A relative file name that the checker can read is not proof that the reader can load it.

Use the fullscreen (arrows-out) icon to enlarge a Mermaid drawing; Escape/Close returns the original diagram. Mermaid has no sidebar inspection button. Clicking the drawing does not open inspection; linked nodes keep their own actions. Use `zoom="none"` to suppress enlargement on a diagram.

## Readability is an observation

Keep a node's identity and short responsibility readable; place its full rules in adjacent prose, a table or a linked card. In a quoted flowchart label, `<br/>` creates a deliberate line break; the renderer also normalizes a literal `\n` sequence there. Put this source inside the existing inert carrier, not parsed child HTML:

```text
flowchart TD
  request["Request<br/>Receive submission"] --> validate["Validation<br/>Check supplied fields"]
  validate -->|valid| accepted["Accepted<br/>Record result"]
  validate -->|invalid| rejected["Rejected<br/>Explain correction"]
```

Good: the node retains its identity and responsibility on meaningful lines, while the document explains the complete validation rules. Bad: one paragraph-wide node label, or missing constraints after shortening it. This example establishes flowchart label syntax; other families follow their own supported grammar.

Shared text uses readable colors and a 16px rendered floor; a view that needs more room scrolls locally. Use Canvas for a rich view and the shared inspector for closer reading. Avoid fixed heights/hidden overflow that crop nodes, legends or labels.

The source checker catches supported syntax and selected render errors. Browser inspection is reserved for a requested visual review or a reported rendering defect, not every authored diagram.
