---
'@sunstack/demistify': minor
---

`initiative.ts` takes `pages`: initiative-level HTML pages, relative to it, that may place `initiative/` records and any project's records by qualified ID. `initiative check` warns about an HTML page under the initiative that binds records while no `design.ts` or `initiative.ts` registers it, and `open` finds the initiative as the owner of a registered page.
