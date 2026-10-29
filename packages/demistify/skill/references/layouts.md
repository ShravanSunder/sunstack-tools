# Widths, grids and tabs

Choose layout from the relationship: document flow for an explanation, a grid for related items read together, tabs for peer alternatives occupying one place. A wide container is not a reason to stretch every paragraph or enlarge its text.

## Reading and Canvas

Reading is 760px; Canvas fills available document room up to 1600px. From 1.10.0 they share one left edge in the room remaining beside the outline/inspector. Earlier centered behavior belongs to earlier bindings. Prose and document headings/dividers stay on Reading even around a wide view. Nested component content follows its local layout; table cells, code and card titles are not separately forced into a centered text column.

Apply `data-width="reading"` or `data-width="canvas"` to the **whole** component or native layout wrapper. A tab set's bar and every panel share that choice. A Canvas wrapper allows visual children to expand while ordinary document prose remains Reading. Narrow windows fit both standards to available room; these are author choices, not reader width settings.

```html
<h2 id="ownership">Who owns recovery?</h2>
<p>The Registry keeps the identity while connections come and go.</p>
<ex-grid cols="2" data-width="canvas">
  <ex-card title="Registry" tone="note" icon="database">
    <p>Retains the session lookup.</p>
  </ex-card>
  <ex-card title="Connection" tone="note" icon="plug">
    <p>Carries messages for the current attachment.</p>
  </ex-card>
</ex-grid>
<p>A lost connection does not itself end the session.</p>
```

The shared layout adds a larger Reading↔Canvas gap on roomy documents and a compact rhythm when narrow. Use that rhythm rather than local width caps, per-page type sizes, hard-coded side margins or extra spacer elements. A lone `ex-stat` remains Reading width. Wide tables scroll locally; use the shared wrapper from `content.md` rather than clipping their columns.

## Grids keep related items together

`ex-grid cols="1"` through `cols="6"` gives the maximum column count; the grid reduces it when narrow and becomes one column on phones. Its direct children may use `span="2"`, `span="3"` or `span="all"`. A direct child with bare `sticky` stays in view beside scrolling content where the layout permits it. Bare `swipe` on the grid makes a sideways row on phones.

```html
<ex-grid cols="3" data-width="canvas">
  <section span="2"><h3>Recovery sequence</h3><p>Reconnect, resolve the original identity, then continue.</p></section>
  <aside sticky><ex-stat value="2" label="Connection attempts" note="One continuing session"></ex-stat></aside>
  <p span="all">The count describes this supplied example, not a runtime measurement.</p>
</ex-grid>
```

Grid children can be ordinary HTML or islands. A grid does not make every item a card, assign reading order, or infer semantic groups. Keep DOM order meaningful when its columns collapse. If a hand-positioned map rather than responsive reading is the purpose, use `maps.md`.

## Tabs switch peer alternatives

Each direct child is a `section` with a concise `label`. Its optional curated `icon` names the subject; absent one, the control uses a neutral file icon. Keep the full explanation inside the panel rather than making the tab label a sentence.

```html
<ex-tabs label="Reconnect explanations" data-width="reading">
  <section id="normal-path" label="Normal path" icon="path">
    <h3>Resolve the original session</h3>
    <p>The new connection resolves the Registry's existing identity.</p>
  </section>
  <section id="missing-path" label="Missing session" icon="database">
    <h3>Report the missing lookup</h3>
    <p>A missing lookup does not prove the execution ended.</p>
  </section>
</ex-tabs>
```

Tabs are selected by click or Arrow/Home/End, with local tab-bar overflow when numerous. A fragment into a hidden panel reveals it. Optional bare `columns` shows peer panels beside each other when they fit and returns to tabs when narrow, retaining the selected view. It is an authored choice, not a reader-facing layout menu. Use supported subject icons; `question` is a tone, not a curated subject icon.

Use `ex-compare` instead when corresponding topics must align across alternatives; use a disclosure for optional depth beneath the current explanation. Keep sequential documents as Pages when that is their actual relationship.

Keep the bar and panels on one width and related controls with their content. Preserve readable DOM order and the shared prose measure; avoid fixed heights or overflow clipping that hide content. For retained interaction in sidebar/fullscreen, load `inspection.md` and return the supported inspection choice and the intended reader behavior.
