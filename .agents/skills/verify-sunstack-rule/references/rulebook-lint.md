# Rulebook lint: pieces, decisions and changed evidence

## Filters: one answer per folder and gated rules

The R1 Program Design correction at `906fc9e` requires unsure answers to retain their selected answer and probability as well as all probabilities. The driver enforces that correction: its unsure case must report `eligible` at 0.3, never substitute `blocked` at 0.7 or confidence 0.99. Prepared/live receipts name this design revision alongside the Verification Design pin below.

This lasting journey follows the filters Specification R1–R9 and Verification Design OD at `dd92e2c`. It is a **draft pending the Lead's E5 candidate declaration and first drive**. The entry is `scripts/verify/sunstack-rule/rulebook/rulebook-lint.ts --case filters`; `rulebook-filters.ts` owns its isolated fixture and assertions. The calling client is Node; real `sunstack lint`, config loading, extraction, filter dispatch, gating, reports and exits stay real. A loopback responder supplies only synthetic model answers. No hosted key or classification-accuracy claim is involved.

Prepare without running a product:

```sh
node scripts/verify/sunstack-rule/rulebook/rulebook-lint.ts \
  --case filters --prepare --evidence <new-authorized-directory>
```

For observation, omit `--prepare` and add `--binary <absolute-candidate> --source-root <checkout> --commit <full-head> --sha256 <digest>`. Use a separately declared, unchanged candidate after E5; the entry checks source identity and binary bytes around every child. On macOS the binary needs the verified Developer ID debug signature described below; `scripts/cargo-debug-signing-runner.sh --sign-only <binary>` applies the repository identity. Candidate signing and a local journey do not prove release publication.

The fixture has two folders with the same display name under different parents, three checked members in one and one member with three paragraph pieces in the other. Each folder's `overview.md` supplies exactly one description field through an inline frontmatter kind. Context-only sentinel text is absent from filter requests. Two gated rules share each folder answer: one mechanical and one asking.

The driver observes these paths:

- Judged inclusion/exclusion: exactly one filter question per full folder identity; only included members send rule questions. Included mechanical/asking rules pass; excluded pairs report judged `not_applicable` with the filter's reason. The included Choice selects `eligible` at exactly 0.7; the excluded folder selects `blocked` at 0.9.
- Unsure: the excluded folder selects `eligible` with probability 0.3 while `blocked` has 0.7 and confidence is 0.99. It gives no gated results, one skip per rule/file despite three pieces, and exit 5.
- Failed question: a controlled nonretryable HTTP refusal gives incomplete filter state, skips and exit 3. A folderless file gives `no_folder`, no filter request and exit 3.
- Member-only path argument: its folder description reaches the filter while context-only files produce no rule results; the other folder is never asked.
- Ungated compatibility: on the same candidate and fixture path, filters absent versus declared with no gates preserve ordered public results, findings, uncertainty, counts and request bodies. No filter question is sent.

Each normal fixture first passes keyless `lint --check` with zero requests. A judged full run with known pass/exclusion rows establishes usable readiness; `--version` or fixture preparation alone does not. Preserve per-child command, cwd, PID, stdout, stderr, exit and source/binary identity, exact HTTP bodies and the harness teardown record. The entry owns its child and loopback server, waits for process/request completion, and closes them in `finally`. Keep evidence after owned teardown; failures retain their output and never write a pass receipt.

Required native joins at the same candidate remain: zero real walks on excluded/skipped pairs (HTTP cannot observe mechanical execution); same-process answer freshness; change-only description dependencies before selected-kind selection and final picking; all R6 path-tagged refusals and process exit 2; R9 missing/unavailable/ambiguous input, actual projector cut metadata and encoded request cap; shared request allowance/cancellation/usage; activation before acquisition; human/SARIF output and emitted-payload schema checks. The binary journey does not substitute for these. Controlled responses leave live inference accuracy unverified.

The colocated `rulebook-filters.test.ts` plants wrong request/report identities, counts and outcomes to check assertion sensitivity. Run it with `pnpm exec vitest run scripts/verify/sunstack-rule/rulebook/rulebook-filters.test.ts`; apply the verification apparatus TypeScript/OXC gates in `SKILL.md`. These checks exercise the driver, not product behavior.

## Rules-as-nudges: S1 and U10

Use `--case nudge` for the accepted 31 Oct rules-as-nudges engine Specification R1-R10 and Verification Design U1-U10 (target-exclude clarification accepted at `a69fe47`, locator order clarified at `6328647`). This additive mode drives a declared signed release target. It leaves the default, selection and amendment modes intact.

