# Demistify API skill trigger

Revision S1. Owner: the Demistify package. One update run, one skill target: `packages/demistify/skill/SKILL.md` (`demistify-explainer`).

## Intent and success

The owner approved separating the presentation authoring workflow in Sunstack `presentation-html` from the library API guide. The current description offers generic engineering explanations and includes a rendering feature list. The approved replacement offers the guide when writing, editing or checking Demistify markup. Success is a canonical, narrower API description, preserved component guidance and relative references, and the same description served from the public `/v1/authoring/SKILL.md` after the normal patch release.

```yaml
description: 'Use when writing, editing, or checking Demistify markup: component syntax, attributes, nesting, layouts, Markdown/Mermaid islands, and data references.'
```

## Decisions

| Decision                                              | Reason                                                                                                                        |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Change only the description in the canonical guide    | The library owns its API; the adjacent Sunstack skill owns the authoring workflow.                                            |
| Preserve the guide body, name and relative references | This run changes the loading condition, not the component API or authoring instructions.                                      |
| Preserve current invocation encoding                  | Plugin manifests, Codex implicit invocation and the Sunstack dependency are coordinated separately by the installation agent. |
| Declare a package patch                               | The guide is a public release asset; publish through the existing version PR and protected deployment.                        |

## Surfaces and proof

Classification: behavior-changing, user-directed intent, not the scoped exception because the trigger changes. Only the trigger surface changes; main path and depth remain byte-for-byte identical. The lens remains author-supplied meaning and HTML composition with library-supplied rendering. No new call sites, schemas, security surface or dependencies.

Source basis: the complete current guide at base `6693b1f23b4585ea09eee122ba8784620357aadc`; owner approval of the exact description in this conversation; the existing release-intent guard requires a versioned Changeset for public guide edits; this proposal selects a patch.

Proof: validate YAML/frontmatter and exact replacement, compare unchanged body and references, run repository quality and release-intent gates, independently review the spec and implementation, and verify exact guide bytes through public HTTP after deployment. Do not claim improved model routing from a static wording check. Fresh implicit host-routing behavior is not evaluated in this run; marketplace packaging and invocation flags remain outside it. The changed asset is usable by direct guide loading regardless of that unmeasured trigger behavior.

## Coordination and scope

Branch `docs/demistify-api-trigger`, isolated worktree, base `6693b1f23b4585ea09eee122ba8784620357aadc`, synchronized to main `e6da71ec47a1509acb3f6fcce5c37bc22dddb8f9` after the separate CI PR merged. The guide baseline is unchanged. No pending edits to the guide at proposal time. A separate CI performance change must not enter this PR. The installation agent has been sent the exact approved wording and asked for any in-flight rehome collision. If that agent changes the canonical path first, reconcile at its owning boundary before editing rather than copying the guide.

Non-goals: plugin rehome, host manifests, invocation flags, dependency declarations, library API/body changes and a claim of measured routing improvement.

Run status: implemented. S1 spec review accepted to implement with all four checks complete and no required revisions. Exact description and patch declaration implemented; source preservation, quality checks, independent implementation review and public release proof follow. Implicit routing remains unmeasured.
