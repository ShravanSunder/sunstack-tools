# Decisions proof

The local proof covers native decision contracts, the public Rust API, real
reqwest transport against controlled HTTP, response admission, budget admission,
and owned cancellation. Hosted Jev integration remains unverified: the actual
mixed-primitives request returned HTTP 404, and the Lead's independent probe
identified the workspace guardrail excluding Jev. Keep the endpoint, model and
ignored hosted test unchanged until access is restored.

The adopted sources are the Rust rule-engine delivery plan, Specification
C4/C5/C8 and V8/V9/V10/V11/V13/V19, and the attempt-contract correction at design
commit `7a72d6f`. The execution trace is thread
`01a1446b-0a1b-7c2d-8e3f-4a5b6c7d8e01`.

## Reproduce the local gates

Run from the workspace root using the pinned Rust toolchain:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo tree -p sunstack-decisions -e normal
```

On the observed host, the installed Homebrew clang selected a nonexistent SDK.
The scoped build environment used `SDKROOT="$(xcrun --show-sdk-path)"`,
`CC=/usr/bin/clang`, and `CFLAGS="-isysroot $SDKROOT"`. No global configuration was
changed. The Apple Silicon target runner signs an immutable copy, verifies its
Developer ID/team, and atomically replaces the executable under a per-binary
lock. Signing failure leaves the original executable untouched.

The normal tree contains no Git, AST-grep, jsonschema or sibling Sunstack crates.
Native integration tests consume only the public decisions crate. Controlled
HTTP tests compare assessed bytes with the actual received bytes, retain native
receipt fields, and check bounded bodies and privacy. A synthetic exact counter
proves accounting mechanics; real tokenizer agreement is not claimed.

## Deadline fixture and cancellation

The real-HTTP deadline test retains a 100 ms timeout and one retry. Its dispatch
claim is exactly two attempt records, both `timeout`. Observed TCP activity is a
separate claim: at least one and at most two accepted connections. The deadline
covers the whole dispatch future, so an attempt may time out before reaching
TCP. The fixture stops through an explicit signal and its server task is joined.

The former fixture parsed every connection as a complete HTTP request. Under
scheduling pressure, a timeout could close a connection before sending bytes;
the parser panicked on empty EOF, dropped the listener, and caused a later
connection failure. Baseline runs 10, 21 and 24 reproduced that fixture defect.
The permanent empty-EOF regression failed before the fix in nextest run
`e8b5bfff-03c5-418b-bb9a-00900f7b7432` (exit 100). The shutdown regression failed
before cancellation handling in run `488ad4a9-8de5-49ca-bad7-e3757bbdcee9`
(exit 100). Both are retained. No timeout, attempt budget or leak setting was
relaxed.

Other cases distinguish pre-dispatch cancellation, actual response bytes read
before cancellation, cancellation during backoff, and cancellation after an
already returned completion. Private test-only observation signals hold the
real adapter at the relevant boundary; production contains no observation hook.
The backend-double drop probe proves only its owned future, separately from the
real-HTTP observations.

## Unexplained nextest stdout/stderr-lifetime observation

The Verifier's original full run on `bc9c1ddd`, run
`30e836ae-d809-443e-a2de-597b21b94a1f`, reported 32 passing tests with one `LEAK`:
`cancellation_before_completed_admission_wins_but_later_abort_cannot_rewrite_result`.
Its synchronous-success backend double owns no spawned task, server or child.
The marker therefore carries no R14 product-work claim either way.

Pinned nextest 0.9.137 source checks whether captured stdout/stderr reach EOF
after the child process exits; its default leak timeout is 200 ms. The marker
establishes that EOF was not observed within that deadline, without identifying
a process or descriptor owner. That owner remains unknown.

Bounded investigation observed no repeat marker in:

- 30 unchanged-source workspace invocations (three separately diagnosed
  deadline-fixture failures; run 10 stopped at 30/32 tests under fail-fast,
  while the later baseline used no-fail-fast; the other runs passed).
- 10 corrected-fixture full suites with all ten test artifacts reset to ad-hoc
  signatures before each run, exercising cold Developer ID signing.
- 120 targeted launches through the runner; all exited 0. From `Popen.wait()`
  return to observing both pipe EOFs, the maximum measured latency was 9.019 ms,
  with no pipe still open at the stricter 100 ms probe bound. The probe does not
  recover the kernel exit timestamp or scheduling delay before wait returned.

The Lead's disposition records this as an unexplained, non-reproduced nextest
stdout/stderr-lifetime observation. Attribution is not required for acceptance;
passing reruns are not presented as its root-cause resolution. Raw local records
are retained under `target/s1-proof/leak-investigation/` in this crate, including
baseline, cold-signing and pipe-lifetime observations. The actual collection
procedure and its timing limit are retained in
`marked-case-pipe-lifetimes-procedure.md` alongside the JSON.

**Standing rule:** any future `LEAK` marker in this workspace must be captured
with the participating PID and descriptor ownership before any rerun. Preserve
the run and process evidence; do not tolerate the marker, raise its timeout, or
retry it away. If ownership cannot be established, retain that gap explicitly.

## Remaining limits

Hosted V11 is blocked by provider access, not an absent-key skip or a proven
adapter defect. Controlled HTTP is a substitute for the hosted service. No
provider accuracy, tokenizer agreement, performance target, production hook,
release, or distribution claim is established by the local suite.

## Final local remediation result

Formatting and clippy exited 0. The default full-workspace nextest run
`4da7fb9d-e946-45da-9d3e-be2dc896e77d` exited 0 with **34 passed, one hosted
skip, and zero LEAK markers**. This includes both permanent fixture regressions.
The runtime remains real reqwest/Tokio with the configured signing runner. Raw
output is retained in `target/s1-proof/leak-investigation/final-remediation-nextest.log`.
This result does not close hosted V11 or identify the original descriptor owner.
