# Optional detail stays local

A disclosure is a concise summary followed by useful optional depth:

```html
<details>
  <summary>Recovery evidence and its limits</summary>
  <p>The supplied trace covers lookup recovery. It does not observe a provider reconnect.</p>
  <ex-code title="Lookup guard" lang="typescript"><script type="text/plain">if (!session) return { kind: 'missing' };</script></ex-code>
</details>
```

It can contain ordinary HTML, Markdown islands, code, diagrams, lists, tables and other useful views. Use an explicit Markdown island inside a plain disclosure if writing Markdown. For tables, apply [Tables](tables.md). Keep its opening compact; an immediate document-sized heading and large section gap make optional explanation look like an unrelated page. A component names itself once, in its own title; the summary, heading or lead-in around it names the larger question it answers instead of repeating that title, and the document headings navigation needs stay. `open` starts it expanded. A disclosure does not create guided stops or download behavior. Gallery browsing belongs on a separate linked page; `traces.md` adds an ordered evidence sequence when that ordering matters.

Preserve complete supplied text/data and source-relative paths. Use the main guide's lint pass; opening tables, links and disclosures is not a separate authoring gate.
