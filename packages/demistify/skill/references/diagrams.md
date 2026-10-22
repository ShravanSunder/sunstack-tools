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
