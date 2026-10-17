---
name: verify-sunstack-rule
description: Use when verifying the Rust Sunstack rule engine, its evidence and decisions libraries, or the sunstack-rule executable, including hosted Jev, skill-lint and handoff-monitor journeys. Not for Demistify or production hook installation.
---

# Verify Sunstack Rule

Verify a named target against the adopted Specification, then report only the outcomes observed there. A passing build, an HTTP success or an unfavorable rule verdict each means something different.

The app is three Rust libraries (`sunstack-evidence`, `sunstack-decisions`, `sunstack-rules`) and the `sunstack-rule` binary from `sunstack-rule-cli`. Evidence owns capture and selection; decisions owns Jev transport and answer admission; rules owns applicability, grouping and verdicts; the executable owns framing, signals and exits. There is no daemon or JS runtime.

## Choose the target and journey

1. From the monorepo root, record `git rev-parse --show-toplevel`, `git branch --show-current`, `git rev-parse HEAD` and `git status --short`. Record the declared product commit, build location, configuration and calling client. Confirm the driven checkout matches that product commit; the skill may be on a separate branch. Stop if the target changes during observation.
2. The adopted oracle is design revision `f259d4b28f5131478e574e123335904c78b0d347`. Product target: `c169d0203c47dc34b0a1a8ae792437e06a3730c0`. Read every oracle file with `git show <design revision>:<repo-relative-path>`, including referenced examples and schemas. Start with `git show f259d4b28f5131478e574e123335904c78b0d347:docs/specs/2026-10-12-rust-rule-engine/specification.md`, especially the selected V rows; use the same method for `program-design.md`, `program-design-contracts.md` and, for connected evidence, `program-design-connected-evidence.md` in that directory. Product design copies may be older and a design working tree may have moved. If the pinned object is unavailable, return that source gap; resolve conflicting expectations with the commissioning Lead before driving the affected case.
3. Select the route below. Read it before running. A draft is a coverage obligation with an open prerequisite, never an executable recipe or a passing observation.

| Surface | Route | Availability at the milestone 1 target |
| --- | --- | --- |
| Native libraries and Rust quality | `references/library-gates.md` | Evidence, decisions and rules harnesses |
| Genuine Jev mixed primitives, V11 | `references/hosted-decisions.md` | Credential-gated real public-API test |
| Genuine rules applicability and routing, V22/V34 | `references/hosted-decisions.md` — hosted rules section | Credential-gated native rules tests or applicable independent hosted evidence |
| Four executable operations, V2/V13 | `references/executable-operations.md` | Real process suite and adopted V2 serializer/source method |
| Skill lint and handoff monitor, V14/V29/V30 | `references/milestone-1.md` | Controlled journeys/measurement and credential-gated hosted campaign |
| Git changes, relationships and chunks | `references/milestone-2.md` | **DRAFT:** milestone 2 target and real fixtures needed |
| Commissioned usage scenarios | `references/usage-scenarios.md` | Whole caller journeys with owned fixtures; US02 has a separately authorized genuine slot |

MUST load `references/library-gates.md` and return the scoped commands, results and remaining coverage before relying on library or build claims.
IF hosted V11 is in scope, load `references/hosted-decisions.md` and return its run identity, validated completion and receipt, or its access/setup gap.
IF genuine V22 or V34 is in scope, load `references/hosted-decisions.md` and return the hosted-rules observation or applicable reused evidence, native answer/receipt checks and any remaining gap.
IF executable behavior is in scope, load `references/executable-operations.md` and return the operation coverage, process evidence and remaining proof limits.
IF milestone 1 journeys are in scope, load `references/milestone-1.md` and return each local/hosted journey's report, measurement or named prerequisite.
IF change scopes or connected evidence are in scope, load `references/milestone-2.md` and return its real-fixture coverage or named open item.
IF usage-scenario verification is in scope, load `references/usage-scenarios.md` and return each selected US ID's supported/unsupported/unknown verdict, observed/reused coverage, relevant V rows, target, process/HTTP evidence and remaining gap, using the return rule below.

For the selected usage scenario, run the selector from the monorepo root. The reference owns the inventory and prerequisites; repeat `--journey` only for requested IDs:

