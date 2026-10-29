# Pictures and screenshot galleries

Use a plain `<figure><img src="…" alt="…"><figcaption>…</figcaption></figure>` for one picture. Clicking its image opens the shared fullscreen inspector with its caption. An image inside a link retains that link's action; slider comparisons and Gallery retain their own controls.

Use `ex-gallery` on its own screen when readers need to browse screenshots, compare viewport variants or filter evidence. An explanation uses an ordinary figure or links to that Gallery page. A Gallery is the browsable picture collection; a Gallery Journey is an optional ordered path through its existing figures. The document still owns Pages, outline, theme and walkthrough selection. The agent supplies captures; do not add upload/Add/Info/lab application chrome.

Put one Gallery directly inside `ex-document`, optionally preceded by one native h1, with `ex-nav` and hidden path declarations outside that document. Keep surrounding explanation on the linking page. The matching authoring checker reports `gallery-page` for embedded or multiple galleries; inert template data does not count as displayed content. Existing runtime support for embedded galleries is distinct from this authoring standard. Earlier checkers, including 1.10.0, do not enforce it; bind checks to the matching release.

## Choose the item, its variations and useful filters

Start with what the reader moves between and what they compare while staying on one item. Different subjects or meaningful UI states are separate items. Alternate captures of that same subject/state belong together as variations. A filter narrows the collection; it does not establish that relationship. Use a plain figure for one capture, a Gallery for browsing related captures, and a Gallery Journey when their explanation has an order.

The existing gallery expresses this with three fields:

| Meaning | Author | Example |
| --- | --- | --- |
| Logical subject | Same `data-screen` for the same screen | `Cart`, then `Payment` |
| Meaningful UI state | Same `data-state` within a comparison group | `Ready` versus a separate `Error` item |
| Capture variation | `data-viewport` category within that screen/state | `Phone`, `Tablet`, `Desktop` |

Inspect supplied captures before assigning these fields. Category names are authored, not a fixed enum or inferred from image width. Use consistent labels supported by the captures; a category called Phone does not prove a physical phone was used. Keep unique figure IDs, one actual capture per screen/state/category for an unambiguous guided variation, and related figures adjacent. Two URLs for one image are delivery alternatives, not two variations.

### Example: compare screen sizes without repeating the journey

This example assumes the supplied collection has Cart and Payment in their Ready state at three sizes. It represents **two logical items, each with three variations**. Keep the Gallery on its own page; the navigation and hidden journey declaration sit outside `ex-document`.

```html
<ex-nav for="content"></ex-nav>
<ex-document id="content">
  <h1>Checkout captures</h1>
  <ex-gallery id="checkout-captures" label="Checkout screen sizes">
    <figure id="cart-phone" data-screen="Cart" data-state="Ready" data-viewport="Phone">
      <img src="captures/cart-phone.png" alt="Cart at phone size">
      <figcaption>Cart, ready to continue, at phone size.</figcaption>
    </figure>
    <figure id="cart-tablet" data-screen="Cart" data-state="Ready" data-viewport="Tablet">
      <img src="captures/cart-tablet.png" alt="Cart at tablet size">
      <figcaption>Cart, ready to continue, at tablet size.</figcaption>
    </figure>
    <figure id="cart-desktop" data-screen="Cart" data-state="Ready" data-viewport="Desktop">
      <img src="captures/cart-desktop.png" alt="Cart at desktop size">
      <figcaption>Cart, ready to continue, at desktop size.</figcaption>
    </figure>
    <figure id="payment-phone" data-screen="Payment" data-state="Ready" data-viewport="Phone">
      <img src="captures/payment-phone.png" alt="Payment at phone size">
      <figcaption>Payment, ready to continue, at phone size.</figcaption>
    </figure>
    <figure id="payment-tablet" data-screen="Payment" data-state="Ready" data-viewport="Tablet">
      <img src="captures/payment-tablet.png" alt="Payment at tablet size">
      <figcaption>Payment, ready to continue, at tablet size.</figcaption>
    </figure>
    <figure id="payment-desktop" data-screen="Payment" data-state="Ready" data-viewport="Desktop">
      <img src="captures/payment-desktop.png" alt="Payment at desktop size">
      <figcaption>Payment, ready to continue, at desktop size.</figcaption>
    </figure>
  </ex-gallery>
</ex-document>
<ex-presentation id="checkout-review" title="Review checkout"
  description="Compare Cart and Payment across the supplied sizes." variations>
  <ex-step to="cart-phone">Review the cart.</ex-step>
  <ex-step to="payment-phone">Review payment.</ex-step>
</ex-presentation>
```

