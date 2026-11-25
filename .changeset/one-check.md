---
'@sunstack/demistify': minor
---

`demistify check` now takes typed pages and folders as well as standalone pages: a page that binds generated design data, or a folder with `design.ts` or `initiative.ts`, runs that tree's typed check once (`--site-root` and `--write` pass through), and every other page gets the page check.
