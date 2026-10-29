# Hosted Jev — native decisions and rules

Select the historical V11 public decisions or V22/V34 native rules replay below, or current runner S3 at the end. The V routes retain their original target/oracle pins and vocabulary from SKILL.md and establish no current-runner result. Their oracle is Specification C4/C7/C10/C13 and V11/V22/V34 at that historical revision, read with `git show`. Inputs: pinned source, working SDK/toolchain, authorized network access and hosted spend, owner's credential supplied in memory, and evidence directory. Return the selected claim, run ID or applicable reused observation, command exit/counts, native answer checks and safe execution receipt. A default ignored-test skip leaves the hosted claim unknown.

Before either hosted-test command below, add `--no-run` to build only the selected test target, without the owner's credential. List that same target with `cargo nextest list -p <selected-crate> --test <selected-test> --list-type binaries-only --message-format json` and take its exact `binary-path`; do not guess a hashed filename. Run `scripts/cargo-debug-signing-runner.sh --sign-only <exact-built-test-binary>`, `codesign -dv <exact-built-test-binary>` and `codesign --verify <exact-built-test-binary>`. Require identifier `dev.shravansunder.sunstack-tools-tests.debug`, the approved local signing team (supply `SUNSTACK_DEBUG_SIGNING_TEAM` for hosted proof), verification exit 0 and no preflight runner warning. Warning, mismatch or failed verification stops before the hosted command, with no request spent. After a passed preflight, a stall of roughly 60 seconds ending in `timeout`/`remote_outcome_unknown` is a finding for the Lead with cause unknown, not an established host-egress cause. `milestone-1.md` owns the corresponding CLI preflight/stall case. Preflight checks setup rather than an immutable later binary: Cargo may rebuild and the unchanged runner checks each execution; retain later warnings and the actual admitted outcomes separately.

The test identity needs its own LuLu approval, separate from the CLI identity.

## V11: launch and drive

Read `crates/sunstack-decisions/tests/hosted_jev.rs` before running. It must still select the actual `JevOpenRouterBackend`, endpoint `https://openrouter.ai/api/v1/systemone`, model `jev-latest`, and exported `run_judgments`; a local server or backend double does not satisfy this route. Confirm the product HEAD and health checks from `SKILL.md`.

Supply `OPENROUTER_API_KEY` from the owner's secret store to this command alone. Keep shell tracing disabled and never print the environment. From the monorepo root, with the SDK prefix from `SKILL.md`:

```sh
cargo nextest run -p sunstack-decisions --test hosted_jev --run-ignored all --success-output immediate
```

`--success-output immediate` preserves the test's safe `hosted receipt:` output. Keep the output and exit status, not just a success summary. The existing test fails when an explicitly requested hosted run lacks its credential; do not replace that failure with a pass. Without authorized credentials/network, report the prerequisite and leave V11 unknown.

The fixture has one shared message and three named questions: `is_blue` (noul), `sky_color` (choice with blue/red labels) and `warmth` (three-level score). Instructions include structured JSON; the literal null is a **choice criterion**. The test configures one 60,000 ms attempt, no retries, finite response/request limits and explicit estimated accounting. Do not silently change those limits to make a failure pass.

**H01 observed hosted limitation:** direct `run_judgments` calls with noul instructions root null, noul true/false criteria root null or score entries null were refused with HTTP 400; choice-criterion null was admitted. This bounds the observed hosted positions, not native rich-JSON/null admission. Rule question instructions are wrapped objects and null common instructions are omitted by the rules encoder; do not generalize the direct-judgment instructions limitation to a RuleSet. Existing admitted fixtures, US02 and US21 avoid rejected positions; no extra diagnosis, contract narrowing or new hosted call follows from this note.

## Observe the result

Require a non-skipped execution of `hosted_mixed_native_primitives_return_validated_model_and_usage`, exit 0, one completed attempt and exactly the three typed native answers. The test asserts answer count/kinds, positive input usage and attempt count. Request-relative admission additionally checks names, kinds, label/probability keys, score/rubric indices, finite ranges and receipt shape. Save the printed `resolved_model` and native `input_tokens`, `output_tokens` and optional `cost_usd`. Cost missing means unavailable, never zero.

