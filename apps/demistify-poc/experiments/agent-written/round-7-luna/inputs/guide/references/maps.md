# Maps, arrows and layers

| Element | Does |
|---|---|
| `<ex-board cols="2">` | `<ex-region id title tone icon>` groups, one region per area, read like the page; children marked `detail` appear when the reader opens a region; readers can switch to a pannable map |
| `<ex-flow cols="3">` | an `ex-grid` whose `<ex-arrow from to label>` children route arrows between its boxes |

A board starts in the reading view. Add `mode="map"` only when the overview is the point: many regions with arrows crossing between them, where seeing the whole shape matters more than reading order. The reader can still switch views.

Hand-placed layout: an `ex-board` without regions keeps your own layout, with arrows routed around every box; add `mode="map"` to pan and zoom it, which a layout wider than the page needs on phones. Put one `<div style="position: relative; width: 1100px; height: 640px">` inside it, place islands with `style="position: absolute; left: …; top: …; width: …"`, and add `<ex-arrow from to label>` anywhere; arrows route around every box. Leave at least 40px between boxes.

Layers: `layer="proposed"` on any element (cards, arrows, diagrams) lets readers switch that layer off and on. Switches appear at the top; place `<ex-layers off="proposed"></ex-layers>` yourself to start a layer hidden. Use it for current versus proposed, or optional detail.
