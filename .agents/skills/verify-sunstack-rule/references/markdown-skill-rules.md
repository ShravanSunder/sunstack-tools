# Markdown rules: retained proof and the rulebook cutover

The 29 Oct rulebook route replaces the old Markdown marker-to-RuleSet lowering and its check/printing drivers. Current lint observations use `references/rulebook-lint.md`, `references/rulebook-check-refusals.md` and the updated S1 journey. The old scripts are retired by the Implementer's explicit deletion inventory; no current driver may import a deleted marker helper.

## Retained proof homes

| Contract retained through the cutover | Current proof home |
| --- | --- |
| Marker syntax, allowed hosts, binding, visible text, limits and spans | `crates/sunstack-evidence/tests/markdown_bindings.rs`, `markdown_unit_selection.rs`, `frontmatter_field_selection.rs`, `markdown_skill_corpus.rs` |
| Native mechanical expect forms, field states and local execution | `crates/sunstack-rules/tests/mechanical_execution.rs` |
| Located AST refusal, native matcher versus ast-grep scan, ancestor context and full-document containment | `crates/sunstack-rules/tests/mechanical_ast_grep.rs` |
| Native mechanical gates, cancellation and retained completed peers | `crates/sunstack-rules/tests/gated_mechanical_rules.rs` and native backend-factory tests |
| JSONL admission through real exec, formerly embedded in process_rule_source_admission.rs | the Implementer's named kept native process test in its deletion handback; inspect that actual path and run before reuse |
| Rulebook assembly, admission and skipped peers | the new sunstack-rules assembly/admission tests, plus `scripts/verify/sunstack-rule/rulebook-check-refusals.ts` and `rulebook-lint.ts` |

Inspect each offered source and raw current-target run. A test must keep its claimed boundary real and reject the named wrong behavior. Preserve native assertions before deleting a mixed-purpose marker-route test; a closed file or a new happy-path check is not proof that its retained assertions moved. The Implementer's deletion handback names every extracted destination. A missing destination or unobserved current-target run remains a gap for the Lead.

The old marker route's observations were S2 `7c529b4b4bc6f0160ec06c1735ab27bdd94eeb6d` and mechanical S4 `bdca8d9a571229a057a60615eb916a2b375b94d7`, under the 17 Oct Markdown design and its amendments. Those are historical observations only. If commissioned to reconstruct that replay, read this reference and the removed drivers using `git show <historical-product-commit>:<path>` at the declared historical target and oracle; current `sunstack lint` cannot earn that old contract. An unavailable object or disputed oracle returns to the Lead.

For current work, return the retained native claims separately from rulebook lint's new evidence/decision claims, each with exact source/binary, observed or reused run, assertion coverage and remaining gaps. No hosted, release or acceptance claim follows from these local proof homes.
