# github-slugger fixtures

`fixtures.json` and `LICENSE` are copied without modification from
[github-slugger 2.0.0](https://github.com/Flet/github-slugger/tree/3461c4350868329c8530904d170358bca1d31448),
commit `3461c4350868329c8530904d170358bca1d31448` (`test/fixtures.json` and
`LICENSE`). The fixture sequence matters: the upstream test uses one slugger
for all 78 entries. `markdownOverwrite` is upstream rendering metadata and is
not input to the slugger.

`github_slug_ranges.rs` is derived from the same commit's generated `regex.js`,
whose `script/generate-regex.js` uses Unicode 13.0.0. To reproduce, check out
that commit and run:

```sh
node github_heading_slug_test/generate-ranges.mjs /absolute/path/to/github-slugger/regex.js github_slug_ranges.rs
```

The script enumerates every valid Unicode scalar, checks each replacement is
either empty or unchanged (rejecting partial surrogate removal), and coalesces
removed scalars into sorted inclusive ranges. Generation was checked with Node
v26.13.0: 736 ranges and zero partial replacements.

`lowercase-scalars.json` contains each valid Unicode scalar for which
JavaScript `toLowerCase()` changes the input, paired with the result of the
pinned upstream stateless `slug(input)` (equivalent to a fresh slugger for each
input). It was generated with Node v26.13.0, Unicode 18.0: 1,508 entries.
Regenerate from the pinned checkout with:

```sh
node github_heading_slug_test/generate-lowercase-fixtures.mjs /absolute/path/to/github-slugger/index.js github_heading_slug_test/lowercase-scalars.json
```

This compares scalar behavior for those runtime versions; it does not establish
lowercasing equivalence for every multi-scalar context or other runtime versions.

The scalar fixture is a non-gating characterization of runtime differences,
not an equivalence assertion. Rust 1.98.1 and Node v26.13.0 already differ at
U+A7DD: the Rust slug is empty and the Node reference is `ɷ`. The product uses
Rust standard-library lowercasing; a renderer using different Unicode mappings
may produce different slugs for affected headings. Run the characterization
with test output visible to record all mismatches and runtime Unicode versions.
