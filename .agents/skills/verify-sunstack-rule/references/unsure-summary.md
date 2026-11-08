# Human unsure summaries and output preservation

Use this journey when default human output or report/readback formatting changes. The adopted 8 Nov 0.10 unsure-summary commission and `crates/sunstack/README.md` require one unsure summary per rule, unchanged finding lines and retained complete detail. This amends O5's earlier individual unsure-line expectation; it does not amend uncertainty, severity, counts or exits.

## Targets and readiness

Declare the candidate's full source head, immutable binary, SHA256 and version. A comparator needs its immutable binary, SHA256, version and provenance. For a released comparison, obtain the actual named release; matching a local binary's source to a release is not observed release behavior. On macOS the driver verifies the candidate's required debug signature and the comparator's Developer ID signature.

Read the driver help, then run from the monorepo root:

```sh
node scripts/verify/sunstack-rule/rulebook/rulebook-unsure-summary.ts --help
node scripts/verify/sunstack-rule/rulebook/rulebook-unsure-summary.ts \
  --binary <candidate> --source-root <checkout> --commit <full-head> --sha256 <digest> \
  --baseline-binary <comparator> --baseline-sha256 <digest> \
  --baseline-version <version> --baseline-kind <released-or-version-only> \
  --evidence <new-directory>
```

`--baseline-kind` accepts `released` or `version-only`. For the 0.10 formatter change, run two comparisons: a frozen 0.10 binary built before the formatter patch (`version-only`) and the signed released 0.9 binary (`released`). Record provenance outside the skill. The helper's `artifact_release` receipt field identifies the comparator's version; the journey's `baseline.kind` distinguishes a local version-only comparator from a distributed release.

One owned ephemeral loopback server supplies deterministic model answers. The binary, extraction, decision routing, serializers, file reads and CLI remain real. Each bounded child has an isolated HOME, credential store disabled and synthetic credentials only for loopback lint calls. Keyless `lint --check` with zero HTTP is readiness; an answering port or version alone is not.

## Deciding observations

The fixture has 66 unsure results for one rule, two for another, two warning findings and pass results. Require:

- One human summary per rule with its exact count and a pointer to a retained unsure ID of that rule. Reject individual unsure rows, duplicate/missing rules, wrong counts and pointers to another result. Every other human line stays byte-identical to the comparator, including findings, related places, status and usage.
- Reports retain 68 unsure results, error0/warn2 and uncertain status/exit5. Compare saved reports and JSON/GitHub/SARIF stdout on identical fixture/config/endpoint bytes.
- When versions match, compare full raw buffers strictly, without normalization. When versions differ, permit only the explicitly identified engine version: JSON/report root `version` and SARIF `runs[0].tool.driver.version`. Validate both expected values, replace that single raw version line and require every remaining byte to match. GitHub and `lint show` have no version exception.
- Read every saved result ID, including pass and finding IDs, using each binary's own saved report. This fixture has 136 distinct result IDs. Require byte-identical detail and zero HTTP for every pair; comparing only unsure IDs or reading both binaries from one version's report leaves historical preservation incomplete.

The old comparator's 68 individual rows are the summary sensitivity control. Pure apparatus checks also reject wrong counts/pointers, another rule's pointer, altered findings, data/whitespace changes outside the version field and any attempt to normalize same-version bytes. Product-native controls cover unsure-only, zero-unsure warning/error output and error precedence alongside unsure summaries; inspect their source and exact-target run before reusing them.

The normal O5 route in `decision-trees.md` remains required for saved request/tree/model evidence, filter skips, incomplete results and source-independent readback. Its human check keeps location/ID assertions on findings/skips/gaps while checking unsure groups by rule/count/pointer.

## Evidence and limits

Keep `pass.json`, per-format saved reports and every child's stdout/stderr/process.json. The single harness captures each comparator and candidate request on the same endpoint, checks source/digests around children and records target identity for that child. Its `harness.json` must show no errors and its server/children must settle; preserve partial receipts after a failed or interrupted run.

An original saved production report can additionally be replayed through the public library renderer with build-input/fingerprint binding, and through both CLI readback implementations. Label that as a saved-report replay, separate from a fresh rule run. No controlled answer, local comparison or saved replay establishes model judgment quality, Linux behavior, a newly installed release or release acceptance.
