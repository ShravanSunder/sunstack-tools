# Connected evidence: change selection, related code and chunks

This is the milestone 2 part of the [Program Design](program-design.md). It realizes Specification C8: Git change selection, connected discovery across supplied repositories, and chunking. The context budget is milestone 1 and lives in the main design. Everything here belongs to `sunstack-evidence`. Rules decides which occurrences these observations select, and whether a group is complete enough to ask a question.

This content was reviewed in the earlier design and carries over. What changed: a rule now reads a change set through a **change-set input**, so rule-authored selectors never name a repository or a seed. Native `prepare_evidence` callers still name repositories and seeds directly, through the full `RelationshipQuery`.

## Roots, scopes and frozen evidence

![Source scope resolution, capture and changed-unit discovery](assets/change-sourcing-flow.png)

[Diagram source](assets/change-sourcing-flow.mmd)

A **repository binding** (E17) is the only authority for a root, its allowed paths and its change scope. A change-set binding names the repository key and uses that binding's change selection; it never declares a second scope. A discovery root is the caller's declared input scope, not the changed-file list. Changed files seed analysis, while unchanged files inside the supplied roots remain available as callers, imports and tests. That is what stops a diff-only filter from hiding affected consumers.

| Shape (`evidence_contracts`) | Payload and meaning |
| --- | --- |
| `ChangeScope` | `full`, `uncommitted { layers }`, `revision_diff { base_revision, head: commit \| worktree }`, `commit_set { commit_ids, parent_policy }`, `recent_commits { reference, window, parent_policy }`, `recent_uncommitted { layers, window, unknown_recency }`, `union { members }`. Nested unions are refused |
| `DirtyLayer` | `staged`, `unstaged`, `untracked`; nonempty and unique. HEAD→index and index→worktree stay distinct |
| `TimeWindow` | `lookback { seconds }` or `absolute { since_utc, until_utc }`, resolved once at admission |
| `CommitParentPolicy` | `first_parent` or `each_parent`. A root commit compares against the empty tree |
| `UnknownRecencyPolicy` | Required: `include_with_diagnostic` or `exclude_with_omission`. There is no default |
| `ResolvedChangeSelection` | Reported once per repository binding whose change selection was actually resolved during the invocation (Specification R21). Resolution is lazy, like any acquisition: an unused change scope is never resolved just to report it, and a full-scope binding used only as context has no entry: `repository`, `anchor_utc` (the one UTC anchor every window resolved against), `members: [ResolvedChangeMember]`, `coverage` |
| `ResolvedChangeMember` | The resolved variant of each scope member, in scope order (a union returns its members in order): `scope_position`, the resolved bounds (`layers`, comparison endpoints, selected `commit_ids` with their parent policy, resolved `since_utc`/`until_utc`), `changed_paths` with their provenance, and `recency: not_time_based \| commit_time \| approximate_file_time { unknown: none \| included { paths } \| excluded { count } }`. Its nonfatal `diagnostics` (`recency_unknown` for included unknown-recency files, `history_incomplete`) live here, and never make an authorized selection incomplete |
| `SourceCaptureLimits`, `DiscoveryLimits` | Positive `max_files`, `max_file_bytes`, `max_total_bytes`, `max_matches`, `include_ignored`; and `max_unit_visits`, `max_relationship_candidates`, `max_fan_out`, `max_history_commits`, `max_local_time_ms`. All required |

`git2` resolves tree, index and worktree comparisons. Git work happens once per repository and scope member, never per rule. Renames may be delete plus add.

