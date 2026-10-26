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

## Readability is an observation

Keep labels short and put explanation beside the diagram. Shared text uses readable colors and a 16px rendered floor; a view that needs more room scrolls locally. Use Canvas for a rich view and the shared inspector for closer reading. Avoid fixed heights/hidden overflow that crop nodes, legends or labels.

For a chart, identify units, categories and series explicitly in nearby content. Do not rely on generic generated series labels to carry their scientific meaning. A chart whose legend or interpolation misstates the supplied data needs a different supported representation, such as a table; a successful SVG render does not settle that judgment.

Finish by inspecting actual SVG, every endpoint and label, one scoped cold link and any linked card interaction. Verify both ends of a wide view remain reachable and that a narrow layout does not shrink or crop meaning. The static checker can catch selected render/syntax problems and empty subgraphs; it does not prove visual semantics, legibility, runtime execution or complete node membership from source-word target checks.
