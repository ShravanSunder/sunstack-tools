# Use Sunstack from an installed TypeScript program

This route observes `@sunstack/sdk`, an additional consumer of the Rust engine. The adopted oracle is `docs/specs/2026-11-11-typescript-sdk/{requirements,specification,program-design,verification}.html`, R1–R15 and O1–O6, at the commission's declared bytes. Read all four, including disclosures. Runtime behavior, public outcomes and caller ownership come from those documents. Do not replace the oracle with the current package tests.

**Draft:** the scripts have apparatus sensitivity tests but have not driven an installed SDK candidate. Every row below needs a real target observation and its named joins. The driver reports partial coverage with exit 3 while joins are open; no row is currently an SDK pass. Product/CI changes belong to Implementer. This route changes only verification apparatus.

## Launch and usable readiness

Run from the monorepo root with Node 26 and the existing TypeScript/Zod apparatus dependencies. Zod belongs to this harness, not the fresh SDK consumer. The supported interface is:

```sh
node scripts/verify/sunstack-rule/sdk/sdk-verify.ts --help
node scripts/verify/sunstack-rule/sdk/sdk-verify.ts \
  --target <absolute-target.json> --journey snapshot \
  --evidence <new-absolute-evidence-directory>
```

Replace `snapshot` with `inline`, `isolation`, `install` or separately authorized `provider`. Each invocation needs a fresh consumer directory and evidence directory; preserve failed runs instead of reusing them. The script writes its positive compiler caller into that consumer with exclusive creation. A successful import or open loopback port proves no SDK behavior. Readiness means the positive installed caller compiled, target guards passed, the real load/check/run completed as expected and requests reached their deciding assertions.

The Implementer supplies an immutable integrated full commit, packed wrapper/native packages and their installation instructions. Do not operate on its moving checkout or freeze it yourself. Source guards cover current product owners and toolchain/manifests; the artifact guards bracket the run. Use a separately declared stable snapshot if product work continues elsewhere. Stop on drift.

`sdk-target.ts` owns the strict target input. Required fields:

| Field | Required evidence |
| --- | --- |
| `sourceRoot`, `commit` | Canonical checkout and full 40-digit integrated Git head, quiescent product paths |
| `consumerRoot`, `packageRoot`, `entry`, `addon` | Canonical fresh installed paths outside source, actual resolved `@sunstack/sdk` entry and `.node` artifact; no workspace fallback |
| `tarball`, `tarballSha256` | Exact packed SDK candidate; also retain all optional-native tarballs and actual installation transcript |
| `files` | Absolute installed paths and SHA256s; include package.json, entry, declarations, all runtime files, native artifact and selected optional package metadata |
| `release`, `contractRevision` | Matched declaration/encoder/native contract; native handshake needs its product proof join, a manifest assertion alone is insufficient |
| `platform`, `arch`, `libc` | Actual cell: darwin arm64/x64 with none; Linux arm64/x64 with glibc/musl |
| optional `cli` | Integrated CLI `binary` and `sha256`, required for inline CLI comparison |

The resolver and artifact guards reject dirty/moved product source, missing declarations in the positive compiler, wrong runtime/OS/CPU/libc, symlinked workspace artifacts, wrong entry/release/hash, ambient consumer Zod and incomplete native pins. The guard walks the complete installed wrapper package and the package owning the selected addon (nearest contained package.json): every file, including declarations and runtime helpers, must appear in the pinned hash list. Omitting a file or adding a new helper fails, even if entry/addon hashes match. Native loader selection itself remains a separate assertion against actual ContractInfo. Record the package-manager resolution and optional-package versions; a top-level tarball plus unrelated registry addon cannot pass the distribution claim.

Controlled journeys refuse an inherited real key, set one synthetic loopback-only key and disable the native credential store before addon import, restoring both afterward. The HTTP stand-in preserves actual SDK/native/Rust encoding, admission and execution. It answers the real Jev protocol at the configured endpoint; it is not ordinary provider proof. Do not run controlled and hosted work concurrently in the same driver process.

## Journey coverage and deciding observations

SDK and CLI report statuses retain the Rust-tagged shape, for example `{ kind: 'findings' }`. A completed run may contain clean, nothing_selected, findings or uncertain; prepared/invalid/incomplete/cancelled tags and flat strings are refused by the completed-report assertion. CLI parity validates the same shape and compares the full status object, without flattening or accepting an alternate representation. Source owners are `crates/sunstack/src/sdk_host/report_projection.rs` CompletedStatus and `crates/sunstack-lint/src/lint_reporting/report_contracts.rs` LintStatus.

