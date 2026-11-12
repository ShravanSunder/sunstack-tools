# Saved cases detail and change-scope refusal paths

Use these journeys for the 0.11.1 CLI fixes: read a completed `lint cases` report with `lint show`, preserve ordinary/prepared readback, and identify the file and configured limit behind a `lint --since` refusal. The governing source is the 0.11.1 commission, Outcomes1–3 and Correction1 including its completed-versus-prepared clarification. Reconfirm that source with the commissioning Lead. Broader O5/O6 behavior stays in [decision-trees.md](decision-trees.md), and unchanged-binary capture history stays in [since-encoding.md](since-encoding.md).

These routes were observed on candidate `6fd3aaaa73174011f2ee67901dd27aada127432e`, runtime0.11.1, signed binary SHA256 `11055f3c717b9ec4484f8f9377231da05b7709413a4860dd52118451953b1c4c`. The saved-report journey passed19 child invocations and the scope journey passed18, including expected refusal exits; both listeners closed with zero requests/errors. The raw numeric27 candidate associations and original/copied input hashes were checked separately. Ordinary/prepared preservation, all three acquisition caps and JSON/SARIF paths passed. The bound native819-pass/7-skip suite includes causal SourceChanged→lint JSON/SARIF and no-file history/time cutoff controls. These are bounded keyless observations, not release acceptance. Future runs must declare their own target and evidence. Fixture preparation and helper tests alone prove no product behavior. No credential, inference, public report-schema expansion, publication or release authority follows from this recipe.

## Prerequisites and target

Declare a clean full source head, immutable signed binary path and SHA256. The shared process harness verifies source/digest before and after each bounded child; macOS requires the verified Developer ID debug signature. Use the existing Node/Zod workspace dependencies from the monorepo root and a new evidence directory outside every fixture/input root. The drivers default the expected runtime version to0.11.1; use `--expected-version` only for a different version explicitly declared by the commission. Do not install dependencies or modify original repositories merely to make a check pass.

The real regression input is the commission-provided R03 completed-cases JSON. Supply its reachable path, exact SHA256 and selectors before observation. The original file is copied byte-for-byte into the evidence root, never edited. The commissioned input has33 string case IDs and7 revision runs, version0.9.0; numeric27 occurs in two runs with distinct owners, one without a case ID. Unique41 and case ID `r03-current-typescript-topics` provide positive readbacks. The preserved SHA is `dfeeb36d1c8240c5b30f24a528f283d14394ea0272f99a37c5cc38ae081790bc`. If that input is unavailable or differs, return the input prerequisite; do not replace it with an invented report and claim the real regression passed.

## Read a completed cases report

```sh
node scripts/verify/sunstack-rule/cli-reports/show-reports.ts \
  --binary <signed-binary> --source-root <checkout> --commit <full-head> --sha256 <digest> \
  --cases-input <real-R03-report> --cases-sha256 <input-digest> \
  --unique-id 41 --ambiguous-id 27 --case-id r03-current-typescript-topics \
  --evidence <new-show-directory>
```

Observe the following through the actual signed CLI, with isolated HOME and credential storage disabled:

- Unique numeric41 returns the selected lint detail, including its exact owner, saved requests/evidence/rulebook files, finding, rewalkable state and gap. Compare it byte-for-byte with `lint show` on that same nested ordinary lint report. Both27 results are passes, so a count/outcome-only check could silently select the wrong revision; owner/content comparison is mandatory.
- String case ID selects the exact case/revision and its saved result, using the existing `{case, revision, detail}` shape. Compare its detail with ordinary lint readback of that revision/result. No new selector syntax or envelope is introduced.
- Ambiguous numeric27 refuses with a named cases-report error and every revision/case/result candidate. Unlinked candidates explicitly say `no case id`. Automatic checks verify candidate identity presence; also read the printed candidate associations against `ambiguity-oracle.json` before reporting this outcome supported. A bare serde message or choosing one result silently fails.
- Unknown IDs and deliberately duplicate-case/duplicate-run saved inputs refuse without raw serde leakage. An unrecognized report is identified as a report-kind problem.
- Produce a current ordinary mechanical lint report, then read it after changing its owned source/config/rulebook. Detail bytes must stay identical; no current-file discovery or model call is needed for saved readback.
- Produce a genuine keyless prepared-cases neighbour whose ordinal1 has no linked result and ordinal2 links nested result1. Read ordinals1/2 correctly and refuse unknown3. Prepared numeric selectors remain stable1-based case-array ordinals, not the completed numeric-result namespace. Synthetic fixture labels are wire metadata, not owner or classifier-quality evidence.

