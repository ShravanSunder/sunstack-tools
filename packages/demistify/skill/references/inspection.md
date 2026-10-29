# Inspect a view beside its explanation or fullscreen

Sidebar inspection keeps the main explanation available. Fullscreen gives a large view of the selected content, then restores it on close. Use these shared controls when inspecting a view helps; avoid adding a second viewer, duplicate content or custom zoom chrome.

## Defaults and explicit choice

Mermaid diagrams offer only the fullscreen (arrows-out) icon. Code, file trees and content comparisons normally offer sidebar/expand controls. On components that install the shared inspector, such as Card, Callout, Tabs, Trace and Pre, bare `zoom` requests whole-island controls and `zoom="none"` removes them. Grid and Flow arrange their children and do not install whole-container inspection buttons, even when a `zoom` attribute parses; use the supported child island's controls. Plain prose and stats need no enlargement.

```html
<ex-card id="recovery-contract" title="Recovery contract" label="Recovery contract" tone="note" icon="database" zoom>
  <p>The Registry retains lookup independently of a connection.</p>
  <details><summary>Expiry boundary</summary><p>The supplied source does not select an expiry policy.</p></details>
</ex-card>
```

Use `label` for a useful inspector name when the visual alone has no clear title. `zoom="true"` is not the flag syntax; write bare `zoom` on a supported owner. Native `details` remain a local disclosure, not an inspector or a guided path. Whole Tabs inspection can serve a different question from its children's controls; Grid/Flow authors inspect supported children rather than adding a container affordance.

Clicking an ordinary image opens fullscreen with its caption. Mermaid opens fullscreen only through its arrows-out icon; clicking the drawing does not open inspection. Closing fullscreen restores the original content. Linked images and diagram nodes keep their link action; dragging to select text does not open a viewer. Explicit sidebar buttons still open beside the explanation. `zoom="none"` on the visual owner disables direct enlargement as well as its inspection buttons. Gallery screenshots use their own fullscreen action and Gallery arrow navigation; they do not open a screenshot inspection sidebar. `pictures.md` owns that branch. Markdown's source button opens the retained source, which is different from enlarging its rendered view.

## What the reader can do

Use the sidebar button to keep context beside the view, or arrows-out to enlarge it. Shared controls provide zoom out/in/reset; Cmd/Ctrl-wheel zooms and modifier-drag pans where the view supports it. Normal scrolling remains native. Escape/Close and applicable backdrop dismissal return to reading.

Sidebar content is an independent interactive copy; fullscreen uses the live content and returns it. A closing action must preserve the main island and restore its state, including selected tabs, folded details, typed fixture state, filters or zoom where supported. Keep ordinary explanations free of editable regression-test inputs.

The inspector preserves readable typography and local overflow. A diagram too wide to read should scroll rather than shrink labels below the shared floor. Long prose stays readable rather than inheriting a larger whole-page heading scale. Use the shared surface, corner radius and elevation; add no per-document shadow or modal implementation.

The library owns inspection and state-restoration regression tests. Routine authors use the documented controls and the main guide's lint pass. Inspect an interaction only when requested or when diagnosing a reported defect.
