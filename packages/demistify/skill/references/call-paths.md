# Source-derived call-path changes

Use aligned `ex-compare` topic rows with rich `ex-trace` steps to explain a changed call path. The comparison supplies corresponding topics and the Trace supplies expandable source/context. A separate stack-capture component is not required for source explanation.

Keep three facts distinct:

- **Entry or call:** an actual function boundary or invocation, with a source/line tag.
- **Branch or outcome:** a check, return or refusal that occurs inside a frame; it is not another call frame.
- **Unknown runtime edge:** a caller, ordering, timing or identity observation not established by the supplied source.

```html
<ex-compare label="Disconnect calls" data-width="canvas">
  <section label="Current">
    <section data-compare-row="disconnect">
      <ex-trace><details icon="code" tone="removed" tag="transport.ts:13">
        <summary><code>registry.delete(transport.sessionId)</code></summary>
        <p>The supplied old call removes the lookup entry.</p>
      </details></ex-trace>
    </section>
  </section>
  <section label="Proposed">
    <section data-compare-row="disconnect">
      <ex-trace><details icon="code" tone="added" tag="transport.ts:13">
        <summary><code>transport.close()</code></summary>
        <p>The supplied replacement closes this connection.</p>
      </details></ex-trace>
    </section>
  </section>
</ex-compare>
```

Put observable consequences and refusal checks in their own corresponding topic row. Retain exact supplied code in one code/diff island and link to it from the explanation. Name the source basis and missing runtime observations next to the view. Do not invent callers, translate every branch into a stack frame, or imply captured execution or automatic frame matching. When actual runtime evidence exists, identify that evidence separately from the source-derived view.
