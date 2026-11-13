# Remove the requirements browser

Owner direction: fully remove the old requirements browser rather than retaining it behind a writing prohibition. Preserve the call-stack component. Story-layout experiments wait until this removal is finished.

## Result

The built runtime, authoring vocabulary, generated metadata, matching guide and current examples expose ordinary HTML composition, shared Pages/Map/Walkthrough navigation and the retained call-stack reader. The requirements, stage and related-document browser UI is removed. Historical records remain historical; no compatibility element, alias or replacement browser is introduced.

## Current coupling

`CallStackIsland` inherits its lifecycle from `RequirementsIsland`. `mount-reader.ts` registers both reader implementations. The retired browser owns its stage/related-document UI; shared collection records also supply call-stack facts and cannot be deleted indiscriminately.

## Implementation

1. Give the retained call-stack element its own existing lifecycle and mount only its required controls. Remove the retired public element and requirements-only registration, controller, stage/related-browser code and styles. Keep any shared behavior proven to be required by calls.
2. Remove the obsolete authoring vocabulary and generated metadata. Keep the existing prepared call-stack input contract and its validation. Resolve retained need/source links to real owner HTML destinations rather than a removed viewer.
3. Convert current examples and shared test fixtures to ordinary visible owner HTML and/or the call-stack component. Retire assertions solely about the removed feature; preserve shared Comments, inspection, source validation, keyboard and call-stack coverage. Do not erase historical work documents or the five comparison experiments.
4. After the skill design review accepts the companion design set, update the Demistify guide inside that accepted boundary. The Lead owns those skill files until that gate passes; product implementation can proceed independently.
5. Declare the experimental API removal with a Changeset. Keep a source change separate from version/release execution.

## Proof

Run scoped OXC formatting/lint/typecheck, library/app build and the existing Node and browser suites appropriate to retained collection and call-stack behavior. Check generated public metadata and packaged guide; the retired element must be absent from active authoring surfaces and runtime registration. Exercise retained call navigation, Comments and inspection at desktop and 390 CSS px in both themes. Preserve meaningful failure checks. Report actual counts and explicitly identify retired-feature tests.

### Deciding link and keyboard observations

An existing call-to-need link resolves to the prepared need record's `page` plus its authored `bodyRef` fragment. The record identifier and body identifier can differ; use the supplied body identity rather than inventing a `#needId` convention. The owner page keeps that body visible as ordinary HTML. A frame's `source` field remains an exact caption; this retirement adds no source-file link feature.

The permanent browser harness must exercise the retained arrow-key selection and its existing text-entry guard; keyboard coverage is not inferred from a handler existing. The packaged guide remains an open gate until the separate skill-review/creation runs finish.

## Boundaries

No Story experiment or production Story styling change; no Markdown HTML execution change; no edits to other agents' design pages, domain/storage design or installed plugin caches; no merge, deployment or release without a new grant. Source base: `b8bd05bd6a0462cac4f6a215ac55d6b25f01c5bc`. One isolated worktree: `fix/demistify-retire-requirements`.
