# Change scope beside unchanged binary assets

This journey checks the repository-capture boundary used by `lint --since`: equal before/after bytes do not need text decoding, while changed content selected for textual capture must still be UTF-8. The adopted 0.10.1 unchanged-binary fix preserves file/byte limits, mutable snapshot checks and present-versus-absent identity. It does not narrow lint's repository-wide change binding or allow lossy decoding.

## Current changed-only capture

For0.12, use [raw candidate identities](git-first.md) and `git-first-edges.ts`. The governing commission's Correction2 preserves raw-byte change semantics while unchanged paths are hashed without being acquired or frozen. Hashing bytes still meet the byte caps, and identity guards still verify unchanged mutable inputs. `ContextCapture::Full` remains the default for connected-evidence callers that need unchanged frozen context; lint explicitly selects `ChangedOnly` for `--since` and default uncommitted scope. A CLI count of selected pieces is not a capture-acquisition count: join the declared permanent public-API consumer and its actual ledger.

Current edge coverage retains equal binary plus changed text, changed binary/binary-to-text/invalid checked text refusals with offending paths, and raw changed/unchanged symlink controls. It also checks staged/unstaged/untracked/default scope, deletion/rename/mode/ignored files and Git-diff blind spots. The current binary-free Markdown comparison uses `--all` with released0.11.1 and exact stdout/saved/prepare equality except root version, because changed-only capture deliberately changes discovery work.

The sections below describe the separately commissioned historical0.10.1/0.10.0 replay only. They are not a current0.12 launch recipe. The legacy driver refuses another candidate version; its equal-binary acquisition/context assertions describe Full-context API behavior, not ChangedOnly work. Current coverage is draft until its own declared-target live pass.

## Historical run and readiness

Declare the candidate's clean full source head, immutable signed binary and SHA256. Also supply a frozen 0.10.0 comparator with version, digest and provenance. Source equivalence does not identify distributed artifact bytes; label a local comparator as local. Read help, then run from the monorepo root:

```sh
node scripts/verify/sunstack-rule/rulebook/rulebook-since-encoding.ts --help
node scripts/verify/sunstack-rule/rulebook/rulebook-since-encoding.ts \
  --binary <candidate> --source-root <checkout> --commit <full-head> --sha256 <digest> \
  --baseline-binary <0.10.0-binary> --baseline-sha256 <digest> --evidence <new-directory>
```

The script owns small disposable Git repositories with a mechanical Markdown rule, two checked documents and an asset outside the checked glob. Only `docs/one.md` changes after the base commit. The same fixture is driven by both binaries. Keyless `lint --check` acceptance is readiness; version alone is identity only. All children use isolated HOME and credential store off, with no provider credential. The owned loopback witness must capture zero requests; no model answer is supplied.

## What decides the result

The old binary must refuse the unchanged eight-byte PNG signature with `UnsupportedEncoding`. The candidate must execute exactly one changed checked piece, `docs/one.md`, without widening the fixture's scope. It must also save a prepared report with that same mechanical result, no model request, and the documented prepare exit. This rejects a superficial success that drops the changed text or checks every peer.

Change the checked document to invalid UTF-8 and require the same explicit refusal. Separately change the asset from binary to text and from its old binary bytes to different binary bytes. Both remain selected changes under repository-wide capture and must refuse; a fix that silently ignores binaries or decodes them lossily fails these controls. Restore each fixture file in `finally` before the next control.

On a binary-free fixture, compare ordinary JSON stdout and saved reports, then prepared JSON stdout and saved reports. Require byte equality except the explicitly identified root engine `version` field; validate both expected versions and leave every other byte and order intact. Named-path or `--all` success alone cannot prove the `--since` fix.

## Native joins and original input

Inspect permanent tests and their current-target raw runs for:

- real Git revision-to-worktree capture with an equal binary and exact changed-source refs/identities/spans;
- identical-blob commit/index comparisons;
- an equal-skipped binary that remains stable when rewritten with the same bytes and refuses `SourceChanged` after different bytes;
- exact file/file-byte/total-byte budgets and just-below limits, including acquisition of equal binary bytes;
- selected invalid before-side and after-side encoding refusals;
- empty additions and deletions remaining distinct from absence;
- unchanged valid UTF-8 frozen-context attribution, plus existing file/index mutation verification.

These are API-layer guarantees; CLI counts alone do not prove them. Product tests belong to the Implementer. Preserve semantic red separately from compilation, fixture or schema-assertion failures.

When the commission names an original affected repository, drive that binary and config there read-only using `--since <base> --prepare <outside-output>` and credential store off. Compare head/status/content before and after, and inspect prepared owners and selected locations against real changes and declared dependencies. Rule/config changes may legitimately widen scope, so establish that condition rather than forbidding widening universally. Keep the exact original unchanged PNG control as a separate disposable fixture; never modify the affected repository to make the probe pass.

## Evidence, completion and limits

Keep both fixtures' Git base/current identities, original binary bytes or digest, commands/exits, stdout/stderr, saved reports, per-child source/binary receipts and `harness.json`. A passing `pass.json` must be joined with an error-free harness and owned teardown. After failure, retain the partial receipt and label setup failures separately. Close only owned children/server; leave the original repository, shared caches and other agents' state untouched.

Report the minimal fixture, changed-encoding controls, binary-free byte comparison, native joins and any commissioned original-input run separately as supported or unknown. No zero-request result establishes provider behavior, Linux/distribution behavior or release acceptance. This recipe is operable only after its declared target live pass; an unrun staged script is draft.
