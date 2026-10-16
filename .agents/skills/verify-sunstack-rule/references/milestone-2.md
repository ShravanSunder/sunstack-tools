# Git changes, connected evidence and chunks — DRAFT

**Open items:** a declared milestone 2 target implementing the selection design at `a08839b669f568aafc3ca442180c3d6e8cccc62b`, `docs/specs/2026-10-12-rust-rule-engine/program-design-connected-evidence.md`, including its path-level comparison and speculative gate precedence; matching change-selection/discovery/chunk types and permanent real Git/filesystem/AST fixtures. Read that design with the pinned `git show` method. The accepted milestone 1 executable exists at `919ca49`, but its full-scope behavior does not establish these change-scoped obligations. There are no operable commands in this reference; do not invent test names or activate it without a separate maintenance assignment.

Oracle: Specification C3/C8, V5/V7/V16–V18, and `program-design-connected-evidence.md` plus exact contracts, read at `SKILL.md`'s pinned revision with `git show`. Activation is a separate maintenance assignment that replaces this draft with verified invocations; until then, return this open item. Inputs for that assignment: declared commit/build/client/CWD, isolated supplied repository bindings, before/after sources, discovery paths/limits, package mappings and independent fixture expectations. Read the landed public harness before recording commands. Return resolved changes/provenance, selected units and chunks, coverage/omissions, process/library results and evidence locations.

## V5/V16 — what changed

Drive real temporary Git repositories through the exported evidence API and, where scoped, the actual process `prepare_evidence`/`evaluate_rules` envelope:

- Commit, staged, unstaged and explicitly included untracked layers; added/deleted/renamed sources; mutation during capture. Locations/content identities refer to captured bytes, never later reads. Rename may legitimately be delete+add.
- Explicit commit sets with merge-parent policy, root and merge commits, before/after comparisons, reachable commit-time windows and explicit unions. Report member provenance and resolved revisions/UTC bounds; nonmonotonic commit dates and reverted net content distinguish commit events from net comparisons.
- Recent dirty candidates intersect file times but include older hunks in a recently touched file. Deleted/unavailable/staging-only/skewed timestamps retain unknown recency. Missing the explicit unknown-recency policy refuses admission; include-with-diagnostic and exclude-with-visible-omission are separate controls. Approximation is not edit history.
- Shallow/missing history, traversal limits and unavailable roots remain coverage gaps. They must not become proof of no changes.

For V7, include target/context/applicability/rule/common-instruction changes on available before/after sides, unchanged neighboring RuleSets, absent baseline, lazy reads and optional/required limits. Line-number movement alone is not changed paragraph identity/content. Uncertain dependency analysis conservatively selects; required evidence refusal never produces a judgment.

## V17 — connected repositories and worktrees

Place a changed definition in supplied repository B and an unchanged caller in A. Include import alias and lexical shadow contradictions, unresolved packages, cycles, duplicate repository identities, two worktrees sharing Git objects, outside-scope references and partial repository failure. Every unit/relationship keeps root, worktree, revision, side and source attribution. The expected result is bounded candidates/contradictions/unknowns, never compiler symbol resolution or an exhaustive call graph.

Observe zero out-of-scope acquisition and explicit finite depth/fan-out/file/node/byte/time cutoffs. Bind discovery only to caller-supplied scopes. A related-units rule uses its bound change-set selection; a competing seed repository/selector is refused at admission. Native queries alone may name explicit seeds, per the adopted selector contract.

A full-scope binding supplies no change delta: `changed_units`, rule-authored `related_units` and native changed-unit seeds must refuse with `no_change_selection`. Keep positive controls for full-scope discovery context supporting another binding's changed seeds and native selector-based seeds. Resolve scopes lazily; an unused binding has no fabricated `change_selections` entry.

Native preparation and rule invocation use the same evidence-owned discovery configuration. A query/declaration or change-set source needing discovery with `discovery: none` refuses admission with `discovery_not_enabled`. For rule invocations, omitted discovery defaults to `none` and omitted package bindings to empty; milestone 1 refuses bounded discovery and nonempty package bindings. Milestone 2 passes both unchanged into the single EvidenceSession, with discovery limits cumulative across groups and RuleSets.

## V18 — coherent units and chunks

Map real before/after diff spans into actual AST/Markdown units and source ranges. Include deletion, giant units, caller/callee excerpts, required imports/signatures and retained relationships. Assert literal source-side bytes/ranges and membership, not output computed by the production grouping function.

Optional omission has a visible reason; required relationship/group overflow refuses evaluation. A rule's required evidence remains one group: no per-chunk judgment/composition in JSONL. Native callers may request per-seed groups and compose them themselves. Preserve unknown-recency attribution when an excluded dirty source is read as context; do not label it unchanged.

The separate activation assignment completes only after real fixture outcomes distinguish these failures from allowed approximate/partial results, the process/native entries requested are covered, and the full milestone 1 suite still passes. Keep unobserved platforms, unresolved links, cutoffs and hosted/tokenizer gaps explicit. Cleanup only owned temporary repositories under the assignment's grant; retain evidence and ask before directory removal.
