# Code and diffs

`<ex-code title lang start>` holds code or a unified diff in `<script type="text/plain">`. `lang="diff"` colours `+`/`-` lines.

Every row has one number, removed lines included; the number the reader sees is the number a note names. Rows count from `start`, or from the `+` number of an `@@ -a +b @@` header, which restarts the count.

Several files: keep `diff --git a/x b/x`, `--- a/x`, `+++ b/x` headers; each file gets its own section and counts. A note then names its file: `<ex-note file="store.ts" line="13" tone="added">…</ex-note>`.
