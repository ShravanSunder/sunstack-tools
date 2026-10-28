# Code and diffs

Use `ex-code` for readable source or a unified diff with displayed line numbers and optional notes. A fenced block inside Markdown is enough for a short ordinary excerpt; choose the island when file sections, notes or inspection help the reader.

```html
<ex-code id="lookup" title="Resolve the session" lang="typescript" start="10"><script type="text/plain">
const session = registry.find(sessionId);
if (!session) return { kind: 'missing' };
</script>
  <ex-note line="11" tone="risk">A missing lookup is not proof that the execution ended.</ex-note>
</ex-code>
```

`title` names the source, `lang` selects its presentation and `start` defaults to 1. `label` supplies an inspection name. `lang="diff"` colors added/removed rows. The source belongs in a direct `text/plain` script; writing literal code as parsed child HTML can destroy `<`, `&` or markup-like text. For a literal closing-script delimiter use the decoded template carrier described in `trees.md`, keeping its formatter-ignore comment. Do not rewrite supplied code to make its carrier work.

Common indentation and surrounding blank space are normalized for display. The displayed excerpt is not a promise of byte-for-byte original-file preservation; retain the original file when exact leading/trailing bytes matter. Inspect complete source as well as colored output.

Every row has one number, removed lines included; the number the reader sees is the number a note names. Rows count from `start`, or from the `+` number of an `@@ -a +b @@` header, which restarts the count.

Several files: keep `diff --git a/x b/x`, `--- a/x`, `+++ b/x` headers; each file gets its own section and counts. A note then names its file: `<ex-note file="store.ts" line="13" tone="added">…</ex-note>`.

```html
<ex-code title="Disconnect change" lang="diff"><script type="text/plain">
diff --git a/store.ts b/store.ts
--- a/store.ts
+++ b/store.ts
@@ -10 +10 @@
-registry.delete(sessionId);
+transport.close();
diff --git a/log.ts b/log.ts
--- a/log.ts
+++ b/log.ts
@@ -4 +4 @@
-log('deleted');
+log('detached');
</script>
  <ex-note file="store.ts" line="11" tone="added">Close this connection, retaining Registry ownership.</ex-note>
  <ex-note file="log.ts" line="5" tone="changed">The source now describes detachment.</ex-note>
</ex-code>
```

In that example the removed store row is 10 and added row is 11; these are displayed-row addresses, not Git's new-file line numbering. Hunk headers have no note number. In multi-file output, use the full file path or an unambiguous path suffix. A note is a direct child of its code island and accepts `line`, optional `file` and `tone`; it does not label an unrelated page line.

Code uses Canvas by default; apply `data-width="reading"` to the whole island for a short text-sized excerpt. Its file disclosures and notes keep their own controls inside a comparison, Trace or inspector. Use the shared sidebar/fullscreen controls instead of embedding another editor. Do not give a containing view a fixed height merely to match its neighbor.

Match note file/row references to the supplied code. Use the main guide's lint pass; a colored diff does not establish compilation or execution.
