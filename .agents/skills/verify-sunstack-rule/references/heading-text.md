# Heading labels from native sections

Use `scripts/verify/sunstack-rule/heading-text/heading-text.ts` for the 0.15.1 heading-text contract and its additive compatibility boundary. Expected outcomes come from the heading-text commission and Corrections 1–5, including the consumer's P3 ask. Read the script's `--help` for its two phases. No provider request, install, build or release action is implied. Baseline and candidate phases have exercised this recipe through the signed CLI. Repeat the scoped drive on each declared target; prior coverage does not establish a later target.

## What the field means

`{ "from": "section", "text": "heading" }` gives the native section's own heading label. `{ "from": "enclosing", "kind": "outline", "text": "heading" }` gives labels for the referenced native outline pieces in the existing containment order. The reference must be a section-only `html_units` or `markdown_units` kind. Containment includes an equal-range section from a separately named outline kind, so a path can include the candidate's own heading; this change introduces no strict-ancestor filtering. An empty path requires no matching referenced piece.

HTML labels are the h1–h6 element's asking projection: tags removed, entities decoded once and whitespace folded, with the current project attribute settings. The resulting visible text is both the raw field value and asking value. It must not be parsed again as HTML: `&lt;T&gt;` becomes literal `<T>`, and `&amp;lt;` becomes literal `&lt;`. Markdown labels retain inline source syntax and entities, removing only leading ATX markers, optional closing ATX markers or a setext underline. Thus `# A *bold* [link](x)` gives `A *bold* [link](x)`, for raw and asking alike. This is a different contract from rendered Markdown heading-anchor slugs.

Spans point at the original heading element, rather than the whole section or its first physical source line. A derived field value need not equal that source slice. Ordinary full/own/enclosing_own fields and `first_line` keep their meanings. Common text budgets and cut rules still apply. An unclosed recovered HTML heading whose range is only the opening tag gives the new heading field a named unavailable value; it does not guess where the text ends or change legacy outline recovery.

Admission must refuse heading text on a non-outline enclosing source, on `same_file`, or inside a kind reached through `target … as_kind` nesting. The last case remains unsupported because the projector has no nested kind definitions; there is no new public projector signature. The added `SectionText::Heading` and `ContextText::Heading` variants still require the SDK's exhaustive-match review cut.

## Frozen inputs and route

Use existing Node/Zod dependencies and an immutable signed released 0.15.0 binary with official provenance, plus the declared candidate's full source head and signed binary digest. `real-corpus.json` pins the input files and config hashes. Supply `--input-root/{html,markdown,j1,ai-tools}`: P3's actual captured five rulebook files plus 13 HTML pages at `5c02896631399f04216fbdc30668cdeb0497be43`, ten Markdown pages at `1500127d185ffdee461e6fb860842e2d0f1af1f0`, the actual judged J1 source/config, and ai-tools at `cde8d5cf5b9141a08264cc5d57e3e3a58f0f2aac`. A missing file or mismatched digest is a prerequisite gap, not permission to regenerate expectations from the candidate.

The P3 config/rulebook lives outside its checked root, as in the consumer run. Putting `std.md` inside that root makes its `**/*.md` target glob select the rulebook and adds two unintended requests. The driver copies the pages and external rulebooks to separate owned locations, explicitly supplies `--root`, and restores the three 0.15.0 convention settings for both subjects. Candidate P3 configs change only the four `heading_path.text` values from `first_line` to `heading`.

```sh
node scripts/verify/sunstack-rule/heading-text/heading-text.ts \
  --phase baseline --baseline-binary <released-0.15.0> --baseline-sha256 <digest> \
  --source-root <checkout> --input-root <frozen-inputs> \
  --common <new-owned-common-directory> --evidence <new-baseline-evidence>

node scripts/verify/sunstack-rule/heading-text/heading-text.ts \
  --phase candidate --baseline-binary <same-release> --baseline-sha256 <digest> \
  --binary <signed-candidate> --source-root <declared-checkout> \
  --commit <full-head> --sha256 <candidate-digest> \
  --input-root <same-frozen-inputs> --common <same-owned-common-directory> \
  --baseline-run <baseline-evidence> --evidence <new-candidate-evidence>
```

Every drive first requires `lint --check`. Both subjects use the same absolute project/config/output paths. ai-tools prepares 499 requests and the restored J1 config prepares 177; their entire saved reports must match released 0.15.0 except the root engine version. No config/hash/raw-state exception applies to this additive comparison.

## Deciding the P3 observation

P3 prepares selection requests, with no settled `piece_evidence` rows. Read `selection_evidence.candidates`, their original fields/spans and selection views, and the exact `request.state.candidates` sent by each prepared request. Join by kind/file/source identity/full span. Local c1 keys and request batch counts can move when heading paths shrink. Reject duplicate, lost or additional candidates. Each request must have an exact bijection between serialized candidate keys and its descriptors, and every serialized candidate field must equal its offered selection view. Shared state must equal the offered shared view and its where.file must match the source; shared inputs stay unchanged between subjects. A recomputed hash alone cannot establish these joins.

The independent source oracle contains 177 closed HTML heading elements and 118 Markdown headings. HTML used a separate Python HTMLParser pass over the frozen source; Markdown inline values were derived by marker-only source parsing, and element spans were source-checked against the released binary's generic ATX/setext AST query before candidate observation. No new heading-field output supplies either oracle. Check all 354 HTML candidates and 820 path values, plus 236 Markdown candidates and 570 values. Every new raw heading value, heading span, model-view field and requested path must match the source oracle in the original enclosing order. All other raw fields and immutable descriptors remain exact. Uncut unrelated projected fields remain exact; changed generic budget cuts require the separate native budget join.

The consumer's “88” and this inventory have different denominators. Correction 5 adopts the wider source inventory: 190 changed scalar HTML path values, 95 S1 plus 95 S2, across 130 candidate paths and 39 heading sources on pages 06/07/11/12. Six old values carry first-line projection cut markers; report their named old state and the bounded heading result separately. Check the unaffected nine pages too. Never relabel 190 as 88 or replace source expectations with observed candidate output.

Own-heading controls also drive ordinary rule requests. One HTML line contains adjacent headings, inline markup, escaped angle text, standard alt/aria-label attributes and a generic configured h1 title attribute; ATX optional closing markers, CRLF, inline Markdown and setext get a separate control. Raw heading values must equal asking values, original element spans must be exact, and each path must retain equal-range inclusion. Driver mutation tests reject raw changes, incorrect spans/path values, arbitrary hashes, duplicates and legacy report changes. Native proof separately covers budgets, missing references, unbounded headings and all admission refusals; read the actual offered tests and failures before joining them.

## Evidence and teardown

Children run serially through the existing bounded process harness with credential storage disabled and an owned loopback listener. Stop if free disk falls below 6 GiB. A timeout permits exactly one unchanged isolated retry, preserving both raw receipts; an isolated pass is host load and a repeated failure is a finding. No timeout or expectation edits.

Keep release/source/artifact provenance, source oracle tables, configs, complete prepared reports, exact commands/exits/stdout/stderr, input before/after guards, account.json, captured.json and the final listener harness.json. The driver closes its listener and rechecks frozen inputs even on failure. Read decisive raw receipts before handback. A captured.json is a capture account; native/schema/admission and SDK joins remain separate. Provider quality, paid P3 scoring, browser rendering, CI, other platforms and release acceptance are unobserved by this route.
