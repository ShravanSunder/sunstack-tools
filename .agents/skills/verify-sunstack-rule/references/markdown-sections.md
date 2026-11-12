# Markdown anchors and section fields

This route drives the signed CLI against every original Markdown fragment link in two fixed repositories and checks real Markdown section fields against an independent outline. Expected values come from the 0.14.0 Markdown-sections commission and Corrections 1–6. The driver is a draft until its declared-target run completes; source and GitHub checks alone prove no engine behavior.

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

The link matcher selects actual inline block AST nodes containing the fixed authored link literals; ast-grep exposes Markdown’s block grammar, while its inline parser is private. A block can contain multiple links and one result row. Every block must match its original raw span and source hash; every original link span inside it must have exactly one target with the expected URL/path/fragment, existence and found/missing status. All 264 link occurrences must be accounted for exactly once. Fixed-corpus collection found no extra occurrences of these literals in code or other non-link source. Incomplete/unsupported for existing Markdown targets fails. Missing files remain distinct from missing anchors. Markdown URLs percent-decode UTF8 paths/fragments without entity decoding; native controls cover this beyond the ordinary real URLs.

## Sections and admission

Full is the existing low-level Markdown section range. Only list items clamp a heading's range; quote headings are excluded as closing candidates but are not clamped to the quote's end. Own removes every other section range contained inside full, concatenating retained source segments with multiple exact spans. Enclosing_own is own of the nearest strictly containing lower-level section; top-level is raw Absent. This differs from HTML's lexical outline and changes no existing Markdown selector range.

Every section's complete raw range, full/own/enclosing text, retained UTF8 spans and source identity must match. The observer checks 68 real sections and 6 controls; apparatus sensitivity rejects duplicates, lost post-list prose and a quote-end clamp. Ordinary fields admit mixed and paragraph-only units. Section fields require exactly `["section"]`; mixed/paragraph section fields and call_site must fail with a named admission diagnostic. Low-level CallSite remains unchanged.

## Actual GitHub rendered IDs

With a separate keyless public-network read grant:

```sh
node scripts/verify/sunstack-rule/markdown/github-samples.ts --evidence <new-sample-directory>
```

The script fetches pinned contents using `Accept: application/vnd.github.html+json` and no credential headers. Twelve found links must have actual user-content permalink IDs. Four missing anchors must be absent in existing rendered pages; the fifth missing link must return 404 and is labelled a file-level miss. Correction 4 permits this 4+1 distribution; no synthetic missing substitutes. Preserve rendered HTML plus URL, HTTP status, headers and hashes. A disagreement goes to Lead before dependent work; HTTP 200 alone proves no anchor.

## Native, parity and limits

Join permanent real-CLI tests and observed reds at the declared target, all 78 upstream fixtures, native suite, fmt/clippy and signed artifact provenance. Check matcher exclusivity, unit-only and section-field restrictions at the public boundary. List Rust API changes for the SDK caller. The HTML anchor control uses an existing .txt target for unsupported fragments; .md targets now belong to this route and resolve normally.

Correction 6 adopts Rust std lowercasing with the pinned slugger 2.0.0 removal regex. GitHub's runtime Unicode version is not observable; recent Unicode case mappings may differ from Rust and the Node oracle. The scalar comparison records those differences as a named known limit; it does not relax the 78 fixtures, real-link comparisons or rendered-ID checks. Any real-repository discrepancy still stops for Lead disposition.

Proof 3 also requires 0.13.0 comparisons for an HTML config and a Markdown config without .md fragments or markdown_units: exact saved bytes except engine version. Reuse only with matching dependency/source identities. Generic JSON normalization or omitted request state cannot stand in for parity. Name missing native/parity/signature joins. This route proves no model quality, provider, browser behavior, CI, other platform or release acceptance.
