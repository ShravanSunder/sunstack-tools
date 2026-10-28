# sunstack-evals

Frozen case storage and pure scoring for candidate-run bundles. This crate owns labels;
`sunstack-rules` and `sunstack-evidence` cannot depend on it.

`freeze_cases(new_directory, &FreezeInput)` writes separate `sources.jsonl` and
`labels.jsonl` files plus `manifest.json`. The caller supplies stable source captures
and every definition file that contributes to the rule's questions, including router
files where applicable. Their bytes are hashed alongside both JSONL files. Capture
paths are retained as absolute paths; this API does not locate live sessions or copy
the caller's evidence. Keep those files in durable storage for a resumable eval.
The destination's parent must exist; an existing destination is refused.

`load_case_set(manifest_path)` verifies every recorded SHA-256 identity, then strictly
decodes both JSONL files. Duplicate keys, unknown fields, duplicate cases, duplicate
source IDs, missing source references, mismatched session metadata, and any session or
supplied project key occurring in both splits are refused. Project keys are optional;
session separation always applies. The selected split is recorded in the manifest.

The returned `FrozenCaseSet` is an immutable validated snapshot. Reload before each new
run: changing a frozen file refuses loading and produces no score report. A previously
loaded snapshot remains its original version. The case-set identity hashes the manifest
bytes. The rule version hashes the JSON array of definition-file content identities in
the caller's declared order, independent of their storage paths.

`score(&bundles, &case_set)` reads only in-memory values. It never runs a judge or reads
a file. The caller supplies bundles for the selected frozen rule, case set and split.
A source anchor matches a candidate by its evidence-owned occurrence identity and rule.

- Filter recall counts each yes-labeled occurrence once. Its first positive case in
  label-file order represents a miss. A missing positive is a miss only with one
  matching located input report and no unread/unfit coverage; otherwise it is filter
  `unknown`. Missing, unlocatable, partial or ambiguous input reports never prove a
  filter miss. Unknown labels never contribute negatives.
- Steps report case IDs for correct, wrong and unknown answers. A missing candidate,
  absent answer, multiple observations for one question, or nonbinary settlement stays
  unknown. A settlement is used as recorded; probabilities are never reinterpreted.
- Only verdict labels supply end-result truth. Finding and clear dropped outcomes are
  binary; uncertain, unresolved and not-processed outcomes are inconclusive. An absent
  candidate is inconclusive unless its actual mechanical rejected sample establishes
  the filter drop. Gate/router labels supply no inferred end truth.
- Usage sums the supplied bundles, preserving absent cost. Invalid or overflowing usage
  is refused. Speed uses all supplied candidates for this rule, including candidates
  with unknown labels. It counts missing `elapsed_ms` values separately.
- Percentiles use nearest rank: sort measured milliseconds and select rank
  `ceil(p * count / 100)`, starting at one. No measurements means no p50 or p95.
  Run wall time is the sum of recorded bundle wall times, never a source of candidate
  measurements. Scheduler timing itself is outside this crate's proof.

The permanent integration tests use a hand-worked nine-case table, independent literal
measurements, strict file round trips and same-size valid label tampering. The initial
callable wrong stubs failed those tests before their implementations were added.
The binary eval operation and label scans through real judge requests belong to P8b.
