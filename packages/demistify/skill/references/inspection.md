# Inspect a view beside its explanation or fullscreen

Sidebar inspection keeps the main explanation available. Fullscreen gives a large view of the selected content, then restores it on close. Use these shared controls when inspecting a view helps; avoid adding a second viewer, duplicate content or custom zoom chrome.

## Defaults and explicit choice

Mermaid diagrams offer only the fullscreen (arrows-out) icon. Code, file trees and content comparisons normally offer sidebar/expand controls on desktop. Shared enlargement controls float along the top edge without reserving heading width; scrollable diagrams keep controls inside their viewport. At 760px and below, the sidebar icon is hidden and Expand opens an edge-to-edge live-content modal. Independent comments remain available. Programmatic sidebar links and the More action retain their existing meaning. On components that install the shared inspector, such as Card, Callout, Tabs, Trace and Pre, bare `zoom` requests whole-island controls and `zoom="none"` removes them. Grid and Flow arrange their children and do not install whole-container inspection buttons, even when a `zoom` attribute parses; use the supported child island's controls. Plain prose and stats need no enlargement.

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

## Bottom workspaces

Use `ex-drawer` for a large bottom workspace that stays interactive beside Comments, such as related documents a reader keeps open while reading. It is in the source library and not yet in the hosted v1 runtime or guide, so use it only at a binding whose guide documents it, such as a named local source preview.

```html
<ex-drawer id="related-documents" label="Related documents" open>
  <header class="ex-inspect-header"><strong>Related documents</strong></header>
  <div class="ex-inspect-body">
    <p>The recovery runbook and the session contract stay beside the explanation.</p>
  </div>
</ex-drawer>
```

`label` is required and names the workspace. Put a header and a body with classes `ex-inspect-header` and `ex-inspect-body` inside it. The drawer owns its geometry, entry animation, Escape and focus return; the document owns the content. Keep the drawer in the original document DOM so comment targets and text selections keep their identity. Outside presses keep it open, and reduced-motion settings disable its animation.

Its lifecycle:

- A static `open` attribute shows it when the page loads; without `open` it starts hidden.
- Escape closes the latest workspace unless a control inside it used the key first. Closing returns focus to where it came from and fires a `close` event.
- Nothing in plain document markup opens it later or reopens it after a close. Triggering is application-embedding API: a page controller calls `drawer.show()` and `drawer.close()`, or toggles `open`, and listens for `close` to restore its own view state. An ordinary declarative document adds no such script; when a reader must reach the content again after closing it, keep that content in the document flow or a native disclosure as well, or name the controller the page embeds.
- `size="content"` opens a detail sheet above its parent workspace, while a large workspace closes the others. Opening one from a modal enlargement first returns that content to its document.

Complete when the drawer has its `label`, header and body, its initial state is the one the task needs, and a reader who closes it can still reach its content or the page names the controller that reopens it.

The library owns inspection and state-restoration regression tests. Routine authors use the documented controls and the main guide's lint pass. Inspect an interaction only when requested or when diagnosing a reported defect.
