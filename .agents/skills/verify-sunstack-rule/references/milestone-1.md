# Skill lint and handoff monitor — milestone 1

Run against product `919ca49712b7aba527bccd6e413ff27f2beb9738`. Oracle: Specification C7/C9–C14 and V2/V13/V14/V29/V30 at `a08839b`, read using the main skill's pinned `git show` method. Each cited Specification row is the full checklist; the recipe and control tables do not narrow it. These are executable/contract journeys, not semantic-accuracy or live-hook qualification.

MUST load `executable-operations.md` and return its declared binary/readiness, local process coverage and relative transcript/cursor control result before relying on this route's process claims. Use the same target/run where coverage overlaps. Inputs: SDK setup, declared checkout/config/client/CWD, authorized evidence storage, and separate network/spend/credential authority for the hosted branch. Expected behavior comes from the oracle, not from a favorable previous model answer.

## Local journeys and fixed measurements

The default CLI or workspace local suite already runs the four `process_journeys` cases. IF that same-target run is unavailable, run the focused command once, with the SDK prefix and monorepo-root CWD:

```sh
cargo nextest run -p sunstack-rule-cli --test process_journeys
```

Require exit 0/four passes. Observe both datasets prepare without dispatch, router preparation not_evaluated, then three intended judged occurrences with native answers/severities/locations; uncertainty/no-route complete with no occurrences and exit 0; router failure incomplete/3 with no fabricated verdict. A working controlled HTTP observer and actual request counts keep zero-dispatch checks meaningful. Local helpers deliberately replace the credential and remote replies; they cannot close genuine hosted V29/V30.

For V14, run the established finite workload once:

```sh
cargo nextest run -p sunstack-rule-cli --test journey_measurements --run-ignored all --success-output immediate
```

It executes **40 processes / 70 controlled HTTP requests**: skill question budgets 32 versus 2, handoff 32 versus 4, five ordered first/repeat pairs per journey/variant. Both members start a new process/EvidenceSession. Assert the same occurrence keys, targets, judgments and question-owner multiset across variants. Request counts are skill 1 versus 3 and handoff 1 versus 2. Preserve all rows, outliers and min/median/max/sample-SD summaries, plus native local/provider/read/attempt counters. These are smaller-question-budget comparators, not an independent per-rule implementation. First/repeat labels do not establish cold storage or model-cache state, and provider elapsed may sum concurrency rather than be subtracted from wall time. No SLA, speedup or savings percentage follows.

Both local and hosted journey fixtures explicitly override reviewed attempt_timeout_ms=10000 to **60000**, retaining retry_count=0. This is the accepted harness setting, not a value to increase after failure. Other limits/budgets remain as declared in `crates/sunstack-rule-cli/tests/support/journey_fixtures.rs`.

## Hosted campaign: sign before spending

This branch requires authorization for one finite campaign, owner-supplied `OPENROUTER_API_KEY` from the owner's secret store and working egress for the CLI identity. Keep tracing disabled and inject the key into **only** the final command's environment. Never record its value, location or a secret reference. The controlled helper must never replace the genuine hosted path.

Build the hosted harness configuration without dispatch, then preflight that signing works for the CLI identity. This is a setup check, not a binding to the binary that will run later. Apply the SDK prefix to the Cargo command; use the unchanged official runner to retain its existing debug identity:

```sh
cargo nextest list -p sunstack-rule-cli --test hosted_journeys
scripts/cargo-debug-signing-runner.sh --sign-only target/debug/sunstack-rule
codesign -dv target/debug/sunstack-rule
codesign --verify target/debug/sunstack-rule
```

The commands above use Cargo's default target directory. If the assignment sets CARGO_TARGET_DIR or a target triple, check the actual `CARGO_BIN_EXE_sunstack-rule` path reported by that build/harness. Require identifier `dev.shravansunder.sunstack-rule.debug`, team `974QD84WVC`, codesign verification exit 0 and **no preflight signing-runner warning**; a warning or mismatch here stops before starting the hosted command. The official runner checks and, if needed, re-signs each child; Cargo may also rebuild. Record paths/hashes and signatures before/after, but attribute a hash change only when its mechanism is established, and never infer immutable artifact identity from the preflight. No firewall/proxy/alternate-identity changes are part of this recipe.

With the favorable preflight and the one-command credential, run exactly one serial, fail-fast campaign:

```sh
cargo nextest run -p sunstack-rule-cli --test hosted_journeys --run-ignored all --test-threads 1 --fail-fast --success-output immediate
```

Two ignored tests each make a fixed first/repeat pair: **four genuine executable invocations total**, one attempt per request, zero retries. Failure, timeout or uncertain/nonroute outcome that cannot close the routed journey fails its test; preserve which invocations actually ran and do not retry until favorable. The ignored `hosted_request_diagnosis` is an offline preserved-request encoder observation and is not this journey or a fallback hosted route.

