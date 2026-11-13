# Structured cut fields in selection state

Use `scripts/verify/sunstack-rule/selection-cuts/selection-cuts.ts` for the 0.16.0 selection-state wire contract. The adopted source is the selection-cut-fields commission and Correction 1. The route uses the real headed P3 inputs from 0.15.1 and checks the complete saved report against released 0.15.1. Its `--help` describes both phases. Candidate coverage remains open until a declared signed target runs it.

## Contract and expected differences

For a bounded selection candidate, `request.state.candidates.<key>["cut-fields"]` is the exact `selection_evidence.candidates[].selection_view.cut` field-name array. `request.state.shared["cut-fields"]` similarly comes from the offered shared view's `cut`. The wire name and array form are the same as rule requests. Empty vectors omit the property; they do not serialize an empty array. Text, inline markers, field values, cut budgets, thresholds and ordering retain their existing behavior.

The vector is authoritative. An authored literal `[cut]` or `[cut: 7 more]` never manufactures cut metadata. Existing recovery cuts also follow the same bounded-view vector; do not infer a different field list from a marker scan. On the fixed P3 corpus, generated marker fields and cut vectors agree exactly and both cut/uncut counts are required.

Adding real wire bytes changes a cut-bearing request hash. A saved selection record whose input hash no longer matches is not reused by `--selections-from`; matching uncut requests remain eligible under the existing source/model replay checks. A version change alone does not reject every previous answer. Inspect the exact native replay tests and changelog before handback.

Correction 1 keeps all request size/token limits unchanged and counts the added bytes. At an exact-fit boundary, only mechanically necessary rebatching or RequestTooLarge caused by those bytes is permitted. No limit increase or metadata exclusion. A native boundary control closes this case. The fixed real P3 comparison stays strict; if it crosses a boundary, preserve the failure and return it before widening that comparison.

## Frozen inputs and readiness

Use existing Node/Zod dependencies, a verified official released 0.15.1 artifact, and the declared candidate's full source head and immutable signed binary SHA-256. No build, install, provider key or publication is implied.

`selection-cuts/real-corpus.json` holds the expected denominators and points to the existing `heading-text/real-corpus.json` source manifest. Supply the same frozen `--input-root/{html,markdown,j1,ai-tools}` files as the heading-text route. The P3 snapshots are 13 HTML pages at `5c02896631399f04216fbdc30668cdeb0497be43` and ten Markdown pages at `1500127d185ffdee461e6fb860842e2d0f1af1f0`, plus their captured rulebooks. J1 uses the actual judged rulebook/pages; ai-tools is pinned to `cde8d5cf5b9141a08264cc5d57e3e3a58f0f2aac`. A hash mismatch or missing source is a prerequisite gap.

The driver reuses the pinned input loader. It explicitly restores the three 0.15.0 convention settings and switches the four P3 heading_path fields to the 0.15.1 heading mode for both subjects. P3 config/rulebook remains outside the checked root; each CLI invocation supplies `--root`. Original inputs are copied to a new owned common directory, never edited. Keep that directory between phases so absolute config/project/output paths stay identical. The old heading-text observer is pinned to the pre-cut-field wire; use this route for current P3 report parity.

```sh
node scripts/verify/sunstack-rule/selection-cuts/selection-cuts.ts \
  --phase baseline --baseline-binary <released-0.15.1> --baseline-sha256 <digest> \
  --source-root <checkout> --input-root <frozen-inputs> \
  --common <new-owned-common-directory> --evidence <new-baseline-evidence>

node scripts/verify/sunstack-rule/selection-cuts/selection-cuts.ts \
  --phase candidate --baseline-binary <same-release> --baseline-sha256 <digest> \
  --binary <signed-candidate> --source-root <declared-checkout> \
  --commit <full-head> --sha256 <candidate-digest> \
  --input-root <same-frozen-inputs> --common <same-owned-common-directory> \
  --baseline-run <baseline-evidence> --evidence <new-candidate-evidence>
```

## Deciding observations

Every project first passes actual `lint --check`, then `--all --prepare`. Prepared exit6 with model_requests0 is expected. The fixed P3 denominator is 354 HTML candidates:18cut and336uncut, in74requests; 236 Markdown candidates:12cut and224uncut, in49requests. Shared views have zero cuts in all123 serialized request occurrences; these are46 logical shared views. Count each denominator explicitly. The known cut fields are full, and generated inline markers remain unchanged.

For each selection request, compare candidate key sets and old offered views, require the exact new candidate/shared cut arrays or omission, and independently compute the hash of the complete new request. Only then remove those allowed added properties from the decoded candidate report, restore the old hashes and root version, and require every remaining report byte to equal the released baseline. No sorting or normalization of unrelated fields, arrays or text is allowed. The added property may occupy any JSON position while all existing fields retain their order. Wrong names, owner paths, duplicate list entries, empty arrays, stale/arbitrary hashes, extra state or changed raw/view/text data must fail.

The J1 177 rule requests and ai-tools 499 rule requests, findings and results compare complete-report bytes with only the root engine version changed. No selection exception applies to a rule record. Saved selection descriptors and all inline cut markers stay exact. A full request hash changes once if any member or shared view gains metadata; other uncut candidates in the same frame do not get a partial replay exemption.

Five owned controls close cases absent from the corpus: literal authored markers without cuts; multiple candidate field cuts; shared string cuts; both owners cut; and shared list reduction ending in `[cut: N more]`. Each runs both subjects at the same path. Controls assert exact body/extra and opening field names, stable text and hashes, omission for uncut owners, and the real generated list marker. Literal-marker owners follow their empty view.cut rather than the marker-equivalence check used for the real corpus.

## Native joins and evidence

Read permanent failing-first tests for exact candidate/shared serialization, list cuts, uncut omission/literal markers, reserved cut-fields collisions, request hashes, fixed request-limit behavior and changed/same-hash replay. Join them to the declared source and full workspace/static gates and signed artifact. This route prepares payloads; it does not itself settle selection answers or prove paid model quality.

CLI children run serially through the existing bounded process harness with credential storage disabled and an owned loopback listener. Preserve both logs for the one unchanged isolated retry permitted after a timeout; an isolated pass is classified host load and a repeated failure is a finding. Never change timeouts or expected results. Stop below 6 GiB available disk.

Keep source/release/artifact identities, original file/config guards, both complete reports for every group/control, exact commands/exits/stdout/stderr, input snapshots, account.json, captured.json and final harness.json. The driver closes its listener and rechecks frozen inputs even after failure. Read the raw receipts before handback; captured.json is a capture account, not acceptance. SDK review cut, independent review, CI, provider/browser behavior, other platforms and release acceptance remain separate.