**Capture.** Selected file bytes are frozen with their content and revision identity. Each capture ends with a verification pass that rereads the mutable files it read, before its selection returns. If relevant content changed during that capture, it is refused once with `source_changed` and `capture_again` advice, with no hidden retry. A change after a capture has returned is a later source change: the frozen bytes stand, as the [evidence session](program-design-contracts.md#prepare_evidence-sunstack-evidence) states. Commit-to-commit sources use blob IDs. A unit or chunk never mixes a span from one version with text from a later read. No transaction spans several repositories.

**Time windows.** One UTC anchor resolves lookback for all repositories. Commit windows walk from the declared references, filter on committer time, and don't stop at the first older commit. Shallow or bounded history is a coverage diagnostic. Commit-set and time scopes keep each commit's own delta; `revision_diff` is an endpoint comparison. These are never merged into a fictional patch.

**Recent dirty files.** Scoped dirty candidates are intersected with file modification times. A recently modified file contributes its whole current diff, including older hunks. Unknown recency follows the caller's explicit policy.

## Change-set selectors in rules

A change-set input binds one repository binding. Its declarations are tagged by `select.kind`:

| `kind` | Payload |
| --- | --- |
| `change_set_literal_text` | `paths: [QueryPath]`, `text` |
| `change_set_regex_text` | `paths`, `pattern` (Rust `regex` dialect: no lookaround or backreferences) |
| `change_set_ast_pattern` | `paths`, `language`, `pattern` (an AST-grep pattern with metavariables, not a YAML rule) |
| `changed_units` | `languages`, `unit_kinds`, `source_sides` |
| `related_units` | `languages` (`rust`, `typescript`, `tsx`), `unit_kinds`, `source_sides`, `directions`, `requested_hops`, `chunk_settings` (grouping must be `single_group`), `minimum_related_units`. It has no seeds and no repository list |

`QueryPath` is tagged: `{ kind: exact, path }` or `{ kind: glob, pattern }`. This replaces the earlier string-or-object union (R37). Globs use `globset` over normalized `/` paths, case-sensitive with literal separators, and are always intersected with the binding's allowed paths. The supported AST languages are the built-in catalog of the pinned AST-grep version. Admission derives accepted names from that catalog, and an unknown name is refused.

**Selection reasons** for change-scoped evaluation are closed:
- `full_scope`, `target_changed`, `context_changed`, `rule_changed`, `new_rule`
- `applicability_changed`, `baseline_unavailable`, `before_analysis_unknown`
- `not_applicable_current`, `no_declared_change`, `selection_diagnostic`

## How change scope selects

Evidence owns both sides of a change; rules owns the selection. Rules asks the invocation's one `EvidenceSession` whether a subject changed (`compare_change_sides`), then turns the answers into an E15 selection record. Rules never reads Git or files itself, and never opens a second capture.

**Subjects and levels.** A comparison's subject is one of:
- `declaration`: an evidence declaration;
- `input`: a whole bound input, for a condition that scans it;
- `definition_file`: a rule file's or RuleSet file's resolved path.

Each comparison runs at one of two levels:
- `path` reads no subject content. It checks the subject's source paths against the binding's resolved changed paths, and classifies inputs that are not change-tracked by kind.
- `content` compares what the subject selects on both sides. It exists for declarations and inputs.

Either level may resolve a binding's change selection, once per binding: that resolution is the binding-level acquisition C8 defines, and its reads are reported in its coverage.

**`ChangeComparison`** (`evidence_contracts`), for one subject:

| Variant | Meaning |
| --- | --- |
| `unchanged` | Same content on both sides of every member delta |
| `changed { members: [scope_position] }` | Content differs in at least one member delta |
| `added { members }` | Absent on the before side, present on the after side |
| `deleted { members }` | Present on the before side, absent on the after side |
| `before_unknown { reason: baseline_unavailable \| history_incomplete }` | The subject has a before side, but it could not be resolved |
| `no_before_side { reason: full_scope \| conversation_input \| payload_input \| outside_change_scope_paths }` | The subject is not change-tracked: it has no before side to compare |
| `refused { diagnostics }` | The comparison could not finish. The causes are an unreadable current side, a source that changed during its capture (`source_changed`), an invalid selection, or an exhausted limit. The diagnostics are the evidence diagnostics `select` already uses. Cancellation follows the session's established cancellation behavior |

**A declaration at content level** is compared by the content it selects, never by its locations, so moved lines are not a change. Evidence compares, per member delta:
- a paragraph's text, by its target ID;
- document text;
- a match set, as the multiset of matched text and capture text;
- context bytes.

A change-set declaration (`change_set_*`, `changed_units`, `related_units`) selects only from the delta, so it is `changed` when its selection is nonempty and `unchanged` when it is empty. Before-side reads come from the member's before side: a commit or tree blob, or the index. They are charged to the session's cumulative limits like any other read. Commit and tree blobs are immutable and need no verification pass.

**An input** is compared as its whole bound content. A document input in a change-scoped binding is compared like `document_text`. A conversation or payload input, a document in a full-scope binding, and the invocation's origin have no before side.

**A definition file** (a rule file, or the RuleSet file that holds `common_ruleset_instructions`) is compared at path level only. It reads no bytes beyond its binding's shared resolution:
- If the resolved path lies inside a change-scoped repository binding's allowed paths, the binding's resolved changed paths decide `added`, `changed` or `unchanged`.
- Otherwise the result is `no_before_side { reason: outside_change_scope_paths }`.

The granularity is the file. An edit anywhere in a RuleSet file marks all its rules `rule_changed`, and a newly added RuleSet file marks them all `new_rule`. This only selects more.

**An occurrence's dependencies** are everything its selection and evaluation read:
- its rule file and its RuleSet file;
- its target declaration;
- its context declarations;
- every input its RuleSet-level and rule-level applicability entries read. That covers the evidence declarations its decisions read, and the operands its conditions read directly: the payload input of a `payload_field_*` test, the input a `pattern_present` test scans, and the origin an `origin_field_equals` test reads.

The origin is rules-owned and never change-tracked, so it maps as `no_before_side` without a call.

**Selection never reads ahead of applicability (Specification R28).** Leading conditions run first, exactly as in milestone 1. An occurrence they stop is `not_applicable`, or applicability failed, and gets no comparison at all. For the rest, selection runs in two steps.

1. **At planning, at path level.** Rules compares every dependency at path level, which reads no subject:
   - If any dependency gives a reason, selection settles with those reasons.
   - If every dependency is `unchanged`, the occurrence is `outside_change_scope { reason: no_declared_change }` and nothing of it is acquired.
   - Otherwise a tracked source path changed, and only content can tell whether the selected part changed. The occurrence is planned as `comparison_pending` and evaluates like a selected one. A binding resolution refused here is carried to step 2 as that dependency's refusal.
2. **At the acquisition frontier, at content level.** When the occurrence's own request becomes ready, the same session compares its pending dependencies by content. Ready means its controlling gates settled and passed, or the request rides speculatively (R38). That is the point where milestone 1 acquires the occurrence's evidence anyway: gate inputs were reached at their gates, and the target and context arrive with the request.
   - If any reason holds, selection settles with those reasons and the questions stay.
   - If every pending dependency is `unchanged`, the occurrence is `outside_change_scope { reason: no_declared_change }`. Its questions leave the request before encoding, as with an evidence refusal. A request left with no questions is `not_sent { reason: outside_change_scope }`.
   - If a comparison is `refused`, the occurrence follows the existing rule for a rule's refused required evidence (Program Design scheduling): `selection_failed { diagnostics }`, because its gates have passed.

An occurrence whose gate does not pass never reaches step 2. It is `not_applicable`, or reduces as for a failed gate, and its pending dependencies are never read. `prepare_rules` runs step 2 for the requests it prepares; an occurrence whose request is waiting stays `comparison_pending`.

**Rules' mapping.** `selected_by` lists every reason found in the step that settled selection, in the closed order. The levels differ in one case: at path level, a declaration or input whose file was modified (`changed`) gives no reason. It leaves the occurrence `comparison_pending`, because moved lines or an edit elsewhere in the file may leave its selected content unchanged. Its `added` or `deleted` at path level does give its reason. Definition files are compared only at path level, so their `changed` gives `rule_changed` directly. Otherwise both levels map as follows:

| Dependency | Comparison | Reason |
| --- | --- | --- |
| The rule file or its RuleSet file | `added` | `new_rule` |
| The rule file or its RuleSet file | `changed` | `rule_changed` |
| The target declaration | `changed`, `added` or `deleted` | `target_changed` |
| Any context declaration | `changed`, `added` or `deleted` | `context_changed` |
| Any applicability dependency | `changed`, `added` or `deleted` | `applicability_changed` |
| Any dependency | `no_before_side`, whatever its reason | `full_scope` |
| Any dependency | `before_unknown { baseline_unavailable }` | `baseline_unavailable` |
| Any dependency | `before_unknown { history_incomplete }` | `before_analysis_unknown` |

A dependency that is not change-tracked can never prove the occurrence irrelevant, so it selects as `full_scope`: that dependency is evaluated as under full scope. A dependency whose before side exists but is unresolved selects with its own unknown reason. Both follow Specification C3: missing evidence never proves irrelevance.

- **A refused comparison** makes preparation or the report incomplete. Peer occurrences keep their own dispositions.
- **Outside change scope** requires every dependency established `unchanged`, at path level or at content level.
- **Selected or pending:** the occurrence's later applicability entries run as in milestone 1. Not applicable is `not_applicable` (the `not_applicable_current` case), and unresolved applicability is `selection_failed` (`selection_diagnostic`).
- **Target deleted:** the occurrence is selected (`target_changed`). Its current-side selection then refuses with `deleted_target`, so it ends `selection_failed`, as in milestone 1. A deletion is never `outside_change_scope`.

**Applicability is compared by its inputs, not its outcomes.** Rules never evaluates conditions or decisions on the before side, so it never makes a before-side model call. A change to any applicability dependency selects the occurrence, a superset of an outcome change: a false-to-true flip always selects, and a true-to-true change with changed inputs also selects. The cost is extra judged occurrences when applicability inputs change without changing the outcome. The gain is no before-side evaluation and no second evaluation path.

**What narrows and what does not.** Change scope narrows only an occurrence whose every dependency is change-tracked and unchanged. The following always select as `full_scope`:
- a definition outside every change-scoped binding;
- a payload, conversation or origin dependency;
- a document from a full-scope binding.

A caller who wants change-scoped precision binds the repositories that hold the definitions and the context with a change scope, and keeps payload, conversation and origin out of the rules it wants narrowed.

## Connected discovery

![Connected discovery across supplied repositories](assets/connected-sourcing-sequence.png)

[Diagram source](assets/connected-sourcing-sequence.mmd)

Both sides of each diff span map to enclosing functions, methods, classes and Markdown units, using real AST ranges. A pure deletion seeds a before-side unit only.

**Seeds and scope for a rule.** A rule's `related_units` declaration has no seeds of its own. Admission builds the native query: seeds are the changed units of the bound change-set input's repository binding, and discovery covers every repository binding the invocation supplies, within each binding's allowed paths. So a changed definition in repository B can bring in an unchanged caller from a supplied repository A, and the rule cannot redirect its seed elsewhere.

**Changed units need a change scope.** Changed-unit seeds require a change selection with an admitted before or after side in the seed repository. A repository binding with `change_scope: full` has no delta, so a `changed_units` selection, a rule-authored `related_units` declaration, or a native `SourceUnitSeeds::changed_units` query over it is refused with `no_change_selection`, never answered with every current unit. A full-scope binding can still be discovery context for another binding's changed seeds, and native `selector_units` seeds work in full or change scopes.

**The relationship graph** is invocation-local:
- Each node is keyed by snapshot, repository, revision, path and range.
- A visit is deduplicated by node, direction and depth, so cycles stop revisiting.
- **Basis facts** come from in-scope declarations, imports, explicit aliases and lexical shadowing.
- **`PackageSourceBinding`** `{ importing_repository_id, module_prefix, target_repository_id, target_path_prefix, language }` maps an import prefix to supplied source. It does not claim an installed version.

**Fixed capability set:**
- `GraphLanguage` is `rust`, `typescript` or `tsx`.
- `RelationDirection` is `callers`, `callees`, `imports` or `imported_by`.

| Language | Located evidence | Stays unresolved |
| --- | --- | --- |
| TypeScript, TSX | Named functions, methods and classes; same-file bare calls; ES named and default imports and aliases; static relative and package-source paths | Receiver and dynamic dispatch, namespace and wildcard imports, re-export chains, computed imports, ambiguous declarations |
| Rust | Function items and impl or trait methods; same-file and static-path calls; non-glob `use` paths and aliases; ordinary `mod` files; package-source paths | Macros, receiver and trait dispatch, glob imports, conditional modules, external crate versions |

**Ambiguity and limits:**
- Ambiguous names stay multiple candidates or an unresolved hint, never first-match.
- Hop depth is the query's neighborhood. The resource limits record cutoffs and their frontier.
- `DiscoveryCoverage` is `complete_within_declared_scope`, `limited { cutoffs }` or `unavailable`.

## Semantic chunks

Evidence groups changed units with related excerpts by the rule's declared relationships. An excerpt keeps:
- its repository, source side and content identity
- its byte span
- its enclosing signature and import context

Shared excerpts are deduplicated. A giant unit can become an explicitly partial excerpt, but a rule requiring the whole unit is refused when it doesn't fit.

| Shape | Payload |
| --- | --- |
| `SourceUnitQuery` | `seeds`, `languages`, `unit_kinds`, `source_sides` |
| `SourceUnitKind` | `function { signature }`, `method { signature }`, `class { signature }`, `paragraph`, `section` |
| `ChunkSettings` | `grouping: single_group \| per_seed` (rules allow `single_group` only; native callers may use `per_seed`), `seed_content`, `related_content`, `max_chunk_bytes`. `single_group` yields `related_group`, and `per_seed` yields `related_groups` with one seed-attributed group per located seed unit ([contracts](program-design-contracts.md#prepare_evidence-sunstack-evidence)) |
| `UnitContent` | `whole_unit` or `bounded_excerpt { max_bytes }` |
| `ContextGroupOutcome` | `complete { group, coverage }`, `partial { group, coverage, omissions, diagnostics }`, `unavailable { diagnostics }` |
| `Excerpt` | `unit: SourceUnitKey`, `span: SourceSpan`, `text` (the captured bytes of `span`), `membership: whole_unit \| bounded_excerpt { max_bytes }`, `context: code { enclosing_signature, import_context: [SourceSpan] } \| markdown`, `content_identity` |
| `DiscoveryCoverage` | `complete_within_declared_scope { query_bounds }`, `limited { query_bounds, cutoffs: [{ limit, frontier: [SourceUnitKey] }] }`, `unavailable { diagnostics }` |
| `PackageSourceBinding` | `importing_repository_id`, `module_prefix`, `target_repository_id`, `target_path_prefix`, `language` |

**A required related selector is ready only when:**
- seeds are located
- the requested sides and whole units are present
- the minimum related units are met
- the declared query scanned without a cutoff or an unresolved required endpoint
- the group fits its bounds

Otherwise the required declaration refuses its owner's evaluation. An optional one contributes only with its coverage and omissions. A rule's required evidence is judged as one group. Judging chunks separately, and combining those results, is native caller code.

## Proof

| V | What it observes here |
| --- | --- |
| V5 | Real repositories: commits, layers, renames, mutation during capture |
| V16 | Time windows, unions, root and merge commits, unknown recency |
| V17 | A cross-repository caller, alias and shadow contradictions, partial source failure |
| V18 | Before and after spans mapped into units and chunks, with deletions, giant units and cutoffs |

All four run against real fixtures. The milestone 1 suite must keep passing.
