# presentation-html integration after the fixes

The fixes branch introduces new library features. The previously published c96a72e runtime and its guide do not support them. Do not edit that frozen public guide or tell agents to use this new syntax against that old runtime.

## Library-owned changes

- `ex-markdown src="./notes.md"` loads a file through the normal Markdown/Mermaid renderer. Its source button uses the shared responsive inspector and retains the exact source beside the rendered content.
- `demistify embed input.html output.html` reads local Markdown references and embeds them safely; it preserves shared library links. The consumer does not fetch a sibling Markdown file. Keep relative page/image resources available when moving output. Full offline packaging separately embeds the runtime/resources with explicit data resolvers.
- `ex-presentation id title description` defines one complete explainer. Only the selected one is displayed. The Presentations button lists names/descriptions; authored `nav slot="presentations"` links may target other files or fragments.
- `ex-nav` keeps the Pages list (`nav slot="pages"`) separate from the active heading outline. Include the complete navigation on every collection page. Breadcrumbs remain normal accessible HTML.
- Named Markdown headings use stable `island-id--heading-slug` fragments. Mermaid uses its component id. Initial and clicked fragments reveal the containing presentation/tab/disclosure before scrolling.
- Pictures, Mermaid and other visual inspection views support modifier-wheel zoom and modifier-drag pan, with normal scrolling and accessible reset controls retained.

## Sunstack workflow update

The integrating agent owns `plugins/sunstack-dev/skills/presentation-html/` in Sunstack. Update that existing source; do not create another component catalog or new workflow skill here.

At scope selection, identify the collection, each presentation's name/description and whether it belongs in this file or another page. At composition, require the explicit presentation/page navigation pattern from the matching canonical library guide. At Markdown inclusion, prefer a reference to the authoritative file when live includes are intended; choose the supported embedding command for a snapshot/no-server artifact. Do not have the agent hand-copy prose or invent a per-page generator. Teach Mermaid fences inside referenced Markdown and the exact-source control. At proof, exercise named presentation selection, page links and a cold deep link into Markdown/Mermaid, in addition to visible source/main retention.

## Binding and evidence

Use a verified public runtime/theme/guide version containing these changes, or a complete matching trusted local build while public deployment is pending. The deployment fork owns the new public major paths. Consuming public assets is still separate from publishing authored pages; no deployment or credential discovery belongs in the presentation skill.

The canonical syntax and working snippets are in `packages/demistify/skill/SKILL.md`. The review gallery is `apps/demistify-poc/experiments/agent-written/component-guide/`; the focused runtime fixture is `navigation-markdown-fix/`. Browser regression sources and proof limits are in `apps/demistify-poc/tests/browser/README.md`. Source counts, generated HTML/data and reusable implementation costs remain separate. No billed/context-efficiency claim follows from source size.
