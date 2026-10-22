# Trace steps and their evidence

A Trace is an ordered explanation with expandable attachments. Tree owns hierarchy, status and links; keep it when the hierarchy is the point. Use `ex-trace` when readers should follow a sequence and inspect the evidence behind each step.

```html
<ex-trace title="Inspect the evidence" data-width="canvas">
  <details icon="code" tone="done" tag="Supplied source">
    <summary>Read the reconnect change</summary>
    <ex-code title="Expiry guard" lang="typescript"><script type="text/plain">if (session.expired) return { kind: 'expired' };</script></ex-code>
    <p>The guard is supplied. Its source does not prove a recovery execution.</p>
  </details>
  <details icon="test-tube" tone="planned" tag="Not executed">
    <summary>Observe the original identity after reconnect</summary>
    <ex-markdown><script type="text/markdown">
Compare the durable identity **before and after** the interruption.
    </script></ex-markdown>
  </details>
</ex-trace>
```

Each direct child is a native `details` with a native `summary`. Add `open` when its attachment should start expanded. Optional step `icon`, `tone` and `tag` use the shared vocabulary. A summary may retain inline code or a link. The body accepts ordinary HTML and rich islands such as cards, code/diffs, Markdown, diagrams, Tree and a nested Trace. Use an ordinary figure for one picture and a native link to a dedicated Gallery page for screenshot browsing. There is no separate attachment registry.

Name the state precisely: **Supplied source**, **Observed**, or **Not executed** describe different evidence. A completed source-reading step is not a claim that the whole feature passed verification. Use short status labels and put the explanation in the body.

Trace headers own only their direct row controls. Embedded code-file disclosures retain their own styling and behavior. Give useful attachment destinations stable IDs; ordinary fragments and walkthroughs reveal containing native details. Use `data-width="reading"` for a short text sequence or `data-width="canvas"` for rich evidence. A bare `zoom` enables whole-Trace inspection; attachments retain their own inspection controls.

### Compose several attachments

Put peer attachments directly after the summary in reading order. A short explanation introduces the evidence; a callout carries a consequential limit; a small table lets readers compare the same facts. Place long code after the table and link to it from a cell. The shared flow separates those blocks; a nested grid keeps its own layout.

```html
<ex-trace title="Review duplicate-event handling" data-width="canvas">
  <details open icon="code" tag="Source supplied">
    <summary>Inspect the duplicate-event guard</summary>
    <p>The supplied guard ignores an event identifier it has already seen.</p>
    <ex-callout title="Execution not observed" tone="risk">
      <p>This source example does not establish retry behavior.</p>
    </ex-callout>
    <div class="ex-table-scroll">
      <table>
        <thead><tr><th>Subject</th><th>Evidence</th><th>Detail</th></tr></thead>
        <tbody><tr>
          <td><strong>Duplicate-event guard</strong><br><small>Request admission</small></td>
          <td><span class="chip">Source supplied</span></td>
          <td><a href="#guard-source">Read the guard</a></td>
        </tr></tbody>
      </table>
    </div>
    <ex-code id="guard-source" title="Duplicate-event guard" lang="typescript"><script type="text/plain">if (seenEventIds.has(event.id)) return;</script></ex-code>
  </details>
</ex-trace>
```

The cells combine ordinary HTML, the existing compact `.chip` label and a native link. Each attachment remains independently readable and retains its own controls. Use the same sibling composition inside a tab panel; the tab selects the view and its content provides the explanation.

For source-derived call changes, compose this with [comparison](comparisons.md) and follow [call paths](call-paths.md). A Trace does not capture a runtime stack.
