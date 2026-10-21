# Demistify

The [component guide](skill/SKILL.md) is the API teaching entry. Its chooser leads to detailed references for [content/tables](skill/references/content.md), [widths/grids/tabs](skill/references/layouts.md), [cards/callouts/numbers](skill/references/cards-and-callouts.md), [navigation](skill/references/navigation.md), [inspection](skill/references/inspection.md) and [Mermaid](skill/references/diagrams.md), alongside the existing rich-view references. Keep authoring detail there rather than making this README another full manual.

Agents author HTML, Markdown and image references. Demistify supplies native elements, rendering, a shared visual language, navigation and inspection. Use the runtime and authoring guide from the same release.

## Open the integrated guide

From the monorepo root:

```sh
pnpm install --frozen-lockfile
pnpm build
pnpm --filter @sunstack/demistify-poc dev:serve --port 4202
```

Open `/experiments/agent-written/component-guide/index.html`. The nine guide pages cover every current and earlier element, with real examples and copyable markup. The PoC server previews the files; it is not a backend required by a reader's document.

For reader delivery, keep HTML small and import the matching shared library from Cloudflare. The default channel is `https://demistify.getagentstudio.dev/v1/`; it serves runtime, CSS, fonts, worker and lazy chunks. No reader-local server or application is required. The live version and source commit are recorded in `/v1/release.json`; the matching API guide is `/v1/authoring/SKILL.md`. A source preview uses the local build so it cannot accidentally combine new markup with an older runtime.

```html
<link rel="stylesheet" href="https://demistify.getagentstudio.dev/v1/theme.css" />
<script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script>
<ex-document>
	<h1>Session architecture</h1>
	<ex-markdown id="requirements" src="./notes/requirements.md"></ex-markdown>
	<ex-card id="lifetime" title="Session lifetime" tone="decision" pick>
		The registry owns the session.
	</ex-card>
</ex-document>
```

## Files and no-server delivery

Small HTML plus shared HTTPS assets is the default. Do not inline the library, fonts or image bytes. Keep images as ordinary local paths or web URLs. Several HTML pages reuse the same Cloudflare assets and browser cache.

| Content                                    | Direct-file delivery                                                                            |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------- |
| Markdown already in the page               | The Cloudflare runtime renders it                                                               |
| Markdown at an HTTPS URL with allowed CORS | `ex-markdown src` loads that URL                                                                |
| Sibling local Markdown                     | Embed its exact source at author time; browsers block fetching it from `file://`                |
| Images                                     | Keep normal relative `img src` paths or HTTPS URLs                                              |
| Explorer JSON                              | Supply inline JSON or a CORS-readable HTTPS source; sibling fetch has the same file restriction |

The existing source CLI embeds only Markdown source and keeps external library/image references:

```sh
node packages/demistify/bin/demistify.ts embed input.html output.html
node packages/demistify/bin/demistify.ts check output.html
```

Run it once while authoring, from the matching dependency-installed checkout. Keep output beside the input or preserve relative page/image paths. Rerun after Markdown edits. The reader double-clicks the HTML; no Python, Node, Vite or web server is required. Internet access is required for Cloudflare assets. The same files can optionally be hosted by any ordinary static HTTP server; it needs no Demistify-specific routes or backend. The older research-only standalone bundler is a separate fully-offline experiment, not the normal delivery.

Markdown islands render GFM tables, fenced code, nested/task lists, links, images and Mermaid fences. The source control opens the exact original Markdown in the shared sidebar. Use a script-backed island when literal `<` must survive HTML parsing:

```html
<ex-markdown id="notes"
	><script type="text/markdown">
		## Recovery

		The literal `<registry>` remains source text.
	</script></ex-markdown
>
```

## Pages, breadcrumbs and presentations

Declare one native page-link list on each file. Its first link is Home; the matching URL supplies the current page name. The library derives the compact sticky header, Pages command bar and breadcrumbs. Real headings populate the page-named, independently scrolling outline. Captions do not add outline entries. Walkthroughs and Theme use the same navigation system.

```html
<ex-nav for="content"
	><nav slot="pages" aria-label="Pages">
		<a href="index.html">Session guide</a>
		<a href="details.html">Implementation</a>
	</nav></ex-nav
>
<ex-document id="content">
	<!-- Existing content islands live here. -->
	<ex-presentation
		id="review"
		title="Session review"
		description="Requirements and the lifetime decision."
	>
		<ex-step to="requirements--recovery">Read recovery.</ex-step>
		<ex-step to="lifetime" open="sidebar">Inspect the decision.</ex-step>
	</ex-presentation>
</ex-document>
```

A presentation is an ordered path, not a content container. Several paths can reuse any existing island or inner anchor; one stop is valid. `ex-route` and wrapper-selection semantics are removed in this integrated vocabulary. Use `#requirements--recovery` for a Markdown heading, `#architecture--Registry` for a node in Mermaid island `architecture`, and `other.html#target` for a cross-page stop. Missing targets show a visible player error. Native fragment links, tree go-to links and presentation stops share target readiness/revelation behavior.