Left/Right moves between Cart and Payment; Up/Down compares the current screen's size captures. The journey has two steps, one per screen/state, rather than six size-specific steps. Bare `variations` belongs on `ex-presentation`, not `ex-gallery`: it adds the journey's category buttons and Up/Down switching. A plain Gallery already browses its authored variations without that flag.

Good: Cart/Ready keeps the same group while Phone, Tablet and Desktop change. Bad: `data-screen="Cart — Phone"` and `data-screen="Cart — Desktop"` split one comparison into unrelated items, or a Phone filter is offered as the only way to compare them. An explicitly requested viewport facet can still filter the collection; it serves a different purpose.

For an exact evidence sequence, omit `variations` and target the exact figure IDs. For a size-switching journey, include it and target one existing figure per logical step. In that guided mode a missing or ambiguous screen/state/category stays on the step and shows a gap; do not invent a capture or silently substitute one. Ordinary Gallery navigation preserves the size category when available and otherwise uses its existing browse fallback. `tours.md` owns the remaining path and return behavior.

Review is the default: a scrollable grouped thumbnail pane beside the selected image. The list icon to the left of search toggles thumbnails and gives the viewer their space; it remains reachable by keyboard and touch. Collapse/expand uses a short shared transition; reduced-motion preference removes that transition. Keep the same control in both cases rather than adding author animation. This visibility choice is retained across Review/Grid, inspection copies and Gallery Journeys. The grid icon switches the same gallery to a contact sheet; the review icon returns with selection and filters retained. `view="grid"` chooses the initial view; `selected="cart-desktop"` chooses an initial figure.

In Review, the selected picture heading shows its title followed by quiet authored tags, for example **Queue · #Monitoring**. Set `data-title="Queue"` on the figure for its concise title; otherwise the image's alt text supplies it, while `data-tags="Monitoring, Needs attention"` supplies the tag labels. Selection replaces the title and tags together in Review; tags from the previous picture must not linger. The fullscreen heading uses the picture title without those tag labels. Keep alt text useful for the picture itself.

The arrows-out button is **View screenshot fullscreen**. Clicking a picture in Review or Grid opens that same large shared dialog with its authored caption. Its arrow keys keep the gallery’s navigation: Left/Right changes logical screens at the current variation, and Up/Down walks variations continuously across screen groups. The dialog stays open while picture, title and caption change together. Escape or the close action returns to the current gallery selection, filters and view. Gallery screenshots do not open a second inspection sidebar; ordinary standalone pictures can still use the document’s inspector.

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

Left/Right moves between logical screens/states and preserves the viewport category when available. Up/Down walks the visible captures group by group: `1a → 1b → 1c → 2a → 2b → 2c`. Down from a group’s last variation goes to the next group’s first; Up reverses that sequence. Navigation stops at the first/last capture of the filtered gallery. The dedicated Gallery and its fullscreen screenshot keep the same arrow navigation. Editing fields, popovers and other inspectors retain their own keys. Fit contains the whole picture; Fill covers the viewer, with minus/plus for manual zoom. The toolbar omits numeric percentages. Cmd/Ctrl+wheel zooms; modifier-drag pans. Normal scrolling remains native.

For a captioned user journey, target the existing figure IDs from `ex-presentation`/`ex-step`; see [ordered walkthroughs](tours.md). The existing player floats beneath the gallery with reserved space. Add the `variations` flag only when readers should switch sizes through a button group or Up/Down. Without it, each authored step uses its exact capture. A missing variation stays on its step and displays the gap. Closing restores the gallery's previous browse state.

For two same-size pictures, `ex-compare slider` overlays its two labeled children with a draggable divider. Use it instead of a gallery when before/after reveal is the whole task. For rich alternatives, shared topic rows or a larger set with pair selection, use [content comparisons](comparisons.md); that form supports any number of alternative sections.

Preserve supplied image paths, capture metadata and captions. Use the main guide's lint pass; Gallery controls, keyboard navigation, transitions and state retention are library regression responsibilities. Inspect only an explicitly requested view or reported defect.