`lint --check` acceptance is readiness for the owned producer fixture. A binary version or answering listener is identity only. The saved-input hashes must stay unchanged after every readback. The owned loopback is wired for current fixture producers; saved R03 endpoints/configs are not rewritten, so that listener is not a general endpoint-egress witness for pure readback.

## Locate a refused change scope

```sh
node scripts/verify/sunstack-rule/cli-reports/change-scope-errors.ts \
  --binary <signed-binary> --source-root <checkout> --commit <full-head> --sha256 <digest> \
  --evidence <new-scope-directory>
```

The driver uses owned Git repositories with one checked mechanical Markdown document and an asset outside the checked path. An unchanged binary asset plus a changed document must still succeed and report exactly that checked piece. A changed binary asset must refuse under real repository-wide `--since` capture, naming `assets/offending.bin` instead of only the config file.

Drive default/human, JSON and SARIF output on the same changed input. Encoding refusal retains exit2; existing bounded-discovery limits retain incomplete exit3. The new optional JSON diagnostic `path` names the offending checked-root path, while config/source identity remains separate. SARIF uses its existing invocation `toolExecutionNotifications[].locations[].physicalLocation.artifactLocation.uri`; diagnostic errors may have no finding in `results`. A path embedded only in message text fails the structured-field check.

File-byte, total-byte and file-count fixtures exercise distinct configured stops. The same diagnostic must name the correct limit and its configured value, not a remaining budget, unrelated version number or config-line number. Check the saved JSON report even when human/SARIF output is selected. Path/value expectations are fixed from authored fixture data and existing capture order before running, not chosen from a product error.

The SourceChanged interaction is joined from permanent deterministic native tests: capture, mutate the owned source, verify the snapshot, and assert the exact offending path through the lint diagnostic boundary. The CLI journey does not introduce a timing race or sleep-based mutation probe. Native tests must also cover non-file/global limits with a named limit/value and no invented path when no file exists.

## Evidence and return

Retain the exact original/copy hash, frozen result/candidate inventory, all producer/readback reports, ambiguity-oracle and the manual association check, fixtures' base revisions/changed bytes, commands/exits/stdout/stderr, saved JSON/SARIF, per-child process receipts and listener teardown. Each pass artifact is joined with `harness.json`; expected nonzero refusal/limit exits are successful negative observations. Preserve every original failure and classify setup/driver/product/expectation gaps before rerunning.

Use the current target's offered tests only after reading their scenario/assertion and input binding. Ordinary/prepared compatibility, SourceChanged mutation and each relevant limit must have meaningful native/component joins; an UnsupportedEncoding CLI example alone cannot prove all producer paths. Required apparatus gates remain those in SKILL.md. Do not weaken cases or change the expected outcome to match a new implementation; product repairs go to Implementer and public-format/meaning disputes to Lead.

Close only owned listeners/children and retain evidence for Lead/Reviewer. Return supported/unsupported/unknown per outcome with target, actual counts/exits, unchanged/preserved shape checks, test gaps and every unobserved path. This is keyless CLI/native proof, not inference quality, browser behavior, platform/distribution proof, CI completion or release acceptance.
