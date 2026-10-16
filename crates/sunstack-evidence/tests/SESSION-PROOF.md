# Native EvidenceSession integration prerequisite

Governing source: Program Design contracts at 94fa7ab, Native EvidenceSession. This native-only API retains evidence ownership; direct prepare_evidence delegates to the same capture/selection path. Its existing wire outcomes and milestone 1 CaptureCoverage shape are unchanged.

| Scenario | Wrong behavior caught | Independent oracle and real layer | Initial red |
| --- | --- | --- | --- |
| sequential_alias_selection_uses_frozen_bytes_after_file_changes | A later alias rereads a changed file or consumes a second file allowance | Literal original bytes and one-file measurement; real filesystem, capture, selector and session | Missing API compile failure (101), not a behavioral pre-fix red. After implementation, disabling the existing document-cache lookup deliberately made this assertion fail (nextest 100). The mutation was restored. |
| distinct_later_selection_obeys_the_same_cumulative_file_limit | A new selection resets cumulative invocation limits | Configured one-file allowance; real distinct files selected sequentially | Missing API compile failure; no behavioral pre-fix red observed. |
| opening_and_refusing_invalid_selection_reads_no_bound_document | Opening eagerly reads every source, or an invalid selection acquires its source | Missing bound files, invalid kind, zero existing file-read measurement; real session admission | Missing API compile failure; no behavioral pre-fix red observed. This count does not observe arbitrary OS reads outside the capture owner. |

First evidence suite after implementation: cargo nextest run -p sunstack-evidence, exit 0, 37 passed (the 34 accepted S2 tests plus 3 session tests), no LEAK marker. Session cache sensitivity run: exit 100, one expected assertion failure, no LEAK. Build logs are checkout-local under crates/sunstack-rules/target/s3-proof; the immutable sub-target's quality logs are declared on the execution root.

This sub-target does not prove rules uses one session. The real staged evaluate_rules source-mutation/alias/aggregate-limit scenario belongs to the full S3a target. Hosted V11 and tokenizer accounting are separate decision-layer evidence. No production stand-in was added.