The **Walkthroughs** chooser uses compact icon/title/description rows. A Walkthrough may span pages and mix text, diagrams and pictures. A **Gallery Journey** is a pictured sequence using the same player. Local picture-only journeys use the images icon; mixed, document and cross-page paths use the path icon. No extra type column or visible close text is needed. Existing `ex-presentation`/`ex-step` markup and native navigation remain.

Keep `ex-nav` on a single-page, single-path example too: it supplies the visible launcher for the hidden `ex-presentation` declaration.

### Reading measure and visual canvas

Typography uses regular 18px Source Sans 3 prose, Source Sans 3 controls, Archivo headings, and IBM Plex Mono for code. Tree roots use 20px/600, branches 17px/500 and leaves 17px/400.

Two shared width standards share one left edge in the space remaining beside the outline or inspector from 1.10.0. **Reading** is 760px for prose, Markdown, lists, quotes, simple text cards and callouts; their text and headings stay left-aligned. **Canvas** fills the document up to 1600px for tables, images, galleries, code/diffs, diagrams, grids, comparisons, Explorer and rich trees. Ordinary prose/headings use Reading automatically; visual islands keep their own layout. Document headings, prose and dividers always share the Reading column at Canvas's left edge, including beside wide tables or images. A visual section can contain Reading text and Canvas islands with its heading and visual starting at the shared left edge. Apply `data-width="reading"` or `data-width="canvas"` to the whole component or native layout wrapper. A tab set includes its tab bar and panels in that width; the same convention applies to code, comparisons, galleries and other islands. The agent chooses the width. A Canvas wrapper leaves document headings and prose on Reading while its visual islands can expand. Component-internal titles and table cells retain their local layout. These are author choices, not reader settings. Nested grid/card/island content follows its component, and narrow viewports fit both standards to the available space. Tables use quiet alternating rows and one separator below the column header, with no rules between body rows. Section dividers always retain the Reading measure and its left edge, including above Canvas sections. Dedicated galleries retain their viewport surface. Remove obsolete per-page width overrides when adopting the standard.

## Gallery

Walkthrough narration uses focused Markdown and optional `ex-step icon="path"` subject icons. Use a bold lead, short paragraphs/lists, links and inline code; keep full diagrams and evidence in the targeted content. See the matching [walkthrough guide](skill/references/tours.md).

Use one `ex-gallery` on its own page; use ordinary figures or a link to that page from an explanation. Dedicated-page checker enforcement is a separate product prerequisite and is not provided by 1.10.0. Review and grid are two views of the same selection and filters. Authors supply evidence; readers do not upload it.

```html
<ex-gallery id="evidence" label="UI evidence">
	<figure id="home-mobile" data-screen="Home" data-viewport="Mobile" data-state="Ready">
		<img src="captures/home-mobile.png" alt="Home, signed in" />
		<figcaption><strong>Ready state.</strong> The account menu remains reachable.</figcaption>
	</figure>
	<figure id="home-laptop" data-screen="Home" data-viewport="Laptop" data-state="Ready">
		<img src="captures/home-laptop.png" alt="Home on a laptop" />
		<figcaption>
			<ex-markdown
				><script type="text/markdown">
					The same **ready state**, with a wider layout.
				</script></ex-markdown
			>
		</figcaption>
	</figure>
</ex-gallery>
```

`data-screen`, `data-viewport` and `data-state` group navigation. Viewport names are authored strings, not inferred device claims. Use comma-separated `data-tags="Journey, Account"` for default content filters. Explicit extra facets use `filters="tags release"`, or a small `script type="application/json" data-gallery-filters` array of `{ "field": "release", "label": "Release", "icon": "package" }`. Values derive from the figures. Icons use the curated subject list in `src/core/subject-icons.ts`.

Left/Right changes logical screens/states; Up/Down changes variations of the same screen. Text fields, popovers and inspectors keep their normal keys. Search/facets share a 200ms trailing debounce; reset and direct navigation are immediate. An unchanged selected capture retains its image and zoom. Fit, Fill and minus/plus replace numeric percentage controls. Review/Grid are a button group. Captions are native HTML or a Markdown island, with the same content in the shared inspector. Avoid putting a second application inside a caption.

For a pictured journey, target gallery figure IDs from the existing `ex-presentation`/`ex-step` path. Its player floats below the viewer with a reserved bottom gutter. Add the optional `variations` flag to enable a size button group and Up/Down switching; otherwise each step uses its authored capture. Missing variations remain visible gaps on the same step. Leaving restores the previous gallery search, filters, view, selection and zoom.

## Composition vocabulary

Rich alternatives use `ex-compare`: keyed `data-compare-row` topics share CSS Grid tracks, missing topics are explicit, and cells may contain supported islands and ordinary pictures, with links to dedicated Gallery pages for screenshot browsing. The author chooses columns or tabs and the whole component's Reading/Canvas width. Optional `choose` selects two live alternatives. Comparisons grow with their content and keep their headings sticky during page scrolling. See the [comparison reference](skill/references/comparisons.md).

