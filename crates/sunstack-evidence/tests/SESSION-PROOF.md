# Native EvidenceSession integration prerequisite

Governing source: Program Design contracts at 94fa7ab, Native EvidenceSession. This native-only API retains evidence ownership; direct prepare_evidence delegates to the same capture/selection path. Its existing wire outcomes and milestone 1 CaptureCoverage shape are unchanged.

| Scenario | Wrong behavior caught | Independent oracle and real layer | Initial red |
| --- | --- | --- | --- |
| sequential_alias_selection_uses_frozen_bytes_after_file_changes | A later alias rereads a changed file or consumes a second file allowance | Literal original bytes and one-file measurement; real filesystem, capture, selector and session | Missing API compile failure (101), not a behavioral pre-fix red. After implementation, disabling the existing document-cache lookup deliberately made this assertion fail (nextest 100). The mutation was restored. |
| distinct_later_selection_obeys_the_same_cumulative_file_limit | A new selection resets cumulative invocation limits | Configured one-file allowance; real distinct files selected sequentially | Missing API compile failure; no behavioral pre-fix red observed. |
| opening_and_refusing_invalid_selection_reads_no_bound_document | Opening eagerly reads every source, or an invalid selection acquires its source | Missing bound files, invalid kind, zero existing file-read measurement; real session admission | Missing API compile failure; no behavioral pre-fix red observed. This count does not observe arbitrary OS reads outside the capture owner. |

First evidence suite after implementation: cargo nextest run -p sunstack-evidence, exit 0, 37 passed (the 34 accepted S2 tests plus 3 session tests), no LEAK marker. Session cache sensitivity run: exit 100, one expected assertion failure, no LEAK. Build logs are checkout-local under crates/sunstack-rules/target/s3-proof; the immutable sub-target's quality logs are declared on the execution root.

This sub-target does not prove rules uses one session. The real staged evaluate_rules source-mutation/alias/aggregate-limit scenario belongs to the full S3a target. Hosted V11 and tokenizer accounting are separate decision-layer evidence. No production stand-in was added.

## Frozen aliases after deletion: corrective target

The paired Verifier found that live canonicalization preceded the document-cache lookup. The governing clarification bdb9ecc requires lookup by admitted source identity before filesystem access, with root/symlink verification on first capture only. Source review found the same premise in conversation alias matching.

Permanent real public-session regressions capture the original file, delete it, then select a not-yet-used alias. The document and transcript deletion cases failed against 0186d595's implementation (nextest exit 100, run 9435734d: 5 passed, 2 failed). The remove-and-replace document variant and first-capture outside-root symlink control passed already; no pre-fix behavioral red is claimed for those controls. No HTTP, filesystem or capture function is mocked.

The correction normalizes lexical identities once on session open, compares admitted aliases without filesystem access, and looks up document captures using repository plus normalized relative path before first-capture I/O. Root/symlink containment and capture verification remain in the existing capture_document owner. It does not revisit a returned capture at close. After correction cargo nextest run -p sunstack-evidence exited 0: 41 passed (34 S2 plus 7 session cases), zero skipped and zero LEAK, run e781108d. The corrected immutable sub-target's full gate and paired observation are declared on the execution root. The earlier 71-green sub-target is superseded and does not establish the deleted-alias guarantee.
