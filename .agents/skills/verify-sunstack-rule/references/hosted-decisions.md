# Hosted Jev — native decisions and rules

Select the V11 public decisions route or the V22/V34 native rules route below. Their oracle is Specification C4/C7/C10/C13 and V11/V22/V34 at the main skill's pinned revision, read with `git show`. Inputs: pinned source, working SDK/toolchain, authorized network access and hosted spend, owner's credential supplied in memory, and evidence directory. Return the selected claim, run ID or applicable reused observation, command exit/counts, native answer checks and safe execution receipt. A default ignored-test skip leaves the hosted claim unknown.

The V11 and hosted-rules test binaries run under the tests signing identity, which also needs LuLu approval; an unapproved identity can stall about 60 seconds and is classified as host egress, as for the CLI, while the signing preflight remains scoped to hosted CLI work. Timeout alone does not establish which host component blocked traffic.

## V11: launch and drive

Read `crates/sunstack-decisions/tests/hosted_jev.rs` before running. It must still select the actual `JevOpenRouterBackend`, endpoint `https://openrouter.ai/api/v1/systemone`, model `jev-latest`, and exported `run_judgments`; a local server or backend double does not satisfy this route. Confirm the product HEAD and health checks from `SKILL.md`.

Supply `OPENROUTER_API_KEY` from the owner's secret store to this command alone. Keep shell tracing disabled and never print the environment. From the monorepo root, with the SDK prefix from `SKILL.md`:

```sh
cargo nextest run -p sunstack-decisions --test hosted_jev --run-ignored all --success-output immediate
```

`--success-output immediate` preserves the test's safe `hosted receipt:` output. Keep the output and exit status, not just a success summary. The existing test fails when an explicitly requested hosted run lacks its credential; do not replace that failure with a pass. Without authorized credentials/network, report the prerequisite and leave V11 unknown.

The fixture has one shared message and three named questions: `is_blue` (noul), `sky_color` (choice with blue/red labels) and `warmth` (three-level score). Instructions include structured JSON and criteria include literal null. The test configures one 60,000 ms attempt, no retries, finite response/request limits and explicit estimated accounting. Do not silently change those limits to make a failure pass.

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
