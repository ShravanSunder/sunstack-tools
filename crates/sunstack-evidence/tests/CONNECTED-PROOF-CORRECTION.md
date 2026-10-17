# S4a V17/V18 proof correction

Base:88b5631ded13c2ed8a33eab504b2a0e81210be3a. The independent report at private verification archive supported the tested native paths and left two explicit coverage portions unknown: returned import/signature context and duplicate repository identities/out-of-scope/unmapped package routes. These were adequacy gaps, with no observed product defect or oracle disagreement.

This correction adds four permanent public-consumer tests and no production/dependency changes or weakened existing assertions. The original target stays unchanged; the correction lives on rule-engine/m2-context-proof in its own worktree. The oracle remains Specification V17/V18 and Program Design connected-evidence Semantic chunks and scoped syntax capability set, adopted through01405fd.

| Test | Wrong behavior caught and independent oracle | Real boundary |
| --- | --- | --- |
| public_preparation_keeps_literal_signatures_and_versioned_import_spans | Empty/wrong enclosing signature, missing/wrong import range or source revision, wrong serialized context. Literal before/after TypeScript signatures and distinct import lines/ranges; index blob OID obtained independently from real Git, after-file SHA-256 from fixture bytes; fixed repository/path and before-index versus after-worktree tags. | Real Git index/worktree, public prepare_evidence, AST/import resolution, typed and serialized returned context. |
| public_session_keeps_import_context_and_signature_after_live_source_rewrite | A later query rereads live replacement text or changes frozen signature/import positions/source identity. Same literal after context retained after replacing live file with different import/function; entire returned readiness stays equal, four actual index/worktree reads total. | Public EvidenceSession with real filesystem rewrite, no hook/mock/wait. |
| public_connected_admission_refuses_duplicate_keys_and_canonical_roots | Duplicate repository key or canonical root is admitted. Distinct-root/key control returns a real change-set match; each duplicate refuses at admission. The duplicate root uses a different lexical path ending in dot. | Public prepare_evidence, actual canonical roots and Git selection. |
| public_package_discovery_keeps_unmapped_and_excluded_targets_unresolved | Unmapped package invents a target or loses visible uncertainty; an excluded package target is read or attributed. Explicit in-scope mapping control returns a Complete group and located provider excerpt/Imports candidate. Unmapped and excluded modes retain Partial/unresolved evidence; required forms are Incomplete. Excluded bytes are absent from serialized evidence, captures and source attribution, with only caller plus allowed visible file read. | Real untracked Git caller, separate plain provider root, package binding, input-path scope, public preparation and actual imports graph. |

## Run order and sensitivity

Both context cases passed first on unchanged production source, as did both scope cases. No pre-fix product failure or tests-first product repair is claimed. The four cases therefore close formerly unasserted public outcomes.

Six separate post-green production mutations were made only in the isolated correction worktree, one at a time, each restored in a finally block. Each focused run exited101 with0passed/1failed by the intended behavioral assertion:

- Empty enclosing signature: literal before signature assertion fails.
- Drop import spans: import count0 versus1.
- Bypass duplicate-key guard: duplicate identity is admitted.
- Bypass canonical-root guard: duplicate identity is admitted.
- Permit unmatched paths: excluded target causes3physical reads versus2.
- Omit unresolved import edge: package uncertainty diagnostic disappears.

These are sensitivity observations, not earlier production defects and not independent Verifier mutations. Source diff against88b5631 is empty after restoration. No clock, public hook, driver, fixture seam or product behavior changed.

Raw evidence: private verification archive,scope-first,sensitivity-*}.log and sensitivity.json. The initial context/scope tests and the subsequent source mutations are preserved in that order. Full gate logs and immutable correction target are recorded on the execution thread. The signing runner and pinned toolchain remain configured; native C builds use command-local xcrun SDK CFLAGS. Build output is an isolated copy-on-write cache; the original observation worktree and its build output were not modified.

Broader proof boundaries remain in CONNECTED-EVIDENCE-PROOF.md: syntax candidates do not establish compiler resolution, and this correction does not establish S4b rules/CLI integration, hosted providers, other platforms, CI or release. Independent observation of this correction and Lead acceptance remain separate. No open stand-in.

## Final correction gates

From the isolated monorepo root, with CARGO_TARGET_DIR=private verification archive and command-local SDK CFLAGS:

- cargo test --locked -p sunstack-evidence: exit0,105passed/0failed.
- cargo clippy --locked --workspace --all-targets -- -D warnings: exit0.
- cargo nextest run --locked --workspace: exit0,139passed/1ignored hosted skipped.
- cargo fmt --all -- --check: exit0.
- cargo tree -p sunstack-evidence -e normal: exit0; unchanged evidence dependency graph.

Logs named evidence-test.log,clippy.log,nextest.log,fmt.log,tree.log are in the same system-temp proof directory. The four new cases account for the increase from101to105evidence and135to139workspace cases. Inspected01405fd..bc831ec: later source changes concern rules routing/read-set planning and payload-condition presence semantics, so these evidence outcomes and their governing meaning are unchanged. No new source, lock or configuration change is bundled with the proof correction.
