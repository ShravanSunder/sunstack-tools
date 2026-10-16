# Skill lint and handoff monitor — DRAFT

**Open items:** committed S3a rules integration and S3b executable, generated-schema agreement, process/hosted harnesses, a qualified public reviewer transcript fixture, and a published AgentJobPacket schema fixture. The initial target contains neither integrated rules nor an operating executable. The reviewed invocation examples are design inputs, not runnable commands: handoff-monitor names an illustrative host transcript and a schema path absent from the examples.

Oracle: Specification C2/C6/C9–C14 and V3/V7/V9/V12/V14/V20–V23/V29–V35; Program Design planning/scheduling/reduction and exact contracts. Read adopted `examples/skill-lint.invocation.json`, `handoff-monitor.invocation.json`, their RuleSets/rule files and `agent-handoff.router.json`. After a target lands, read its real harness and replace this draft with source-checked process commands. Use the executable envelope's `evaluate_rules`, not an invented subcommand.

Inputs: declared commit/build/client, fixed monorepo-root CWD, controlled public/synthetic fixtures and authorized genuine Jev. Return parsed report, exits, captures/locations, request/receipt references, origin and measurements. Use the exact contracts' fields, not abbreviated shape summaries in design prose.

## Skill lint — V30

Use `skill-lint.invocation.json` as the invocation input, wrapped in protocol version 1 with operation `evaluate_rules`. Its explicit definition root is the examples directory; repository `sunstack-tools` has root `.` and admits only the example `SKILL.md`. It binds document `skill` to that file. Keep the real paragraph targets U001/U002/U003.

Observe three distinct occurrences: `skill.activation`, `skill.output.specificity`, `skill.verification.present`. The report must correlate native answers and paragraph spans to their rule/group. Compare span bytes to the captured fixture, not later live reads. The first uses a noul verdict and informational choice; the second has a composite of score/noul; the third is block with threshold 0.8/band 0.3. Severity defaults to nudge where omitted. Require genuine Jev receipts and complete report/exit 0 for a complete evaluation; do not prescribe the model's semantic values. Check code-side verdict arithmetic from its observed answers.

## Handoff monitor — V29, V22 and V34

Use the handoff invocation with explicit hook origin and its reviewer commission payload. Replace the illustrative transcript with a **qualified fixture**, not a live provider session/store; bind only that supplied path. Supply the actual generated `agent-job-packet` schema through the catalog. Keep payload case and schema ownership. No host hook is installed.

Drive router → applicable RuleSet/group → declared conversation/payload selections → genuine Jev → report. Observe native Choice confidence/floor and route wiring; hook origin must survive exactly as provenance, with missing observations explicit. It grants no source access. For a routed review handback, trace the three `reviewer.*` rules to selected tool records, commission sources and final whole assistant message; never claim that a tool call proves understanding. Follow judgment references into request answer maps, one receipt/attempt history per actual request and returned transcript cursor.

The fixture/model may legitimately select a different label or fall below the floor. That result can prove its router outcome but does **not** complete the routed-handback journey; retain it and report that route still unobserved. Controlled HTTP provides deterministic boundary controls; genuine Jev must separately reach the required routing and applicability surfaces.

## Controls the integrated harness must supply

| Rows | Deciding observations |
| --- | --- |
| V3/V20/V21 | Invalid definitions/bindings refused before evidence reads/calls, nonempty/tag/duplicate/wiring rules, same ID across different RuleSets allowed; three groups differ, identical groups remain distinct, limits cumulative. Include normalized duplicate RuleSet/within-label route refusal and cross-label reuse allowance. |
| V7/V22 | Conditions true/false/unresolved; thresholds inclusive; failed decision gives incomplete/no rules. For `[D,C]`, D nonpass/fail/cancel means zero C scans; passing D reaches C. Missing payload field is unresolved. Required evidence refusal stays per owner and is encoded only for surviving questions; all-refused is `not_sent/evidence_refused`. |
| V23/V33/V35 | Failure carries no verdict; informational `verdict` name cannot overwrite verdict binding. At threshold0.8/band0.3, values0.30/0.31/0.79/0.80 give block/nudge/nudge/pass. Composite dimensions are own noul/score only; weights positive/sum1; no invented noul confidence. Invalid informational answer invalidates the whole combined response. |
| V9/V31 | Actual encoded bodies preserve owner labels/projections and deduplicated attributions, isolate RuleSets and common instructions, exclude policy metadata. One request per group unless deterministic rule-boundary splits; reassess each split; oversized single rule refused. Same-source aliases ride; bound-but-unread inputs stage; one controlling gate/router answer across candidates/splits. |
| V22/V31 | With speculation off, `[A(same), B(condition), C(other)]` asks A alone, then B, then C, then rules. Stopped B causes zero C-only reads. Adjacent different-input G/H remain separate gates. A routed candidate's scanning condition waits for router and every earlier pass; nonroute/other label means zero scan. Acquisition-free pruning cannot override earlier gate failure or lose the single controlling router slot. |
| V32 | Fixed independent canned answers reduce equivalently under staged/speculative/split modes; randomized completion order cannot change dispositions. With gate nonpass, early/late speculative failure both give not applicable with failure/cost retained; with gate passing, incomplete/evaluation failed. Discarded answers never become verdicts. |
| V34 | Confidence0.59/0.60 around floor0.6 gives uncertain/routed. Distribution[0.6,0.3,0.1] has native confidence0.4 and does not route. Uncertain/no-route complete with zero occurrences/exit0; router failure incomplete/exit3. Wiring document/conversation `subject` names stays owner-specific. |
| V12/V14 | Same definitions in lint/eval with independent expectations and failure-to-pass controls. Declare cold/warm workload, local/provider measurements and request comparator before running; preserve selection correctness; claim no unmeasured SLA/savings. |

Also observe staged rules using **one** EvidenceSession: mutate/delete/replace only after first capture completion, then later alias selections retain original bytes/locations; distinct acquisitions consume cumulative limits. A native session test alone does not establish rules integration. Use executable-operations' fixed-CWD relative transcript/cursor control.

Complete activation only after both real executable journeys and genuine V22/V29/V30/V34 paths have evidence, plus relevant deterministic controls. V11 alone is insufficient. Leave every unmet route explicit; no production hook, installed-host qualification or semantic-accuracy claim follows.
