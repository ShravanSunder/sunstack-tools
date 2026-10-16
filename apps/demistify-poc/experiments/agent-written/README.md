# Agent-written explainers

The early rounds were written by fresh Haiku-class subagents that read one guide and one fact brief from `apps/demistify-poc/briefs/`, with identical prompts. Pages are kept verbatim (excluded from formatting). After `pnpm build`, open `/experiments/agent-written/<round>/<page>.html` on the lab preview.

| Round | Guide | Pages |
| --- | --- | --- |
| `round-3` | `skill/SKILL.md` at `1911a3f` (1,459 tokens; vertical example) | planning, page-loading, code-review |
| `round-4-control` | same guide as round 3 | domain-map, component-deep-dive |
| `round-4-treatment` | `skill/SKILL.md` at `8b9e987` (1,405 tokens; Markdown first, layout patterns, stats, routes, board) | all five briefs |

Round 4 results (desktop 1400 px; "mixed" = share of islands beside a different kind of island):

| Brief | Tokens control → treatment | Mixed rows | Desktop height |
| --- | --- | --- | --- |
| planning | 4,434 → 4,035 | 0% → 29% | 3,336 → 2,512 px |
| page-loading | 2,353 → 2,312 | 0% → 60% | 4,805 → 4,465 px |
| code-review | 1,669 → 2,058 (adds stats, chart, routes) | 0% → 50% | 3,608 → 3,513 px |
| domain-map | 2,704 → 2,304 | 0% → 78% | 4,715 → 2,607 px |
| component-deep-dive | 2,338 → 2,328 | 0% → 29% | 5,645 → 5,620 px |

Every treatment page used routes and stat tiles; two used the board; none needed a Markdown wrapper (control: 7–14 per page). One treatment page had one authoring issue (a route step to a missing id), reported by the issues panel. No console errors in any page.

Later measured rounds: [Luna baseline/starter](round-7-luna/README.md), [matched Sol/Luna code review](round-8-sol-luna/README.md), and [full explorer reproduction](round-9-explorer/README.md). Each keeps its own fixed inputs and proof limits.

[Round 10](round-10-guide/README.md) records a fresh Sol/Luna pair after guide clarification, including the initial eight-error Luna review, its repair and the fixture assistance.
