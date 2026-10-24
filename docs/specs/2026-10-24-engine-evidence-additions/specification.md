# Engine evidence additions: Specification

The observable contract for [Requirements U32–U35](requirements.md). These obligations extend the engine's specification (`docs/specs/2026-10-12-rust-rule-engine/specification.md`). Everything not named here keeps that specification's meaning. Revised after design review (`engine-additions-design-review.md`, F1–F9).

## Entities

No new entity. These obligations use:

- E9 Rule, E12 Target location, E29 Verdict;
- E31 Conversation record, E33 Tool use, E34 Conversation selection, E38 Transcript format;
- E18 Change selection.

Three existing entities change:

| Entity | Change |
| --- | --- |
| E34 Conversation selection | When tool use is included, it may carry an **input pattern**: a regular expression in the engine's existing regex syntax, searched (not anchored) in each tool use's **input text**. A tool use is kept only when its input text contains a match. The input text is defined in R43. Without a pattern, selection is unchanged |
| Change-set literal, regex and AST selections | **Sides**, required: a nonempty set of `added`, `removed`, `after_file` and `before_file`. `added` keeps after-side matches whose span overlaps an added line range. `removed` keeps before-side matches whose span overlaps a removed line range. `after_file` and `before_file` keep every match on that whole side |
| E12 Target location | A **change-match target**: the whole selection's matches, each with its source (repository, path, revision and side) and its line and byte span. It may be empty. The **conversation-records target** lists each selected record with both its E31 record ID (for a tool use, the ID carries its native call ID) and its position in the capture. It may be empty. This replaces the ID-only list for every conversation target, judged or mechanical, in a hard cutover |

## Obligations

| R | Obligation | Traces |
| --- | --- | --- |
| R42 | A Codex rollout `custom_tool_call` record MUST normalize as an E33 tool use: its name, its recorded input, and its native call ID. A `custom_tool_call_output` with the same call ID MUST become that tool use's result. Category comes from the tool's name only, never from its input: `apply_patch` is `edit_file`, `exec` and any shell tool name are `shell`, and any other name is `other`. Outcome is `succeeded` or `failed` only when the output states it; otherwise it is result present with unknown status. A recognized custom record MUST NOT be a coverage gap. An output with no call stays an orphan gap | U33 |
| R43 | A tool use's **input text** is its recorded input's decoded string when that input is a JSON string, and otherwise the compact JSON serialization of the recorded input. A conversation selection with an input pattern MUST keep only tool uses whose input text contains a match. Its omissions MUST report, as their own reason, the count of tool uses that passed the name or category filter and were excluded by the pattern. That count is separate from window, projection and limit omissions. An invalid pattern MUST refuse admission, through `sunstack exec` (rule admission for `check_rules` and `evaluate_rules`, and selection admission for a standalone `prepare_evidence`), before any source is read. `sunstack lint` admits only single-document RuleSets, and keeps refusing conversation and change-set RuleSets as unsupported inputs | U32 |
| R44 | A change-set literal, regex or AST selection MUST declare its sides. A missing or empty sides set MUST refuse admission before any source is read. Each match MUST be kept only when its span satisfies at least one declared side. Each kept match MUST carry its source with revision and side, and its line and byte span | U34, U35 |
| R45 | A mechanical rule MUST be admissible with a change-set selection or a conversation selection as its target, using the presence check, and with no other check (any other check stays `invalid_target`). The rule's target is the whole selection, empty or not. Settlement follows the table below. A failing or passing occurrence MUST list every selected item as a target location. It MUST make no model request | U32, U34, U35 |

### R45 settlement

A selection is **complete** when its coverage and omissions show no limit, cutoff or unavailable source that could have hidden an item.

| Check | Selected items | Complete? | Result |
| --- | --- | --- | --- |
| `present: false` | one or more | either | fail, listing every item |
| `present: false` | none | yes | pass |
| `present: false` | none | no | incomplete, exit 3, with the limiting omission |
| `present: true` | one or more | either | pass, listing every item |
| `present: true` | none | yes | fail |
| `present: true` | none | no | incomplete, exit 3 |

## Proof obligations

| V | What decides it | Reference |
| --- | --- | --- |
| V36 | `prepare_evidence` over the Tool Portal Implementer rollout (`rollout-2026-10-20T22-50-05-01a15c5d-…jsonl`). The custom call named `exec` at source line 311 is an E33 tool use with category exactly `shell`, its input contains `ctx.storage.put`, and it is paired by call ID with its output at the line the output record occupies. Its outcome equals what that output states, or unknown status if it states none. The custom records are no longer coverage gaps. No decision backend exists on this path (`operation_execution.rs:54`) | the rollout records; R42 |
| V37 | `evaluate_rules`, mechanical rule `present: false`, tool use with input pattern `ctx\.storage\.(put\|get)`. It fails on that rollout, listing the line-311 call among its items. It passes on a control Codex rollout that has shell and edit tool uses and none that match (the lint pilot Implementer's rollout, `01a16f99`), where the pattern-exclusion count equals that rollout's eligible tool uses | R43, R45 |
| V38 | `evaluate_rules`, mechanical rule `present: false`, edit-file tool use with input pattern `node:sqlite`, over the identity Lead's Claude transcript (`…/2febb449-db01-4859-8fdb-fdc41a74abfc.jsonl`). It fails, and its items include call `toolu_01TmbsRetrfucicnDwsKG2QT`: the plan Write in the assistant record at source line 142279, paired with its result at 142280. Line 142267 is a request-blob copy, not a normalizable record. Its item count equals the count of distinct matching calls that an independent script finds in the same transcript. It passes on `…/6852afea-7ca8-44d5-8f44-039f53c2a204.jsonl` (4 edit-file uses, none matching), with a pattern-exclusion count of 4 | R43, R45 |
| V39 | A scratch Git repository with snapshots A (no call), B (A plus a `this.ctx.storage.put(...)` call), C (B plus an edit on lines disjoint from the call) and D (B with the call's own line changed). With an added-side AST rule `$OBJ.storage.put($$$)` and `present: false`: A→B fails at the call's file and line; B→C passes; B→D fails at the changed line. With `sides: ["removed"]`, B→A fails at the before-side line. With `sides: ["after_file"]`, B→C fails at the unchanged call | R44, R45 |
| V40 | Each `evaluate_rules` run in V37–V39 reports zero model requests and zero inference attempts. Through `sunstack exec`, admission refuses with exit 2, before any source is read, for an invalid input pattern, a missing sides set and an empty sides set. This holds both for `evaluate_rules` and for a standalone `prepare_evidence`, each checked beside a valid peer definition that is admitted. A valid conversation RuleSet given to `sunstack lint` is still refused as unsupported input, unchanged | R43, R44, R45 |
| V41 | The R45 settlement table, row by row. Each row is decided by an owner test that a wrong build fails: one that always applies `present: false`, one that ignores completeness, and one that reports only the first item | R45 |
