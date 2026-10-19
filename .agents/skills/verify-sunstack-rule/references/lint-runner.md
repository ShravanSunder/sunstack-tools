# S1 — lint a repository

Drive `sunstack lint`, compare its report and actual exit with literal fixture expectations, and retain stdout/stderr separately. This route owns current lint behavior, not historical `sunstack-rule` replay. Expected outcomes come from runner Specification R1–R15/C1–C4 and Verification Design U1/U2/U4/U5/U6/S1 at the target-policy revisions in SKILL.md. `RuleSelector` is a written string in reports; other runner enums are kind-tagged. The authored band field is `error_at_or_below`.

**Draft:** fixture preparation and apparatus checks do not establish product behavior. A declared target with implemented lint is required for the first observation. The real binary, engine, files and Git stay real. Loopback HTTP supplies deterministic replies and counts calls; it proves neither Jev inference nor HTTPS. S3 owns those.

## Prepare and drive

From the monorepo root, read `scripts/lint_fixtures.py --help` through the pinned inline uv dependencies. Use a new authorized evidence directory per invocation; the driver refuses reuse. Unset Python optimization and uv index/config overrides as in the main skill. Keep `PYTHONDONTWRITEBYTECODE=1`, and put `UV_CACHE_DIR` in the authorized scratch home.

```sh
uv run --no-config --no-env-file --no-python-downloads --default-index https://pypi.org/simple --script \
  .agents/skills/verify-sunstack-rule/scripts/lint_fixtures.py \
  --prepare --evidence <new-evidence>
```

Preparation writes synthetic repositories, a literal `expected-results.json` beside each, a manifest and real scripted Git commits/readbacks. It calls no binary and reads no credential. `fixtures-prepared.json` explicitly says `product_observed: false`.

For a commissioned observation, omit `--prepare` and supply `--binary <declared-absolute-binary> --source-root <declared-product-checkout> --commit <full-product-HEAD>`. Record the declared binary SHA256 externally and compare it with the receipt; the driver checks source HEAD, product dirty state and binary hash before/after each child. Run on the dev macOS arm64 binary and separately on the Linux musl target. `--case <name>` selects a bounded rerun; a selected subset receipt never proves the full route.

Readiness is a successful valid-fixture `lint --check`, then a `mechanical-error` run returning its actual finding and exit 1. Missing binary, stale/dirty source, unsupported lint or invalid fixture is a setup/observation failure, never a product pass. The process has 30 seconds to settle. Cancellation waits until an owned HTTP request is in flight, then sends SIGINT to only the owned process group; a killed or truncated process cannot satisfy exit 4.

## Linux release target

The same S1 driver accepts `--container-image <adopted-Alpine-digest>` and `--container-control-host <declared-hostname>`. Read its `--help`; both options are required together. It checks cached image architecture and static ELF identity, then drives the same fixtures and comparisons through Docker. `runner_containers.py` owns the shared pins and launch settings; `runner_process.py` owns each child, signal and timeout.

Every no-call command uses network `none`, so its configured control endpoint is unreachable. Judged cases use the declared route from the container to this run's loopback server, with synthetic authorization only. A separate HTTP probe establishes that route; it proves connectivity, not product readiness or Jev behavior. Product commands use a read-only root/binary/fixture, minimal environment and an empty tmpfs HOME. Only `--init` create/refuse commands receive a writable fixture, because their observed outcome is creating/refusing the config. Cancellation signals the exact owned container; timeout cleanup names only that container. Comparisons translate `/fixture` as the container namespace and keep literal finding identities unchanged.

## Released config schema

Run `scripts/schema_agreement.py --help` through its inline pinned dependencies. Supply `--schema <exact-release-schema>`, `--fixtures <completed-S1-fixtures>`, `--binary`, `--source-root`, `--commit`, a new `--evidence`, and the same container options when checking Linux. It records the schema hash, refuses external `$ref` fetching, and compares each config with a real no-key `--check` command. It also checks C4 and nonzero integer limit boundaries.

Structural cases must agree on acceptance/refusal. Filesystem/loaded-rule semantic cases and malformed glob syntax must pass the schema and be refused by the binary (Specification R15/V12 at `59b12d3`). Numeric limits must carry their full minimum and maximum; values above the native u32/u64 range must be refused by both. Numeric spelling is binary-only: an integral value written as `1.0` passes the schema and must receive a located binary refusal (R15 at `26122bf`). Duplicate JSON names and syntax are parser refusals, recorded separately; a missing config has no schema input. The full per-case table and any mismatches survive a failing aggregate exit. A changed or overpermissive schema cannot turn a binary refusal into agreement.

