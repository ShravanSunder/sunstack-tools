# HTML heading sections and asking text

This route checks the signed Sunstack CLI's lexical HTML sections and complete asking projection. The well-formed launch/probes/cleanup have been exercised; the added C7 recovery controls are draft until a corrected declared-target pass. Repeat the scoped journeys on each target. Expected outcomes come from the 0.13.0 HTML-sections commission and its Corrections1–7, including the J1 consumer gap; the current output never supplies the oracle.

## What a section means

Each h1–h6 starts a section ending before the next same-or-higher heading in document order, or EOF. A directly enclosing section is the nearest earlier lower-level heading still enclosing that range. DOM wrappers, tabs and details do not override this lexical ownership. Template/script/comment headings and Markdown-island source headings do not enter the outline; island source remains text in its block.

Rulebook kinds select it with exactly one matcher: `html_units: {mode: "unit", units: ["section"]}` instead of `rule` or `inline_rule`. `select` retains its model-selection meaning. Filters use starts_with or pattern under the selector, matching projected visible text rather than raw HTML tags; emitted section spans and fields stay raw. Unit mode only; call_site is refused. Low-level EvidenceSelection::HtmlUnits and this matcher share one outline owner.

Fields use `{from: "section", text: "full" | "own" | "enclosing_own"}` on outline kinds only. Full is the section's complete range; own excludes child sections. Top-level enclosing_own is raw Absent and normal judged input, never an unavailable failure. Existing enclosing/remainder and non-HTML fields keep their meanings.

## Original inputs and launch

Supply frozen regular files flat.html, wrapped.html and mixed.html outside any checkout being written. The three corpus-*.json files beside the driver pin their hashes, source-relative paths, source commits where available, and independent heading/range/parent expectations. They contain no product-generated output. The expectations were counted with a separate HTMLParser source pass and byte-offset calculation, then inspected against the pages:23flat/36wrapped/55mixed sections, including H1.

Flat is codex-router's docs/specs/2026-11-06-central-execution-model/program-design.html at c1d68fc2736cc41b083b3e2e2d7b9d245e7c50a1. Mixed is agentstudio's docs/specs/2026-10-15-agentstudio-daemon/explainer/features.html at a3c9457389046df9a9fe1e0171c3bb842575844d. Wrapped is the explicitly approved byte snapshot of codex-router workspace-storage requirements.html, not a claimed committed revision; it must match its pinned SHA. A missing original is a setup gap, never permission to regenerate expectations from the engine.

From the monorepo root, with its existing Node/Zod dependencies:

```sh
node scripts/verify/sunstack-rule/html-sections/sections.ts \
  --binary <immutable-signed-candidate> --source-root <checkout> \
  --commit <full-head> --sha256 <digest> \
  --input-root <frozen-three-file-corpus> --evidence <new-outside-directory>
```

The declared head must be clean and artifact provenance/source reuse explained. Signature and version prove identity; each owned rulebook's real --check proves readiness. No build, install or provider key is implied by loading this recipe. Children disable credential storage and use the existing bounded process/owned loopback harness.

## Deciding observations

The driver copies the original bytes to owned projects, then runs `lint --all --evidence section`. Every real section must match its independent original UTF8 byte range, full/own field text and spans, source identity, and direct parent's own range; counts alone cannot pass. It verifies original corpus hashes again after cleanup.

Owned controls exercise skipped levels through H6, duplicate heading text, excluded fake headings, EOF and mismatched DOM nesting. A parent has7prose blocks followed by a labelled Mermaid diagram: its own asking text and the child's enclosing text must both include that diagram. Top-level fields remain null in the asking state, with no unavailable-field failure and all7requests prepared normally. Original HTML stays in raw pieces and fields.

A small max_chars fixture must actually truncate fields and carry every model-view cut into the saved request's cut-fields. A byte-acquisition refusal is separately Incomplete/3 with no prepared request, not a partial no-picture judgment. The legacy non-outline HTML asking fixture must project `A B`, retain `[alt: …]`, `[label: …]` on ex-mermaid and `[aria-label: …]` at their positions, decode entities, and preserve raw mechanical text.

Keep raw commands/exits/stdout/stderr, saved reports, exact pins, original guards, owner joins and the final harness receipt. A pass is written only after owned cleanup and original recheck succeed. HTTP0 refers to the owned listener and report model_requests0; it is not global egress proof.

## Recovered heading openings

C7 recognizes h1–h6 opening tags even when missing end tags put them under recovered/ERROR parser nodes. The same lexical ranges, direct parents and excluded template/ex-markdown scopes apply. A heading that cannot be placed must produce a named incomplete result rather than empty success. The control module recovery.ts owns eight explicit two-heading cases: both unclosed, newline neighbours, each direction of mixed closed/unclosed, siblings, proper markup, Unicode prefix and recovered headings in excluded blocks. It checks all16raw full/own/enclosing spans plus16prepared requests, not merely a successful exit. Prepared full/own/enclosing text is exact, including recovered tag-boundary spaces (Parent Child instead of ParentChild), Unicode and normal top-level null. Empty Prepared/0 and duplicate owners are rejected. The original114-section corpus remains fixed and is rerun with these controls to detect changes to ordinary pages.

## Every HTML asking path

Correction5 requires one evidence-owned project_kind_model_view for ordinary rule, selection candidate/shared, and filter asking views. The driver prepares separate AST-based HTML selection and filter projects, so a working html_units/rule path cannot hide either bypass. Both use8copies of the independently authored projection sample:1320raw characters versus591projected characters, under an admitted1000-character view budget. The expected text is fixed before a run. Selection candidate and shared fields must both be fully projected, and the filter state must be fully projected; retained candidate/shared/filter raw fields must remain the complete1320characters. This also catches cutting raw markup before projection or reusing an ordinary view that charged shared and candidate fields together. No selection answer or provider dispatch is needed for these saved prepared requests. The filter fixture uses a mechanical dependent rule to keep its only asking request the classification. Prepare can also enqueue gated asking rules; the separate gate journey covers that contract.

## Native and parity joins

Check the offered permanent tests and their wrong behaviors before reuse. Join the low-level and public rulebook routes to the same collector; reject multiple matcher keys, unsupported modes/units/filters, empty-matching regexes and section fields on non-outline kinds. Inspect typed schema/Serde acceptance and refusal, actual spans and cuts, not snapshots alone. Mechanical tools must keep raw HTML and Markdown behavior must remain unchanged.

The Implementer's Proof4 must compare ordinary Markdown --all --prepare with released0.12.1 by exact saved bytes except engine version. Legacy HTML without html_units must retain all other behavior; rule, selection and filter asking-projection differences and their derived request identities must be inspected explicitly, never removed by general JSON normalization. C4 keeps AST rule identity separate from native kind identity; C5 centralizes projection at the EvidenceProject boundary; C6 exposes ask_filters(plans, folders, evidence: FilterEvidence, default_max_chars, dispatcher), borrowing pieces, kinds, project and matches as one evidence snapshot. No callback, synthetic rule identity or compatibility shim is used. Inspect both changes in the public SDK signature list and actual consumer tests. Existing html-evidence's R1 asking expectation also needs the documented boundary spaces; its fixed source is independent of product output.

Return supported/unsupported/unknown for Proof1–3, the native/parity joins and every uncovered path. This route proves no model judgment quality, hosted provider, browser rendering, CI, other platform or release acceptance. Keep current evidence in the task home, outside this recipe.
