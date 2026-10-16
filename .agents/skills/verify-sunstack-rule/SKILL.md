---
name: verify-sunstack-rule
description: Use when verifying the Rust Sunstack rule engine, its evidence and decisions libraries, or the sunstack-rule executable, including hosted Jev and milestone journeys. Not for Demistify or production hook installation.
---

# Verify Sunstack Rule

Verify a named target against the adopted Specification, then report only the outcomes observed there. A passing build, an HTTP success or an unfavorable rule verdict each means something different.

The app is three Rust libraries (`sunstack-evidence`, `sunstack-decisions`, `sunstack-rules`) and the `sunstack-rule` binary from `sunstack-rule-cli`. Evidence owns capture and selection; decisions owns Jev transport and answer admission; rules owns applicability, grouping and verdicts; the executable owns framing, signals and exits. There is no daemon or JS runtime.

## Choose the target and journey

1. From the monorepo root, record `git rev-parse --show-toplevel`, `git branch --show-current`, `git rev-parse HEAD` and `git status --short`. Record the declared product commit, build location, configuration and calling client. Stop if the target changes during observation.
2. Read the adopted `docs/specs/2026-10-12-rust-rule-engine/specification.md`, especially the selected V rows, and `program-design-contracts.md` for exact shapes. The initial oracle is design revision `8ae37613ff86dcad812a85bd51d597999ae85e59`; the initial product source is `6d4a87acd9dbb15d9c6a26e3dbcde526ba9ffd93`. These are separate revisions. If the product checkout has older design text, use the declared design checkout at the adopted revision. Resolve conflicting expectations with the commissioning Lead before driving the affected case.
3. Select the route below. Read it before running. A draft is a coverage obligation with an open prerequisite, never an executable recipe or a passing observation.

| Surface | Route | Availability at the initial product target |
| --- | --- | --- |
| Native libraries and Rust quality | `references/library-gates.md` | Existing evidence/decisions harnesses; rules composition remains open |
| Genuine Jev mixed primitives, V11 | `references/hosted-decisions.md` | Credential-gated real public-API test |
| Four executable operations, V2 | `references/executable-operations.md` | **DRAFT:** process implementation and fixtures needed |
| Skill lint and handoff monitor | `references/milestone-1.md` | **DRAFT:** integrated rules, executable and hosted fixtures needed |
| Git changes, relationships and chunks | `references/milestone-2.md` | **DRAFT:** milestone 2 target and real fixtures needed |

MUST load `references/library-gates.md` and return the scoped commands, results and remaining coverage before relying on library or build claims.
IF hosted V11 is in scope, load `references/hosted-decisions.md` and return its run identity, validated completion and receipt, or its access/setup gap.
IF executable behavior is in scope, load `references/executable-operations.md` and return the operation coverage or each draft prerequisite.
IF milestone 1 journeys are in scope, load `references/milestone-1.md` and return each journey's observed report or named open item.
IF change scopes or connected evidence are in scope, load `references/milestone-2.md` and return its real-fixture coverage or named open item.

## Build and readiness

Run Cargo from the monorepo root with the pinned toolchain in `rust-toolchain.toml`. On macOS this host needs a command-local SDK correction for C dependencies:

```sh
SDKROOT="$(xcrun --show-sdk-path)" CC=/usr/bin/clang CFLAGS="-isysroot $(xcrun --show-sdk-path)" cargo build --workspace
```

Use that same prefix on **every Cargo command** shown in the references on this host. Do not change global compiler settings. Other platforms use their declared native toolchain; they are separate, unobserved targets.

On Apple Silicon, `.cargo/config.toml` routes Cargo/nextest execution through `scripts/cargo-debug-signing-runner.sh`. It gives the app and tests stable, separate debug identities. Signing is best effort: a warning means the original binary still runs and LuLu may require approval. Record that condition; do not replace the runner or relax gates.

Read-only health check: compare the target identity again, confirm the selected harness and fixtures exist, and check prerequisites without displaying credentials. The local test result must contain nonzero executed tests. For V11, readiness is a completed, validated public-API decision run with receipt. For a future process target it is a completed `operation_result` envelope, not `--help`, an empty binary exit or a port response. The initial `crates/sunstack-rule-cli/src/main.rs` is a placeholder; building it proves no process behavior.

## Evidence, cleanup and return

Keep run evidence in an assignment-authorized directory under system temp, conventionally `yyyy-mm-rule-engine-verify-<run-id>/`. Record the absolute directory on the work thread. Store the exact command, working directory, product/design revisions, configuration, exit code, test counts/run ID, raw output and safe receipts. For process runs keep stdin fixtures, stdout JSON and stderr separately, plus report/cursor readbacks. Use synthetic/public fixtures: results may contain selected evidence.

Supply `OPENROUTER_API_KEY` from the owner's secret store only for the hosted command. Keep the value in process memory; disable shell tracing. Never put a credential value, its location or a secret reference into this skill, artifacts or reports. A missing credential or a skipped hosted test is **unverified**. Controlled HTTP proves transport behavior and leaves hosted inference open.

Preserve any failure before a rerun. A nextest `LEAK` requires participating PID and descriptor evidence before any rerun; do not retry it away or raise its timeout. Report an unknown owner as unknown. Product failures go to the paired Implementer; disputed expectations and target changes go to the Lead. Repair only assigned verification apparatus.

These commands start no persistent service. Await each owned test/process; its harness owns fixture/server teardown. Stop only a process this run started, using its recorded PID/session, and never by name. Leave shared Cargo caches and other sessions alone. Retain evidence; delete only owned disposable files when authorized, with directory removal requiring explicit permission. After cleanup, reopen the receipt/log and verify it survived.

Return each scoped V claim as **supported**, **unsupported** or **unknown**, with observed/reused/draft coverage, target, command/results and evidence path. Reuse earlier evidence only when its product source, dependencies, configuration and exercised route still match. Name all stand-ins, unobserved entry points and draft prerequisites. This recipe reports evidence; the Lead owns acceptance, and no release or production hook claim follows.
