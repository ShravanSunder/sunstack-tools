# Changed-only capture from raw Git object identities

Use this route when checking0.12 lint `--since` or default uncommitted capture. The adopted source is the engine-since-git-first commission, Proof1–4 and **Correction2**, which supersedes the initial Git-diff/metadata-widening rule. The original/API and owned-edge routes were exercised at declared target `5b4965ba147b0ebc571278fdcc7297e8b4dcb89d`. They are operable with the explicit prerequisites below; every future target needs its own source/artifact/input binding and applicable native joins. Source preparation, helper tests and an earlier uncommitted receipt alone prove no current product outcome.

## What must stay true

Candidate discovery compares raw Git blob identities across the existing catalogue views. Worktree files use raw bytes; links use their payload. Commit/index views use stored blob IDs. No stat cache, Git filters, attributes or index flags decide whether raw content changed. Equal bytes, including mode-only and equal-payload file/link changes, stay unchanged. Candidates are then acquired under the existing encoding, byte/file-budget, span and snapshot rules. Hashing reads consume the existing byte budget; only actual acquisitions consume `max_files`.

Lint explicitly requests `ContextCapture::ChangedOnly`; other public capture callers retain `Full` by default and can still read unchanged frozen context. Unchanged lint inputs now come from its existing filesystem fallback at lint time. Do not present this as zero I/O: every enumerated worktree path may need hashing. No new limit or provider permission follows.

Declare a clean full source head and two immutable signed artifact paths/digests: the main `sunstack` binary and the already-built `change_selection` test executable containing the permanent public-API parity consumer. The latter is a separate native target bound to the same source; the journey never compiles product code. Also declare the released0.11.1 comparator's digest and provenance. A local build with the same source is not a released artifact.

## Original frozen command and full oracle

```sh
node scripts/verify/sunstack-rule/git-first/git-first-original.ts \
  --binary <candidate> --source-root <checkout> --commit <full-head> --sha256 <digest> \
  --capture-binary <immutable-test-executable> --capture-sha256 <digest> \
  --baseline-binary <released-0.11.1> --baseline-sha256 <digest> \
  --input-root <commission-frozen-checkout> --input-head <full-input-head> \
  --base b37419ca --config plugins/skill-authoring/skills/domain-skill-crafting/rules/skill-rules.config.json \
  --evidence <new-directory-outside-input> --tracked-count 1397
```

The commission's original is detached3e513b4e, with1397 tracked entries and `CLAUDE.md -> AGENTS.md`. The driver checks the declared full HEAD/clean status, every regular file's bytes/mode, link payloads, and Git HEAD/index/config before and after. It never edits or repairs that root. Outputs/config copies stay outside it, including through symlink aliases. A guard mismatch stops the run and goes back to the Lead; no retry on a live or changed input is authorized.

Readiness is candidate `lint --check` on the original config. The original command is actual `lint --config <relative-original-config> --since <base> --prepare <outside-report>` from the frozen root. Successful preparation is **exit6**, not0; require prepared status, no model dispatch and a nonempty selected-file witness. Keep raw stdout/stderr and the saved report.

The old released comparator must independently reproduce exit3 at the default2000 acquisition cap. Its full oracle uses a config copy outside the checkout, changing only `max_files` to10000 and rebasing rulebook markdown/rules plus `ast_grep` paths to their original absolute inputs, then passes `--root <same-frozen-root>`. Every other config value stays the same; original files are untouched. Reconfirm that original config/rulebook dependencies did not change across the selected revisions, so rebasing the config path does not alter widening. Selected files come from `file_counts` entries whose kind has `picked > 0`; represented result/request/selection owners are a separate account (`Rule.piece.file`, selection owner `file`, selection entry `where.file`, and no invented file for folder-owned filters). Both complete prepared reports must have identical selected files, represented owners and entire results/requests/filter/selections/needs-answer payloads, with the explicitly declared root versions0.11.1/0.12.0. Do not normalize questions, state, IDs, hashes or request order after observing a difference.