## What must agree

The fixture manifest names each wrong build. Compare every expected finding's rule, file, line, column and severity, reject extra/missing findings, compare severity counts, status, incomplete parts, model-request counts and actual exits. Do not regenerate expected results from product output. Additive report fields and incidental prose may differ; required identity and evidence cannot.

The built cases cover clean/error/warning-limit/invalid/incomplete/cancelled exits; error plus incomplete; empty candidates; clean candidates outside scope; named clean paths; staged/unstaged/untracked targets; `--since` and `--all`; changed rule and rule-set definitions; unrelated edits; config widening; ignored and Git-ignored targets; severity wildcard/exact/ordered overrides/off; nested and explicit config; missing config/path/Git; syntax/unknown/duplicate/glob/input/selector/rule-set refusals; old severity refusal; rules listings and init; judged bands, missing backend/key, off, missing cost and typed answers/evidence. Treat the manifest as the executable case inventory, not a second list to maintain here.

The docs-only history keeps rules outside target globs. Confirm the baseline blob stays unchanged for rule/config/unrelated edits, and that the changed paths are the intended ones. This catches a lint prefilter that misses dependency-affected clean targets or incorrectly lints unrelated clean documents. Ignore exclusions never have a context exception: R12 was corrected because one document input cannot express cross-file context.

Controlled judged runs compare `evidence_seen` with the exact states received by the loopback server and resolve native typed answers, not answer references. Missing provider cost remains absent, never zero. For formats, compare default/GitHub/SARIF with the same JSON finding tuples; inspect user-visible messages as well. A format that drops a finding fails even when exit 1 survives.

Invalid `--check` cases compare the readable diagnostic with the authored fixture file, line and column span and its offending name or category. The same fixture then runs `lint --all --format json`, where the structured diagnostic must have its exact closed code and location; prose containing a code cannot substitute for that field. Cancellation must retain the expected unresolved rule/file and `not_started` reason, with status `cancelled` and exit 4.

`--init` must create only the `$schema`/empty `rule_sets` skeleton, refuse a second write without changing bytes, refuse the empty skeleton on `--check`, and accept it once a valid entry is added. `--rules` includes off rules, source/entry/kind/authored/configured settings; file-specific output identifies the deciding override.

## Join native proof and name limits

The full VD rows need these current-target joins in addition to driver results. Inspect the Implementer's real tests and raw current-target run, verify each can fail for the named wrong behavior, and reuse applicable evidence instead of writing product tests here:

- Rust library versus CLI JSON equality, field for field, through the real engine. Npm equality is S4.
- Every mechanical/judged classification row, off-only model gates and shared gates needed by enabled peers, native request-evidence materialization, receipt cost present/absent/mixed, and all closed invalid codes.
- Published schema versus binary structural acceptance/refusal; filesystem/duplicate-file/unknown-rule semantic cases remain binary-only. Obtain the actual released schema, not a fixture-generated stand-in.
- `--check` does not read targets: the unreadable-file fixture checks that target content is unnecessary; join source inspection or a permitted file-access trace to exclude ignored failed reads. An unreadable-file success alone cannot prove zero read attempts.
- Current `sunstack exec` process suite with unchanged protocol apart from the adopted vocabulary/dispatch/readiness cutover. Historical replay is not this proof.

Missing joins stay unknown. A product target may expose a driver assumption; resolve it against the oracle before changing the check. A claimed exact classification or complete U row requires its native join, not only a green fixture summary.

## Evidence and completion

Keep `fixtures/`, per-process `stdout.log`, `stderr.log`, `process.json`, loopback bodies, binary/source identity, and any failure before a fresh rerun. Each run owns and awaits its children and server; it kills only its own timed-out process group. No repository cleanup or directory deletion occurs. Reopen evidence after teardown. `s1-pass.json` is written only after selected cases and teardown; it retains unclosed native-proof gaps.

Return each U outcome supported/unsupported/unknown, exact target/hash/oracle, selected/executed case and HTTP counts, exits, raw evidence, native joins and gaps. No hosted, release or acceptance claim follows from S1.
