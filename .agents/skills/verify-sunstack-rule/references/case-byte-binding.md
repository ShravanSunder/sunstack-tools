# Exact ast-grep case byte binding

The adopted E5 Specification R1–R6 and Verification Design O1–O3 govern the expected outcomes. The route was exercised at `e695af9e3b6ec0bdbc143cabd45a402e4b42d1f7` on a macOS arm64 CLI and normally installed SDK/addon. Its successful controller exit is **3**: the CLI/SDK observations are partial and require the separate native and source joins below. Repeat against each declared target; do not infer other platforms, CI or release from that run.

The intended journey binds adjacent HTML paragraphs by the exact half-open UTF-8 byte ranges already produced by ast-grep, prepares and scores each independently, and reads back the correct original source and result. The fixture has opposite labels, fixed literal offsets and SHA-256 values, a multibyte character, CRLF, byte-preserving Git attributes and a later available unselected commit. The bounded Git forwarder records `git archive` arguments and delegates to real Git; it does not observe in-process git2 revision resolution. Its isolated sensitivity test is not a product observation.

## Prerequisites and partial route

Use an explicitly commissioned, quiescent source/CLI/build and a freshly packed, normally installed matching SDK/addon/compiler. Declare the complete installed target JSON expected by `sdkTargetSchema` including its matching CLI; record source, artifact, configuration and compiler identities. The whole observer batch must remain frozen. An owned loopback backend must bind at the actual operation; never use real credentials or a paid endpoint. The driver requires a fresh external evidence directory and checks target pins before and after.

```sh
node scripts/verify/sunstack-rule/case-byte-binding/verify-byte-cases.ts \
  --target <absolute-target.json> --compiler <absolute-tsc> \
  --old20 <absolute-document> --old20-sha256 <sha256> \
  --old21 <absolute-document> --old21-sha256 <sha256> \
  --evidence <new-absolute-directory>
```

At the cited candidate the route observed prepared/scored and individual-selection joins, duplicate bindings, genuine equal-range ambiguity, CRLF, historical readback, archived negative ranges, direct CLI/SDK controls, installed SDK compilation/codec and old-document incompatibility. It recorded process outputs and HTTP captures through the existing rulebook observer. Recheck these on the declared target, preserving failures and diagnosing without weakening the oracle.

## CLI-only released binary route

For a separately commissioned released-CLI check, use the same literal fixture and case/readback comparators without an SDK. Pin the existing clean source commit, the actual released or installed binary and its SHA256, and a new evidence directory outside the source checkout:

```sh
node scripts/verify/sunstack-rule/case-byte-binding/verify-byte-cli.ts \
  --source <absolute-source-root> --commit <full-source-commit> \
  --binary <absolute-cli> --sha256 <binary-sha256> --evidence <new-absolute-directory>
```

This exercises adjacent cases through real prepare, score, individual selection, exact request parity and original-span readback with controlled loopback answers. It does not establish that the binary came from a release channel: separately authenticate the tag/archive and compare the installed binary SHA to the archive member. Its CLI-only result does not replace the native child/read-attempt and git2 source joins, SDK observations, hosted judgment, other platform checks or O4. A new release needs a declared target and separate observation grant.

The composed CLI-only route was exercised on the retained `e695af9e`-built CLI using the clean `7554f141` source whose product/build inputs were checked equivalent. That capability run does not establish behavior of the later 0.24.0 release.

## Required separate joins

- Join the all-selected pre-dispatch child observation at the injected executable product-test seam. A late invalid range with HTTP silence alone cannot prove no earlier child started. Source-review the git2 selection edge separately.
- Repeat the regular readable-sentinel acquisition comparison through both candidate CLI and installed SDK with the same valid demand. Those public counts detect successful distinct acquisitions; pair them with the product's counting-source test for attempted reads. A FIFO cannot detect an attempted checked-source read rejected before open.
- At e695af9e, the injected-child and counting-source tests were inspected and joined from the Implementer's workspace run; they were not independently traced by the CLI controller. Git2 selection-before-resolution was joined from source, not observed by the external Git forwarder. Re-establish each join for a new target.

`old-specimen.ts` is a helper for a separately authorized old-codec probe; its `--help` lists the required pinned installed entry, synthetic root, revision and new output. It invokes the actual old public writer, checks the revision it produced, and does not rewrite an existing document. It does not itself establish that the new codec rejects that document. Never read unrelated frozen prototype inputs.

Retain original commands, exits, actual spans, case/result links, archived commit log, request bodies, package/compiler identities and exceptions. Close owned servers and retain evidence. O4's real 38-case join, hosted judgment, foreign platforms, CI and release are separate and cannot be inferred from this route.
