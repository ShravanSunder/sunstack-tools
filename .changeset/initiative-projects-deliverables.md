---
'@sunstack/demistify': minor
---

Organize typed design as an initiative of numbered projects. `initiative.ts` replaces `project.ts`: each project names its design, a number (the same number runs side by side; 2 comes after 1), what it achieves and what the owner checks. Deliverables may depend on deliverables in any project, and the check warns when one depends on a higher-numbered project. `demistify initiative check` replaces `project check`; `demistify initiative status` prints the plan, and the generated view lists what is ready now, what blocks the most and what is built but not accepted. The initiative overview opens with the projects by number, and every state, level and id prefix carries a plain explanation.

This experimental v1 update renames the levels. Rename `project.ts` to `initiative.ts` with `projects: { name: { design, number } }`, `project-decisions.ts` and `project-assumptions.ts` to `initiative-decisions.ts` and `initiative-assumptions.ts`, `project/` references to `initiative/`, and `ex-project-overview` with `project.generated.json` to `ex-initiative-overview` with `initiative.generated.json`. Regenerate the design SDK and views with the matching CLI.