`ex-trace` is an ordered sequence of native disclosures with status and arbitrary rich attachments. Tree remains the hierarchy/navigation view. The [Trace reference](skill/references/traces.md) shows its syntax; [source-derived call paths](skill/references/call-paths.md) compose it with comparison while keeping branches distinct from call frames and runtime evidence.

| Elements                                                               | Use                                                                                          |
| ---------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `ex-document`, `ex-nav`                                                | Document and collection navigation                                                           |
| `ex-markdown`, `ex-mermaid`                                            | Full Markdown/file islands and scoped diagrams                                               |
| `ex-card`, `ex-callout`, `ex-region`, `ex-stat`                        | Titled semantic content, findings, regions and figures                                       |
| `ex-code`, `ex-note`                                                   | Code/unified diffs with neutral margin comments beside colored changes                       |
| `ex-tree`                                                              | Collapsible Markdown tree; plain labels, separate leading disclosure and trailing go-to icon |
| `ex-pre`, `ex-file-tree`                                               | Literal text/TUI drawings and plain file trees                                               |
| `ex-gallery`                                                           | Screenshot review and grid views over one collection                                         |
| `ex-compare`, `ex-trace`                                               | Aligned arbitrary alternatives and ordered expandable evidence                               |
| `ex-grid`, `ex-flow`, `ex-arrow`                                       | Native composition and labelled connections                                                  |
| `ex-tabs`, `ex-board`, `ex-layers`                                     | Alternative views, boards and optional layers                                                |
| `ex-presentation`, `ex-step`                                           | Named/described ordered stops through existing content                                       |
| `ex-explorer`                                                          | Searchable graph document, selection/history, tracing and evidence                           |
| `ex-zoom`, `ex-decision`, `ex-diagram-diff`, `ex-timeline`, `ex-layer` | Earlier lab vocabulary; reviewable in the legacy guide page                                  |

`src/vocabulary.ts` owns element attributes and validation/editor metadata. The packaged `skill/` teaches the matching markup; component-guide pages provide runnable examples.

## Shared visual and inspection behavior

The theme owns colors, fonts, spacing, radii and motion. Semantic roles for tone, selection, keyboard focus, links, presentation position, comments and diffs are distinct. Related items use 16px gaps; neutral boundaries are 1px, semantic accents 2px. Chosen is a separate badge and does not recolor a card's meaning. A small slow marker sits outside the current presentation target; reduced motion leaves it static.

Reading and Canvas share a left edge in the space available beside the outline or inspection drawer. Prose stays measured; grids, code and diagrams can use the document canvas. A document containing only a gallery, optionally preceded by its h1, uses the available page width and height. Keep extra explanation on the linking page rather than turning the dedicated viewer back into an embedded gallery. Explicit author CSS can still choose another layout.

The shared reading scale uses 18px regular paragraphs, 46px title, 36px section heading and 22px subheading, with smaller headings on narrow screens. Callouts have a bounded reading width and restrained semantic border/wash; their text fills that width. Use the existing `eyebrow` class for a short optional label: inside a card it takes the card's tone. Inline code and disclosure summaries use the matching shared accents. Do not repeat these defaults as per-page CSS.

For two or three short alternatives with the same criteria, show columns so matching topics are visible together. Use tabs when each alternative needs a longer explanation or diagram. HTML and islands remain the content of either layout. The author chooses the presentation; readers should not have to configure column counts or layout before they can understand the options.

Existing HTML that imports the shared `/v1/` theme receives compatible appearance updates after deployment and refresh. An embedded/frozen stylesheet or an explicit author override keeps its own appearance. A shared style update does not add newly authored content or interactions to an older page.

Sidebar inspection copies content and preserves the main view. Fullscreen uses the shared native dialog; Escape, Close or a backdrop click dismiss it and restore focus. Cmd/Ctrl+wheel zooms and modifier-drag pans images/diagrams; ordinary scrolling remains native. Controls provide the same actions without gestures. The library subsets actual Phosphor glyphs into the matching build.

## Authoring measurements

Six single-run trials compared the same two-page review authored with raw HTML/CSS/JS and Demistify 1.2.0:

| Profile     | Raw reader-source tokens | Demistify reader-source tokens |
| ----------- | -----------------------: | -----------------------------: |
| Sol, medium |                    8,477 |                          3,074 |
| Luna, high  |                    5,963 |                          3,487 |
| Luna, xhigh |                   15,947 |                          3,315 |

Total author-session output was lower with Demistify for Sol medium and Luna xhigh, and higher for Luna high. Content errors remained, including exact-text mistakes that structural checks missed. Read the [full measurements and proof limits](../../docs/benchmarks/demistify-authoring-2026-10-17.md) before drawing cost or quality conclusions.

## Proof and source owners

```sh
pnpm check
pnpm test
pnpm test:browser
pnpm build
```

Browser tests use real built assets and native browser inputs. Checker output, HTTP browser behavior, direct-file behavior and hosted release integrity are separate evidence boundaries.

Reusable runtime, styles and Node packaging live in `packages/demistify`; authored pages and browser fixtures live in `apps/demistify-poc`. There is no per-document router, renderer fork or application build.
