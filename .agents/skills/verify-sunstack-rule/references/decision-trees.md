# Decision trees: reports and case runs

Use the Specification, Program Design and Verification Design at `2fea5950c4055c37f7541e1d7d0d00f928c25047`, under `docs/specs/2026-11-06-decision-trees-everywhere/`. O4's filter recipe is in [rulebook-lint.md](rulebook-lint.md#filters-tree-routes-and-shared-gates-o4). Preparation proves no product behavior.

## Entry and readiness

Node drives the declared real `sunstack` binary from the monorepo root. RulebookProcess controls only Jev replies with an ephemeral loopback endpoint and synthetic credentials. It isolates HOME, disables credential storage, checks source/hash around each child, preserves process/HTTP receipts, bounds completion and owns teardown. The signed binary, full source commit, config and client must be declared before observation.

Read the selected driver's `--help`, then prepare in fresh authorized directories:

```sh
node scripts/verify/sunstack-rule/rulebook/rulebook-report.ts --prepare --evidence <new-directory>
node scripts/verify/sunstack-rule/rulebook/rulebook-cases.ts --prepare --evidence <another-directory>
```

Observation omits `--prepare`, adding `--binary <absolute> --source-root <checkout> --commit <full-head> --sha256 <digest>`. Require keyless configuration checking with zero requests, then authored positive results through lint. An answering port or binary version alone is not readiness. Stop if source or binary changes.

The scripted routes are operable. Obtain the declared candidate before each run, then retain the results for that target. Preparing fixtures or checking source alone never counts as observing the product.

## O5: short output and saved detail

The fixture supplies pass, warning, unsure, excluded, filter skip and incomplete paths. Human warning/unsure/skip/gap lines must have location and ID without request text; sentinels distinguish copied evidence from authored messages. Run default output with `--report` outside the immutable fixture. Compare saved exact questions/state/answers/models with captured requests and controlled replies; compare full config/Markdown/rules content and hashes with original files, and routes/bars with the authored trees.

Drive `sunstack lint show <report> <id>` for each required result kind. Require one selected entry with its related detail, not the whole report, and zero HTTP requests. Unknown IDs fail and name the valid range. Change/delete only fixture rulebook files, then explain the same results from the saved file. Excluded rules must be marked not re-walkable because no rule request was made.

This catches copied section text, altered/missing saved requests, requested model substituted for responding model, lost unused answers/tree/bars, show returning every entry and explanation depending on current files. The keyed ai-tools run stays separate Lead-owned evidence.

The generated locator checks `locator_context`, a field absent from authored questions. Require that field in the sole rule request and saved request, with the locator's question beside the authored verdict. A separate filter fixture declares a long opening used only by mechanical contains and AST-match steps. Require one filter request that omits the opening, a saved raw opening containing its middle control token, a cut projection without that token, and the collected AST matches under the filter ID in `piece_evidence`. `lint show` must retain that evidence and identical detail after deleting its source context, match rule and rulebook files. Raw data and matches reconstruct the mechanical route; a completed model answer alone cannot.

## O6: cases through lint

Isolated Git revisions hold a whole small repository with its config in `configuration/skill-rules.config.json`. The ordinary comparison archives initialize fresh Git metadata only inside those disposable trees, preserving the normal repository root for nested config; they copy no real repository metadata and use no worktree-add path. The historical revision has a skill reference, folder filter context and a loaded file outside checked paths. The current revision changes all three. Captured requests distinguish historical context from today's files. Match a rule result or filter-skip location only when its file and both first/last lines exactly equal the case range; wider and partial ranges are unmatched. Hash exactly the archived case lines. Normal lint records each filter result's `reads_sha256` (compact JSON of collected unprojected declared field values, lexical keys, without reference/cut metadata/folder). Filter cases compare that hash. Rule cases hash exactly their own archived source lines with line endings retained; a rule result's full-piece `text_sha256` is provenance. `--evidence` is not used to widen context collection. Recorded bars explicitly name `for`, `step`, `bound`, `value` and `set_from`.

