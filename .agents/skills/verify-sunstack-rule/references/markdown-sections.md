# Markdown anchors and section fields

For the additive 0.15.1 own-heading and enclosing-heading fields, use [the heading-label journey](heading-text.md). Its derived raw values and heading-element spans differ from the raw full/own fields below; ordinary first_line remains a physical source-line operation.

For 0.15.0 and later, Markdown heading fragments require `evidence.markdown.heading_anchors: "github"`. Without it, heading targets are named unsupported/incomplete; explicit HTML IDs still work. Use [the config/default journey](config-only.md) for current-target proof and the exact restore config. The drivers and observations below retain their historical implicit-scheme contract; do not treat them as unchanged current-target proof.

This route drives the signed CLI against every original Markdown fragment link in two fixed repositories and checks real Markdown section fields against an independent outline. Expected values come from the 0.14.0 Markdown-sections commission and Corrections 1–9. Capture, collector controls, the real-link and section journeys, admission and owned teardown have been exercised through the signed CLI. The Correction 8 image-heading and ordinary raw-asking controls have been exercised on the corrected signed candidate; repeat them with the corpus on each declared target. Repeat the scoped drive on each declared target; source and GitHub checks alone prove no engine behavior.

## Inputs and independent expectations

The fixed corpus is ShravanSunder/sunstack (ai-tools) at `cde8d5cf5b9141a08264cc5d57e3e3a58f0f2aac` and ShravanSunder/sunstack-tools at `b060be3ff57f22163ae4348761e51a223fab2255`. `markdown/links.json` contains 264 actual links with original UTF8 byte ranges, raw text, source hashes, decoded targets and expected states. All are inline links; none needs reference resolution or entity decoding. This denominator supersedes the initial rough grep count. ai-tools has 168 (165 found, 2 absent headings, 1 missing file); tools has 96 (94 found, 2 absent headings).

