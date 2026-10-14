# Maps, arrows and layers

| Element | Does |
|---|---|
| `<ex-board cols="2">` | a pannable, zoomable map of `<ex-region id title tone icon>` groups, one region per area; children marked `detail` appear when the reader opens a region; built-in Map/Read switch |
| `<ex-flow cols="3">` | an `ex-grid` whose `<ex-arrow from to label>` children route arrows between its boxes |

Hand-placed layout: an `ex-board` without regions pans and zooms your own layout. Put one `<div style="position: relative; width: 1100px; height: 640px">` inside it, place islands with `style="position: absolute; left: …; top: …; width: …"`, and add `<ex-arrow from to label>` anywhere; arrows route around every box. Leave at least 40px between boxes.

Layers: `layer="proposed"` on any element (cards, arrows, diagrams) lets readers switch that layer off and on. Switches appear at the top; place `<ex-layers off="proposed"></ex-layers>` yourself to start a layer hidden. Use it for current versus proposed, or optional detail.
