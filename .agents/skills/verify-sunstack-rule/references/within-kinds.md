# Within one or more kinds: amended U14/U16 and W6

This route follows the amended R30/R40 and U14/U16 rows at design `9dfdb10`, plus the engine's `within-kinds-impl-commission.md` W1–W6. It is draft until driven on the declared frozen within-kinds target. The expected containment is inclusive union over final pieces, with each match counted once. `within` accepts a nonempty list of distinct kind names in config order; the old scalar form is refused. Product code, Rust tests, canonical repo fixtures, exported schema and release notes have their own owners.

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

When the skill's workflow candidates are answered yes, exact file/byte-start records require a step-only item, an item under Standards that also is a chosen step, and a reference Standards item whose workflow candidate is answered no. Paragraphs outside both containers must be absent. The overlap contributes one piece and one mechanical lint result. When the skill's candidates are answered no, step-only guidance disappears while both Standards items remain. Exact source-range readback, step-state controls and lint result file/line/count comparisons reject first-container-only, intersection, duplicate overlap and raw unchosen-step behavior. The reference file exercises the second container when no selected step exists there.

## Check-only refusals

Selection refusal mode retains the canonical R40 cases and adds five isolated configs: scalar `within`, empty list, repeated kind, undefined second entry and a cycle present only through the second entry. Each must exit 2, emit a related config/file/line diagnostic for its actual cause, dispatch nothing and need no key. The cycle diagnostic must name its path's kinds. Clean listing includes the one-item list, and the union fixture lists both kinds in config order.

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