Inspect every hosted child's `target/cli-process-evidence/<test-pid>-<sequence>/stderr.log` for the runner's `runs unsigned` line. An observed warning is a host signing/egress setup condition: the campaign cannot confirm the signed-binary setup. Retain every invocation, its outcome and any unknown remote effect. Genuine admitted answers/receipts remain evidence for the narrow paths exercised, including warning-free children's observations; the warning neither erases those results nor establishes an adapter or model defect. The runner warns and still execs, `response()` accepts short warning-bearing stderr, the first/repeat loop does not check it, and `--fail-fast` does not act on the warning. Readback cannot stop that campaign: later children may already have run. No further campaign runs without a new Lead decision. A within-campaign or pre-dispatch warning stop would require a driver change, which this skill does not claim.

A binary lacking the intended signature/egress can stall roughly 60 seconds and end with timeout/remote_outcome_unknown and no receipt. Record that host-egress setup condition, not a completed provider result. A matching signature alone does not prove network access, and timeout alone does not prove LuLu dropped traffic; definitive historical flow cause remains unobserved. An HTTP result from a direct probe or alternate identity cannot replace the real binary journey. Keep failed campaigns and unknown remote effects separate from later admitted receipts; cancellation/timeout cannot undo inference or billing.

## Inputs and deciding report readbacks

`crates/sunstack-rule-cli/tests/support/journey_fixtures.rs` loads the reviewed examples without editing them. It supplies a declared endpoint, 60000 ms attempt bound and zero retries. Handoff replaces illustrative host paths with a temporary fixture built from the accepted public Codex composition plus **synthetic** reviewer request/read attempts/final handback; it uses the actual generated AgentJobPacket schema fixture and its recorded provenance in `crates/sunstack-evidence/tests/fixtures/README.md`. It does not read a live provider session or install a hook. Fingerprints bind the ten reviewed files, schema and composed transcript before/after each observation.

| Journey | Observe from the actual complete_report, not just exit 0 |
| --- | --- |
| Skill lint, V30 | Exactly skill.activation, skill.output.specificity and skill.verification.present; informational task_kind, specificity and names_file; captured U001/U002/U003 paragraph bytes/spans and source hash; nudge/nudge/block declared severity. Resolve answer references into completed requests and independently apply noul/composite arithmetic, thresholds and band. Favorable verdicts are not required. |
| Handoff monitor, V29 | Genuine router reaches review_handback and all three reviewer.* judgments; declared block/block/nudge severities; source-read/change/final-message target record IDs, whole final assistant message, generated commission schema, hook host/event/session/turn/subagent provenance and canonical-bound-source cursor. Read attempts are fixture evidence, not proof an agent understood a document. |
| Each hosted invocation | operation_result/evaluate_rules/complete_report, exit 0; exactly one **sent** request and one completed HTTP200 attempt with admitted resolved_model/native usage/optional cost. Planned not_sent candidates may remain; do not count all planned records as network calls. Resolve every answer reference; model values may differ legitimately. |

The distribution/floor remains native Choice confidence. Legitimate uncertain/no-route outcomes are complete/0 with zero occurrences, but do not close routed V29; a router failure is incomplete/3. Keep raw results and return the missing path rather than manufacturing verdicts or repeating calls. A rule fail with block activated still exits 0. The exact contracts' SelectedReason paragraph exempts accepted milestone 1 shapes from the milestone 2 change-scope fields/variants: `change_selections`, judged `selected_by`, occurrence/preparation `selection` and `outside_change_scope`. PreparationFailed is covered only by the explicitly limited native serializer/exit seam in the executable recipe; runtime reachability remains pending an owner proof-gate decision.

## Evidence, teardown and completion

Record nextest run IDs, exact commands/config/CWD, child exits and test counts. The process evidence directories are documented in `executable-operations.md`. `target/cli-process-observations/<test-pid>-v14-controlled.json` retains 40 measurement rows and summaries; frozen skill/handoff files retain inputs/fingerprints. Hosted `<test-pid>-hosted-{skill,handoff}-{frozen,cold_session_first,warm_path_repeat,pair}.json` files retain safe inputs/reports/receipts. Child stdout/result is the independent readback for those pair summaries. Exclude authorization headers/environment credentials. Sum only admitted receipt costs; unknown earlier attempts stay unknown.

Copy this run's artifacts/logs to the granted evidence directory before cleanup, then reopen them. The harness owns/joined its children and fixture servers; fixture directories are temporary. Leave shared caches, unrelated processes and evidence alone. No persistent app or hook is installed.

Return local journeys, V14 and hosted V29/V30 separately as supported/unsupported/unknown, with observed/reused coverage and remaining gaps. Complete only for the scoped paths actually reached. Missing credential/egress is a prerequisite gap, not a green hosted result. Installed hosts/hooks, semantic accuracy, platform/release artifacts, exact tokenizer agreement and milestone 2 remain outside this milestone 1 confirmation.
