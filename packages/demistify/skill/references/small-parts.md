# Compact labels, exact counts and phase breakdowns

Use small parts when a compact comparison helps the reader scan facts. When the requested row combines identity, status, checks and comparable phase measurements, use one native table for the complete combination: authored identity/avatar, status badge, exact checks count centered in its ring with the subject beside it, total duration and named phase bars. Keep status separate from count and timing. A complete count does not imply success. Ordinary prose and values remain useful when a visual comparison adds nothing.

## C: combine identity, checks and timing

Keep the supplied fields together: person and workspace, run and identifier, authored state, exact checks count, duration and timing. The header names the shared phase order, and the caption states the share scale once. A compact timing cell contains only the bar and one line of exact values. Its accessible description retains the observation, scale, phase names and values. Shared colors keep each phase recognizable across views.

```html
<div class="ex-table-scroll" data-width="canvas">
  <table>
    <caption>Illustrative assessment runs. Each bar shows the phase shares of one run; values follow Connect / Work / Transfer.</caption>
    <thead>
      <tr>
        <th scope="col">Identity</th>
        <th scope="col">Run</th>
        <th scope="col">State</th>
        <th scope="col">Checks</th>
        <th scope="col">Duration</th>
        <th scope="col">Connect / Work / Transfer</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td><span class="ex-identity"><span class="ex-avatar" aria-hidden="true">AP</span><span class="ex-identity-text"><strong>Asha Patel</strong><span class="ex-identity-detail">North workspace</span></span></span></td>
        <td><strong>Document import</strong><br><small>A-148</small></td>
        <td><ex-badge tone="done">Complete</ex-badge></td>
        <td><ex-progress label="Checks" value="6" max="6"></ex-progress></td>
        <td>228 ms</td>
        <td><ex-breakdown label="Document import phases" unit="ms" compact>
          <data value="28" series="1">Connect</data>
          <data value="156" series="2">Work</data>
          <data value="44" series="3">Transfer</data>
        </ex-breakdown></td>
      </tr>
      <tr>
        <td><span class="ex-identity"><span class="ex-avatar" aria-hidden="true">NC</span><span class="ex-identity-text"><strong>Noor Chen</strong><span class="ex-identity-detail">North workspace</span></span></span></td>
        <td><strong>Address assessment</strong><br><small>A-149</small></td>
        <td><ex-badge tone="note">Running</ex-badge></td>
        <td><ex-progress label="Checks" value="2" max="6"></ex-progress></td>
        <td>371 ms</td>
        <td><ex-breakdown label="Address assessment phases" unit="ms" compact>
          <data value="42" series="1">Connect</data>
          <data value="264" series="2">Work</data>
          <data value="65" series="3">Transfer</data>
        </ex-breakdown></td>
      </tr>
      <tr>
        <td><span class="ex-identity"><span class="ex-avatar" aria-hidden="true">LD</span><span class="ex-identity-text"><strong>Luis Diaz</strong><span class="ex-identity-detail">West workspace</span></span></span></td>
        <td><strong>Photo review</strong><br><small>A-150</small></td>
        <td><ex-badge tone="risk">Failed</ex-badge></td>
        <td><ex-progress label="Checks" value="1" max="6"></ex-progress></td>
        <td>512 ms</td>
        <td><ex-breakdown label="Photo review phases" unit="ms" compact>
          <data value="34" series="1">Connect</data>
          <data value="418" series="2">Work</data>
          <data value="60" series="3">Transfer</data>
        </ex-breakdown></td>
      </tr>
      <tr>
        <td><span class="ex-identity"><span class="ex-avatar" aria-hidden="true">AP</span><span class="ex-identity-text"><strong>Asha Patel</strong><span class="ex-identity-detail">North workspace</span></span></span></td>
        <td><strong>Identity lookup</strong><br><small>A-151</small></td>
        <td><ex-badge tone="done">Complete</ex-badge></td>
        <td><ex-progress label="Checks" value="6" max="6"></ex-progress></td>
        <td>96 ms</td>
        <td><ex-breakdown label="Identity lookup phases" unit="ms" compact>
          <data value="18" series="1">Connect</data>
          <data value="55" series="2">Work</data>
          <data value="23" series="3">Transfer</data>
        </ex-breakdown></td>
      </tr>
      <tr>
        <td><span class="ex-identity"><span class="ex-avatar" aria-hidden="true">NC</span><span class="ex-identity-text"><strong>Noor Chen</strong><span class="ex-identity-detail">West workspace</span></span></span></td>
        <td><strong>Delivery reconciliation</strong><br><small>A-152</small></td>
        <td><ex-badge>Queued</ex-badge></td>
        <td><ex-progress label="Checks" value="0" max="6"></ex-progress></td>
        <td>0 ms</td>
        <td><span>Not started</span></td>
      </tr>
      <tr>
        <td><span class="ex-identity"><span class="ex-avatar" aria-hidden="true">LD</span><span class="ex-identity-text"><strong>Luis Diaz</strong><span class="ex-identity-detail">West workspace</span></span></span></td>
        <td><strong>A deliberately long document classification label</strong><br><small>A-153</small></td>
        <td><ex-badge tone="note">Running</ex-badge></td>
        <td><ex-progress label="Checks" value="4" max="6"></ex-progress></td>
        <td>1240 ms</td>
        <td><ex-breakdown label="A deliberately long document classification label phases" unit="ms" compact>
          <data value="120" series="1">Connect</data>
          <data value="830" series="2">Work</data>
          <data value="290" series="3">Transfer</data>
        </ex-breakdown></td>
      </tr>
    </tbody>
  </table>
</div>
```

