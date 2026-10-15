# Worked examples: from files to verdicts

Both milestone 1 journeys, traced from authored files to one request and then to results. The files are in [`examples/`](examples/) and validate against [`schemas/`](schemas/). The answers below are illustrative canned values, the kind the V32 equivalence oracle uses, not model output.

## Skill lint

**Files:**
- [`skill-quality.ruleset.json`](examples/skill-quality.ruleset.json): one `document` input named `skill`, no applicability, and common instructions.
- [`skill-rules.jsonl`](examples/skill-rules.jsonl): three rules over paragraphs U001–U003 of [`SKILL.md`](examples/SKILL.md).
- [`skill-lint.invocation.json`](examples/skill-lint.invocation.json): binds `skill` to that document in one group, with origin `direct`.

**Plan.** One RuleSet × one group, no gates, so there is one `rules` request with five questions:

| Request name | Owner | Kind |
| --- | --- | --- |
| `q000001` | verdict of `skill.activation` | noul |
| `q000002` | informational `skill.activation` / `task_kind` | choice |
| `q000003` | informational `skill.output.specificity` / `names_file` | noul |
| `q000004` | informational `skill.output.specificity` / `specificity` | score (0–3) |
| `q000005` | verdict of `skill.verification.present` | noul |

`skill.output.specificity` has a composite verdict, so it adds no verdict question. Its verdict reads `specificity` and `names_file`.

Every question's instructions arrive wrapped. Here is `q000005`:

```json
{
  "owner": "rule:skill.verification.present",
  "reads": ["rule:skill.verification.present/verification_paragraph"],
  "common_ruleset_instructions": { "context": "You are reviewing one paragraph of an agent skill. Judge only the paragraph labeled as this rule's target." },
  "question": "Does this paragraph require checking the produced output before it is returned?"
}
```

The state holds three labeled paragraphs (`.../activation_paragraph`, `.../output_paragraph`, `.../verification_paragraph`), each with its text, document and span.

**Reduction** with illustrative answers:

| Rule | Answers | Verdict | Severity |
| --- | --- | --- | --- |
| `skill.activation` | `q000001` yes 0.91 | pass (0.91 ≥ 0.7) | none |
| `skill.output.specificity` | `specificity` 2.0 of 3 → 0.667; `names_file` yes 0.88 | composite 0.75×0.667 + 0.25×0.88 = 0.72 → pass (≥ 0.67) | none |
| `skill.verification.present` | `q000005` yes 0.42 | fail (0.42 < 0.8) | block rule, but 0.42 is above the 0.3 band, so it acts as nudge (`nudge_above_band`) |

The report is complete (exit 0), with one nudge. At 0.25 the same rule would block. Every native answer stays in the report, and `task_kind` is informational: no verdict depends on it.

## Handoff monitor

**Files:**
- [`agent-handoff.router.json`](examples/agent-handoff.router.json): the Specification C13 router.
- [`review-handoff.ruleset.json`](examples/review-handoff.ruleset.json) and [`review-handoff-rules.jsonl`](examples/review-handoff-rules.jsonl): three reviewer rules.
- [`commission-completeness.ruleset.json`](examples/commission-completeness.ruleset.json) and [`commission-rules.jsonl`](examples/commission-rules.jsonl): one commission rule.
- [`handoff-monitor.invocation.json`](examples/handoff-monitor.invocation.json): a Codex Stop hook origin, a reviewer `AgentJobPacket` and a transcript path.

**Admission:**
1. The packet validates against `agent-job-packet`.
2. The router's group binds `message` and `session`.
3. Wiring derives two groups:
   - `review-handoff`: `subject ← session`, `commission ← message`
   - `commission-completeness`: `commission ← message`
4. `review-handoff`'s leading condition `/role = "reviewer"` passes.

**Planning.** The router reads both sources (`message` and `session`).
- `review-handoff`'s rules read both, so the router rides in its request.
- `commission-completeness`'s rule reads only `message`, which does not cover the router's read set, so it waits for the router.
- `reviewer.handback-addresses-target` has a rule-level decision over `session`, which the rules read, so it rides too.
- `reviewer.stayed-read-only` has a rule-level condition, `/limits/access = "inspect"`. That passes for this packet, which is evaluated in code.

The first request carries:

| Request name | Owner |
| --- | --- |
| `q000001` | router (choice over `review_handback`, `clarifying_question`, `commission`, `other`) |
| `q000002` | rule decision `reviewer.handback-addresses-target`, entry 0 |
| `q000003` | verdict `reviewer.read-required-sources` |
| `q000004` | informational `reviewer.read-required-sources` / `unread_sources` |
| `q000005` | verdict `reviewer.stayed-read-only` |
| `q000006` | informational `reviewer.handback-addresses-target` / `coverage` |
| `q000007` | informational `reviewer.handback-addresses-target` / `has_findings` |

The transcript is captured once. The state holds deduplicated contents with these labels:
- `router/commission`, `router/recent_turns`
- `rule:reviewer.read-required-sources/reviewer_reads`, `.../commission_sources`
- `rule:reviewer.stayed-read-only/reviewer_changes`
- `rule:reviewer.handback-addresses-target/handback`, `.../review_target`, `.../final_message`

The final message and the router's last-two-turns selection share content entries where their records overlap.

**Reduction**, case A. The router answers `review_handback`, with confidence 0.82 ≥ 0.6 (routed):

| Rule | Answers | Result |
| --- | --- | --- |
| `reviewer.read-required-sources` | verdict yes 0.21; `unread_sources` = `some` | fail; block rule and 0.21 ≤ 0.3, so it **blocks** |
| `reviewer.stayed-read-only` | verdict yes 0.95 | pass |
| `reviewer.handback-addresses-target` | decision yes 0.9 ≥ 0.6, applicable; `coverage` 2 of 2 → 1.0; `has_findings` 0.8 | composite 0.94 → pass |
| `commission.outcome-checkable` | never sent; the router did not choose `commission` | no occurrence |

The report is complete (exit 0). The hook sees one block and can continue the reviewer with that finding.

**Case B.** The router's distribution is [0.45, 0.3, 0.15, 0.1]. Its native confidence is about 0.27, below 0.6:
- The router outcome is `uncertain_intent`, with that distribution.
- The router's answer is the controlling answer. The six rule answers from the same request are discarded and reported with the request's cost.
- There are no rule occurrences. The report is complete (exit 0).

**Case C.** The request fails, for example with a provider timeout:
- The router slot fails, so the router outcome is `router_failed`.
- The report is incomplete (exit 3), with the request's diagnostic and attempt records.
- No verdict exists.

**Case D.** The router answers `commission` with confidence 0.7:
- The `review-handoff` answers are discarded with their cost.
- The `commission-completeness` request is sent next, with one verdict question over the `outcome` payload field.

**Next run.** The report returns a cursor for the transcript. The hook passes it back with any selection that uses `since_cursor`, and only new or changed records are selected. These examples use fixed windows, so they don't depend on the cursor.
