# Demistify — intent and cloud-agent iteration handoff

Date: 2026-10-14. Stage: experimental implementation and migration, with scoped checks and browser evidence. Repository: https://github.com/ShravanSunder/sunstack-tools. Active working branch: `claude/serene-pascal-llse28` (owner-confirmed on 2026-10-14; initial migration branch: `feat/demistify-html-islands`). Initial migration base: `f51342350ccdb38867bb24bfd86c9d8b3b62e386` on main. Head: the commit containing this document; obtain its exact identity with `git rev-parse HEAD`. Plan identity: none. Authority is Shravan's direct request to build five independent presentation experiments, iterate the HTML Islands concept, migrate the full lab to these package/app homes, and commit/push. This is not a completed canonical design/implementation-plan cycle or a production release claim.

## Intent: what we are trying to do

Coding agents repeatedly generate whole HTML/CSS/JavaScript applications just to explain engineering work. We want them to generate a small visual document instead. HTML is the composition language; a reusable library handles the expensive presentation and interaction implementation. The document should be useful before implementation: examine source, explain a proposal visually, let the developer inspect it, then revise the content without rebuilding the component library.

Markdown is an island, rather than the document's mandatory outer format. Mermaid is an island. A Mermaid fence in Markdown renders through the same island as an explicit `ex-mermaid`. Other components add decisions, comparisons, callouts, tabs, phases, layers, repository changes, detail inspection and arrows. Components can nest. The agent supplies meanings and content, while the theme owns colors and geometry.

This is a compact visual-document language for coding agents, not another generative application framework. A plain HTML page must be able to load the built runtime and stylesheet directly. No application backend, React/Lit/Astro application, router, or per-explainer Vite project is required. Vite is used once to build the shared library and separately to host the lab.

The lab compares approaches without assuming the research report's preferred architecture is a winner. It is not a controlled agent-generation, comprehension or performance experiment yet.

## Accepted owner corrections — preserve these

- Keep five prototypes on separate independently inspectable pages. Explain the same captured concept with comparable narrative, node identities, relationships, jobs, statuses, boundaries and sources.
- Separate reusable implementation from actual agent-authored usage. Count written content, composition, attributes and document-specific layout. Exclude renderer/library implementation, shared CSS/fonts/engines, packaging and generated output. Potential future library ideas never enter usage totals.
- HTML owns layout: ordinary flow, grid/flex, and absolute-positioned whiteboards are all legitimate. Avoid forcing every document into one reading layout. Arrows may calculate endpoint geometry; JavaScript must not position the islands or drive normal document scrolling.
- Markdown and Mermaid are explicit islands. Mermaid fences inside Markdown also render diagrams; other fenced code remains literal. A `text` fence can show Mermaid source.
- Every content island now automatically provides small circular sidebar and centered-view inspection/magnification controls. `ex-zoom` is the generic wrapper for ordinary HTML; document scopes, navigation and arrows are not content inspectors. Optional hidden detail content is revealed on inspection.
- Remove D2 from Prototype 5 and the Demistify runtime. Keep D2 in the other experiments that use it.
- Use Phosphor regular web icons as the shared standard. Icon controls need accessible names and straightforward interactions; there are no persisted viewer settings or author-managed state objects.
- Navigation comes from actual h1–h6, including rendered Markdown. It lives on the left. Keep a full outline visible when horizontal space permits; collapse to a rail on smaller screens. Hover a rail line to preview that exact section. Distinguish the hover preview from the current reading section. Group page headings and other-page links, with breadcrumbs above content.
- Do minimum meaningful experiment checks. Do not grow or repeatedly run a pedantic suite. Use scoped lint/format/types/build and real browser interaction for the changed UI.

## Repository shape and ownership

