# Pictures and screenshot galleries

Use a plain `<figure><img src="…" alt="…"><figcaption>…</figcaption></figure>` for one picture. It opens through the shared inspector beside the text; fullscreen uses the same dialog as other islands.

Use `ex-gallery` when readers need to scroll screenshots, compare viewport variants or filter evidence. It can be one island among prose or the main island on a page. The document still owns Pages, outline, theme and presentations. The agent supplies captures; do not add upload/Add/Info/lab application chrome.

## Group the actual evidence

Inspect the supplied captures first. `data-screen` groups related screens, `data-viewport` names an authored size/category, and `data-state` describes a UI state. Laptop/Mobile/Widescreen are useful starting labels, not a fixed enum. Use labels supported by the actual captures; a wide image alone does not prove a laptop identity. Two URLs for the same capture are delivery alternatives, not different viewport evidence.

Keep related figures adjacent and their category names consistent:

```html
<ex-gallery id="evidence" label="Account menu evidence">
  <figure id="menu-mobile" data-screen="Account menu" data-viewport="Mobile" data-state="Open">
    <img src="captures/menu-mobile.png" alt="Account menu on Mobile">
    <figcaption>The account action remains reachable.</figcaption>
  </figure>
  <figure id="menu-laptop" data-screen="Account menu" data-viewport="Laptop" data-state="Open">
    <img src="captures/menu-laptop.png" alt="Account menu on Laptop">
    <figcaption><strong>Open state.</strong> The same account actions at the wider size.</figcaption>
  </figure>
</ex-gallery>
```

Review is the default: a scrollable grouped thumbnail pane beside the selected image. The grid icon switches the same gallery to a contact sheet; the review icon returns with selection and filters retained. `view="grid"` chooses the initial view; `selected="menu-laptop"` chooses an initial figure.

## Facets and captions

For a gallery page that fills available space, put one `ex-gallery` directly in `ex-document`, optionally preceded by one h1. No sizing CSS or flag is needed. Other prose/sections/disclosures make it an embedded gallery in normal page flow. Short windows keep usable content and normal page overflow; grid rows retain their picture/caption size and scroll within the island.

Viewport choices live in Filters. The viewer has no repeated capture/viewport arrow buttons or category selector. Its keyboard icon opens shortcut help; Review/Grid are one named radio group. Search uses available width and has a clear action through the same settled query pipeline.

Present `screen`, `viewport` and `state` values become default filter options. Use `filters="screen viewport release"` with `data-release="Candidate"` to choose fields. For a custom field label/icon, supply a small declaration:

```html
<script type="application/json" data-gallery-filters>
[{"field":"screen","label":"Screen","icon":"file"},
 {"field":"viewport","label":"Viewport","icon":"terminal"},
 {"field":"release","label":"Release","icon":"package"}]
</script>
```

`field` names the `data-…` suffix; values come from the figures, not a second option list. Icons use the library's curated subject names. Within a facet, selections are OR; across facets and search, they are AND. Search/filter changes settle after a shared 200ms debounce. Reset and direct navigation are immediate. An unchanged selected image keeps its DOM node and zoom.

A caption is native HTML or a Markdown island inside `figcaption`. Use normal paragraphs, emphasis, links, lists or code as the evidence needs; headings stay out of the page outline. Keep it about the picture rather than rebuilding a second viewer or application. A Markdown-file caption follows the main guide's live/embedded source rules. Images remain ordinary relative paths or HTTPS URLs, including in Markdown captions.

Up/Down selects screenshots. Left/Right switches viewport categories and retains the related screen when available; absent variants are not invented. Editing fields keep their normal arrow keys. Fit/100%, zoom, sidebar, fullscreen and original-image actions are divided control groups. Cmd/Ctrl+wheel zooms; modifier-drag pans. Normal scrolling remains native.

`ex-compare` is for two before/after children with `label`; `slider` overlays same-size images with a draggable divider. Use it instead of a gallery when the comparison itself is the whole task.

Complete by decoding every image, checking actual facet-to-figure matches, switching review/grid, using both key directions, and comparing the authored caption in main/sidebar/fullscreen. A missing image, invented variant, lost selection or per-gallery navigation/theme fork is a failed result.