The same journey drives exactly one permanent public-API test through the frozen test executable. It sets `SUNSTACK_CAPTURE_PARITY_ROOT`, `SUNSTACK_CAPTURE_PARITY_BASE` (resolved full revision) and `SUNSTACK_CAPTURE_PARITY_RECEIPT` in an isolated keyless environment and invokes `changed_only::capture_parity::public_capture_modes_produce_equal_raw_changes_with_fewer_acquisitions --exact --nocapture`. It requires exactly one executed passing test and a fresh outside receipt. The test calls the actual public capture API with ChangedOnly2000 and Full10000, identical other inputs/budgets, comparing raw changed payloads/spans and paths. Verify its source and source/artifact binding before driving it.

Join the receipt's input fingerprints/HEAD/index with the stronger original guard, compare both path lists and raw rows, and record actual acquisition ledgers. Candidate acquisitions must correspond only to changed sides and be fewer than original tracked files; the full oracle must cross the old2000-file boundary. Selected-file count alone cannot prove this. The SDK observation and signed CLI selected-owner/request observation are separate proof layers.

## Owned edge and Markdown controls

```sh
node scripts/verify/sunstack-rule/git-first/git-first-edges.ts \
  --binary <candidate> --source-root <checkout> --commit <full-head> --sha256 <digest> \
  --baseline-binary <released-0.11.1> --baseline-sha256 <digest> \
  --evidence <new-owned-edge-directory>
```

The driver builds isolated Git repositories with a mechanical Markdown rule, checked peers and assets. It tests staged-only, unstaged-only, untracked, deleted, renamed, mode-only, unchanged/changed binary, binary-to-text, invalid checked text, changed/unchanged raw symlinks, equal-payload file/link, ignored file, assume-unchanged, skip-worktree, CRLF, clean filter and preserved-mtime/stat-cache cases. Git CLI must actually miss each authored blind-spot raw edit; a fixture that never reaches that case fails setup. Input oracles are fixed before each product invocation.

Both candidate and released full capture must retain the authored selected files/results, or refuse changed non-UTF-8 with exit2 and the correct structured path. Changed raw links are configured rulebook inputs whose two followed JSON targets are byte-identical. The pointer change must trigger existing config-dependency widening to both checked peers; dropping the raw link misses that consequence. The checked-file walker itself excludes symlinks, so no selected-symlink result is invented. Prepared mechanical positives exit6 with no invented model request. Staged/unstaged/untracked controls also run through default scope. A deleted path may have no current checked piece; the API join must still prove its raw deletion.

The two-document mechanical `--all` control compares ordinary and prepared stdout/saved bytes with released0.11.1, except root engine version only. It proves that named fixture, not broad inference parity. `--prepare --evidence <new-directory>` on the driver only provisions fixtures and writes `product_observed:false`; it is never a product pass.

## Native joins and limits

Read the current permanent native tests and final source-bound runs for Full default/unchanged context, tiny acquisition caps, hashing of unchanged files under file/total-byte caps, mutation after capture for changed and unchanged sources, exact staged/unstaged/untracked/deleted/rename witnesses, binary/path refusals, links/mode/type equality, flags/CRLF/filter/stat blind spots and gitlinks. Preserve the first semantic reds separately from compilation, fixture/index-cache or harness failures.

Gitlinks keep the old catalogue behavior: revision-to-worktree skips commit-tree gitlinks and can descend the checked-out nested repository, treating its child file as an addition; index staged/unstaged views retain `RevisionUnavailable` for the gitlink path because it is treated as a blob. Confirm the owned native fixture and report that behavior, not new submodule-pointer support.

Retain all target/source hashes, native and CLI process receipts, exact input guards, original/config-copy provenance, raw API paths/payload hashes/spans/counts, all reports and helper/gate failures. Close only owned children/listeners, then reopen evidence. Original endpoints are not rewritten for prepare; the zero-request loopback is not a global egress witness. No keys, inference, other-platform, distribution, CI or release acceptance is implied. Return each proof row as supported/unsupported/unknown with observed/reused coverage and its actual target.
