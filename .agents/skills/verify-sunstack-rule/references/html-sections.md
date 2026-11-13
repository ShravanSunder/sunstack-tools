# HTML heading sections and asking text

This route checks the signed Sunstack CLI's lexical HTML sections and complete asking projection. Launch, original-input guards, recovered heading/range/request probes and owned cleanup have been exercised. Repeat the scoped journeys on each declared target. Expected outcomes come from the 0.13.0 HTML-sections commission and its Corrections1–7, including the J1 consumer gap; the current output never supplies the oracle.

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

## Malformed asking text (0.14.1)

The 0.14.1 commission and Corrections1–9 replace the deferred opaque-fragment limit. Asking recovers words in ERROR spans, including literal `->` prose and text after stray closing tags. The selector/mechanical helper preserves its earlier behavior. Well-formed script/style text stays as before; script/style/template/ex-markdown/comments are excluded only inside new ERROR recovery. An unsafe unterminated tag preserves the safe prefix, carries `[cut: html_projection_unterminated_markup]`, and marks its containing top-level field in both model-view cuts and request cut-fields. The same cut ownership must hold for nested targets and lists in the native join.

`html-sections/error-text.ts` extends this journey. Its header/`--help` describe the baseline and candidate phases. It requires the existing Node dependencies, a signed released0.14.0 comparator with provenance, a declared signed0.14.1 candidate, frozen original inputs, and a new evidence directory. No build, install, model key or release action is part of this route. The recovery route has been exercised through signed CLI artifacts, including the real malformed page, ampersand preservation, exact parity and safe-prefix cuts. Repeat it on each declared target; source checks alone do not close that observation.

Input tables beside the driver are source-derived oracles. `error-corpus.json` pins13J1 pages from snapshot Git revision5c02896631399f04216fbdc30668cdeb0497be43, all43agreed case line fingerprints, and3real Router pages with ampersand ERROR fragments from46faaabbbadeb623d058b722c0425e3b8c6d10fe. Supply those regular files under `--input-root` with their table-relative names. `error-source.json` records an independent HTMLParser data/entity pass over raw own fields, with raw-field SHA fences; it contains no product asking output. Its word-order scan permits added attribute labels and rejects dropped/reordered source words. It establishes text preservation, never J1 judgment quality. The raw fields used for those ranges remain fenced by the exact released comparison.

`--parity-root` contains the previously frozen original ai-tools Git checkout (cde8d5cf5b9141a08264cc5d57e3e3a58f0f2aac), HTML-sections project with the three corpus pages, and Markdown-anchor project. Keep their existing config/rules/data. The driver copies the HTML project to an owned common path and sets request limits to100000characters/128000input tokens for both subjects, so all114requests prepare without hiding large-page refusals. It never changes the original. J1 uses native sections and own/enclosing asking fields, not the paid J1 questions or verdicts. A single common `--shared-output` keeps config/report absolute paths identical across subjects.

Run the baseline first:

```sh
node scripts/verify/sunstack-rule/html-sections/error-text.ts \
  --phase baseline --baseline-binary <released-0.14.0> --baseline-sha256 <digest> \
  --source-root <candidate-checkout> --input-root <frozen-16-pages> \
  --parity-root <frozen-original-projects> --shared-output <common-owned-path> \
  --evidence <new-baseline-directory>
```

Run candidate with the same inputs/common path and additionally `--phase candidate --binary <signed-artifact> --commit <full-head> --sha256 <digest> --baseline-run <baseline-directory>`, using a new evidence directory. Never drive an undeclared or superseded target.

The deciding checks are:

- All43J1 cases join their original line SHA and exact native owner. Baseline has one own-text loss (j1-dev-006); candidate must preserve every source word in all43. Show the exact loss and recovery rather than treating preparation as a judgment.
- The released parser itself lists ERROR spans on the real requirements page and the three ampersand pages. Host ast-grep is a discovery locator, not parser-version proof.
- Entire J1 and original HTML reports differ only by root version and source-literal restorations on the two pinned versions of the requirements page, plus recomputed SHA256 of those exact requests. Flat/mixed pages and every raw field/span/outline/owner/selection remain exact. `error-parity.ts` holds the source-literal restorations; mutation controls reject raw changes, un-restored payloads and arbitrary hashes. J1 snapshot page03 and the older wrapped page are distinct byte snapshots of the same logical page.
- Markdown-anchor prepared report (127results/3findings/0requests), ai-tools499requests and ampersand asking reports (62title/heading requests plus the real Components & design button on3pages) are byte-identical except root version. Ampersand ERROR listings must also match that boundary. Every one of the10ERROR records (5distinct source spans) must be contained by a prepared owner from the same source identity, and its literal words must reach that request state. A listing-only or model-view-only match cannot pass.
- Literal stray/nested/entity/comment/unsafe/script controls establish text recovery, safe-prefix cuts and unchanged well-formed raw script/style text. CDATA controls preserve terminated inner text (including a literal entity) through the matching `]]>` and name unterminated content as a safe-prefix cut. Script/style slash controls treat the trailing slash on an excluded non-void tag as opening the recovery exclusion, so hidden text stays out. The recovered script/style entity and CDATA probes must suppress every hidden sibling and retain the following visible words. Unterminated CDATA inside an excluded recovery scope must not mark a cut or abort later visible text; the separate unexcluded unsafe CDATA control still requires the named cut. Native tests separately cover missing terminators, nested lists/targets, recovery exclusions, one traversal and selector/mechanical preservation.

Keep exact diff reports, all raw child receipts and parser listings, the43-case audit, comparator/source/artifact identities, original-input guards and final listener receipt. A final `captured.json` only accounts for capture; close native joins, raw evidence review and any named outstanding item before a supported handback. Provider quality, browser rendering, CI and release acceptance remain outside this route.

Once Asking recovery opens a construct, its raw lexical scan owns the field bytes until that construct closes (CDATA `]]>`, matching excluded closing tag, comment end) or the field ends, and skips AST nodes within that consumed range. Selection never advances that cursor. If another actual case breaks this recovery premise, stop and return the evidence to the Lead; the commission reserves a named-incomplete fallback rather than another inferred recovery mechanism.
