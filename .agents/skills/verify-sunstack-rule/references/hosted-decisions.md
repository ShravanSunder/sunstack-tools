# Hosted Jev decisions — V11

This route observes one genuine Rust public-API call, as required by Specification C4/C7 and V11. Inputs: pinned source, working SDK/toolchain, authorized network access and hosted spend, owner's credential supplied in memory, and evidence directory. Return run ID, command exit/counts and the safe execution receipt.

## Launch and drive

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

Await nextest completion; no persistent app/server is started. Ensure credential injection ends with the command. Open the saved log/receipt after cleanup. Complete only when V11 has a real result or named access gap at the pinned target. V22/V29/V30/V34 need the separate integrated milestone 1 routes; V11 never substitutes for them. Real tokenizer agreement remains unverified.
