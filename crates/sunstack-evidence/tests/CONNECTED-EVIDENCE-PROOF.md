# Connected evidence proof account (S4a)

Scope: evidence-owned milestone2 E17-E21, V5/V16/V17/V18 and primitive/native preparation, on accepted EvidenceSession pin6d4a87acd9dbb15d9c6a26e3dbcde526ba9ffd93. The isolated M1 accounting repair2c59ae5 and F3 supplement cursor correction7387c12 are retained. Rules scheduling, change-scope application to rule occurrences, CLI/report plumbing outside evidence, providers and hosted delivery belong to other slices.

## Adopted contracts

The canonical2026-10-16 delivery plan and reviewed Specification govern. Follow-up Program Design commits are74d1b7d (all read sources and selected text), b6cbf96 (Full has no changed seeds),94fa7ab (verify before Ready),afe8531 (native per-seed output),bdb9ecc (cache identity before per-path filesystem work),1152997 (resolved change report),db15fd3 (chunk-owned unknown-recency context),8ae3761 (lazy change reports),01405fd (discovery_not_enabled).

EvidenceSession remains the one invocation owner of frozen documents, captures and cumulative limits. Git helpers take private cached input from that owner; worktree bytes enter its authoritative document cache, while index/commit blobs stay distinct. Acquisition verification runs once before the batch becomes usable. After Ready, subsequent selections retain frozen bytes; session close performs no source or Git IO.

Public results carry SelectionCoverage single_source or change_set. A change-set selection attributes every frozen file/version read, including unchanged and nonmatching inputs. Resolved change selections are emitted once per actually resolved repository on prepared/incomplete/cancelled-partial outcomes and session summaries. Unused change scopes remain unopened. Authored related_units has no seeds and permits single_group only; native RelationshipQuery supports selector/changed seeds and per_seed groups.

## Executable claims

All tests compile as external library consumers. Git repositories, commits/index/worktree, filesystem, tree-sitter/AST-grep, public preparation and serialization stay real. Synthetic adapter data is used only where that public adapter boundary itself is the subject.

| Tests | Outcome established / wrong implementation caught |
| --- | --- |
| change_selection,change_recency,change_union | Separate cancelling dirty layers, explicit untracked input, rename sides, root/merge parent deltas, commit events versus net revert, linked worktrees, absolute/lookback intervals, reachable nonmonotonic dates, approximate whole-file recency, deleted/unknown policy, union order; explicit and walked history caps |
| source_units | Real Rust/TS/TSX declarations and Markdown paragraphs/sections; deleted before-only locations; paired requirements; recovered syntax labels without fabricated ERROR-root functions; zero-width insertion/deletion never seeds an unchanged neighbor |
| relationship_discovery | Cross-repository package and relative imports, aliases, static calls and inverse directions, cycles, lexical shadow contradiction, ambiguous declarations, dynamic/namespace/receiver uncertainty, Rust conditional modules refused rather than guessed |
| semantic_chunks,discovery_limits | Frozen text/spans/signatures/import context, whole units versus bounded excerpts, chunk-byte overflow, min-related/unresolved/cutoff causes, frontier/omission evidence, actual files/bytes/candidates/visits/fanout bounds |
| frozen_queries,repository_context | Primitive text/regex/AST over changed sides only, normalized exact paths, all read-source attribution, partial match cutoff, Full plain-directory context, explicit refusal of changed seeds without a change selection |
| connected_preparation | prepare_evidence connects changed library to unchanged full-scope caller and deleted before unit to current caller; Full changed-unit refusal; valid admission followed by source failure retains successful B attribution; missing root is separate preflight refusal |
| connected_groups | Native seed-key order, self-contained shared related excerpts with one capture, independently Complete/Partial groups, required unresolved/minimum/overflow/limit refusal versus optional partial retention; unknown-time context label, known-context control, recency-excluded unread source never attributed as captured |
| connected_session | Document-to-worktree and worktree-to-document alias reuse after deletion/replacement, distinct blob/current capture at one path, lazy unused non-Git scope, report dedup, validation-refused document/Git acquisitions still spend file/byte budget |
| resolved_reports | Public include/exclude policy and nonfatal recency warnings, no source reads for recency exclusions, ordered resolved union bounds/root provenance, one invocation UTC anchor across repositories, cancelled admitted partial preserves completed report |
| public_limits,change_admission | Public required file/file-byte/total-byte/visits/candidates/fanout refusal, real500-file/1ms LocalTime cutoff, cumulative native visits/matches/history across selections and repositories, invalid windows/empty head and schema unique arrays, exact discovery_not_enabled admission |
| capture_mutation | Real actor publishes unique complete versions by atomic rename throughout601-file capture; SourceChanged/CaptureAgain before return, no retry/test hook/injected clock |

