---
'@sunstack/demistify': patch
---

Remove `demistify initiative migrate` from the CLI. The one-time conversion and re-cut of `project.ts` trees runs from a source checkout as `node packages/demistify/scripts/initiative-migration/run.ts`, documented beside the script, and is not part of the published CLI or guide.
