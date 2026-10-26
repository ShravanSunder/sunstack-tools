# demistify-explainer: authoring verification scope

Revision: 1. Target: `packages/demistify/skill`. One run, update; implemented.

## Problem and success

Owner Oct26: “remove the phone verification”; component-development checks belong in this repository's AGENTS.md, not repeated in Demistify/presentation authoring skills; “make new prs”; “pointless verifications burn tokens.” User-directed intent, not a reproduced model defect.

Ordinary authoring must not require a phone/narrow viewport, desktop/phone screenshot pair or light/dark regression matrix, including through references. Retain binding/source/markup, authored content and link checks, task-specific interactions and truthful proof limits. Retain responsive component behavior documentation. Component developers own responsive UI checks in repository instructions.

## Decisions

| Decision | Rationale |
| --- | --- |
| Remove viewport/theme matrix obligations from existing homes, no new authoring flag/helper | Less recurring work; one verification owner |
| Keep author-content checks and explicit task requirements | Removing library regressions must not excuse broken authored content |
| Keep responsive API descriptions | They explain what the library provides, without assigning a test matrix |
| No browser/product suite rerun for instruction-only deletion | It cannot establish instruction uptake; no rendering code changes |

## Surfaces and proof

Trigger unchanged. Main removes mandatory viewport/theme observations and calls returning them. Depth removes matching obligations from existing references. No new reference/schema/tool. Proof: inspect complete changed files and references for remaining mandates; validate Markdown/frontmatter/local links and packaging/version consistency with existing tools where available; independent source review. Strongest claim is source agreement with owner intent; fresh-agent application unobserved/deferred, no improvement/reliability claim.

## Coordination

Base origin/main `7403724327af73cd507cdd1bda494b1d4cb842e1`; branch already isolated, no prior skill edits. Separate new PR, ready/unmerged, no release/cache/merge. Product AGENTS.md gets developer-owned responsive verification; patch package Changeset records guide update.

Non-goals: runtime/CSS/test implementation, storage/identity/API changes, deleting responsive behavior documentation, broad verification-policy rewrite.

Run status: implemented. Review record: revision1 accepted-to-implement by independent skill-review SPEC stage; report retained in the owning lab. Implementation review follows focused proof and Root assessment.

## Revision 2 — additional owner-directed variation teaching

Owner Oct26 supplies an agent transcript that treated related screenshot sizes as a filterable list rather than variations, then asks to fix the skills “in a generalized way ... with an example for screen sizes.” This is user-directed teaching, not a faithful reproduced agent failure. Revision1 remains accepted/implemented for the phone-matrix removal; its decisions remain in force.

Success: teach the distinction between a logical item and alternate captures, variations versus filter facets versus separate items/steps, using only the bound component API. Include a worked screen-size example, meaningful capture-backed categories and missing/ambiguous capture behavior. A guided comparison uses one stop per logical screen/state with variations enabled; exact-capture evidence may intentionally omit it. No invented generic variant attribute, runtime change, automatic image inference, manufactured captures or author phone/theme regression matrix.

| Added decision | Rationale |
| --- | --- |
| Teach subject/variation/filter distinction before syntax | Prevent the flat-list interpretation reported by the owner |
| Screen size is a worked example, not a fixed enum | Apply the judgment to supplied evidence without inventing metadata |
| Preserve actual screen+state / viewport contract | General teaching must not claim arbitrary runtime axes |

A second run, same named skill. Main pictures call returns grouping/variation decisions. Depth updates existing pictures.md with one complete gallery/journey recipe and good/bad distinction; tours.md connects its optional variations flag to that grouping. Keep Gallery standalone-page constraint, stable original figure IDs, source-relative references, and truthful file/binding limits.

Proof for added teaching: manual source/API walk against current galleryScreenKey (screen+state) and viewport navigation; static check the literal product example against unchanged matching checker when available; current frontmatter/link/version checks and independent review. Fresh author transfer remains unobserved/deferred. No browser/product-suite or paid evaluation rerun for prose-only changes. Same base/newPR/version allocation as above; patch Changeset/changelog now describes both changes.

Run status: revision1 phone scope implemented/awaiting implementation review; revision2 variation teaching implemented after independent rev2 SPEC acceptance. Acceptance of revision1 does not cover the new teaching.
