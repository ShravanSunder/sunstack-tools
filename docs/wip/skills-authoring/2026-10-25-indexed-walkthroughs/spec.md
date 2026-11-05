# Demistify guide: shared collection walkthroughs

Revision1; one named skill update: demistify-explainer, owned by packages/demistify/skill. Run status: implementation in progress after separate teaching and structure spec reviews; Root verified both accepted-to-implement recommendations.

## Problem and evidence

Owner asks for walkthroughs to be available throughout a collection like Pages, with token-efficient generation and editing. Real10-page reader already has correct15-route indexes, reproduced exactly with existing source embed/index functions. Runtime's visible launcher condition ignores indexed routes. Its API guide's tours reference says menus list loaded-page paths and explicit links, omitting generated collection destinations; navigation/index guidance otherwise owns the right machinery. The rendered-file UI was not observed because CUA disallows file: content access. No claim that guide wording caused the runtime bug or measured token waste.

## Success definition and basis

User-directed behavior-changing update. An author can keep one walkthrough definition on its owning page, include a no-local-path peer in the explicit indexed collection, and deliver the existing supported markup/commands so the repaired runtime exposes that shared route on both pages. For an edit, the author works on the retained source/Markdown and regenerates indexed readers, without manually copying paths or editing generated index data. Scope is the common runtime+guidance repair, independent of the unapproved collection-builder proposal.

## Decisions

| Decision | Rationale |
| --- | --- |
| Reuse existing source-only embed/index/check commands and native ex-nav/ex-presentation/ex-step | Current functions already reproduce the actual collection; no extra API is needed to repair availability. |
| Indexed walkthroughs/journeys belong to the declared collection, including peers without local paths | Matches the owner's requested Pages-like discovery; no implicit filesystem discovery or browser fetching. |
| Keep definitions, facts and steps in their owning source pages; generated indexes are derived | Avoids a second catalogue an agent must maintain. |
| Source-first edits and concise checks, no raw generated-index dump | Serves the owner's authoring/edit context need without asserting measured token savings. |
| No new collection command/schema in this revision | That public authoring-contract decision is pending separately. |

## Surface allocation

Trigger unchanged. Main route's existing composition/navigation call may name the collection-wide availability obligation briefly. Depth stays in navigation.md and tours.md: explain generated-index discovery versus unindexed/local paths, require ex-nav on every indexed peer, preserve source-owned paths and labels, and keep documented command syntax in its current owner. Existing content.md source/freshness teaching is reused, not duplicated. Proof uses existing checker/indexer and real build/browser routes.

## Proof and claims

No invented RED for skill behavior. Draft from the owner's success definition; source/checker and actual fresh candidate application can establish named applications only. A fresh author receives a realistic collection/edit request with matching source-preview guide/tools; checks written beforehand require no duplicated walkthrough definitions, explicit shared index, source edits, preserved facts/labels and no generated-data hand editing. Inspect the actual candidate reads/actions and outputs. Runtime RED/GREEN stays separate; unavailable native file proof remains named.

## Coordination and non-goals

Base d60434596a267b8ac40066d99947a088b2b0296c on fix/demistify-collection-walkthroughs. Runtime/test correction and guide wording are implemented; independent implementation review and fresh author application remain required. Product repair needs a patch Changeset; consumed package version remains1.13.0 in feature source. No installed-cache mutation, new CLI/build contract, runtime manifest fetch, new framework, unrelated typography or Agent-VM page edits. Do not claim release from a source PR.

Spec review and implementation review are required independently. Acceptance/coverage belongs to separate reports; this file records no pre-awarded verdict.
