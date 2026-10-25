# demistify-explainer default-comments teaching — revision 1

Target: only `packages/demistify/skill/` in sunstack-tools. Owner is the Demistify API guide, not presentation-html. Classification: update, behavior-changing, user-directed intent. Run status: implemented.

## Promise and success

An author using the matching new runtime creates an ordinary ex-document with no feedback boilerplate; readers get Comments and semantic content targets automatically. The author can deliberately opt out or override logical identity/revision after learning their consequences. Existing choice selection/details and single feedback handoff remain native.

## Basis and evidence

Owner request: October 25 Sun relay router://0ff962c5-7fa3-4c18-a5ca-1bbe8db09e89/push/01a1745c-165a-7f01-a629-82a359f59994. Current guide at d604345 requires explicit ex-comments, document-id/revision and labeled targets (`references/choices-and-comments.md`, section Bind one review to explicit identities); this is current opt-in teaching, not a reproduced model failure. Governing product design: docs/specs/2026-10-25-default-comments/*.md. Author from that approved requested outcome, no historical RED or causal-improvement claim.

## Decisions

| Default | Rationale |
| --- | --- |
| Keep trigger and invocation unchanged | Same markup-authoring situation; only the API defaults change |
| Main path states Comments is automatic and requires no boilerplate | Every ordinary page author needs the new default, without loading a long feedback reference |
| Detailed identity/source/opt-out semantics stay in existing choices-and-comments reference | That reference already owns review authoring; no new parallel home |
| Keep explicit question/option IDs | They identify responses; automatic feedback targets do not replace choice identity |
| Explain URL-move and content-edit namespaces | Prevent claiming old comments vanished or relocated; no storage migration |
| Describe matching-build behavior only | Source preview does not establish released/installed availability |

## Surface allocation and call sites

Trigger: unchanged YAML. Main path: replace opt-in-only sentences with automatic Comments plus explicit choice-only opt-out choice; feedback details still use the existing conditional reference table. Depth: replace explicit-review-only section with defaults then optional overrides, source-fingerprint boundary, generated semantic targets, preserving store/export/keyboard details. Example: ordinary document without ex-comments, document-id/revision or feedback labels; keep IDs only when links/choices need them. Include separate minimal override and comments="off" snippets. No new reference or call site needed.

## Proof and claim boundary

Reference-skill proof: check the real example against the matching source checker/build; inspect one ordinary page with automatic target controls and saved/reloaded feedback; a bounded fresh author application should use defaults and discover opt-out/overrides correctly. Checks written before any author run; inspect loaded source and actual actions if the host supplies them. If fresh-run action records are unavailable, return that exact gap and claim only source review/validated examples, not transfer or stochastic improvement. Product runtime tests independently prove runtime behavior. No old-run evidence is relabeled as new.

## Coordination

Base: feat/demistify-default-comments from origin/main d60434596a267b8ac40066d99947a088b2b0296c. Pending: four design docs; no skill edits. Land with runtime in one product PR and a minor Changeset, without version consumption, merge or release. Source-only independent spec review precedes skill writes; implementation review follows examples/proof assessment. No installed-cache writes, no new schema or helper scripts, no workflow-plugin changes in this run.

Review record: revision1 accepted-to-implement by independent skill SPEC review (tmp/2026-10-25-default-comments-review/demistify-spec-review.md); implementation changed only main/default and existing feedback reference inside that boundary. Matching CLI checked eight examples/fixtures with zero errors/warnings. Current rendered/application proof and independent implementation review remain owed; no released/installed claim. Non-goals: product redesign, response persistence, protocol migration, rewriting unrelated guide sections.
