# Executable operations — DRAFT

**Open item:** a committed `sunstack-rule-cli` process implementation, framing/input bounds, signal handling and permanent process fixtures must be declared as a target. The initial product target has `fn main() {}`. There is no operable process journey here, and no guessed argument syntax or fixture filename should be run.

Oracle: Specification C14/V2/V13/V15 and `program-design-contracts.md` Envelope plus the four operation sections, read at the pinned revision by the method in `SKILL.md`. Each cited Specification row is the full checklist; this table adds harness controls and never narrows a row. V15 artifact integrity is release proof, open unless separately requested. Activation is a separate maintenance assignment that replaces this draft with verified invocations; until then, return this open item. That assignment inspects the landed protocol entry and process harness. Inputs are a declared binary/build/config/client, fixed working directory and real fixture sources. Return stdout, stderr, exit and owned-settlement evidence per case.

## Contract to preserve when activating

One invocation process takes one bounded JSON object on stdin:

```json
{"protocol_version":1,"operation":"prepare_evidence","input":{}}
```

This illustrates the **envelope only**; `{}` is not a valid operation fixture. Use the operation's complete adopted input. A successful framing result is `kind: operation_result`, `protocol_version: 1`, `result: { operation, outcome }`. Protocol failures are `kind: protocol_error` with diagnostics and no known-operation result. Observe parsed stdout JSON, separately captured stderr and the child's actual exit. `--help` and exit 0 alone prove nothing.

| Operation / input | Exit 0 | Exit 2 | Exit 3 | Exit 4 |
| --- | --- | --- | --- | --- |
| `prepare_evidence` / `EvidencePlanInput` | `prepared`, requested selections/captures/cursors/measurements | `refused` | `incomplete` for required evidence refusal | `cancelled`, with correct plan-admission progress |
| `prepare_rules` / `RuleInvocationInput` | `prepared { preparation }` | `invalid` | `prepared_with_failures` or `preparation_failed` | `cancelled`, with correct preparation progress |
| `run_judgments` / `RunJudgmentsInput` | `completed`, correlated answers/receipt/attempts | `invalid_request`, including budget refusal without attempts | `provider_failure` or `invalid_response` | `cancelled`, attempts retained |
| `evaluate_rules` / `RuleInvocationInput` | `complete_report`, whatever verdicts | `invalid` | `incomplete_report` or `preparation_failed` | `cancelled`, before report or with admitted partial report |

Unknown operation/version, malformed framing, duplicate/unknown envelope fields and over-limit stdin must give protocol error with exit 2 and no fabricated operation outcome. Keep the implementation's declared exact finite input limit in the activated recipe.

## Required controls

- `prepare_evidence` and `prepare_rules` make zero provider calls. Establish this through the real process fixture's observed transport, not lack of an environment key. Prepare rules runs only leading conditions (before each owner's first decision), leaves routers/decisions not evaluated, and records request readiness as `prepared { budget, evidence }` or `waiting { on }`. It acquires evidence only for requests evaluation would send before any router/decision answers, under the invocation's speculation setting. A request waiting on a router/decision acquires nothing; prepared requests may encode/assess budget but never dispatch. A non-carrying routed candidate waits with speculation off and runs concurrently with it on; a scan-only leading source outside its request's read set, and its request, wait for routing in both modes.
- Routed candidate gate/entry/request keys carry their route label, and prepared/reported group bindings carry `routed { router_group, label }`; differently wired candidates never share a controlling decision slot. Final occurrence keys remain RuleSet/group/rule ID. Evidence/report `change_selections` contains only bindings whose change selection was resolved; an unused or full-scope context binding has no fabricated entry.
- Prepare evidence exposes only caller-requested selected evidence in stdout; diagnostics/stderr/default logs omit sentinel content and credentials. Check useful closed diagnostics and bounded guidance remain.
- A completed failing block verdict still exits **0**. Failure/incomplete execution has no fabricated verdict or activated severity. Missing/truncated output is execution failure even if the child died quietly.
- Drive SIGINT/SIGTERM through the real process at controlled lifecycle boundaries. Before report admission, cancellation carries no report; after admission it retains completed peers in the partial report. Terminal exit 4 follows owned settlement. A timeout or arbitrary sleep is not proof of that owner boundary; named barriers and process/server readback are open harness requirements.
- State CWD for every run. Definition root uses the executable's CWD; RuleSet paths resolve from that root and rule files from the RuleSet directory. Include a relative transcript binding and relative supplied cursor path control: the same relative spelling from two working directories must identify the intended different source, and continuation must refer to its bound capture. Inspect the landed cursor-input mechanism before writing commands; the current wire contract carries cursor entries, not an assumed `--cursor-file` option.

IF only library evidence is in scope, load `library-gates.md` and return its scoped results; they do not close these process obligations. The separate activation assignment completes only after all four operations' 0/2/3/4 cases, protocol errors, privacy, cancellation and relative-path controls have real process evidence at the declared target.
