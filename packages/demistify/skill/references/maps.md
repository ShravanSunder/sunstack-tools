# Maps, arrows and layers

Use a Flow for dependencies between responsive boxes, a Board for ownership regions or a spatial map, and Layers for independently optional content. A Mermaid sequence/state view belongs in `diagrams.md`; do not force that question into hand-positioned cards.

## A responsive Flow

```html
<ex-flow id="handoff" cols="2" data-width="canvas">
  <ex-card id="automation" title="Automation store" tone="note" icon="database"><p>Owns schedules, wakes and an owed message.</p></ex-card>
  <ex-card id="interaction" title="Interaction store" tone="note" icon="database"><p>Owns approvals, questions and message custody.</p></ex-card>
  <ex-arrow from="automation" to="interaction" label="handoff · receipted call"></ex-arrow>
</ex-flow>
```

Direct boxes have stable unique IDs; direct `ex-arrow` children name those IDs with `from` and `to`. `cols` accepts 1–6; direct box children may use grid spans and sticky behavior from `layouts.md`. Keep the boxes' DOM order readable when the layout collapses. An arrow does not infer its endpoints from a displayed title.

Flow routes through the gaps between its boxes. In 1.10.0 it measures complete captions, wraps them into multiple lines and reserves minimum row/column room. Facing ports align where the cards permit it. Do not squeeze this room away with local fixed-gap overrides or move caption text into unrelated page positions. Shorten an overlong explanation into a useful edge caption and keep its full meaning beside the view.

If no collision-free caption position exists, this release can omit that caption and report an authoring issue. Open `?ex-check` and inspect the root issue count/console; add room or simplify. A promised fallback caption placement is still an open product requirement, not an implemented API. Check every visible arrow and complete caption after fonts load and after a narrow-layout change; valid endpoint syntax alone cannot establish readable routing.

## Ownership regions and spatial maps

| Element | Does |
|---|---|
| `<ex-board cols="2">` | `<ex-region id title tone icon>` groups, one region per area, read like the page; children marked `detail` appear when the reader opens a region; readers can switch to a pannable map |
| `<ex-flow cols="3">` | an `ex-grid` whose `<ex-arrow from to label>` children route arrows between its boxes |

A board starts in the reading view. Add `mode="map"` only when the overview is the point: many regions with arrows crossing between them, where seeing the whole shape matters more than reading order. The reader can still switch views.

Hand-placed layout: an `ex-board` without regions keeps your own layout, with arrows routed around every box; add `mode="map"` to pan and zoom it, which a layout wider than the page needs on phones. Put one `<div style="position: relative; width: 1100px; height: 640px">` inside it, place islands with `style="position: absolute; left: …; top: …; width: …"`, and add `<ex-arrow from to label>` anywhere; arrows route around every box. Leave at least 40px between boxes.

```html
<ex-board cols="2" data-width="canvas">
  <ex-region id="host" title="Host" tone="note" icon="cpu">
    <p>Owns machine-local execution.</p>
    <ex-card title="Execution detail" tone="note" icon="terminal" detail><p>The native backend remains on this Host.</p></ex-card>
  </ex-region>
  <ex-region id="gateway" title="Gateway" tone="note" icon="globe"><p>Routes addressed client requests.</p></ex-region>
  <ex-arrow from="gateway" to="host" label="addressed request"></ex-arrow>
</ex-board>
```

Regions belong inside the Board. Their `title`, `tone`, `icon` and `label` name the group; children marked `detail` are optional map detail rather than a separate page or path. The reading view preserves document flow; the map view exposes spatial overview and pan/zoom. A hand-placed canvas must leave real channels around boxes, not hide overflow to make it fit. Free Board arrows additionally accept `from-anchor`/`to-anchor` as `left`, `right`, `top` or `bottom`; Flow chooses its own ports. Free-map captions do not inherit Flow's measured multiline layout, so check their actual space separately.

## Independently optional layers

Layers: `layer="proposed"` on any element (cards, arrows, diagrams) lets readers switch that layer off and on. Switches appear at the top; place `<ex-layers off="proposed"></ex-layers>` yourself to start a layer hidden. Use it for current versus proposed, or optional detail.

```html
<ex-layers off="proposed" label="Show"></ex-layers>
<ex-card title="Current lookup" tone="note" icon="database"><p>The supplied current owner retains lookup.</p></ex-card>
<ex-card id="proposed-owner" title="Proposed owner" tone="planned" icon="database" layer="proposed"><p>This is an alternative, not an adopted change.</p></ex-card>
```

Layer names connect related elements and their switches; `off` is a space-separated list. Mark corresponding arrows with the same layer as their optional boxes. Use tabs/Compare for mutually exclusive alternatives; layers may be visible independently. Fragments and guided stops reveal a hidden target's containing layer.

Finish by checking endpoint identity, complete captions, map/read switching, narrow containment and every layer's intended content. Source-derived dependencies remain different from observed calls. An arrow passing a syntax check is not evidence that a real request crossed that boundary.
