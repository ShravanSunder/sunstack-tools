# Configured document conventions and standard defaults

Use this route for the 0.15.0 contract: HTML custom-tag behavior and Markdown heading-anchor schemes come from the project's evidence config. The adopted source is the 0.15.0 config-only commission and Corrections 1–7. The driver is `scripts/verify/sunstack-rule/config-only/config-only.ts`; its `--help` describes both phases. Native tests, SDK review, CI and release acceptance remain separate joins.

## Contract

Under the existing project's `evidence`, these additions restore 0.14.1 behavior:

```json
{
  "html": {
    "skip_tags": ["ex-markdown"],
    "text_attributes": { "ex-mermaid": ["label"] }
  },
  "markdown": { "heading_anchors": "github" }
}
```

Absent settings mean no custom skipped tags or configured text attributes, and no assumed Markdown heading-anchor scheme. Standard `template` outline exclusion and `alt`/`aria-label` asking text stay. Explicit HTML IDs inside Markdown and HTML targets remain supported without a scheme. A heading fragment without a scheme is `unsupported` with `heading_scheme_not_configured`; other unsupported formats use `unsupported_document`. Existing missing files stay distinct. Unknown schemes, non-string names and empty names must refuse at `lint --check` with the config location.

The low-level project loader and document selector take explicit `DocumentEvidenceSettings`. The project owns them; separate projects in one process must not share mutable convention state. `TargetAnchor::Unsupported` now carries a typed reason, changing its saved wire shape. The `sunstack exec` EvidencePlanInput remains unchanged and its document callers use standard defaults; it cannot carry these settings. Check these public changes against the SDK review cut before claiming integration.

## Inputs and readiness

Use existing Node dependencies and an immutable signed released 0.14.1 binary with verified release provenance. Supply a declared signed 0.15.0 candidate, full source head and binary SHA-256. No install, build, credentials, provider request or publication is implied.

`config-only/real-corpus.json` pins the actual J1 judged rulebook plus 13 source pages, the original three HTML section pages, the original Markdown-anchor fixture and ai-tools at its fixed Git revision. Supply these under `--input-root/{j1,sections,anchors,ai-tools}`. Every named file/config digest must match; ai-tools must have the pinned Git identity and unchanged tracked bytes. A missing source is a readiness gap. Never replace J1's actual judged rulebook with a convenient synthetic rule or regenerate the expected counts from the candidate.

The driver copies inputs to a new owned `--common` directory outside the inputs and source checkout. Both subjects then use the same absolute project, config and output paths there. Keep that common directory between phases. It requires successful `lint --root <project> --config <config> --check` before each prepared run. A nested config does not select the checked root by itself.

```sh
node scripts/verify/sunstack-rule/config-only/config-only.ts \
  --phase baseline --baseline-binary <released-0.14.1> --baseline-sha256 <digest> \
  --source-root <checkout> --input-root <frozen-inputs> \
  --common <new-owned-common-directory> --evidence <new-baseline-evidence>

node scripts/verify/sunstack-rule/config-only/config-only.ts \
  --phase candidate --baseline-binary <same-released-0.14.1> --baseline-sha256 <digest> \
  --binary <signed-candidate> --source-root <declared-checkout> \
  --commit <full-head> --sha256 <candidate-digest> \
  --input-root <same-frozen-inputs> --common <same-owned-common-directory> \
  --baseline-run <baseline-evidence> --evidence <new-candidate-evidence>
```

## Deciding observations

Configured reports must match released 0.14.1 byte for byte except the root version and that one config snapshot's exact adopted additions plus recomputed SHA-256. Requests, hashes, ordering, raw fields and all other metadata remain exact. Expected prepared requests are J1 177, HTML sections 114 and ai-tools 499; original Markdown has 127 mechanical results, three findings and no requests. Any other config-dependent difference is a governing gap; do not widen the comparison exception.

Default HTML asking compares to a released witness. The released parser locates whole `label=…` attributes on real `ex-mermaid` elements. Only those spans are padded with spaces, preserving byte length and newlines, on the owned copy at the same absolute path. The released binary then performs projection and budgeting. Original source is restored before candidate runs. Candidate raw fields, owners and spans match the unchanged original report; only asking model views and request payloads may match the witness, with hashes recomputed from the exact request. Removing tokens after budgeting is an invalid oracle because labels can shift cuts and retained tails. Keep every located span and mutation record. The fixed real pages have no additional HTML headings inside `ex-markdown`; confirm the original owner/range counts. A controlled literal page makes the positive skip delta observable.

Default Markdown transitions are checked per owner. The original corpus's existing Markdown targets contain no explicit HTML IDs; a conservative source guard refuses that assumption if an ID appears. All 167 existing fragments become typed no-scheme unsupported; a grouped owner becomes incomplete even when it also contains a missing file. Only findings from those owners disappear. Everything outside these exact state/result/count/status changes remains byte-exact. A separate seven-link control checks explicit HTML IDs in Markdown, HTML found/missing IDs, missing Markdown headings, a file-only target and another unsupported format. The ai-tools all-prepare fixture has no fragment targets or HTML projection, so its default report stays exact except version.

Literal asking and outline controls run standard, restored and unrelated custom-tag settings. They retain standard attributes, original raw HTML and zero usage; `template` is always excluded. Mutation tests must reject unrelated config snapshots, raw changes, arbitrary hashes, old heading success and wrong source identities. These are driver sensitivity checks, separate from the Implementer's native failing-first proof.

## Evidence and limits

Children run serially. Correction 7 permits exactly one unchanged isolated rerun of a timed-out case, preserving both receipts; an isolated pass is classified as host load and a second failure is a finding. Never change timeout or expected outcome. Stop if available disk falls below 6 GiB. The driver uses an owned loopback listener with credential storage disabled; zero captured requests and zero reported model requests are required. It closes the listener and rechecks frozen inputs even after a failed assertion.

Keep original and candidate reports, witness reports and span ledgers, command/exit/stdout/stderr receipts, input guards, signatures/digests, source manifests, `account.json`, `captured.json` and the listener's `harness.json`. Read decisive raw receipts before the final handback. `captured.json` accounts for the drive; it does not close native/schema/refusal/product-name or SDK review joins. Both baseline and candidate phases have exercised this route through the signed CLI. Repeat the scoped drive on each declared target; an earlier pass does not establish a later target. Browser rendering, provider quality, other platforms and release acceptance remain unobserved.