The expected result is **valid correlated native answers plus model and usage**, not predetermined semantic values or a particular dated resolved model. Different allowed answers or a newer resolved Jev model may pass. HTTP success with missing/wrong answers, an invalid receipt or a non-completed outcome must fail. A control that drops validation could remain green for a favorable hosted response; local V8/V9 rejection tests cover malformed responses. This one hosted call does not establish model accuracy or all validator sensitivity.

Source path: exported API (`src/lib.rs`) → `judgment_execution::run_judgments` validates/encodes/budgets → `run_admitted_judgments` owns the bounded attempt → `decision_backends::dispatch` sends the admitted bytes over reqwest → `judgment_validation::decode_jev_response/validate_response` → completed answers and receipt. The real provider is the external edge. The credential stays in the transport's sensitive authorization header, outside serialized requests/results.

## Failure and cleanup

Keep safe failure class, status, diagnostics, attempt evidence and run ID. A timeout/cancellation may still have incurred remote inference/billing. Report `provider_resource_unavailable`, authentication/authorization, credits or network failures as observed; none proves an adapter defect by itself. Historical `PROOF.md` access failures do not settle today's access, and a later success does not explain those failures.

Await nextest completion; no persistent app/server is started. Ensure credential injection ends with the command. Open the saved log/receipt after cleanup. V11 completion requires its real result or named access gap. V22/V34 uses the hosted-rules section below; V29/V30 is selected through the main skill's `milestone-1.md` call. V11 never substitutes for them. Real tokenizer agreement remains unverified.

## V22/V34: genuine native rules applicability and routing

Oracle: Specification C10/C13 and V22/V34; the complete rows govern deterministic controls as well as the genuine call. Local threshold/order/isolation/failure controls live in `library-gates.md`, selected by the main skill's mandatory library route. This hosted route proves actual provider use through rules, not every deterministic case or model accuracy.

Read `crates/sunstack-rules/tests/hosted_rules.rs` and `crates/sunstack-rules/tests/support/rule_fixture.rs` before using this route. They call exported `evaluate_rules` with the real evidence/decisions owners, real definition/document fixtures and genuine Jev via `https://openrouter.ai/api/v1/systemone`, model `jev-latest`. The tests set a 60,000 ms attempt timeout, zero retries and max_requests=3. Credentials enter only the native backend's in-memory argument from the command environment, never definitions or requests.

First check for an independent hosted observation that still applies. Read its report and raw run log, then compare its exact product commit with the declared target for `crates/sunstack-rules/src`, `crates/sunstack-rules/tests/hosted_rules.rs`, `crates/sunstack-rules/tests/support`, `crates/sunstack-decisions`, `crates/sunstack-evidence`, their manifests, dependency resolution, toolchain and runner/configuration. Inspect any differences rather than equating a matching test name with applicability. Record unchanged paths and any unrelated changes with their reason. An applicable prior genuine observation may close this route as reused; a static source pass alone cannot. If applicability fails, keep the route available and its genuine claim unknown until a separately authorized observation. Do not spend merely to refresh a matching pass.

When a fresh hosted observation is authorized and needed, use the SDK prefix from `SKILL.md`, run from the monorepo root and supply `OPENROUTER_API_KEY` from the owner's secret store to this command alone, with tracing disabled:

```sh
cargo nextest run -p sunstack-rules --test hosted_rules --run-ignored all --success-output immediate
```

Require exit 0, two non-skipped tests and safe `hosted rules receipt:` output. The assertion checks run inside the public-API consumers; their stdout prints receipts, not full report/answer dumps:

| Genuine case | Required observation |
| --- | --- |
| `hosted_applicability_decision_preserves_native_answer_and_receipt_through_rules` — V22 | CompleteReport contains the evaluated applicability decision; passed or did_not_pass resolves through its request reference to an admitted native Noul answer. Either valid answer is allowed. A failed/unadmitted decision cannot count as hosted success. |
| `hosted_router_preserves_native_choice_confidence_and_source_linked_receipt` — V34 | CompleteReport contains a router outcome routed, uncertain_intent or no_route, referencing an admitted native Choice answer, including its native confidence/distribution. A legitimate uncertain/no-route result proves that native outcome; it does not prove a particular route was reached. Router failure cannot count as completion. |
| Completed requests in either report | At least one genuine admitted receipt, observed resolved_model/native usage, positive input_tokens and exactly one attempt per completed request; absent cost stays unavailable. No fabricated fields or receipt on failed execution. |

