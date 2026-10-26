# Pictures and screenshot galleries

Use a plain `<figure><img src="…" alt="…"><figcaption>…</figcaption></figure>` for one picture. Clicking its image opens the shared fullscreen inspector with its caption. An image inside a link retains that link's action; slider comparisons and Gallery retain their own controls.

Use `ex-gallery` on its own screen when readers need to browse screenshots, compare viewport variants or filter evidence. An explanation uses an ordinary figure or links to that Gallery page. A Gallery is the browsable picture collection; a Gallery Journey is an optional ordered path through its existing figures. The document still owns Pages, outline, theme and walkthrough selection. The agent supplies captures; do not add upload/Add/Info/lab application chrome.

Put one Gallery directly inside `ex-document`, optionally preceded by one native h1, with `ex-nav` and hidden path declarations outside that document. Keep surrounding explanation on the linking page. The matching authoring checker reports `gallery-page` for embedded or multiple galleries; inert template data does not count as displayed content. Existing runtime support for embedded galleries is distinct from this authoring standard. Earlier checkers, including 1.10.0, do not enforce it; bind checks to the matching release.

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

Review is the default: a scrollable grouped thumbnail pane beside the selected image. The list icon to the left of search toggles thumbnails and gives the viewer their space; it remains reachable by keyboard and touch. Collapse/expand uses a short shared transition; reduced-motion preference removes that transition. Keep the same control in both cases rather than adding author animation. This visibility choice is retained across Review/Grid, inspection copies and Gallery Journeys. The grid icon switches the same gallery to a contact sheet; the review icon returns with selection and filters retained. `view="grid"` chooses the initial view; `selected="menu-laptop"` chooses an initial figure.

In Review, the selected picture heading shows its title followed by quiet authored tags, for example **Queue · #Monitoring**. Set `data-title="Queue"` on the figure for its concise title; otherwise the image's alt text supplies it, while `data-tags="Monitoring, Needs attention"` supplies the tag labels. Selection replaces the title and tags together in Review; tags from the previous picture must not linger. The fullscreen heading uses the picture title without those tag labels. Keep alt text useful for the picture itself.

The arrows-out button is **View screenshot fullscreen**. Clicking a picture in Review or Grid opens that same large shared dialog with its authored caption. Its arrow keys keep the gallery’s navigation: Left/Right changes logical screens, and Up/Down changes the current screen’s variations. The dialog stays open while picture, title and caption change together. Escape or the close action returns to the current gallery selection, filters and view. Gallery screenshots do not open a second inspection sidebar; ordinary standalone pictures can still use the document’s inspector.

Keep the three controls distinct: the gallery’s list icon shows or hides screenshot thumbnails; Pages navigates between HTML documents; the document outline navigates headings. Do not add a second navigation engine or build another gallery around the same pictures.

## Facets and captions

For a gallery page that fills available space, put one `ex-gallery` directly in `ex-document`, optionally preceded by one h1. No sizing CSS or flag is needed. Include the normal `ex-nav` construct for Pages and the Walkthroughs launcher; hiding gallery thumbnails does not hide those document controls. Move other explanation, sections and disclosures to its linking page rather than converting the Gallery into a nested application. Short windows keep usable content and normal page overflow; grid rows retain their picture/caption size and scroll within the island.

Use tags for useful content groupings, such as `data-tags="Account, Journey"`. Tags become the default filter choices when present. Screen and viewport metadata group navigation; they are not repeated as automatic picture filters. The keyboard icon opens shortcut help; Review/Grid use one named button group. Search uses available width and has a clear action through the same settled query pipeline.

Authors can explicitly request other facets using `filters="tags release"` with `data-release="Candidate"`. For a custom field label/icon, supply a small declaration:

```html
<script type="application/json" data-gallery-filters>
[{"field":"tags","label":"Tags","icon":"file"},
 {"field":"release","label":"Release","icon":"package"}]
</script>
```

`field` names the `data-…` suffix; values come from the figures, not a second option list. Icons use the library's curated subject names. Within a facet, selections are OR; across facets and search, they are AND. Search/filter changes settle after a shared 200ms debounce. Reset and direct navigation are immediate. An unchanged selected image keeps its DOM node and zoom.

A caption is native HTML or a Markdown island inside `figcaption`. Use normal paragraphs, emphasis, links, lists or code as the evidence needs; headings stay out of the page outline. Keep it about the picture rather than rebuilding a second viewer or application. A Markdown-file caption follows the main guide's live/embedded source rules. Images remain ordinary relative paths or HTTPS URLs, including in Markdown captions.

Left/Right moves between logical screens/states and preserves the viewport category when available. Up/Down changes variations of the current screen/state; without a matching variation it stays put. The dedicated Gallery and its fullscreen screenshot keep the same arrow navigation. Editing fields, popovers and other inspectors retain their own keys. Fit contains the whole picture; Fill covers the viewer, with minus/plus for manual zoom. The toolbar omits numeric percentages. Cmd/Ctrl+wheel zooms; modifier-drag pans. Normal scrolling remains native.

For a captioned user journey, target the existing figure IDs from `ex-presentation`/`ex-step`; see [ordered walkthroughs](tours.md). The existing player floats beneath the gallery with reserved space. Add the `variations` flag only when readers should switch sizes through a button group or Up/Down. Without it, each authored step uses its exact capture. A missing variation stays on its step and displays the gap. Closing restores the gallery's previous browse state.

For two same-size pictures, `ex-compare slider` overlays its two labeled children with a draggable divider. Use it instead of a gallery when before/after reveal is the whole task. For rich alternatives, shared topic rows or a larger set with pair selection, use [content comparisons](comparisons.md); that form supports any number of alternative sections.

Complete by decoding every image, checking actual facet-to-figure matches, switching review/grid, using both key directions, and comparing the authored caption in the gallery and fullscreen dialog, then closing to confirm selection and filters remain. Change the selected picture in Review and confirm title/tags update without stale labels; in fullscreen, confirm the picture title and caption update. Check the thumbnail show/hide action separately from Pages and heading navigation, including its short transition and its immediate reduced-motion behavior. When browser access prevents those observations, return that exact gap. A missing image, invented variant, lost selection or per-gallery navigation/theme fork is a failed result.