Cover warn/pass/must-pass, right/wrong filter labels, unsure/stale/unmatched/excluded, historical context and changed bars. Retain the legacy `ambiguous` ID with its two paragraphs on lines 1 and 3: its spanning [1,3] case must now be `no_matching_piece`, with no result link or bars, in mixed, changed-bar and always-unsure runs. Genuine ambiguity is covered at the native pure-matching layer by `exact_range_matching_keeps_duplicate_exact_results_ambiguous`: one exact child plus a wider parent selects the child; two results with that same exact range remain ambiguous with no selected ID. Inspect that test and reuse a passing receipt only when its source and target still apply. This is an exact-duplicate witness, not a claim that the live extractor emitted duplicates. Every ID occurs exactly once in a scored outcome or listed disposition; assert identities and exact counts. Excluded must-pass does not meet its obligation. Compare lint/cases on identical text; ordinary lint bytes before/after stay identical. Changed bars are unassessed. An always-unsure planted rule fails both must-warn and must-pass.

Case labels/hashes/spans stay out of model requests. Authored reference examples can contain their own labels; compare exact state instead of banning the word `expected`. Keep case output/requests, revision/hash identity, before/after bytes, changed-bar and always-unsure evidence. Cases write nothing lint reads.

An unrelated checked folder remains in both complete archives. Compare case runs with ordinary lint on authored selected rule files and checked members of selected filter folders, never `--all`. A held-out-only selected rule case has a two-request limit: one filter plus one rule; all development cases remain unselected. Require exact HTTP equality with direct named-path lint and no unrelated-folder request. Repeat with the same result and unchanged threshold but held-out, different-owner and unknown `set_from` IDs; only the affected bar must be unassessed with a reason. Valid development provenance stays assessed. A known unsure filter-skipped rule case must retain its actual Choice bar and probability, while unmatched/ambiguous cases have no invented crossing.

Selected case files/folders that are historical symlinks or traverse one must be listed as `not_judged`, reason `selected path is a symlink`, with no result link or bars (Lead disposition `01a1b392-b51f-74f3-917d-bd8b703fdcf7`). The O6 symlink helper commits a leaf file alias, an ancestor directory alias and a filter-folder alias to its own fixture root; every target stays within that owned repository/archive. Alias-only selection must send zero HTTP requests. Normal-only and mixed selections must retain the regular case's exact request bodies, lint results and counts; aliases remain listed without being scored. Context loads through symlinks keep their existing behavior. These controls do not authorize following aliases to host paths.

This catches missing cases, scoring stale/ambiguous cases, excluded cases counted as success, reading current historical dependencies, missing external loads, stored results gating lint and always-unsure satisfying must-pass.

## Native rows and limits

O1–O3/O10 require offered tests and raw runs at the declared candidate. Check each scenario/assertion against the VD and name the wrong behavior it rejects. O1 covers five reserved state keys, reference paths, every primitive's missing route, old/undeclared filter terminals, only_when labels and plain instructions naming no fields. O2 covers unchanged reference, unavailable fields, skill openings, location omission and all questions/line-finder in one request. For unavailable fields, keep the boundaries explicit: an actual request can show an empty map; nonempty named metadata is checked from collected evidence through the state builder, with the normal walker refusing before HTTP and naming the unavailable field. Do not dispatch missing named text just to obtain a capture. O3 covers the full adopted branch/cut/unresolved/unavailable matrix. O10 includes unsure plus error reporting failed.

Read tests before reusing runs: a workspace pass alone is not coverage. Product/test fixes belong to the Implementer; disputed expectations go to the Lead. Scripted proof leaves judgment quality, hosted behavior, ai-tools adoption and release publication unverified. Keep evidence after owned teardown and report each O row as observed or not observed with its named gap.
