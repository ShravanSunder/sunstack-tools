# Within one or more kinds: amended U14/U16 and W6

This route follows amended R30/R40 and U14/U16 at `9dfdb10` plus `a4696af` (WK1/WK2), the engine's `within-kinds-impl-commission.md` W1–W6 and the Lead's amendments. U14, U16 and W6 were observed locally at `ef968f91`, including exact per-file counters, present-null refusal and owning-key locations. The earlier `93fff155` location/null counterexamples remain recorded in the target's report. The expected containment is inclusive union over final pieces, with each match counted once. `within` accepts a nonempty list of distinct kind names in config order; only omission means no containment. Product code, Rust tests, canonical repo fixtures, exported schema and release notes have their own owners.

Use the main skill's source/binary pinning and synthetic-only policy. The lasting rulebook entries and their `--case selection` mode keep their names and flags:

```sh
node scripts/verify/sunstack-rule/rulebook-lint.ts --case selection \
  --binary <frozen-binary> --sha256 <digest> --source-root <checkout> \
  --commit <full-head> --evidence <new-directory>
node scripts/verify/sunstack-rule/rulebook-check-refusals.ts --case selection \
  --binary <frozen-binary> --sha256 <digest> --source-root <checkout> \
  --commit <full-head> --evidence <another-new-directory>
```

The Implementer's canonical selection fixture must already use one-item lists at the declared target. There is no automatic scalar-to-list compatibility adapter in the current-product journey. The sole intentionally authored scalar in the lasting refusal inventory is the old-form rejection case. Baseline scalar generation in the W6 comparator below is exclusively for the old executable.

## Union and once-only observation

`within-kinds-journey.ts`, called by rulebook lint selection mode, adds a small controlled skill and a reference with `## Standards`. The `sentence` kind names `['step', 'standards_section']`; that order also differs from alphabetical order, so its keyless `--check` listing must retain it exactly.

The skill file has exactly four sentence-rule matches: a step-only item, a Standards-only paragraph, an item inside both, and an outside paragraph. A separate reference Standards item is the no-chosen-step-file control. Yes answers give three skill pieces and three picked pieces; no answers give two, while the raw match count stays four. The reference is always matches/pieces/picked 1/1/1. Both evidence and lint JSON must have those exact per-file `file_counts`, taken from fixture facts rather than derived from the returned piece list. `within-kinds-journey.test.ts` and retained-artifact sensitivity require a counter-only corruption to fail while the correct pieces stay identical.

Exact file/byte-start records and source-range readback exclude outside text and duplicate overlap. The overlap contributes one piece and one mechanical lint result. Skill steps change from two to zero on no, dropping only the step-only guidance; both skill Standards pieces and the reference piece remain. Four total mechanical results become three. These controls reject first-container-only, intersection, duplicate overlap, raw unchosen-step behavior and counters that drift while pieces remain correct.

## Check-only refusals

Selection refusal mode retains canonical R40 cases and adds six isolated configs: scalar `within`, present null, empty list, repeated kind, undefined second entry and a cycle present only through the second entry. Each must exit 2, emit a related config/file/line diagnostic for its actual cause, dispatch nothing and need no key. A separate omitted-within control must succeed; clean listing includes the one-item list, and the union fixture lists both kinds in config order. Null must reach a non-list decode refusal rather than silently becoming omission. The cycle diagnostic names its path's kinds.

For the cycle and undefined-container fixtures, the diagnostic must locate the owning evidence-kind key, with the exact authored line and column. An earlier matching string value or a nested field with the same name is not that key. These controlled two-space fixtures put kind keys at six-space indentation; the assertion checks that fixture invariant before comparing the location. Comparator tests and retained c0 wrong-location readback reject the earlier-value defect that the original generic location-presence check missed.

Join the direct native constructor/decode/schema controls (including `minItems: 1` and `uniqueItems: true`), every-entry dependency/build-order controls, and the structural keyless/no-checked-source `check` route. A located refusal caused by an unrelated invalid field is insufficient. The binary journey does not replace the native schema or plan proof.

## W6 one-item equivalence

The standalone driver is lasting apparatus for the cutover comparison:

```sh
node scripts/verify/sunstack-rule/within-kinds-equivalence.ts \
  --baseline <frozen-v0.3.0-binary> --baseline-sha256 <baseline-digest> \
  --binary <new-frozen-binary> --sha256 <new-digest> \
  --source-root <checkout> --commit <full-head> --evidence <fresh-directory>
```

It requires baseline version `sunstack 0.3.0` and both declared hashes. It copies the canonical rulebook selection fixture to one physical path and uses one owned loopback endpoint with fixed selected/no/uncertain answers. The reference executable gets the equivalent scalar config generated from the current one-item list; the new executable gets only the list. There is no product compatibility shim or runtime fallback.

For each case, compare every serialized byte of `pieces`, `selections` and `file_counts`, retaining all nested fields, ranges and order; also compare exact ordered selection request bodies. No path, field or timing normalization is applied to those fields. Exits are 0/0/5 for selected/no/uncertain, respectively. A nonempty request and selection record must prove the fixture reached the model seam. A deliberately corrupted first piece's `where.first_line` must fail the same comparator, with the altered report and rejection receipt preserved. Passing equality without that negative control is incomplete W6 proof.

Keep config variants, exact argv/hash/exit/stdout/stderr and raw requests, the successful equality receipt and planted failure. Children have the existing bounded lifecycle and held stdin; stop only owned child groups on timeout, close the loopback and verify both target hashes/source pins through the run. Remote model quality, real-key calls and release/install behavior remain unobserved. R42 on configs without `within` remains a separate compatibility contract.

Return U14, U16 and W6 separately as supported/unsupported/unknown on the declared target, with native joins, raw evidence and any apparatus or product gap. No prepared fixture or unit-only comparator check is a live product pass. The Lead owns acceptance.