The independent inventory used remark-parse 11.0.0 with remark-gfm 4.0.1, leading front matter masked, and official [github-slugger 2.0.0](https://github.com/Flet/github-slugger/tree/3461c4350868329c8530904d170358bca1d31448). Its index.js lowercases, removes the generated Unicode regex characters, replaces each literal space with a hyphen, and resolves collisions against all prior slugs, including literal suffixed headings. The package emulates GitHub and supplies no Markdown renderer. Its 78 fixtures are sequential stateful cases, not separate reset calls. Rendered heading text retains code/link text and drops emphasis syntax. Native proof separately covers inline rendering, setext, ignored code/comments/front matter, collisions and explicit HTML id/a-name anchors. Never regenerate expectations from engine output.

`markdown/sections.json` fixes 68 real sections: skill-design SKILL.md and humanizer.md in ai-tools, plus the long engine program-design.md in tools. Humanizer includes two real quote headings. Another 6 owned sections include Unicode, a list-scoped child and a quote heading extending past its quote. These expectations use original positions from a different parser, inspected against the adopted low-level contract.

## Prepare and drive

From the monorepo root, use existing Node/Zod dependencies. Both Git object pins must already exist. No install, build or credentials are implied. Capture only named fixed blobs to a new outside directory:

```sh
node scripts/verify/sunstack-rule/markdown/capture.ts \
  --sunstack <ai-tools-git-repo> --tools <tools-git-repo> --output <new-corpus-directory>
node scripts/verify/sunstack-rule/markdown/anchors.ts \
  --binary <immutable-signed-candidate> --source-root <declared-checkout> \
  --commit <full-head> --sha256 <binary-digest> \
  --input-root <corpus-directory> --evidence <new-outside-directory>
```

Capture reads immutable Git objects and verifies required file hashes. The observer checks source/binary identity and signature, copies original files, and requires each rulebook's `lint --check` before `lint --all --format json` or `--evidence section`. Existing bounded children and an owned listener run with credential storage disabled. Original inputs are rechecked after teardown. Keep raw stdout/stderr, commands/exits, owners, source identities, counts, harness receipt and pass.json.

The link matcher selects actual inline block and pipe_table_cell AST nodes containing the fixed authored link literals; ast-grep exposes Markdown’s block grammar, while its inline parser is private. A block can contain multiple links and one result row. Every block must match its original raw span and source hash; every original link span inside it must have exactly one target with the expected URL/path/fragment, existence and found/missing status. All 264 link occurrences must be accounted for exactly once. Fixed-corpus collection found no extra occurrences of these literals in code or other non-link source. A rule author needs `any: [{kind: inline}, {kind: pipe_table_cell}]` to reach prose and table links. `kind: inline` alone omits table cells; the exposed block grammar has no generic link node. The focused collector control requires one prose link and one table link while excluding an identical fenced-code lookalike. Incomplete/unsupported for existing Markdown targets fails. Missing files remain distinct from missing anchors. Markdown URLs percent-decode UTF8 paths/fragments without entity decoding; native controls cover this beyond the ordinary real URLs. Correction 7 routes `foo.md#x` as a path under PathOrSkill by testing the extension before the fragment; it no longer treats the whole string as a skill name. The focused native uses_path regression must fail on the old whole-string ends_with logic and preserve skill-name and explicit-Skill controls. The real-repository CLI journey uses resolve:path; do not present it as a separate PathOrSkill CLI observation.

## Sections and admission

Full is the existing low-level Markdown section range. Only list items clamp a heading's range; quote headings are excluded as closing candidates but are not clamped to the quote's end. Own removes every other section range contained inside full, concatenating retained source segments with multiple exact spans. Enclosing_own is own of the nearest strictly containing lower-level section; top-level is raw Absent. This differs from HTML's lexical outline and changes no existing Markdown selector range.

Every section's complete raw range, full/own/enclosing text, retained UTF8 spans and source identity must match. The observer checks 68 real sections and 6 controls; apparatus sensitivity rejects duplicates, lost post-list prose and a quote-end clamp. Ordinary fields admit mixed and paragraph-only units. Section fields require exactly `["section"]`; mixed/paragraph section fields and call_site must fail with a named admission diagnostic. Low-level CallSite remains unchanged.

## Actual GitHub rendered IDs

With a separate keyless public-network read grant:

```sh
node scripts/verify/sunstack-rule/markdown/github-samples.ts --evidence <new-sample-directory>
```

The script fetches pinned contents using `Accept: application/vnd.github.html+json` and no credential headers. Twelve found links must have actual user-content permalink IDs. Four missing anchors must be absent in existing rendered pages; the fifth missing link must return 404 and is labelled a file-level miss. Correction 4 permits this 4+1 distribution; no synthetic missing substitutes. Preserve rendered HTML plus URL, HTTP status, headers and hashes. A disagreement goes to Lead before dependent work; HTTP 200 alone proves no anchor.

## Image alt text and heading anchors

Correction 8 excludes image alt text from heading IDs while preserving surrounding whitespace, including images nested in links. Ordinary Markdown asking still includes alt text. `image-headings.md` pins the exact first H1 from [tdewolff/minify README](https://github.com/tdewolff/minify/blob/eaad54db7f2205ed37db251a460cec7fd5f82127/README.md), followed by authored linked-image, image-only duplicate and plain headings. The full pinned README SHA256 is `27703cbf2b0d13f7fe5ff3582a58648ab069c684cce39c412c0a5ac1acdf8dc5`; the combined fixture SHA256 is checked in the driver. Actual keyless GitHub HTML supplies the first H1 text `"Minify    "` and permalink `minify----`.

The image control checks eight exact target states: the Minify permalink, `before--after` for a linked image, empty and `-1` image-only slugs, plain text unchanged, and three rejected alt-inclusive slugs, including the Minify phantom. Its separate prepared ordinary AST-remainder asking request must retain the complete raw `# Before [![Linked badge](badge.svg)](https://example.com) After` line and trailing newline, including the image alt text, in both request state and original field. This route preserves raw Markdown; the native prose-projection guard separately keeps rendered alt text. An actual released 0.13.0 comparison established this boundary by exact saved-report bytes except the root version. Correction 9 restores this Remainder/AST-field oracle; join same-fixture previous/corrected-candidate raw report preservation separately, retaining configuration paths and other saved evidence. Do not normalize away report differences or infer prose projection for raw fields. It makes no provider request. Re-run the complete 264-link corpus and 74-section journey on the new signed candidate; old-binary results are historical for this correction. Join the failing-first native image regressions and renewed saved-report parity so the anchor fix cannot silently change ordinary asking projection.

## Native, parity and limits

Join permanent real-CLI tests and observed reds at the declared target, all 78 upstream fixtures, native suite, fmt/clippy and signed artifact provenance. Check matcher exclusivity, unit-only and section-field restrictions at the public boundary. List Rust API changes for the SDK caller. The HTML anchor control uses an existing .txt target for unsupported fragments; .md targets now belong to this route and resolve normally.

Correction 6 adopts Rust std lowercasing with the pinned slugger 2.0.0 removal regex. GitHub's runtime Unicode version is not observable; recent Unicode case mappings may differ from Rust and the Node oracle. The scalar comparison records those differences as a named known limit; it does not relax the 78 fixtures, real-link comparisons or rendered-ID checks. Any real-repository discrepancy still stops for Lead disposition.

Proof 3 also requires 0.13.0 comparisons for an HTML config and a Markdown config without .md fragments or markdown_units: exact saved bytes except engine version. Reuse only with matching dependency/source identities. Generic JSON normalization or omitted request state cannot stand in for parity. Name missing native/parity/signature joins. This route proves no model quality, provider, browser behavior, CI, other platform or release acceptance.