```text
packages/demistify/
  src/                      Native elements, rendering adapters and CSS
  src/packaging.ts           Optional Node packaging adapter
  scripts/build-library.ts  Build runtime/chunks/theme/font once
  dist/                     Generated drop-in assets (ignored)
  README.md                 Component/authoring reference
apps/demistify-poc/
  experiments/              Five authored usages and extra showcases
  fixtures/                 Shared captured planning corpus
  references/               Research, source HTML and extracted CSS
  src/experiment-builders/  Comparison-specific adapters, graph model, report/counts
  scripts/build-experiments.ts
  public/experiments/        Generated outputs and library copy (ignored)
  dist/                     Generated local/static demo (ignored)
docs/wip/demystify/
  2026-10-14-demistify-handoff.md
  style-study/              Sanitized CSS research
```

The library has no dependency on the PoC or its fixtures. The PoC uses a workspace dependency and copies the built library output. This is a real reusable boundary: a browser consumer can copy the library's dist directory and load its runtime/theme without the app. The Node packager exports TS source for Node 26 tooling; packaging/publishing this entry as distributable Node JavaScript remains a future release concern.

Pre-existing `docs/specs/` rule-engine documents are unrelated and were not changed. The original presentation lab remains available locally; migration did not delete its files. Do not reintroduce personal agent configuration, local session traces, caches, original private screenshots or machine paths into public source.

## Start and view from the monorepo root

```sh
pnpm install
pnpm check
pnpm build
pnpm --filter @sunstack/demistify-poc preview --port 4174
```

Preview binds loopback. The port is an example; choose another available local port. For development, `pnpm dev` builds both packages then serves the PoC. After changing library code, rebuild the library before inspecting its copied assets. To package new authored HTML against an already built library, use `pnpm --filter @sunstack/demistify-poc islands:package`; the full build refreshes all comparisons and measurements. Packaging usage never needs to create another application.

Routes beneath the local origin:

| Page                                               | Purpose                                                                    |
| -------------------------------------------------- | -------------------------------------------------------------------------- |
| `/`                                                | Lab index                                                                  |
| `/experiments/report.html`                         | Usage counts, supplied inputs and separate library/delivery sizes          |
| `/experiments/visual-plan.html`                    | Prototype 1 source, findings and rendered output                           |
| `/experiments/quick-json.html`                     | Prototype 2 inspector                                                      |
| `/experiments/markdown-directives.html`            | Prototype 3 inspector                                                      |
| `/experiments/graph-diff.html`                     | Prototype 4 inspector                                                      |
| `/experiments/html-islands.html`                   | Prototype 5 inspector                                                      |
| `/experiments/html-islands-live.html`              | Main native-island output                                                  |
| `/experiments/html-islands-layouts-live.html`      | Same islands in grid, flow and absolute whiteboard layouts                 |
| `/experiments/html-islands-composition-live.html`  | Markdown tables/code, explicit/fenced Mermaid and nested decision/callouts |
| `/experiments/html-islands-capabilities-live.html` | Additional layers, timeline, file-tree and composition examples            |

The three extra native showcases are outside benchmark usage totals. Download links expose standalone artifacts. Generated output is intentionally not checked in: reproduce it with `pnpm build`.

## The five experiments

| Prototype              | Authored usage                                                    | Implementation                                                              | What it establishes / limitation                                                        |
| ---------------------- | ----------------------------------------------------------------- | --------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| Visual Plan MDX        | `experiments/visual-plan/planning.mdx`                            | Pinned vplan 0.24.0                                                         | Existing compiled MDX/React vocabulary and diagram expansion; graph declarations repeat |
| Quick JSON             | `experiments/quick-json/planning.json`                            | Unmodified pinned upstream renderer under `experiments/quick-json/upstream` | Compact schema, cards/textual edges, static HTML; no connected graph view               |
| Markdown directives    | `experiments/markdown-directives/planning.md`                     | Remark + named D2 references rendered during packaging                      | Readable authored prose and six SVGs; resolver recognizes the corpus                    |
| Semantic graph changes | `experiments/graph-diff/planning.md` plus `planning-changes.json` | Validated stable-ID flat graph + explicit add/remove/change operations      | Changed nodes/edges declared once; generic Mermaid/D2 semantic parsing not implemented  |
| HTML Islands           | `experiments/html-islands/planning.html`                          | This reusable native component library                                      | Arbitrary HTML composition with Markdown/Mermaid and shared inspection/navigation       |

