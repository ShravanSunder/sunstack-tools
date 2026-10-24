# Engine evidence additions: Requirements

An addition to the rule engine's design set, `docs/specs/2026-10-12-rust-rule-engine/`. It continues that set's identifiers: U32–U35, R42–R45 and V36–V41. Lead: 🦁 claude-local `68844339`, 2026-10-24. Status: proposed, for design review.

## The need

The owner wants `skill-authoring:skill-audit` to hold every rule and run it easily, including:

- rules over agent session logs: what an agent did;
- rules over code changes: what a commit added.

A Sol spike on released 0.1.0 ran real inputs (scratch `sunstack-evidence-spike/report.md`). Jev judgments work. Three things the rules need do not:

- **Mechanical rules can't target a conversation or a change set.** Admission allows only Markdown units and frontmatter fields (`sunstack-rules/src/definition_admission.rs:148–152`). So "an agent wrote to a wrangler config" needs a model call even when the transcript shows it exactly.
- **Codex edits are invisible.** Codex rollout `custom_tool_call` records fall to the unsupported branch (`sunstack-evidence/src/transcript_normalization.rs:297`, `:388`). Current Codex makes its file edits through the `exec` custom tool, with the patch in the call's input: across all 7,119 October 2026 rollouts on this machine (228,659 tool calls), no call is named `apply_patch` (`engine-additions-verifier-feasibility.md`, addendum). The Tool Portal Implementer's `ctx.storage.put` write is one of its 368 dropped `exec` records.
- **Change rules see whole files.** A change-set AST or text selection scans both complete sides of each changed file (`sunstack-evidence/src/frozen_queries.rs:306`). So a rule against a forbidden API fails on every file that already used it, not only on the commit that added it.

## Sources

| Id | Source | Class |
| --- | --- | --- |
| S1 | The owner, 2026-10-24: "teh skil adut can have all the rules we can run them easily then"; "the susntack jev thing if orgto the name can use the session logs and ast gre its great lets kuse it adn if we need fix and imrp0ovement i ok?"; "dont worry about cost"; "i expect multiple process fixes and use of skunstack toosl no need for any ci stuff. agetns can call them with teh righ tskilsl" | normative |
| S2 | The spike report, with real runs and exact anchors | observational |
| S3 | The engine's accepted design set (C2, C3, C8, C11, C12), at sunstack-tools main `c49d2f2f` | normative for everything not changed here |

## Requirements

| U | Need, and why it matters | Source | Authority | Priority |
| --- | --- | --- | --- | --- |
| U32 | A rule over a session log can decide from what the log records, with no model call, when the fact is structural. An example is that a tool use's recorded input names a file or contains a call. Structural facts are exact, cost nothing and can't drift; a model is for meaning | S1, S2 | authorized (the owner's OK for fixes and improvements) | P1 (Lead) |
| U33 | A Codex session's file edits and commands are evidence, as a Claude session's are. Without them, no rule can see what a Codex Implementer changed | S1, S2 | authorized | P1 (Lead) |
| U34 | A rule over a change judges what the change added, or removed, rather than whole files, so code that was already there doesn't fail a new rule | S1, S2 | authorized | P1 (Lead) |
| U35 | A mechanical rule over session logs or changes reports each occurrence where it fires (the record, or the file and lines), so the owner and agents can act on it | S1 | authorized | P1 (Lead) |

## Goal boundary

| Field | Content |
| --- | --- |
| Goal | Conduct rules over session logs and change rules over diffs run mechanically on the released CLI, with occurrence locations |
| Foundation reused | The engine's capture, normalization, selection, admission, mechanical check and report contracts (C2, C3, C8, C11, C12), and the Codex rollout format already registered |
| Changes | Admission of two more mechanical targets; one conversation selection filter; one required change-set selection field; Codex custom call normalization |
| Protected | Judged rules and the decision backend; Markdown and frontmatter rules, which keep their behavior; caller scope and read authority |
| Non-goals | Triggers or hooks that run the rules (the owner: "lets figoure otu the triggers later"); new transcript formats; semantic code identity (types or symbols); any CI job |
| Acceptable complexity | No new crate, dependency or runtime. Contract additions only where named here |
| Acceptable evidence | Real CLI runs on the Tool Portal session logs and a scratch two-commit repository, each rule failing its planted bad case and passing its good case with zero model requests, plus the engine's own tests |
| Owner choices | None open. The owner OK'd fixes and improvements; release follows the repository's process after the owner merges |
