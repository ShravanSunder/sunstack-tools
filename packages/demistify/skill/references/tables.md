# Tables use the shared surface

Markdown tables receive their scroll wrapper automatically. An authored HTML table uses that wrapper explicitly:

```html
<div class="ex-table-scroll" data-width="canvas">
  <table>
    <thead><tr><th>Owner</th><th>Responsibility</th></tr></thead>
    <tbody>
      <tr><td>Registry</td><td>Retain the session</td></tr>
      <tr><td>Connection</td><td>Carry messages</td></tr>
    </tbody>
  </table>
</div>
```

Both native tables and wrappers use a neutral frame with shared 8px corners, a gray header and quiet stripes. Use the existing wrapper when wide columns need horizontal scrolling; a narrow native table needs no wrapper for its corners. Native corners are CSS-only, preserving saved comment target ancestry. Preserve a real `thead` or a first header-only row. Do not add status colors or per-page border/radius overrides to reproduce the shared treatment. `layouts.md` owns Reading/Canvas and horizontal containment.
