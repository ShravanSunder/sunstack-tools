# Engine evidence additions: Verification Design

How each requirement is shown to hold. Real-input rows run the built CLI from the branch, as the spike did. The spike's saved requests in scratch `sunstack-evidence-spike/` are starting fixtures. Owner tests use the engine's existing harnesses (`crates/*/tests/`), with no new framework. Revised after design review (F5–F9).

| Requirement | Proven by | Target and journey | Passes when (a wrong build fails) | Stays unverified |
| --- | --- | --- | --- | --- |
| U33 | V36 | `sunstack exec prepare_evidence` on the Tool Portal Implementer rollout. Evidence class: the preparation path, which has no decision backend (`operation_execution.rs:54`), not a counter | The `exec` call at line 311 is a tool use of category exactly `shell`, input containing `ctx.storage.put`, paired with its output. Its outcome matches what that output states, read from the record before the run. A build that classifies by input (calling it `edit_file`) or leaves it a gap fails | Custom tool names not seen in real rollouts |
| U32 (Codex) | V37 | `evaluate_rules`, mechanical conversation rule, on the same rollout and on the lint pilot Implementer's rollout (`01a16f99`) as the control | Bad: fail, with the line-311 call among the items. Control: pass, with the pattern-exclusion count equal to its eligible tool uses (counted independently). A build that ignores the pattern fails the control | Conduct-rule fitness, which belongs to the role rules |
| U32 (Claude) | V38 | `evaluate_rules`, edit-file tool use with input pattern `node:sqlite`, on transcript `2febb449…jsonl`, with `6852afea…jsonl` as the control | Bad: fail. Items include `toolu_01TmbsRetrfucicnDwsKG2QT`. The item count equals an independent script's count of distinct matching calls. Control: pass, with exclusion count 4. A build that drops items or ignores the pattern fails | none beyond V37's |
| U34 | V39 | A scratch Git repository with snapshots A, B, C and D (specification V39), outside every worktree | A→B fails at the call's line. B→C passes. B→D fails at the changed line. `removed` B→A fails at the before-side line. `after_file` B→C fails at the unchanged call. Structured results and exact locations are compared, not only exit codes. A whole-file build fails B→C; an after-only build fails the `removed` case | Languages beyond TypeScript |
| U35 | V37–V39 reports, V41 | the same runs | Every item's location is listed. A conversation item gives both its record ID and its capture position; a change item gives its source with side and span. Each `evaluate_rules` run reports zero model requests and zero inference attempts | none |
| (admission) | V40 | Definitions with an invalid pattern, missing sides and empty sides, each beside a valid admitted peer, through `sunstack exec` `evaluate_rules` and a standalone `prepare_evidence`. Plus a valid conversation RuleSet given to `sunstack lint` | exit 2 before any source is read for each invalid definition, while its peer is admitted; lint's unchanged unsupported-input refusal | none |
| (settlement) | V41 | owner tests in `crates/sunstack-rules/tests/mechanical_execution.rs`: `present: true` and `false` × empty and nonempty × complete and incomplete, multiple items, and a non-presence check on these targets | Each R45 row gives its stated result. Builds that always apply `present: false`, ignore completeness, list only the first item, or admit another check each fail a row | none |

Owner tests beside the real-input runs:

- `transcript_normalization` tests for the custom arms, including an orphan output;
- `conversation_selection` tests for the input pattern, with a string and a structured input containing quotes and a newline (each with one specified answer), and an omission count separated from name and projection exclusions;
- `frozen_queries` tests for each side;
- admission tests for the accepted and refused pairs.

The existing suites stay unchanged and passing. The paired Verifier observes the real-input runs once, at the implementation head.
