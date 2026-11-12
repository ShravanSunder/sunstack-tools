# Read HTML and check related documents

Use this journey when a rulebook checks real HTML documents, follows an HTML fragment, acquires related document pieces, or lives outside the project it checks. The user supplies a signed candidate and wants observable results from that binary. This is verification, not release acceptance.

The adopted oracle is the 11 Nov 0.11.0 engine-any-language commission including Corrections 1–7. Reconfirm that source with the commissioning Lead. These scripts exercise different parts of one journey; none grants provider-keyed inference, edits to checked projects, or delivery authority.

## Prerequisites

- Declare the full clean source head, immutable executable path and SHA-256. On macOS the candidate must have the verified Developer ID debug signature checked by the shared process harness. Any source/head/digest change stops the run.
- Use the existing Node/TypeScript/Zod workspace dependencies. Run from the monorepo root. Each evidence directory must be new and outside every checked input root.
- For Markdown parity, declare an immutable 0.10.x binary, version and provenance. The original 0.11 commission selects released 0.10.1; verify its release archive digest, extracted binary digest and signature before use. A local binary with a version string is not evidence of distribution.
- The source repository must retain object `022701fba2f2b1e133980a90892cac164361ea3f`. The consumer checkout must retain the declared ai-tools commit and the original `plugins/skill-authoring/skills/domain-skill-crafting/rules/skill-rules.config.json`, with the practices-how and debug-investigation skills.
- Checked-root observation uses the five real SDK pages under `docs/specs/2026-11-11-typescript-sdk/` in the commissioner-named SDK checkout. Read them first. They may be live/untracked; the script records bytes/hashes and stops if they drift. Do not edit or freeze that other agent's checkout yourself.

## Drive the real documents and labelled controls

```sh
node scripts/verify/sunstack-rule/html-evidence/html-evidence.ts \
  --binary <signed-candidate> --source-root <checkout> --commit <full-head> \
  --sha256 <binary-digest> --evidence <new-html-evidence-directory>
```

The driver copies five **real** pages from the exact Git object into its own evidence root, retaining per-file hashes and an independent source inventory. This respects the pre-existing containment contract; the copies are not invented fixture content. A plugin-outside-project check is observed separately below.

It validates each authored config with real `lint --check`, then runs keyless `lint --prepare`. Expected observations:

- Each of the ten R rows in specification is checked against the four Verification Design obligation rows. Each of the five E rows is checked against the four Program Design binding rows. The saved account identifies every found/missing owner; literal, manually inspected binding cells expect all fifteen found. Each document field has its own source file spans. The VD query uses `inside`/`stopBy: end` and the actual `ex-document#document` opening tag, selecting only `<tr>` obligation rows within it.
- The fixed five-page set has zero actual fragment hrefs. Escaped `body-U1` examples are not DOM IDs. The zero-case real run is an inventory observation, not evidence that anchor checking works.
- A real R1 asking request is prepared with plain text in its state. An unanswered request has no settled result: join its retained raw evidence by the exact request owner and span, never by inventing a result row. Raw piece text and mechanical body retain markup, with byte offsets that delimit the original source. A labelled Unicode/entity/attribute control independently expects `Café & Unicode 文 body.` with no attribute text.
- Labelled controls exercise found/missing HTML IDs, escaped fake IDs, missing files, file-only links, unsupported Markdown fragments, standard named/numeric entities, whole-token positives and Unicode letter/digit/underscore negatives. R1 must not pass solely because R10 occurs. Missing documents, empty needles and empty href captures are incomplete. Mixed incomplete/finding runs exit 3; findings-only runs exit 1; clean preparation exits 6.

## Compare unchanged Markdown inputs

```sh
node scripts/verify/sunstack-rule/html-evidence/markdown-parity.ts \
  --binary <signed-candidate> --source-root <checkout> --commit <full-head> --sha256 <digest> \
  --baseline-binary <released-0.10.1> --baseline-sha256 <baseline-digest> --baseline-version 0.10.1 \
  --consumer-root <ai-tools-checkout> --consumer-commit <full-consumer-head> \
  --evidence <new-parity-directory>
```

