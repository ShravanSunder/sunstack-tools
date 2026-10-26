# Inspect a view beside its explanation or fullscreen

Sidebar inspection keeps the main explanation available. Fullscreen gives a large view of the selected content, then restores it on close. Use these shared controls when inspecting a view helps; avoid adding a second viewer, duplicate content or custom zoom chrome.

## Defaults and explicit choice

Diagrams, code, file trees and content comparisons normally offer sidebar/expand controls. On components that install the shared inspector, such as Card, Callout, Tabs, Trace and Pre, bare `zoom` requests whole-island controls and `zoom="none"` removes them. Grid and Flow arrange their children and do not install whole-container inspection buttons, even when a `zoom` attribute parses; use the supported child island's controls. Plain prose and stats need no enlargement.

```html
<ex-card id="recovery-contract" title="Recovery contract" label="Recovery contract" tone="note" icon="database" zoom>
  <p>The Registry retains lookup independently of a connection.</p>
  <details><summary>Expiry boundary</summary><p>The supplied source does not select an expiry policy.</p></details>
</ex-card>
```

Use `label` for a useful inspector name when the visual alone has no clear title. `zoom="true"` is not the flag syntax; write bare `zoom` on a supported owner. Native `details` remain a local disclosure, not an inspector or a guided path. Whole Tabs inspection can serve a different question from its children's controls; Grid/Flow authors inspect supported children rather than adding a container affordance.

Clicking an ordinary image or Mermaid drawing opens fullscreen, with the original image/caption or diagram restored on close. Linked images and diagram nodes keep their link action; dragging to select text does not open a viewer. Explicit sidebar buttons still open beside the explanation. `zoom="none"` on the visual owner disables direct enlargement as well as its inspection buttons. Gallery screenshots use their own fullscreen action and Gallery arrow navigation; they do not open a screenshot inspection sidebar. `pictures.md` owns that branch. Markdown's source button opens the retained source, which is different from enlarging its rendered view.

## What the reader can do

Use the sidebar button to keep context beside the view, or arrows-out to enlarge it. Shared controls provide zoom out/in/reset; Cmd/Ctrl-wheel zooms and modifier-drag pans where the view supports it. Normal scrolling remains native. Escape/Close and applicable backdrop dismissal return to reading.

Sidebar content is an independent interactive copy; fullscreen uses the live content and returns it. A closing action must preserve the main island and restore its state, including selected tabs, folded details, typed fixture state, filters or zoom where supported. This is behavior to verify on the authored composition, not an excuse to add editable test inputs to an ordinary explanation.

The inspector preserves readable typography and local overflow. A diagram too wide to read should scroll rather than shrink labels below the shared floor. Long prose stays readable rather than inheriting a larger whole-page heading scale. Use the shared surface, corner radius and elevation; add no per-document shadow or modal implementation.

## Check the return, not just the opening

Open the exact island, make a meaningful supported state change, expand it, close it and compare the original content/state. Confirm the main view remained available during sidebar inspection; check labels and controls in a narrow window. Exercise a nested disclosure/code view separately from the containing island. Lost live nodes, blank main content, controls hidden by the header or an unrelated keyboard owner losing its keys are failures.

When access prevents these observations, keep the authored markup and exact limitation separate from an interaction-success claim. A source check cannot establish that a dialog restored the reader's state.