```sh
node scripts/verify/sunstack-rule/rulebook/rulebook-lint.ts \
  --case nudge --prepare --evidence <new-authorized-directory>
```

For observation, omit `--prepare` and add `--binary <signed-release-binary> --source-root <product-checkout> --commit <full-head> --sha256 <digest>`. The paired Implementer declares that target at the plan's batch boundary after the version slice, integration gate and apparatus merge. The driver checks the verified Developer ID debug signature and unchanged source/hash. Each prepared fixture must pass `lint --check` without requests before the real run. No real key is used.

`rulebook-nudge.ts` drives fourteen isolated fixture variants through real lint. The line-17 input is a byte-preserving copy of the spike's frozen `ops-observability-stack` tree, with provenance in the fixture README. Expected questions, complete sent states, locations and result rows come from the Specification and fixture files. Its controls cover:

- Resolved line 17, low-confidence/default line 7, over-line-limit omission, unoffered-label invalid_response through the real native validator, and an all-mechanical located route with zero requests. The finding's span stays the body span. JSON and SARIF exact request bytes agree; SARIF uses line 17 and preserves ordered related places.
- Standard, loaded opening ranges and heading spans in related order; a rule without related fields lists only its standard. Long loaded openings expose opening_cut. Native joins cover fields-only, unresolved and empty-opening omissions.
- A two-question request whose gate alone is read, judged pass/drop, judged gate then unavailable mechanical field, judged gate then authored-error mechanical finding, and mechanical-only findings. Exact rows require paths, only read answers, correct basis, and basis absence for incomplete.
- Over-cap and under-cap bodies beside an authored cut field: exact request and evidence_seen state preserve the authored text, engine cut-fields and result cut. Mixed Markdown/JSON findings, the JSON question's source line, and a broken JSON file beside a valid one. Warning-only exit 0, warning-limit exit 1, judged authored-error exit 1 and route severity override.

Retain raw requests, both output formats and every child receipt. The scripted server replaces only answers. S1 does not establish U6/U7, request-budget/enclosing locator cases, dispatch-refusal/pre-walk history, the schema or real model choice: inspect the declared target's S3 tests and the Lead's S4 evidence for those joins. A prepared fixture proves none of the product behavior.

Related-file identities come from their owning sources: the standard uses the canonical rulebook path, while checked files and loads retain repository-relative paths. Related line ranges name the last line touched by a half-open span, so a heading ending at the next line's column 1 names only its heading line. Criteria comparison checks the complete line-label/text map; the native map's lexical wire order carries no numeric-order promise.

U10 runs separately:

```sh
node scripts/verify/sunstack-rule/rulebook/rulebook-nudge-compatibility.ts --help
```

Supply the signed 0.5.0 release artifact and its SHA256 (`--baseline`, `--baseline-sha256`), plus the current signed release binary/source/full commit/hash and a fresh `--evidence`. A release artifact is identified by version, signature and bytes; the driver claims no local source commit for it. Both binaries lint the **same path**, the existing K001-K003 legacy fixture, with the same scripted answers. It compares every old report field and value recursively, including emitted object-key and array order. Additions are allowed; root version must be 0.6.0. It compares exact request multisets without sorting the report and rejects a planted changed step. Permanent comparator checks also reject removed/renamed/reordered fields, integer-key order changes, changed outcomes/locations/counts/types and duplicate keys. Schema validation and the engine diff's generic vocabulary remain separate S3/source joins. No version normalization or legacy-field omission is allowed.