| Journey | Script behavior and wrong behavior it rejects | Required joins before closing the O row |
| --- | --- | --- |
| `snapshot` · O1/R1/R2/R11 | Real nested config load from below the config; mutate standard A→B; run held A, copied edited A and reloaded B. Actual requests must contain the appropriate distinct standard. Check file/edited origins and content hashes; missing config is unavailable, malformed config invalid. Catches rereading, ignored edits, fabricated hashes and generic failure substitution. | Sibling-CWD/explicit-path discovery; unreadable dependency; root/filter edits; receipt-only override; tampered provenance/format/definition IDs; deep freezing; no-recapture join. No atomic multi-file snapshot claim. |
| `inline` · O2/R3/R5/R7/R11 | Complete typed data with referenced utility, matcher, evidence kind, filter, rule, Markdown and questions executes with only checked inputs in root. Both choice/filter and noul/rule requests must occur. Definitions live outside the checked root; native requests must include its checked sentinel. Optional pinned CLI executes equivalent file definitions; compare exact encoded HTTP bodies plus status/counts/file counts/filter skips. | Blocked filter control; every root/filter/kind/matcher/utility/Markdown source ID/content/digest; full report semantics. Observe hidden temporary writes/deletion and out-of-root writes. Any needed origin/time normalization requires an explicit narrow field inventory grounded in R7; do not strip substantive fields. |
| `isolation` · O4/R6/R7/R8 | Real installed asking/filter check sends no HTTP. Pre-abort beats non-data encoding. Two same-process real addon calls reach distinct listener gates; abort A, release B, retain A partial report and B result/context. CWD/environment must be unchanged. | No credential-store or checked-input acquisition (zero HTTP alone misses these); A task drain; every exact completed/invalid/unavailable/incomplete/cancelled/failed ending; reply-loss/invalid-reply at actual native reply boundary; before-dispatch ABI/encoder failures. Count facade invocation separately from legitimate Rust provider retries. |
| `install` · O5/R10/R13/R14/R15 | Same positive compiler and inline execution on the actual declared cell, target pins, no resolvable consumer Zod or cargo/rustc. Must use fresh packed candidate topology. | Actual fresh install transcript, selected optional native package, ContractInfo release/revision/target; unsupported-runtime/platform, missing-addon and intentional-skew controls before effects; lossless numeric return and O3; all six separate cells. A static CLI archive or cross-build never closes addon execution. |
| `provider` · O6/R7 | Explicit `--allow-provider` plus a separate commission; ordinary Rust host resolver and backend, synthetic checked content, no controlled endpoint. Require actual completed native request outcomes and provider receipt; controlled listener must remain unused. | Owner-authorized existing credential route, bounded request/budget, genuine usage/receipt validation and safe logs. Missing key/prepared/HTTP success alone is unknown. The flag acknowledges a grant; it grants no credential/provider authority. |

O3 remains Implementer-owned compiler/native tests: generated concrete input/result shapes, intended compiler negatives after a positive compiles, 29 canonical authored language spellings versus snake_case results, wrapper property/default parity, upstream ignore/refusal distinctions, strict Sunstack unknown keys, reference/marker/constructor refusals and bigint transport/return/overflow/nonzero. Link exact declared-target commands and receipts once provided. This script's positive compiler caller is not those negative tests. No hand-written TS policy or mandatory base-package Zod replaces Rust.

The 0.12 joins include FieldSource.document, found/missing/unsupported TargetAnchor, anchor_exists and whole-token contains_field (R1 versus R10, Unicode, empty/unavailable needles), contained/missing/outside-root identities and shared budgets, HTML asking text versus raw mechanical source. Use the current `html-evidence.md` and its native join inventory. Runtime matchers need all supported engine languages; no unshipped grammar-map safety is implied.

O6 also requires existing CLI journeys from `rulebook-lint.md`, `rulebook-check-refusals.md`, `cli-report-errors.md`, `git-first.md`, `since-encoding.md` and `html-evidence.md` where the changed seam applies. Preserve discovery from invocation CWD, config-derived implicit target root and named paths based at CLI invocation CWD. SDK absolute root is exact, never widened; relative roots fail before acquisition/CWD access. Preserve checked diagnostic path separately from definition origin. Default/all/since, no-match/outside-root, saved reports/show/cases, output bytes and exits need explicit comparisons. Join current native proof for ChangedOnly lint capture and Full connected context; report equality alone cannot prove the caller policies.

## Platform and CI use

Run `--journey install` once per darwin-arm64, darwin-x64, linux-arm64-glibc, linux-x64-glibc, linux-arm64-musl and linux-x64-musl. Each environment needs Node 26, the explicitly supplied compiler, Git for source guards and candidate-installed files. Cross-compiled addon bytes do not execute themselves. Named emulation/container use must remain visible in its receipt. Remove cargo/rustc from the consumer environment through an isolated image/PATH rather than removing shared toolchains.

Implementer owns CI and chooses the package-manager install command after the packed optional-package topology exists. CI invokes this same driver with the exact target/cell manifest and new evidence directory. Preserve exit 3 as partial coverage; do not mask it as success. Join all required evidence before any distribution-readiness conclusion. Public publication, tags and release changes are not part of this route.

## Evidence and cleanup

Preserve target/artifact identities, positive compiler stdout/stderr/exit, controlled raw request bodies, actual SDK outcomes with exact bigint encoding, CLI stdout/stderr, and the final receipt's observed portions and gaps. Retain package installation logs, compiler/native/product test commands/counts/exits and provider receipt separately. No credentials, credential locations or environment dumps enter artifacts. Use synthetic inputs throughout.

The driver closes only its own listeners and owned CLI child; isolation aborts both owned signals in teardown and awaits their settlement. A native operation that will not settle is a product/harness gap, never a pass; retain its evidence and stop only the exact driver process you launched. Confirm evidence survives cleanup. Do not remove another consumer, checkout, cache or server.

Apparatus gates from root:

```sh
pnpm exec tsc -p scripts/verify/sunstack-rule/tsconfig.json
pnpm exec oxlint --type-aware --type-check --tsconfig scripts/verify/sunstack-rule/tsconfig.json --max-warnings 0 scripts/verify/sunstack-rule/sdk/*.ts
pnpm exec oxfmt --check scripts/verify/sunstack-rule/sdk/*.ts
pnpm exec vitest run scripts/verify/sunstack-rule/sdk/sdk-apparatus.test.ts
```

These gates qualify the harness only. A draft target gap stays open until an immutable installed candidate drives the real journey. Return each O row as observed/reused/unreachable with exact target, deciding receipts, remaining product/native/CLI/platform/provider joins and their owners. The Lead owns acceptance.
