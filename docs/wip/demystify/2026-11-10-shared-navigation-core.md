# Shared document navigation core

Owner instruction: “ok lets move on core”, following the shared-JSON model and separate worktrees/PRs for standalone compilation and direct-TS Node/Bun hosting/launcher/idle shutdown. Work alone. Base: origin/main `1abce0b`.

## Authoring and ownership

Every collection page uses `<ex-nav for="content" src="./navigation.json"></ex-nav>` and ordinary HTML content. One JSON source owns ordered Pages entries, the semantic Map and complete question-led Walkthroughs. Relative page addresses resolve beside that JSON source, including when a page lives in a subdirectory. HTML owns stable target IDs, diagrams and explanations. Map and Walkthroughs reference content rather than copying it.

The version-1 JSON has `pages` (id, title, href, subject icon), `map` (ordered nested label/icon/target/children nodes), and `walkthroughs` (id, title, description, ordered target/narration/optional-icon steps). A target references a page ID and optional fragment. Page IDs and route IDs must be unique; missing page references, unknown fields/icons and unsafe URLs fail validation. This is new source capability, not supported by the previously published runtime.

The Map uses the existing shared drawer, with an always-expanded vertical list, real subject icons, whole-row links and search across group/item labels. Page outline remains derived from the current page's headings. Pages and route discovery are available on every page using the shared source. Cross-page route continuation uses that source's route definition and the existing target resolver/player.

## Owning boundaries

- `core/shared-navigation.ts`: one schema/type authority and pure reference resolution.
- `layout/shared-navigation.ts`: fetch/validate a source, bind typed data to existing navigation consumers, dispose on disconnect.
- `layout/shared-map.ts`: semantic Map rendering/search/keyboard behavior; reuse the existing Map drawer lifecycle.
- Existing `navigation.ts`, `page-navigation.ts` and `route.ts`: their present chooser/player responsibilities, accepting shared input rather than a second navigation engine.
- Existing Node checker: validate a declared local JSON source through the same schema; report missing/bad input.
- App: an ordinary multi-page consumer and the owner's real copied How work runs example. No author script should rebuild library navigation.

## Execution and proof

1. Add a real multi-page HTTP fixture whose only shared-navigation declaration is `src`, and permanent browser assertions for Pages, Map and JSON Walkthroughs. Run against the unchanged built runtime to observe the intended failure.
2. Implement the shared source and bind existing controls. Keep loading failures readable while the normal content/outline remain usable; reject mixed authored/shared navigation rather than silently merging two owners.
3. Add focused schema/reference/checker cases for bad sources and relative destinations. Prove cross-page menus, Map links, fully expanded icons/search, and route start/continuation from a peer page through the actual runtime.
4. Run scoped OXC/types/build and existing affected navigation/route coverage. Inspect desktop and 390px, light/dark, with the real built library. Use computer use for the owner's on-screen example; distinguish desktop narrow viewport from physical iPhone/Safari.
5. Update matching library authoring guidance and show a copied How work runs page using one shared JSON across its peer pages. Preserve original diagrams and original source files. No public deployment, release, merge, standalone compiler or product server/launcher in this slice.

Scope is the reusable shared-navigation core and its working example. Earlier rejected layouts and unfinished single-column/gallery changes stay in the old worktree. The existing released requirement/call readers are outside this slice; their proposed retirement/independent call-stack contract remains separate work.
