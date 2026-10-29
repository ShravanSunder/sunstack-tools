# Rulebook lint: pieces, decisions and changed evidence

This journey drives the 29 Oct rulebook pipeline through the actual `sunstack lint` entry. Expected outcomes come from `docs/specs/2026-10-29-skill-rules-evidence-and-decisions/{specification,verification-design}.md`, especially R10–R15, R20–R28 and U2/U5/U8/U10/U12. The fixture is the Implementer's pinned S0 tree under `crates/sunstack/tests/fixtures/rulebook-lint/`; its `expected/` values come from the source copies and ast-grep 0.51.0, independently of the builder.

Inputs: a declared source commit, local binary path and configuration/client, unchanged product sources and binary bytes, the pinned fixture tree, and a fresh authorized evidence directory. The current driver is `scripts/verify/sunstack-rule/rulebook_lint.py`; read its header and `--help`. The entry returns the source-declared EvidenceListing envelope; finding locations use Finding.line and combined values use JudgmentSummary.value. Recheck these types at the declared S3 target. Until a real target run succeeds, this recipe is draft; fixture preparation establishes no product behavior.

From the monorepo root, using the main skill's clean uv environment:

```sh
uv run --no-config --no-env-file --no-python-downloads --script \
  scripts/verify/sunstack-rule/rulebook_lint.py \
  --prepare --evidence <new-authorized-directory>
```

For observation, omit `--prepare` and add `--binary <absolute-local-binary> --source-root <product-checkout> --commit <full-source-commit>`. Build in the checkout's `target/` using the pinned toolchain and main skill SDK prefix. The calling client is this local Python driver; loopback replaces only model answers. It starts no browser and makes no hosted call. Target checks bracket every child; a source or binary change stops observation.

The driver observes these four parts:

1. `--evidence` for every configured kind equals the independently scanned file/range tuples, in order; selected step/load fields match source expectations. Step 3 is bytes 6114–7909, lines 49–60, heading `3. Choose each observation`, completion line 59; its load resolves the named reference. The description decodes to 450 characters. No key is supplied and no request is sent.
2. A full K001–K003 run uses captured, scripted HTTP replies. K002 is an error at line 3; K001 combines 0.30 × 0.6 + 1 × 0.4 = 0.58 and warns at line 59; K003 at 0.91 passes. A separate step-3 fixture without completion settles at line 49 with no request; ast-grep independently checks that the fixture still reaches that step. Questions remain structurally identical to the authored native question objects, with their strings unchanged, and each request contains one piece's state without raw `text`. Separate K001-only 0.65 and top-choice-0.55 fixtures return uncertain/exit 5; the full fixture returns findings/exit 1 while retaining uncertainty. Stdin stays open and empty until exit so an input wait fails the completion bound.
3. Temporary Git repositories establish step-3-only selection, five unchanged steps excluded, changed-load selection, rule-file widening, a wholly selected untracked skill and repeated semantic output/request order. Fixture Git writes use explicit synthetic identity and no hooks/signing. Preserve logical report request order and compare exact bodies in that order; concurrent arrival-side bodies are a multiset. Never sort the report to manufacture determinism. Where the CLI exposes indexed request states, join captured exact bodies to those states in emitted report order. Usage timings/receipts are compared separately, without requiring equality.
4. An extra rule naming an undefined kind is skipped while peer semantic results stay equal to baseline, coverage is partial and exit is 3; a malformed whole config is invalid/exit 2 with zero reported checked/target acquisitions. Join the recording-source test below for actual content acquisition and change-capture invocation.

Before driving, readiness is a successful fixture `--check` and its known mechanical K002 error, as covered by S1. Keep per-child stdout, stderr, actual command/exit/PID, source commit and binary hash, raw HTTP bodies and harness cleanup evidence. The driver owns its process groups, responder and server; process completion and request events drive waits. Stop only those owned resources. Fixture copies are disposable after cited evidence is preserved; cleanup is governed by the assignment.

## Required native joins and honest limits

The journey is a binary observation, not all of U1–U12. Inspect and reuse the Implementer's current-target tests where they already drive the real boundary:

- U1/U3: linked project loader equals `ast-grep scan` for every project rule, including composed utils, path globs and multiple YAML documents; each standard util has positive and negative witnesses. The binary piece listing alone does not expose every helper rule's raw match table.
- U2/U5: field-state/target/root-read tests, scalar/list/multi-target reductions and raw-field checks, location/threshold/walk tables, TooLarge lazy asks and unavailable-field isolation.
- Amended R11/R17: a resolved target whose read fails makes its whole target field unavailable. An unavailable field explicitly named by any question's check/against/context blocks the first ask/combine with no HTTP. Unreferenced unavailable fields are omitted from sent state and block nothing; the exact-state comparator applies that omission rule.
- U6: capture a non-ASCII escape-heavy/long-question body with a generous cap, accept that exact body at its Unicode-character count, then refuse it at count minus one while a smaller peer completes. Byte length must exceed character count so the equality case rejects a byte-count guard. Compare the exact body encoder with actual HTTP bytes. A refused request has no same-run endpoint body to measure.
- U7/U12: `crates/sunstack-lint/tests/rulebook_lint_gates.rs` records run-path content acquisition/capture gates and unchanged peer work; a self-reported counter alone is insufficient. U9 instead joins the check-only structural call path and no-writer FIFO witness in the refusal journey.
- R27: compare pieces, fields, results and request records in emitted logical report order. Exact captured bodies are joined to request-state records without sorting the report; HTTP arrival order may vary under concurrency. Shuffled-completion native proof separately checks result/request-record collection.

Report each row supported, unsupported or unknown with the exact target and observed/reused evidence. Missing joins stay open. Loopback proves request/decision mechanics; real hosted admission, judgment quality, release contents and #171 consumer coverage remain separate. Complete when the selected drives, their native joins or named gaps, and owned teardown are accounted for; a prepared fixture or draft recipe is never a pass.

`usage.checked_files_acquired` counts distinct checked/target files acquired by change capture or checked_file_source. Frozen after-side text is reused; snapshot-verification rereads are excluded. Join zero acquisitions/no capture before the key gate or in check, plus one acquisition per changed file. This is not a physical-read counter or claim.