```sh
env -u PYTHONOPTIMIZE -u UV_INDEX -u UV_EXTRA_INDEX_URL -u UV_INDEX_URL -u UV_FIND_LINKS -u PIP_INDEX_URL -u PIP_EXTRA_INDEX_URL -u UV_DEFAULT_INDEX -u UV_CONFIG_FILE -u UV_ENV_FILE \
  UV_CACHE_DIR=<authorized-evidence>/uv-cache PYTHONDONTWRITEBYTECODE=1 \
  uv run --no-config --no-env-file --no-python-downloads --default-index https://pypi.org/simple --locked --script \
  .agents/skills/verify-sunstack-rule/scripts/usage_scenarios.py \
  --binary <declared-binary> --evidence <authorized-evidence> --journey <US-ID>
```

## Build and readiness

Run Cargo from the monorepo root with the pinned toolchain in `rust-toolchain.toml`. On macOS this host needs a command-local SDK correction for C dependencies:

```sh
SDKROOT="$(xcrun --show-sdk-path)" CC=/usr/bin/clang CFLAGS="-isysroot $(xcrun --show-sdk-path)" cargo build --workspace
```

Use that same prefix on **every Cargo command** shown in the references on this host. Do not change global compiler settings. Other platforms use their declared native toolchain; they are separate, unobserved targets.

On Apple Silicon, `.cargo/config.toml` routes Cargo/nextest execution through `scripts/cargo-debug-signing-runner.sh`. It gives the app and tests stable, separate debug identities. Signing is best effort: a warning means the original binary still runs and LuLu may require approval. Record that condition; do not replace the runner or relax gates.

Read-only health check: compare the target identity again, confirm the selected harness and fixtures exist, and check prerequisites without displaying credentials. The local test result must contain nonzero executed tests. The CLI's `sunstack-rule: ready` line witnesses signal registration; usable readiness requires a completed `operation_result` envelope with the expected outcome and actual exit, not `--help` or the ready line alone. V11 requires a completed validated public-API decision run with receipt. Before hosted CLI work, run the signing preflight in `references/milestone-1.md`; afterwards inspect every hosted child's `stderr.log` for the runner's `runs unsigned` warning and apply that route's host signing/egress classification. That warning cannot stop the child's dispatch, and `--fail-fast` does not act on it.

## Evidence, cleanup and return

Keep run evidence in an assignment-authorized directory under system temp, conventionally `yyyy-mm-rule-engine-verify-<run-id>/`. Record the absolute directory on the work thread. Store the exact command, working directory, product/design revisions, configuration, exit code, test counts/run ID, raw output and safe receipts. For process runs keep stdin fixtures, stdout JSON and stderr separately, plus report/cursor readbacks. Use synthetic/public fixtures: results may contain selected evidence.

Supply `OPENROUTER_API_KEY` from the owner's secret store only for the hosted command. Keep the value in process memory; disable shell tracing. Never put a credential value, its location or a secret reference into this skill, artifacts or reports. A missing credential or a skipped hosted test leaves the claim **unknown**. Controlled HTTP proves transport behavior and leaves hosted inference open.

Preserve failures and LEAK counts before any rerun. Interpret LEAK markers only through the disposition in `references/library-gates.md`; the gate remains nextest exit 0. Product failures go to the paired Implementer; disputed expectations and target changes go to the Lead. Repair only assigned verification apparatus.

These commands start no persistent service. Await each owned test/process; its harness owns fixture/server teardown. Stop only a process this run started, using its recorded PID/session, and never by name. Leave shared Cargo caches and other sessions alone. Retain evidence; delete only owned disposable files when authorized, with directory removal requiring explicit permission. After cleanup, reopen the receipt/log and verify it survived.

Return each scoped V claim as **supported**, **unsupported** or **unknown**, with observed/reused/draft coverage, target, command/results and evidence path. Reuse earlier evidence only when its product source, dependencies, configuration and exercised route still match. Name all stand-ins, unobserved entry points and draft prerequisites. This recipe reports evidence; the Lead owns acceptance, and no release or production hook claim follows.