The initials, names, workspace, run, status, count and duration are supplied author content; the renderer does not infer identity or outcome. The bars use the default `scale="share"`, so they compare phase composition without implying equal total durations. Each row keeps Connect in slot 1, Work in slot 2 and Transfer in slot 3. The header supplies the shared visible names and order while each compact breakdown retains exact values alongside its bar. Keep the repeated run title and share explanation out of each cell. Compare a promoted recipe with its selected experiment using the same records and fields before claiming it matches. Use the library's shared styles; do not add page-specific paint.

## Identity in a cell, sentence or Trace

Use the shared native classes for an authored identity. Supply the initials, complete readable name and detail yourself; do not infer initials or fetch an avatar. The shared avatar is a 36px circle. The initials are decorative and hidden from assistive technology, while the name and detail remain ordinary text. The same composition works in a table cell, prose or a Trace step and uses library styling without page-owned paint.

```html
<span class="ex-identity">
  <span class="ex-avatar" aria-hidden="true">AP</span>
  <span class="ex-identity-text">
    <strong>Asha Patel</strong>
    <span class="ex-identity-detail">North workspace</span>
  </span>
</span>
```

When there is no meaningful phase comparison, leave the bars out. For example, `6 / 6 checks; 228 ms` in a sentence already communicates the supplied facts. Do not add a breakdown to decorate ordinary prose.

## Short status labels

`ex-badge` is a normal-text label that fits beside the fact it names. It has no required tone, so neutral wording remains neutral. An optional `tone` uses the existing semantic tone vocabulary. IF the authored label needs a semantic tone, load `cards-and-callouts.md` and return the existing tone that expresses that supplied meaning. That reference also owns the separate `.chip` class.

```html
<p>The request was <ex-badge tone="risk">Delayed</ex-badge> after reconnect.</p>
<p><ex-badge>Observation supplied</ex-badge> The follow-up is not classified.</p>
```

Use a badge for a compact status or category label. In the combined row, `done` uses the shared green added-state paint, `note` uses the accent family, and `risk` uses its existing risk paint. Keep a known queued state neutral by omitting `tone`. These appearance rules apply to `ex-badge`; other components keep their own tone styles. Keep a sentence in prose, and do not use badge color to stand in for a count or phase category.

## Exact counts and the selected ring

