# HTML Islands authoring reference

HTML owns composition and layout. Islands can live in ordinary flow, CSS grid/flex, or absolute-positioned whiteboards. The library renders content and shared interactions; it does not arrange or move the islands. Prototype 5 uses Markdown and Mermaid. D2 is excluded.

```html
<link rel="stylesheet" href="islands/theme.css" />
<script type="module" src="islands/runtime.js"></script>

<div style="display:grid;grid-template-columns:1fr 1fr;gap:60px">
	<ex-markdown id="requirements" label="Requirements">
		<script type="text/markdown">
			## Requirements

			State the outcome and boundary.
		</script>
	</ex-markdown>
	<ex-mermaid id="design" label="Design">
		<script type="text/plain">
			flowchart TD
			  Requirements --> Design --> Plan
		</script>
	</ex-mermaid>
	<ex-arrow from="requirements" to="design" label="informs"></ex-arrow>
</div>
```

Markdown Mermaid fences create the same Mermaid island as explicit tags. A `text` fence preserves source literally. Inert source blocks preserve HTML/Markdown whitespace; generated diagram islands use inert templates internally. Mermaid `src` references are resolved by packaging from the supplied corpus.

Markdown, Mermaid, callouts, decisions and other content islands automatically provide small circular sidebar and centered-view controls; no opt-in attribute is needed. `ex-zoom` is the generic wrapper for ordinary HTML or compositions. It keeps natural inline layout and opens a larger inspector with magnification controls. Optional direct children with `slot="details"` are hidden inline and revealed by the inspector. Content is moved and restored with its original place reserved; it is not duplicated.

Diagrams are centered and use natural height by default; content is never cropped into a preview automatically. Scrollable Markdown code blocks and tables own their overflow. If a layout needs a bounded diagram viewport, explicitly give that island `style="max-height:420px;overflow:auto"`.

Add `<ex-nav></ex-nav>` once for a slim left heading rail. It reads the document's actual `h1`–`h6`, including rendered Markdown, follows the active tab, assigns missing anchors and marks the current section. On wide screens the outline stays visible; on narrower screens hover a marker or click the list icon to browse it. A hovered marker highlights its matching outline row; click a marker or heading to jump. No second table of contents or routing setup is needed. By default it reads the first `ex-document` or `main`; `for="content-id"` selects another container. Ordinary relative links inside the component provide page navigation:

```html
<ex-nav>
	<a href="./architecture.html">Architecture</a>
	<a href="./implementation.html">Implementation</a>
</ex-nav>
```

Phosphor is the shared icon vocabulary. The library includes the regular weight from pinned `@phosphor-icons/web@2.1.2`; agents can use `<i class="ph ph-arrow-right" aria-hidden="true"></i>` without importing another library. Shared inspector controls use expand, sidebar, magnification, reset and close icons with accessible labels and tooltips. Live documents reuse the local WOFF2 asset; standalone packages embed it. Icons need no JavaScript or persisted settings. The full regular weight supports arbitrary authored icons; its measured asset size is shown separately from authoring usage in the report.

`ex-arrow` connects declared HTML IDs. Optional `from-anchor` and `to-anchor` choose `left`, `right`, `top` or `bottom`; otherwise the renderer selects facing edges. Connector geometry updates on scroll, resize and island rendering. It never changes authored positions. Arrow overlays ignore pointer input.

`planning.html` is the complete equal-content comparison input. Layout CSS/attributes are authored usage and count as such. Renderer code, shared styles, fonts and packaging are excluded. `layouts-demo.html`, `composition-demo.html` and `capabilities.html` are extra demonstrations outside the five-way totals.

Light DOM preserves normal HTML/CSS composition and inherited typography. Fonts and themes are local. Native dialogs provide modal and side-panel inspection. Semantic graph colors are separate from contributor/workflow colors. Known boundaries: Mermaid uses beautiful-mermaid's supported subset; the diagram resolver recognizes the supplied corpus; a whiteboard's positions are authored CSS; direct file/WebView behavior remains unverified under the host browser policy.

After building the library once, `pnpm --filter @sunstack/demistify-poc islands:package` packages revised usage documents with the existing runtime. The full lab build refreshes the comparison report and all five outputs.