The nudge --check controls are in [rulebook-check-refusals.md](rulebook-check-refusals.md#rules-as-nudges-s2).

For the additive 30 Oct selected/contained-kind cases, load [evidence-selection.md](evidence-selection.md) and use this entry's `--case selection` mode. The default journey below keeps its existing expectations; a prepared selection fixture is not a product pass.

This journey drives the 29 Oct rulebook pipeline through the actual `sunstack lint` entry. Expected outcomes come from `docs/specs/2026-10-29-skill-rules-evidence-and-decisions/{specification,verification-design}.md`, especially R10–R15, R20–R28 and U2/U5/U8/U10/U12. The fixture is the Implementer's pinned S0 tree under `crates/sunstack/tests/fixtures/rulebook-lint/`; its `expected/` values come from the source copies and ast-grep 0.51.0, independently of the builder.

Inputs: a declared source commit, local binary path and configuration/client, unchanged product sources and binary bytes, the pinned fixture tree, and a fresh authorized evidence directory. The current driver is `scripts/verify/sunstack-rule/rulebook/rulebook-lint.ts`; read its header and `--help`. The entry returns the source-declared EvidenceListing envelope; finding locations use Finding.line and combined values use JudgmentSummary.value. Recheck these types at the declared S3 target. Fixture preparation establishes no product behavior.

From the monorepo root, directly with Node:

```sh
node scripts/verify/sunstack-rule/rulebook/rulebook-lint.ts \
  --prepare --evidence <new-authorized-directory>
```

For observation, omit `--prepare` and add `--binary <absolute-local-binary> --source-root <product-checkout> --commit <full-source-commit> --sha256 <declared-digest>`. Build in the checkout's `target/` using the pinned toolchain and main skill SDK prefix. The calling client is this local Node TypeScript driver; loopback replaces only model answers. It starts no browser and makes no hosted call. Target checks bracket every child; a source or binary change stops observation.

The driver observes these four parts:

1. `--evidence` for every configured kind equals the independently scanned file/range tuples, in order; selected step/load fields match source expectations. Step 3 is bytes 6114–7909, lines 49–60, heading `3. Choose each observation`, completion line 59; its load resolves the named reference. The description decodes to 450 characters. No key is supplied and no request is sent.
2. A full K001–K003 run uses captured, scripted HTTP replies. K002 is an error at line 3; K001 combines 0.30 × 0.6 + 1 × 0.4 = 0.58 and warns at line 59; K003 at 0.91 passes. A separate step-3 fixture without completion settles at line 49 with no request; ast-grep independently checks that the fixture still reaches that step. Questions remain structurally identical to the authored native question objects, with their strings unchanged, and each request contains one piece's state without raw `text`. Separate K001-only 0.65 and top-choice-0.55 fixtures return uncertain/exit 5; the full fixture returns findings/exit 1 while retaining uncertainty. Stdin stays open and empty until exit so an input wait fails the completion bound.
3. Temporary Git repositories establish step-3-only selection, five unchanged steps excluded, changed-load selection, rule-file widening, a wholly selected untracked skill and repeated semantic output/request order. Fixture Git writes use explicit synthetic identity and no hooks/signing. Preserve logical report request order and compare exact bodies in that order; concurrent arrival-side bodies are a multiset. Never sort the report to manufacture determinism. The parser preserves raw JSON order before JavaScript can reorder integer keys. Join captured exact bodies to those indexed states in emitted order. Usage timings/receipts are compared separately, without requiring equality.
4. An extra rule naming an undefined kind is skipped while peer semantic results stay equal to baseline, coverage is partial and exit is 3; a malformed whole config is invalid/exit 2 with zero reported checked/target acquisitions. Join the recording-source test below for actual content acquisition and change-capture invocation.

Before driving, readiness is a successful fixture `--check` and its known mechanical K002 error, as covered by S1. Keep per-child stdout, stderr, actual command/exit/PID, source commit and binary hash, raw HTTP bodies and harness cleanup evidence. The driver owns its process groups, responder and server; process completion and request events drive waits. Stop only those owned resources. Fixture copies are disposable after cited evidence is preserved; cleanup is governed by the assignment.

Run `node scripts/verify/sunstack-rule/rulebook/rulebook-boundaries.ts` with the same declared-target/evidence arguments for the supplemental U4 native `exec` old-key controls and the U2/U6 non-ASCII raw-versus-cut view probe. It accepts a valid native RuleSet first, then refuses each old marker-route header field without HTTP. Its Unicode case retains the middle token in 7,012 raw characters, omits it from the 6,000-character model view, and captures that exact view on the wire. These are observations through the binary; the exact request-cap boundary still joins the native tests below.

## Required native joins and honest limits

The journey is a binary observation, not all of U1–U12. Inspect and reuse the Implementer's current-target tests where they already drive the real boundary:

- U1/U3: linked project loader equals `ast-grep scan` for every project rule, including composed utils, path globs and multiple YAML documents; each standard util has positive and negative witnesses. The binary piece listing alone does not expose every helper rule's raw match table.
- U2/U5: field-state/target/root-read tests, scalar/list/multi-target reductions and raw-field checks, location/threshold/walk tables, TooLarge lazy asks and unavailable-field isolation.
- Amended R11/R17: a resolved target whose read fails makes its whole target field unavailable. An unavailable field explicitly named by any question's check/against/context blocks the first ask/combine with no HTTP. Unreferenced unavailable fields are omitted from sent state and block nothing; the exact-state comparator applies that omission rule.
- U6: capture a non-ASCII escape-heavy/long-question body with a generous cap, accept that exact body at its Unicode-character count, then refuse it at count minus one while a smaller peer completes. Byte length must exceed character count so the equality case rejects a byte-count guard. Compare the exact body encoder with actual HTTP bytes. A refused request has no same-run endpoint body to measure.
- U7/U12: `crates/sunstack-lint/tests/rulebook_lint_gates.rs` records run-path content acquisition/capture gates and unchanged peer work; a self-reported counter alone is insufficient. U9 instead joins the check-only structural call path and no-writer FIFO witness in the refusal journey.
- R27: compare pieces, fields, results and request records in emitted logical report order. Exact captured bodies are joined to request-state records without sorting the report; HTTP arrival order may vary under concurrency. Shuffled-completion native proof separately checks result/request-record collection.

Report each row supported, unsupported or unknown with the exact target and observed/reused evidence. Missing joins stay open. Loopback proves request/decision mechanics; real hosted admission, judgment quality, release contents and #171 consumer coverage remain separate. Complete when the selected drives, their native joins or named gaps, and owned teardown are accounted for; a prepared fixture or draft recipe is never a pass.

`usage.checked_files_acquired` counts distinct checked/target files acquired by change capture or checked_file_source. Frozen after-side text is reused; snapshot-verification rereads are excluded. Join zero acquisitions/no capture before the key gate or in check, plus one acquisition per changed file. This is not a physical-read counter or claim.

## 31 Oct amendment: neighbours, uncertain routes and incomplete reasons

The adopted source is the 31 Oct amendment's R11/R18/R44 and V22–V24, with VD U2/U5/U18. Use the current declared committed source and signed binary; no earlier target's pass transfers. Before any binary runs on macOS, run the repository runner with `--sign-only <binary>`, then verify the debug identifier, Developer ID authority and team with `codesign -dvv` and `codesign --verify`. `rulebook-amendment.ts` verifies that signature before launching children. A LuLu hold is an inconclusive host result, never the expected connection case.

Run the existing entry with `--case amendment` plus its normal `--binary`, `--source-root`, `--commit`, `--sha256` and fresh `--evidence` arguments. This mode observes:

- One subject paragraph's `next_lists` and `previous_lists` text and byte spans through `lint --evidence subject`, compared with the fixture's ast-grep 0.51.0 scan oracle; zero requests.
- An in-band gate handing off to a verdict in the same request. Verdict probabilities 0.9/0.5/0.1 give pass/uncertain/not-applicable. Every pair reports `verdict`; the uncertainty view reports the same step. Each case sends exactly one request containing both questions.
- An owned ephemeral port closed before use. The JSON invocation exits 3 and carries exactly `{kind: provider_failure, failure_class: connection}` without `http_status`; a separate default invocation prints `provider failure (connection)`. Successful signed controlled-endpoint requests bracket this probe. Retain `closed-port.json`, both output formats and the positive captures. This proves local connection classification, not a real firewall hold or live provider bodies.

V23's compatibility comparison is a separate command. Read `node scripts/verify/sunstack-rule/rulebook/rulebook-compatibility.ts --help` and supply both signed binaries' paths/hashes, their separate clean source paths/full commits and a fresh evidence directory. The baseline must have product source identical to main `7870513`; retain its build provenance, empty product diff and clean source identity. Existing source and binary guards remain active on both targets.

The comparator drives every prepared existing rulebook fixture and the complete legacy journey with identical scripted answers. It compares each ordered pair's entire public `results` entry, full `findings` and `uncertain` views, exits and actual HTTP counts. It never sorts pair results. Only two separately reported R44 changes are allowed: retired bare provider failure becomes rate-limit/429, or becomes invalid-response for the deliberately invalid answer. It rejects a planted wrong step; its permanent sensitivity tests also reject changed outcomes, result order, counts and unlisted reason changes. Public results have no `path_taken`: the native walk tests prove that field, with no new report contract.

Required native joins are `sunstack-evidence::neighbour_fields` (real parser/builder, both actual rule orders for same-range list/item, stable node identity, continuation piece position, exact copied positives, parent/barrier negatives, count 2, remainder and R12); the new walk and real-request handoff tests; rulebook/selection failure loopbacks; and the real report schema/default/SARIF tests. The native workspace gate keeps the decision crate and existing no-route expectations intact. Declare the Verifier target only after these joins, apparatus gates, recipes and binary journeys are complete. Report the exact observation, raw evidence and unknown hosted/firewall/platform layers; this recipe grants no acceptance or release authority.
