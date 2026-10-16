# Demistify fixes — owner task list

Updated 2026-10-16. Fixes belong to branch `fix/demistify-mermaid-scroll`; Cloudflare publishing belongs to the separate deployment fork. Implementation, exercised behavior and remaining integration are reported separately in [the proof and handoff](2026-10-16-fixes-proof.md).

## Layout and inspection

- [x] Keep wide Mermaid diagrams locally scrollable, endpoint labels reachable and inspection controls in view; contain outer overflow at 1600px and 390px.
- [x] Keep main content within the pinned-navigation gutter instead of shifting it far right.
- [x] Wrap tree text under its label while keeping icons beside the first line; preserve inline code, loose paragraphs and folding.
- [x] Keep a single top accent and CHOSEN badge on picked cards, with neutral 1px side borders and no full ring.
- [x] Keep the rendered main island when inspecting a sidebar copy; restore live content and invoking-control focus after fullscreen Close/Escape.
- [x] Support Cmd/Ctrl+wheel zoom and modifier drag in visual inspection. Actual picture and actual Mermaid cases exercise magnification, ordinary scrolling, available-axis pan, reset and retained main content.

## Presentations, pages and fragments

- [x] Add named/described `ex-presentation` sections; display only the selected presentation.
- [x] Add a Presentations button/menu with local presentations and ordinary relative links to other explainers.
- [x] Keep Pages separate from the active heading outline, with the current page indicated and normal HTML breadcrumbs.
- [x] Reveal the containing presentation/tab for cold Markdown-heading and Mermaid-component fragments.
- [x] Supply collection navigation on each review-gallery page and a focused three-presentation/two-page example.

## Markdown and source

- [x] Implement `ex-markdown src="./notes.md"` with relative loading, busy state and visible error handling.
- [x] Render real Mermaid SVG from fences in referenced Markdown through the shared renderer.
- [x] Support exact-source sidebar inspection without replacing the rendered main content.
- [x] Preserve source/reference through local embedding and preparation, including literal HTML/script-boundary text.
- [x] Add `demistify embed input.html output.html` through the library packaging owner; no custom per-page generator or hand-copied prose.
- [x] Update the source checker, canonical vocabulary and authoring guide. Explain inline/embedded no-app-server viewing, sibling-file fetch restrictions and the separate full-offline path.

## Verification and review gallery

- [x] Add useful Vitest Browser Mode coverage with an Implementer Sidekick and independent Verifier; preserve Node tests and existing checks.
- [x] Run 36 Node tests and 12 real Chromium browser scenarios; preserve original native Mermaid pan failure, the bounded CSS correction and later passing results.
- [x] Check all six gallery pages and three focused fixtures: zero source-check errors/warnings.
- [x] Provide examples, descriptions, composition guidance and copyable markup for all 28 registered components across six review pages. Mark earlier vocabulary explicitly.
- [x] Manually review representative content/card, visuals, layout/navigation and graph-explorer pages using one retained browser tab.
- [x] Complete the independent Verifier's initial frozen-build launch-to-cleanup pass; preserve its exact ten-case scope and limits separately from the compact-menu correction.
- [x] Commit/push the scoped product changes and hand the revision to deployment/skill owners; final documentation preserves verification boundaries.

- [x] Reproduce and repair the compact heading-menu hover/click dismissal; the final regression rejects the old runtime, all 11 current cases pass, a narrow manual page transition works, and the independent focused correction check passes on fda39cb.

## Cross-owner integration

- [x] Give the Sunstack Skills Lead the new API/guide delta and old-public-pin boundary; acknowledgement received.
- [x] Deployment owner publishes matching public runtime/theme/guide at `https://demistify.getagentstudio.dev/v1/`; all 28 assets pass actual HTTP byte/MIME/CORS/cache checks. The existing `c96a72e` pin remains unchanged. See [v1 handoff](2026-10-16-v1-deployment-handoff.md).
- [ ] Skills Lead updates the existing `presentation-html` plugin against that verified public version. Its draft remains on the old pin until then.

Earlier code/source-retention and neutral code-comment background repairs are documented in [the inspection fixes](2026-10-16-inspection-fixes.md); this list tracks the subsequent fixes fork.

## Explicit proof limits

The focused browser suite is a focused regression suite, not an exhaustive audit of every component or browser. Exact pointer-anchor stability, every responsive transition while a drawer is open, loading-state timing, keyboard accessibility across every gallery component, and original private-explorer visual parity are not established. Cmd zoom/pan/reset on real pictures and Mermaid are established. Embedding without a sibling Markdown file was exercised over HTTP; full offline/file-protocol browser operation and Cloudflare/CORS/release behavior are separate, unverified here. The public skill/deployment work remains assigned to its existing owners.

## Follow-up: Mermaid hover

- [x] Repair disappearing highlighted connector strokes and arrowhead enlargement on node hover; the real regression fails before the fix and passes in both themes. Node/browser/check/build gates pass; scoped follow-up delivery is recorded in the proof.