## Red-first and sensitivity boundaries

Early source-unit and relationship routes were exercised against unfinished implementations:3 and2 behavioral failures respectively. The first chunk test failed on missing content; the second initially failed setup due to a wrong-language fixture, which is not counted as a behavioral red. It was corrected and later sensitivity-tested. Initial public preparation had3 behavioral failures (unsupported/invalid selections and wrong Full refusal) before session integration.

Subsequent observed regressions include zero-width neighbor selection, blob identity, conditional Rust modules, lost partial match evidence, required-cause erasure, per-seed cutoff contamination, deleted cross-repository callers, missing parse measurements, unrequested import edges exhausting a caller budget, exclusion applied after freezing, failed-acquisition accounting, and document/worktree cache bridge attribution. Each has an appropriately named red/green log in target/s4a-proof.

Final boundary tests failed on explicit CommitSet history cap, normalized ./path matching and duplicate admission before repair. History cutoff coverage also exposed a stale Unavailable override. F4's public test failed with unsupported_relationship_query before the adopted dedicated code. These are implementation observations, not independent Verifier findings.

Seven earlier isolated post-green mutations were restored after behavioral failures: omit pre-return verification, disable minimum related count, fabricate Full seeds, ignore lexical shadowing, ignore visits, ignore fanout, ignore match cutoff. An initial attempt without this host's CFLAGS failed native compilation and is not evidence; the retained sensitivity logs are later behavioral runs. Four final mutations also fail behaviorally: reset shared history allowance, omit failed Git file charging, bypass native unique-array admission, mislabel every captured context source as unknown recency. Those are sensitivity observations, not earlier defects.

The original concurrent actor used truncate/write. Under workspace nextest both reads observed the repeatedly published empty state, so its monotonic-mutation premise was false. The fixture now publishes complete unique versions by atomic rename from outside the capture root and runs until the capture owner returns, then stops and joins. No product change, retry, wait, public seam or increased bound was used. The corrected actor passed20/20stress iterations; deliberately omitting automatic verification makes it fail behaviorally. Both the failed gate and correction evidence are retained (final-nextest.log,actor-stress.log,sensitivity-final-atomic-actor.log). Same-content verification and post-Ready frozen reuse remain separate positive controls.

## Gates and limits

Run from the monorepo worktree root using the pinned Rust toolchain. This host needs command-local CFLAGS pointing at xcrun's macOS SDK for native parser/Git compilation; no machine configuration or signing runner changed.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo test -p sunstack-evidence
cargo tree -p sunstack-evidence -e normal
```

Observed final combined gates: evidence cargo test101 passed/0 failed (exit0); workspace nextest135 passed/1 skipped (exit0); workspace fmt and clippy exit0. The clippy rerun only adds the standard test-only panic allowance in change_admission.rs; runtime behavior and assertions are unchanged from the passing runs. The immutable target and receipts are recorded on the execution thread. Raw final logs: target/s4a-proof/final-evidence-test.log,final-nextest-green.log,final-fmt.log,final-clippy-combined-green.log,final-tree.log. Hosted ignored tests are unverified. The evidence normal dependency graph has no provider/HTTP client or decisions/rules dependency; git2 default features are disabled. Library real-path tests establish the affected runtime behavior without requiring a separate CLI surface. Syntax candidates do not claim compiler resolution or exhaustive history. No open stand-in. Lead acceptance and paired Verifier observation are separate from this implementer proof.

## Correction after independent observation at88b5631

The independent observation found no product failure but left returned V18 import/signature context and explicit V17 duplicate-binding/outside-scope/unmapped-package cases unknown. Four public real-path cases now exercise them, with six separately restored behavioral sensitivity failures. See CONNECTED-PROOF-CORRECTION.md for each test's independent oracle, real boundary, actual run order and evidence. This is a proof-only correction; production source remains identical to88b5631. Original gate counts above describe that earlier target; correction gates are declared separately on the execution thread.
