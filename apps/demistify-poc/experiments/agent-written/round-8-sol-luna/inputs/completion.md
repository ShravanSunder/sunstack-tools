# Completion procedure

Use the provided starter and the Demistify vocabulary. Include the complete brief: verdict, exact diff and findings with line references, reconnect flow, repository changes, all three fix options and evidence table. Include an ex-tree demonstrating repository changes. Its source must be a Markdown list, not HTML ul/li. Example:

<ex-tree title="Repository changes">
- src/
  - session/
    - added: registry.ts
    - changed: resume.ts
</ex-tree>

Do not invent tags or write presentation CSS, scripts or new library behavior. A small layout style is allowed. Do not add surrounding application navigation. Keep all content in one HTML document; library URL demistify/.

Save initial.html BEFORE first checker, then copy to final.html for repairs. From the stated monorepo root, run: node packages/demistify/bin/demistify.ts check ABSOLUTE_FINAL_HTML. Capture each checker output to check-1.txt etc and exact exit code (use e.g. checker_exit, never zsh status). At most 3 checker calls total; stop once it reports 0 errors and 0 warnings. Do not use --fix.

Return receipt.json containing filesRead (all complete paths), commands, each exit code, repair descriptions, and any unverified claims. Read only this input set; no source code, examples, other agents or network. Do not view browser; the Lead independently does the browser check. Your final page is an experimental result; report failures honestly rather than changing a checker/library.
