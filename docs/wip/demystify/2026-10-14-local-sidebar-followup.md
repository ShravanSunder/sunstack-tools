# Local sidebar follow-up — communication for the cloud agent

## 2026-10-14 16:17 UTC — contribution and coordination boundary

Shravan asked the local agent to contribute to `claude/serene-pascal-llse28` and communicate through a document. The initial full migration is commit `a31c1da50b9b3120650a1e18f169472348c6ee0b` on `feat/demistify-html-islands`. At this checkpoint the cloud branch had not yet been published to GitHub: fetching that exact ref returned `couldn't find remote ref`, and subsequent exact-ref reads returned no matching head. These observations do not mean the cloud agent has no local work.

The owner relayed that the cloud agent intends to create an append-only `docs/wip/demystify/2026-10-14-takeover-log.md` alongside its decisions document, then write step 1 code. That file was not available on a published branch at this checkpoint. This separate document avoids editing the other agent's sole-writer log. Please preserve both records. The contribution will be pushed normally, without force, to the exact owner-named branch; if that ref is still absent, this publishes the known base plus the bounded follow-up. If your local branch already contains new commits, fetch/read the remote and integrate the changes before pushing. Do not overwrite either agent's work or rewrite history to bypass a non-fast-forward rejection.

## What this follow-up changes

- The heading observer ignores DOM mutations inside its own rail and portaled outline. Previously the documented body-only fallback could observe the outline it had just rendered and repeatedly rebuild itself. This is a source-grounded causal correction at the existing observer boundary, not a polling workaround.
- The popover's left edge touches the rail, avoiding a pointer gap that could close the outline while moving from a marker into the panel.
- Keyboard focus on a marker opens its corresponding preview. Native `beforetoggle`/`toggle` events own the popover transition guard, so focus restoration cannot re-enter an unfinished native operation.
- Disconnect/reconnect reattaches the outline and observation listeners, and resets a canceled frame handle.
- Existing island inspection triggers now have small 28px circular buttons and 16px Phosphor glyphs.
- A new extra showcase, `/experiments/html-islands-navigation-live.html`, uses no main/ex-document wrapper and contains h1 through h6 rendered by Markdown. It is outside benchmark usage totals.

No inspector behavior/API rewrite was made in this slice. The cloud agent's step 1 implementation can own the fuller all-islands control change below. No framework, backend, polling loop, persistence or positioning dependency was added.

## Owner's latest navigation and inspection requirements

These are owner requirements, not findings inferred from a screenshot:

1. A lightweight component enumerates actual h1–h6, including headings produced by Markdown islands. Avoid a separately authored table of contents or application-router setup.
2. Navigation is on the left. If there is sufficient horizontal space, leave the outline visible. Collapse it to a compact rail when space is limited. The current viewport breakpoint is only an experimental implementation choice.
3. Hovering a rail marker must identify that exact corresponding section in the preview; do not open an undifferentiated list with the wrong row highlighted. Keep preview selection distinct from the current reading section.
4. Design page headings, links to other pages and breadcrumbs coherently. Current proposal: On this page + Other pages in the left outline, Explainers › Current page above content, and ordinary relative links.
5. All content islands should offer small circular icons for a right sidebar or a large centered expanded view. Use Phosphor, accessible labels/tooltips and simple interactions. The user should not supply viewer-state objects. The current inspector still uses opt-in zoom attributes and one chosen action; the circular styling in this follow-up does **not** claim the all-islands/two-destination behavior is finished. Please reconcile that API in your step 1 work and show both affordances where appropriate.
6. Layout remains normal authored HTML: grid, flow, flex and absolute whiteboards. Navigation and expansion must not force an application framework or rearrange authored islands. Markdown/Mermaid can have their own useful content viewports; headings and layout wrappers should not acquire accidental clipping or scrollbars.

## Proof and remaining uncertainty

`pnpm fmt`, `pnpm check` and `pnpm build` passed before this note was written; the final tiny styling/doc addition gets the scoped checks again before commit. The new showcase is generated and the runtime is served by the actual local preview. Existing automated test sources are retained; no test suite was run, respecting the owner's experiment instruction.

Browser control became unreliable during the narrow hover check: a mouse-input call took an unexpectedly long time and the page was no longer the requested explainer afterward; a later body-only navigation attempt timed out creating an inspection context. These observations do not prove a runtime defect or a successful hover check. Earlier migrated desktop proof still establishes visible grouped sidebar/breadcrumbs, active-tab heading updates, local Phosphor font, zero author-cropped visible diagrams, modal zoom to 125% and restoration, with no recorded errors on that earlier page. The follow-up's body fallback, reconnect and narrow hover transitions need a fresh focused browser check by the cloud agent. Do not claim that check already passed.

Other known iteration issues from the initial handoff remain: native changed-node labels containing nested brackets can expose the renderer's label limitation, and very wide viewports can leave excess space between a fixed left outline and centered authored content. Improve these through bounded changes rather than rebuilding the experiment lab. Keep the original private research captures and personal session traces out of public commits.
