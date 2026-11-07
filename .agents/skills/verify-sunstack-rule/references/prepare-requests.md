# Build exact requests without sending them

**Observed on source `73d7fc78d96a4f8b28e8460a4fb4d38259ad5d8b`, binary SHA256 `c30dd3c8dfaae536dc730b6bade46c6ebfaf5399730dbebd915880f06025c921` (0.9.0).** The governing source is the engine 0.9 prepare commission (7 November 2026), its Admission section, and the Lead clarification: static decision routes create no request dependencies; only selection-dependent piece existence does. Filter-gated rule requests are prepared with gate metadata. This route changes no rules or skills and runs no paid provider inference.

## Target and entry

Use the declared full source commit and immutable binary SHA256, from the repository root. The CLI flags and prepared-report shape must match that candidate’s README and offered tests. Read the driver’s help:

```sh
node scripts/verify/sunstack-rule/prepare/prepare-requests.ts --help
```

Once the target is declared, run in a fresh authorized evidence directory:

```sh
node scripts/verify/sunstack-rule/prepare/prepare-requests.ts \
  --binary <absolute-binary> --source-root <product-checkout> \
  --commit <full-head> --sha256 <binary-digest> \
  --prepare-exit <documented-prepare-code> --evidence <new-directory>
```

The Implementer selected exit 6 for a completed prepare-only run; invalid, cancelled, incomplete or mechanically failing runs keep their documented precedence. Case gaps retain exit 1; malformed input uses 2, runtime incomplete 3 and cancellation 4. Unassessed bars alone do not fail preparation. Prepared-case readback uses a 1-based case-entry ordinal with linked revision/request evidence, as settled by the Lead. A prepared fixture or a passing apparatus unit test is not product proof.

The existing RulebookProcess guards source/binary identity, starts one ephemeral owned loopback listener and bounds each child. It isolates HOME and disables credential storage. Prepare and show children receive no key. Ordinary neighbors receive only synthetic loopback credentials. The ordinary neighbor’s actual requests prove the listener can observe dispatch; an answering port is not readiness. Require both fixtures’ keyless `lint --check` acceptance before the drive.

## What the driver compares

Run ordinary lint and `--prepare` against the same input/config/endpoint. The prepared file is outside the fixture and is read from disk. For each prepared owner/revision/piece or folder, compare its persisted `request.state` and `request.questions` to the ordinary saved request and the captured HTTP body. Strip only JSON presentation whitespace outside strings. Preserve key/list order, Unicode, escapes and numeric spelling; never sort or reserialize parsed payloads. Reject missing and duplicate owners, unaccounted captures and invented prepared answers.

The request fixture has a mechanical prefix and rule, ungated and filter-gated judgments, long cut text, ordered Texts, reference keys that expose integer-key reordering, and a locator reading a field absent from authored questions. The two ordinary gate scenarios make exactly eight and ten requests; prepare must make none while retaining all ten potential requests with the gated rule’s filter and labels. These literal counts are independent of the product’s report totals.

The selection fixture retains a selected workflow kind and its contained step kind. Prepare must retain the selection request, list both the selected kind and its descendant in `needs_answer`, and link them to an actual prepared selection request ID. Do not invent a selected piece or guess its answer. Addition 2 requires ordinary saved reports to retain selection requests/answers, resolved model/version and input hash. Compare selection payloads to both those saved bytes and actual HTTP. Then use that exact saved report with `--prepare --selections-from`: matching answered selection inputs must yield downstream prepared requests byte-equal to the ordinary continuation, with `from_report` and its source request ID. Change one supplied candidate while keeping questions/config fixed: that selection and its descendants must return to `needs_answer`, with no guessed downstream payload or HTTP. An unanswered or question-mismatched source report is likewise unresolved. The positive fixture pins the owned provider’s exact resolved model name. Separate alias and version changes remain unresolved under the recorded replay policy documented in the target README; the matching native controls also remain part of the proof join. Replayed selection entries use state/status `from_report`, `source_request_id`, and the original completed outcome/receipt, resolved model and `selection_evidence` bytes. New downstream entries keep state/status `prepared` with no outcome. The driver checks the compact request hash for every entry and rejects an unrelated source ID, changed recorded outcome/evidence/model, or invented downstream request. This format was driven on the declared target above. New targets still require identity checks and an applicable observation.

The case fixture uses native `lint cases --prepare` on historical Git revisions, today’s overlay and an obsolete historical rule YAML. Every selected case stays present with its original hash/range/label; stale and spanning ranges stay listed rather than prepared. Require historical reference/external sentinels, current rulebook reference, and absence of obsolete historical definitions. No manual directory purge stands in for the engine’s overlay.

Read one prepared entry through `lint show`, then repeat after deleting only the owned fixture’s current rulebook files. The readback stays identical and sends no HTTP. For prepared cases, read a request-bearing case from each revision and a stale reason-only case by 1-based ordinal; compare each linked revision, request and rulebook record with the persisted run. Repeat after removing the owned current config and rulebook files. Ordinals zero and past the final case must refuse with exit 2. Keep unknown-ID handling tied to the candidate’s documented contract; do not flatten colliding result IDs across revisions.

## Coverage and return

Before calling the route observed, join offered permanent tests for mechanical-only behavior; unavailable fields; cut and locator parity; request/choice/budget refusals; known filter-exclusion preservation at the matching boundary; duplicate-exact case accounting; selected kinds and within descendants; ordinary output compatibility; and invalid/incomplete/cancelled exit precedence. Read scenario and assertions, then reuse only exact-target proof. The driver alone does not close those joins.

Keep prepared and ordinary report bytes, actual raw HTTP, commands/exits, saved evidence/config contents/hashes and all failed attempts. The harness joins every child and closes its own listener in `finally`. Leave shared services and other sessions alone. Report every row supported, unsupported or unknown at the named source/binary; a prepare exit is never a judgment-quality pass.

The ai-tools held-rule/development-run comparison is a separate Lead-directed observation. Establish unchanged evidence, rulebook and model-request identity before comparing with development run 2. No held-out or keyed inference follows from this recipe.

The Lead clarified the filter proof on 7 November (Router push `01a1b68d-3227-7ef2-8b25-debfff12b26b`): filters obtain answers through the provider before walking their tree, so prepare cannot derive a filter label or exclude a folder. Retain every potential gated request and its metadata; use the ordinary controlled-provider neighbors for actual admitted/excluded behavior. A matching test supplied with an existing exclusion proves preservation only. Do not add offline filter evaluation to make a verification scenario reachable.

Observed evidence is retained under the engine-0.9-prepare-2026-11-07 verifier artifact home: `observations.md` and `candidate-73d7fc78-run2/`. The completed journey exits 0: ten potential requests, ordinary gate neighbors with eight/ten captured requests, one recorded selector plus six downstream payloads, five unresolved controls, sixteen accounted revision cases and self-contained show. Twenty-four children settle; prepare/show make zero HTTP requests, the ordinary neighbors make twenty-five, and the owned listener closes without errors. The first attempt's cleanup-path error is retained as harness evidence; the corrected driver uses the reused fixture's path constants. Native 750/750 (seven skipped) and full apparatus 203/203 are scoped joins; the cleanup-only successor reuses their unaffected coverage. No paid/model-quality or ai-tools comparison is implied.
