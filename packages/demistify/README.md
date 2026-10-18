# Demistify

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

Declare one native page-link list on each file. Its first link is Home; the matching URL supplies the current page name. The library derives the compact sticky header, Pages dropdown and breadcrumbs. Real headings populate the page-named, independently scrolling outline. Captions do not add outline entries. Presentations and Theme use the same navigation system.

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

## Gallery

Use one `ex-gallery` among other content or as the whole page. Review and grid are two views of the same selection and filters. Authors supply evidence; readers do not upload it.

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

`data-screen`, `data-viewport`, `data-state` provide default facets when present. Viewport names are authored strings, not inferred device claims. Add your own `data-…` fields and `filters="screen viewport release"`, or a small `script type="application/json" data-gallery-filters` array of `{ "field": "release", "label": "Release", "icon": "package" }`. Values derive from the figures. Icons use the curated subject list in `src/core/subject-icons.ts`.

Up/Down selects screenshots; Left/Right switches viewport categories while retaining related screen context when possible. Text fields keep normal editing keys. Search/facets share a 200ms trailing debounce; reset and direct navigation are immediate. An unchanged selected capture retains its image and zoom. Captions are native HTML or a Markdown island, with the same content in the shared inspector. Avoid putting a second application inside a caption.

## Composition vocabulary

| Elements                                                               | Use                                                                                          |
| ---------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `ex-document`, `ex-nav`                                                | Document and collection navigation                                                           |
| `ex-markdown`, `ex-mermaid`                                            | Full Markdown/file islands and scoped diagrams                                               |
| `ex-card`, `ex-callout`, `ex-region`, `ex-stat`                        | Titled semantic content, findings, regions and figures                                       |
| `ex-code`, `ex-note`                                                   | Code/unified diffs with neutral margin comments beside colored changes                       |
| `ex-tree`                                                              | Collapsible Markdown tree; plain labels, separate leading disclosure and trailing go-to icon |
| `ex-pre`, `ex-file-tree`                                               | Literal text/TUI drawings and plain file trees                                               |
| `ex-gallery`, `ex-compare`                                             | Screenshot review/grid and before/after comparison                                           |
| `ex-grid`, `ex-flow`, `ex-arrow`                                       | Native composition and labelled connections                                                  |
| `ex-tabs`, `ex-board`, `ex-layers`                                     | Alternative views, boards and optional layers                                                |
| `ex-presentation`, `ex-step`                                           | Named/described ordered stops through existing content                                       |
| `ex-explorer`                                                          | Searchable graph document, selection/history, tracing and evidence                           |
| `ex-zoom`, `ex-decision`, `ex-diagram-diff`, `ex-timeline`, `ex-layer` | Earlier lab vocabulary; reviewable in the legacy guide page                                  |

`src/vocabulary.ts` owns element attributes and validation/editor metadata. The packaged `skill/` teaches the matching markup; component-guide pages provide runnable examples.

## Shared visual and inspection behavior

The theme owns colors, fonts, spacing, radii and motion. Semantic roles for tone, selection, keyboard focus, links, presentation position, comments and diffs are distinct. Related items use 16px gaps; neutral boundaries are 1px, semantic accents 2px. Chosen is a separate badge and does not recolor a card's meaning. A small slow marker sits outside the current presentation target; reduced motion leaves it static.

Reading documents center in the space available beside the outline or inspection drawer. Prose stays measured; grids, code and diagrams can use the document canvas. A document containing only a gallery, optionally preceded by its h1, uses the available page width and height. Keep extra introduction text in a separate reading section when it would turn that dedicated viewer back into normal document flow. Explicit author CSS can still choose another layout.

The shared reading scale uses a 15px body, 46px title, 36px section heading and 22px subheading, with smaller headings on narrow screens. Callouts have a bounded reading width and restrained semantic border/wash; their text fills that width. Use the existing `eyebrow` class for a short optional label: inside a card it takes the card's tone. Inline code and disclosure summaries use the matching shared accents. Do not repeat these defaults as per-page CSS.

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
