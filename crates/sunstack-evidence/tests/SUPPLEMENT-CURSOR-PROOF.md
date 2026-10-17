# Supplement cursor correction (F3)

Scope: milestone1 correction commissioned by Lead, exact design01405fdb06aadd7c78da927e59ecdf24b2d42ad0, Program Design Cursors. Base2c59ae5b32c349dc7329b909af10b467f350557c; no milestone2 source or dependencies. The earlier S2 proof row named late_supplement_repeats_until_transcript_materializes_it is superseded by this adopted correction.

The source cursor owns an optional supplement content digest. Incremental selection returns a supplement on first capture or digest change. It retains the digest across a continuation with no final text, and clears it when normalized transcript content materializes that message, even when the caller supplies no final text then. Committed record watermarks remain independent, so materialization returns the committed record once. All/last turn windows still see the current supplement. No process-global state or provider is used.

Three real-file public capture/selection tests replace the earlier repeat expectation:

- unchanged_supplement_is_not_replayed_after_its_cursor: first supplement, unchanged continuation, all-window control, absent-final-text continuation, same supplement again.
- changed_supplement_is_returned_once_with_the_whole_new_text: whole changed text returns once, repeated changed text does not.
- materialized_supplement_returns_the_committed_record_once_then_clears_its_digest: append a real JSONL assistant record, omit caller final text, return the committed ID once, clear digest, then empty delta.

The three tests were run before production changes. Two failed behaviorally because repeated supplement records reappeared. The third already had correct committed-once behavior; its new digest-slot assertion failed because the prior cursor had no such slot. This is not a claim of a previous materialization-loss defect. All three then passed; the complete conversation binary passed10/10.

Two separate post-green source mutations were restored after their focused tests failed behaviorally (exit101): dropping the pending digest loses the changed-text delta; retaining the materialized digest fails the cleared-cursor assertion. These are sensitivity observations.

Raw logs and gate receipts: private verification archive No source mutations remain. Tests use the real filesystem, built-in JSONL adapter, capture, cursor encode/decode and projection. Cursor prefix/source identity, streamed-message and late-tool-result controls remain in the same binary. Automated real-path tests provide runnable library proof; hosted providers, installed host hooks and CLI delivery are unverified by this correction.

Required gate commands, from this isolated worktree root with command-local CFLAGS=-isysroot to the xcrun macOS SDK:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
```

Observed gates: fmt0, workspace clippy0, workspace nextest0 with81 passed/1 skipped (47 evidence tests). Immutable target is recorded on the execution thread. No open stand-in. Acceptance belongs to Lead after independent observation.