`ex-progress` names the subject with required `label`. Supply both `value` and `max` for a reported count. They are non-negative whole numbers for the same subject. In the combined row, the exact fraction is centered inside its ring, which is at least 40px and grows to fit longer counts; the subject follows beside it. The progress fill uses the shared accent, while phase bars keep their separate blue/amber/teal slots.

```html
<ex-progress label="Checks" value="2" max="6"></ex-progress>
```

Use the ring when the selected row compares progress alongside phase composition. The visible count retains both authored integers; a full ring says `6/6 Checks`, not that the work succeeded. Pair it with a separate status only when that status is known. If a progress ring was explicitly requested, keep it even when valid fractions are equal. Use ordinary count text for unhelpful fractions only when no progress visual was requested.

A zero count with a positive total keeps the neutral track and has no colored fill; positive counts retain their supplied ratio. A missing `value` is **Not reported**, whether or not `max` is present. Do not turn a missing count into zero. `0/0` has no measurable denominator and is shown as an empty total without a ring or percentage. A count greater than its total, a negative value or a fractional value is invalid; it remains visible as authored and has no ring. Correct the source rather than clamping it.

## Phase values, share and elapsed scale

`ex-breakdown` compares one to three direct native `<data>` children. The parent requires `label` and `unit`. Each phase needs a non-empty unique name, a finite non-negative `value` and a unique `series` from `1`, `2` or `3`.

```html
<ex-breakdown label="Request phases" unit="ms">
  <data value="28" series="1">Connect</data>
  <data value="156" series="2">Work</data>
  <data value="44" series="3">Transfer</data>
</ex-breakdown>
```

With the default `scale="share"`, the bar compares each phase to the sum on a labelled 100% scale. This example reports `28 ms`, `156 ms` and `44 ms`; their total is `228 ms`. The bar shows their composition, not their elapsed lengths against an unstated maximum.

Use `scale="elapsed"` only when every comparison uses an explicit, common, positive `max`. The sum must not exceed it:

```html
<ex-breakdown label="Request phases" unit="ms" scale="elapsed" max="1240">
  <data value="28" series="1">Connect</data>
  <data value="156" series="2">Work</data>
  <data value="44" series="3">Transfer</data>
</ex-breakdown>
```

This keeps the `228 ms` total visible against `1240 ms`. It does not stretch the phases to fill the bar. Do not compare elapsed bars with different maxima as if their lengths shared one scale.

A zero sum is not missing data. Share zero is labelled as a zero total; elapsed zero retains its supplied maximum, such as `0/1240 ms`. Neither draws a nonzero segment. Missing phases, invalid numbers or slots, duplicate names/slots, an invalid elapsed maximum and a total above that maximum produce an unavailable result. The checker reports those authoring errors; the runtime keeps authored values in visible fallback text without drawing a misleading bar.

A phase keeps its chosen `series` when its position changes. Slot 1 is blue, slot 2 amber and slot 3 teal in both light and dark themes. The shared theme supplies those colors to bars and their keys. Keep phase names, exact values and the table's shared order available as text; color never carries the only meaning.

A normal breakdown shows its own named phase key. Use that in prose or a rich Trace attachment where each phase name should sit beside its value. In a compact share table, let the table header provide the shared phase names and order and state the share scale once in its caption or introduction. Positive compact shares omit repeated visible titles and scale text; elapsed, zero and invalid results retain their explicit explanations. The bar has soft outside corners and straight internal joins. Let the native table wrapper own narrow scrolling, and let Trace own its disclosure and spacing. Do not add a wide outer card or change document Reading/Canvas widths for these parts.

## Check and observe

1. Check each finished HTML file with the matching library source:

   ```sh
   node packages/demistify/bin/demistify.ts check /absolute/path/to/page.html
   ```

   Require a non-zero file count and zero errors and warnings. Fix malformed input; do not present an intentionally invalid example as a working recipe.
2. Open the actual artifact with its supported route. Observe badge text and wrapping, exact counts and durations, phase order and values, the optional progress view where used, local table scrolling, and the relevant light/dark themes at a narrow width.
3. Keep supplied facts visible. The checker proves recognized markup and values, not that the surrounding explanation is true or that the rendered result is useful.
