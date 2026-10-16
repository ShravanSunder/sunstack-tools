# Demistify authoring guide experiment

Status: implemented after prospective independent spec acceptance. Round 10 supplies fresh named-example browser/content evidence; actual reference retrieval remains unverified and independent implementation review returned targeted-revision with one open proof gate. The earlier explorer reference was an experimental draft when round 9 used it. Do not call this a reviewed or released guide.

## Target and promise

One skill: `packages/demistify/skill/SKILL.md` and its `references/explorers.md` depth. This is the application library's embedded authoring guide, not the Sunstack workflow plugin. The guide helps an HTML author express engineering content and layout using the supplied library, with source validity checked before human/browser review.

Baseline: committed core guide at `0e3f7a2`; new explorer component and draft reference are pending in the same branch. Proposed complete core and reference text are in ignored `tmp/authoring-guide-review/`. No trigger/frontmatter changes. The proposed main-path edits clarify section headings, make the summary example's four-column arithmetic explicit, and add one conditional explorer reference call. The reference teaches existing JSON reuse, authored introduction placement, stable IDs and the checker/browser/content proof boundary. This is behavior-changing, not mechanical or the one-home scoped class.

## Authoring basis and success

Owner-directed intent: reduce author context, support small models, validate cheaply, preserve full explorer interactions and keep the library separate from document usage. Observed supporting evidence: round 8 Luna produced no section h2 headings and an orphan summary stat; source-valid output did not establish good navigation/layout. The same feedback caused Sol to replace already-working Markdown headings with literal HTML; the phrase "real h2" was ambiguous. Round 9 first outputs placed the reading guide before the explorer title; the component and revised reference now support it as child content. These observations diagnose these specific runs; they do not establish general model reliability or a causal effect of one guide line.

Success: fresh Sol and Luna authors can discover the supplied explorer reference from the core guide, write a source-valid page using the unchanged graph document, put explanatory content after the page title, and avoid recreating library/data/application code. On the small code-review task, the guide should make useful section headings and a filled summary row straightforward. Proof records what actually happens; no claim of model superiority or statistical improvement follows from one pair.

## Decisions and boundaries

| Decision   | Default and reason                                                                                                                                                                                                                        |
| ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Invocation | Keep the existing description unchanged; this remains the same authoring task.                                                                                                                                                            |
| Main path  | Keep HTML/content composition and static checking; add only the observed layout/heading clarification and conditional graph-document branch.                                                                                              |
| Depth      | Explorer-only API details live in one conditional reference, keeping unrelated authors from loading them. The caller requests an authored page; the reference teaches how to compose and verify it.                                       |
| Proof      | User-directed intent permits authoring without claiming a causal RED. Keep the captured failures and prior outputs; run fresh measured authors after spec acceptance, then independently inspect the real browser and referenced content. |
| Security   | No shell hooks, credential use, network destination, model dispatch, external-content instruction or new permission is added by this guide. The supplied outer HTML remains trusted; the data parser validates graph content.             |

## Coordination and sequence

Branch `feat/demistify-distribution-authoring-experiments`, base `0e3f7a2`. Other pending product changes belong to the same authorized experiment but are outside this skill review. No release tag or version bump; this is unreleased 0.x lab source. Preserve old input snapshots verbatim. Before applying the proposed core changes, an independent review lead runs spec review through `skill-review`; after measured proof, it reviews the changed guide. Review records go in `tmp/authoring-guide-review/` and the outcome is summarized here.

Non-goals: rewrite the whole guide, review product implementation, add a general workflow framework, require more per-page tooling, prove domain truth with the static checker, or claim deployment/production readiness.

Run status: implemented. Independent spec review returned great / accepted-to-implement with four checks complete and no findings. The complete accepted drafts were copied into the two target paths. No trigger or unrelated reference changed. Fresh Sol/Luna assignments and evaluator checks were fixed before launch. Independent implementation review completed seven selected checks: source checks were clean, with PF-1 blocking ship advancement because actual guide/reference reads remain unverified. Run status stays implemented; this branch preserves an experimental draft, not a stable release.

## Current proof boundary

Round 10 preserves exact initial/final pages, captured assignment text, source-token ledgers and parent Chrome observations. Both authors composed valid final pages and used the intended heading, summary and introduction shapes. Luna repaired eight static reference errors and received a fixture path correction. Complete native tool-action transcripts were unavailable, so actual reference retrieval and the exact loaded guide copy cannot be inferred. Keep that acceptance item open; do not replace it with correct output or a self-report.