The canonical explanation has three perspectives, 25 view nodes and 24 relationships, with complete job/status/boundary/source facts. Source statuses are a dated snapshot, not live PR or implementation claims. The implemented-source graph has two nodes and one edge; the proposal adds six nodes and six edges, producing eight nodes/seven edges. “Added” describes this graph proposal, not implemented software.

Usage counts come from exact UTF-8 author files via js-tiktoken 1.0.21, encoding o200k_base, with per-file SHA-256. They are local source-token counts, not model billing or measured generated-token savings. Referenced corpus files are separate supplied inputs. Output/library size is a delivery metric, not an authoring-token metric. Read generated measurements rather than copying stale counts from earlier discussion. The first four normalized input counts were 2863, 5541, 2391 and 3011; native usage changes as its authored markup evolves.

## Library implementation map

The source layout moved into folders (core, islands, layout, render, node, styles) after this handoff was written; [the library README's source layout](../../../packages/demistify/README.md#source-layout) is the current map.

Light DOM allows normal HTML composition. No Shadow DOM isolation is currently required. Content rendering is guarded across reconnects; inspection moves real nodes instead of cloning live state. The renderer owns colors for added (green), removed (red), modified (amber), unchanged (neutral) and focused (accent). Contributor/workflow colors are separate meanings.

Navigation currently uses a CSS/media-query breakpoint of 1440px, rather than measuring each author's free space. That is a reversible experimental choice. Explicit author heading IDs are preferable for durable links. Generated IDs derive from heading text; duplicate labels get suffixes. Hidden tab/detail headings do not clutter the active outline. Breadcrumbs use ordinary `nav`/links with shared CSS, not an application router.

## Root causes behind the clipping corrections

The previous main native page explicitly set six diagrams to `max-height:360px;overflow:hidden`. Chrome showed roadmap content height 711 versus container height 360, and changed/proposed graph height 1189 versus 360. These declarations cropped real content; a positioning library would not restore it. Removed them so diagrams use natural height, with optional explicitly authored bounded scrolling.

Global `overflow-x:auto` on each Markdown island computed overflow-y:auto. Heading-only islands had a two-pixel font overflow and acquired vertical scrollbars. The fix keeps text/layout natural and gives code/table wrappers their own overflow. SVGs now center in their island and inspector. The modal/sidebar body legitimately scrolls when expanded content exceeds the viewport.

Floating UI is a credible optional adapter for future anchored popups; its official docs distinguish clipping ancestors from content sizing and recommend portals for reliable ancestor escape. Panzoom is a candidate for diagram-specific pan/pinch interaction. Neither was installed to mask the CSS defect. The current inspectors use the browser's top layer. Phosphor regular icons are bundled locally; upstream Mermaid Google Fonts imports are removed.

## Current proof and limits

The public package split must be validated from current files, not by trusting the original lab's old results. The governing proof exception is the owner's explicit request for minimum experiment checks and no pedantic test runs.

Current migration gates: root `pnpm fmt`, `pnpm check` (type-aware lint, formatting and strict TypeScript), and `pnpm build` (independent library, all five packages, static Vite output). Observed migration outcomes: install, formatting, type-aware lint, formatting check, strict TypeScript and independent library/full lab build all exited 0. Chrome on the migrated local HTTP preview showed the pinned grouped sidebar and breadcrumbs, outline updating with the selected tab, zero cropped visible diagrams, local Phosphor font loaded, and an icon-driven modal magnified to 125% then closed/restored. The tab recorded no browser errors. This is a scoped smoke, not a complete browser audit. Existing test sources are preserved and moved where ownership changed; no new suite was added and no current full-suite pass is claimed.

Historical local-lab browser proof established five independently rendered outputs, explicit/fenced Mermaid, Markdown code/table preservation, themes, tabs, modal/sidebar inspection and all three layout forms. Earlier 25/29-test runs predate subsequent UI changes and migration; they are not current migration proof. Browser file-protocol navigation was blocked by that host, so direct file opening and WebView behavior remain unverified.

Other limits: beautiful-mermaid implements a subset of Mermaid grammar; generic repository resolution/imports, arbitrary semantic diagram patches, graph obstacle avoidance, a drag editor, full accessibility/contrast/mobile audits, controlled comprehension results, agent generation cost and performance distributions are not established. The whiteboard is authored absolute-positioned HTML, not a drag editor. There is no deployment, npm publication, CI/release proof, merge or production claim.

## Public-source hygiene

The owner authorized publishing the full lab into this existing public repository, with private information excluded. The original private agent instructions and local traces were not copied. Captured research screenshots containing account UI or private discussion were excluded; sanitized written CSS findings and the extracted stylesheet remain. Original private captures stay outside this repo. Inspect the new files for secrets/machine/session references before any further public push. This is a bounded pre-publication inspection, not a comprehensive security audit.

## Known iteration issues

The native Changes diagram previously exposed a beautiful-mermaid label limitation with nested bracket status text. The generator now uses a middle-dot status suffix instead; inspect the generated SVG labels for the actual proposed nodes. This is a bounded generator correction, not full Mermaid-language compatibility. A very wide viewport can leave a large gap between the pinned sidebar and the centered authored document. Responsive hover preview is implemented, but the final migrated smoke exercised the pinned desktop mode; narrow-viewport hover/touch behavior needs its next focused check. Existing tests were not run after migration, and historical suites must not be presented as current green proof.

## Next iteration: what to preserve and what to explore

The owner subsequently named `claude/serene-pascal-llse28` as the cloud coordination branch. Read [the local sidebar contribution and latest owner requirements](2026-10-14-local-sidebar-followup.md), then the cloud agent's `2026-10-14-takeover-log.md` and decisions document when present. Preserve each writer's append-only record. The later resumed local contribution implements the automatic paired sidebar/centered-view controls and removes the old per-island mode opt-ins. Fresh browser verification of these paired controls remains blocked by the local Chrome surface. Read the latest timestamped contribution section before duplicating that work.

Preserve the package/app boundary, plain HTML entry point, minimal vocabulary, comparable five-way corpus and cost separation. Do not restart with five unrelated bespoke stories or include renderer implementation in usage totals. Do not add D2 to native islands. Keep diagrams and long text inspectable without crop-by-default or wrapper scrollbars.

The next cloud agent should first run/view this version and discuss one bounded improvement with Shravan. Likely questions: whether responsive outline should use a document layout contract rather than the current viewport breakpoint; how rich detail inspection should behave for arbitrary nested layouts; whether pan/pinch is worth a lazy diagram-only dependency; and how semantic graph changes should reference existing source identities without duplicating full diagrams. Research existing renderer capabilities before proposing a new diff parser. Measure an experiment before recommending a winner.

Do not invent formal design approval or production readiness. No canonical implementation plan exists. Design iteration is authorized; broader framework, security, persisted-data or deployment decisions still need their own owner intent. Use minimum meaningful checks, inspect the real browser result, and update this handoff when obligations or evidence change.

## Cloud-agent continuation prompt

> Work in ShravanSunder/sunstack-tools on branch claude/serene-pascal-llse28. Read AGENTS.md, this handoff and packages/demistify/README.md first. Demistify is a reusable native Web Components visual-document library; apps/demistify-poc is its independent demo and five-way experiment lab. Agents author compact HTML/Markdown/Mermaid composition, and the library owns style/rendering/navigation/inspection. Preserve arbitrary HTML layout, separate usage from library costs, no native D2, and the comparable corpus. Build/view current code before making claims. Continue design iteration with Shravan without assuming a winning architecture or requiring another application per explainer. Use minimum meaningful checks; do not grow a pedantic test suite. Keep unrelated rule-engine docs and private captures untouched. There is no canonical implementation plan or prior independent review to infer. Do not assume access to previous chat; report any missing referenced files or branch mismatch before editing.