Preserve failures, safe diagnostics, attempt uncertainty and actual executed test counts; do not retry until favorable. The existing HTTP/failure controls provide response-admission sensitivity at the cheaper boundary. Hosted passing answers establish no semantic accuracy or exhaustive routing coverage. The fixture harness owns temporary files and joins its request work; await completion, end command-local credential injection and reopen retained evidence after teardown. Return V22 and V34 separately as supported, unsupported or unknown, marking genuine evidence observed or reused and naming any remaining route/setup gap.

## S3 — rulebook questions on genuine Jev without a CA bundle

This current-runner route earns the 29 Oct rulebook Verification Design U6 hosted portion while preserving the earlier static Linux/no-CA boundary. Historical V routes above retain their own pins. **Draft until observed** on a separately declared static Linux arm64 release binary; this Verifier's no-hosted commission cannot execute it.

Inputs: declared binary SHA256, source commit/version, the S0 rulebook fixture tree, cached pinned Alpine arm64 image from `container_matrix.py`, Docker networking and separately authorized hosted spend. The Lead supplies the existing owner-only regular credential file privately; its mode is 600 and the invoking user owns it. Its location and value never enter the skill, fixtures, process command or report. `hosted_lint.py` reads it only into one Docker command's environment, passes the variable by name and removes it afterwards. No credential is read in preparation mode.

From the monorepo root using the main skill's clean uv environment:

```sh
uv run --no-config --no-env-file --no-python-downloads --script \
  .agents/skills/verify-sunstack-rule/scripts/hosted_lint.py \
  --prepare --evidence <new-authorized-directory>
```

For the separately commissioned real observation omit `--prepare`, pass `--binary <static-release-artifact> --sha256 <declared-digest> --commit <full-source-commit> --version <version-without-v>`, and privately supply the authorized credential-file argument. Use a fresh evidence directory. The driver checks static ELF arm64 linkage/hash before credential access. Missing prerequisites leave this hosted claim unknown.

The fixture uses `rulebook/check/evidence/decision_model`, selects verification-design/SKILL.md and carries K001–K003. `decision_model` uses the genuine Jev model/endpoint defaults, with bounded request/concurrency/attempt limits. K002's 450-character description guarantees the mechanical error/exit 1; semantic values of real K001/K003 answers are not fixed in advance. `sunstack lint --config skill-rules.config.json --all --format json` runs in read-only Alpine with empty `/etc/ssl` tmpfs. The child confirms no trust-store file exists, emits `CA_BUNDLE_EMPTY`, and launches with only OS PATH, empty HOME and the command-local key.

Pass requires the declared version/hash, positive real inference usage, request evidence for both K001's and K003's tagged standard, no incomplete parts or skipped rules, and the expected mechanical exit. The real runtime's schema-valid answer admission must be inspected and joined to its native producer/consumer tests; a transport success or aggregate usage without both standard identities cannot substitute. Findings and uncertainty retain native answers where exposed by the report. Do not demand predetermined model judgments or a particular resolved model revision. Optional cost stays absent when unreported. This proves actual admitted hosted interaction, not judgment quality.

The helper performs one invocation and no automatic rerun. Engine attempts remain visible in receipts; one CLI invocation is not one billed attempt. Timeout records remote outcome/billing unknown, safe partial output and the result of removing only the owned container. A credential leak withholds raw output and keeps a safe marker. Preserve failed output before any authorized retry.

After teardown reopen stdout/stderr/process plus safe pass/timeout records, confirm the owned container is gone and credential injection ended. Return U6 supported/unsupported/unknown at the declared artifact, request/usage evidence and gaps. A local debug build, controlled HTTP or preparation pass cannot prove this static hosted target; private CAs, other platforms and consumer #171 coverage remain unverified.