The driver exports tracked consumer plugins into an owned disposable repository, verifies the named config/skill bytes against Git and leaves the config unchanged. Both binaries check and prepare the same explicit skills. Every saved report byte must agree except the identified root product-version field. Nonzero requests/results and zero reported model requests are mandatory. This proves prepared-state and mechanical-result parity; inference judgments remain unobserved. The original config keeps its endpoint, so this companion's loopback listener is not an endpoint egress witness.

## Check a project from an external plugin config

```sh
node scripts/verify/sunstack-rule/html-evidence/checked-root.ts \
  --binary <signed-candidate> --source-root <checkout> --commit <full-head> --sha256 <digest> \
  --checked-root <named-sdk-checkout> --evidence <new-root-directory>
```

The config/evidence definitions are outside the checked repository. `--root` selects the real SDK project while config-relative definition paths keep their existing meaning. The driver checks the five real HTML pages and independently inventories each actual fragment link. Every real link is compared by file and href, including outside-root inputs as incomplete; it never replaces a named gap with a clean-total claim. Saved raw spans must still delimit the original files.

A named path outside the root must exit 2 and name both path and root. Four labelled absolute-path controls cover document/target acquisition inside and outside their own checked root. Inside succeeds; outside remains incomplete with `outside_root`, not a finding. Native joins separately establish `--since` uses the root's repository and whether `lint cases` shares the root option; this script does not imply either.

## Refuse deferred fields before acquisition

```sh
node scripts/verify/sunstack-rule/html-evidence/document-admission.ts \
  --binary <signed-candidate> --source-root <checkout> --commit <full-head> --sha256 <digest> \
  --evidence <new-admission-directory>
```

Correction7 preserves the existing acquisition phase boundary. A selection question that names a Document field is refused before a run; an unreferenced Document field in that selected kind remains admissible. A target's `as_kind` cannot name a kind containing Document fields, because nested loads do not acquire deferred fields. No recursive acquisition is introduced. Existing Target refusals and their diagnostic text remain unchanged.

The script drives eight labelled `lint --check` controls: four refusals (both Document guards and both existing Target guards), four accepted neighbours (local selection, selection with unreferenced Document, local-only target kind, and an explicitly used plain Document kind with no select and no nested reference). It checks exit2 plus the named diagnostic/config input for refusals, and exit0 plus rule listings for positives. Every child is keyless with an owned configured loopback listener. It never prepares or dispatches selection/model requests. These are structural admission observations, not acquisition or inference proof; the real document journeys above establish acquisition separately.

## Evidence and limits

Keep configs, snapshots/inventory, per-ID and per-link accounts, prepared JSON, signature/provenance records, every process receipt/stdout/stderr, listener ledger and pass.json. A pass.json appears only after all comparisons succeed. Preserve a failed run; correct only assigned apparatus, or hand product failures to the Implementer and disputed expectations to the Lead. New targets need explicit redeclaration.

Join the exact target's permanent native tests for all 29 pinned SupportLang mappings/old-report decoding, strict versus tolerant parser failure, YAML admission on non-Markdown, shared acquisition budgets/snapshots, scoped document exclusion, unsupported anchors, root/since/cases behavior and required quality gates. The CLI empty-capture control becomes an absent identifier under the existing capture-source contract and must be incomplete on that field; the native mechanical walker separately exercises an actual empty Text needle. A native receipt from a different source cannot silently cover new product code.

For HTML target references, decode standard semicolon HTML entities, then percent-decode physical paths and fragments as UTF-8 for resolution/comparison. Retain raw source fields. The original encoded-href red remains a required pass expectation; percent-space/Unicode fragments and encoded physical filenames have positive controls. Standard semicolon HTML entities in IDs and asking text are decoded. Legacy semicolon-less shorthand remains raw by explicit design. No provider judgment, browser rendering, inference quality, other platform, merge, release or owner acceptance follows from these keyless observations. Report the post-#214 page search with source/byte identity; modification time alone does not establish authorship after that merge.

All children are bounded by the existing process harness, use isolated credential-free homes and leave inputs unchanged. The driver closes its owned listener in `finally`. Retain decisive evidence; remove only owned disposable residue under the project's cleanup rules.
