# Rulebook check refusals

This journey owns U9/R24's public boundary: the built `sunstack lint --config <path> --check` accepts a valid config and refuses each crafted invalid config, rule entry, marker or evidence-project construct before running checked content. Its oracle is the 29 Oct Specification R24, including the R1/R3/R5–R9 refusals it references. Expected refusal cases are authored in `scripts/verify/sunstack-rule/rulebook_check_refusals.py`, not generated from product diagnostics.

Inputs: the declared source/binary/configuration/client from the main skill, S0 fixture tree and a fresh authorized evidence directory. Read the script's header and `--help`. From the monorepo root:

```sh
uv run --no-config --no-env-file --no-python-downloads --script \
  scripts/verify/sunstack-rule/rulebook_check_refusals.py \
  --prepare --evidence <new-authorized-directory>
```

Preparation writes otherwise-valid copies with one targeted invalid construct each and a manifest; it runs no product. For an observation omit `--prepare` and add `--binary <absolute-local-binary> --source-root <product-checkout> --commit <full-source-commit>`. Use the main skill's command-local environment isolation. No real key or keychain read is permitted; a configured endpoint is owned loopback. The actual source/binary must stay pinned across the run.

Readiness is clean `--check` exiting 0 and listing K001–K003. Every refusal case must exit 2 and give a located diagnostic for the mutated construct. Old keys must name their replacement. The inventory covers missing/unknown/duplicate config fields, malformed syntax, marker consistency/hosts/size, kind/field/source/target definitions, evidence project/utility/parser failures, question schema, instruction field references, graph references/routes/cycles, tool/field shapes, weights, threshold bounds/equality, choice probability bounds and evidence size floors.

A case must reach its own refusal. Preserve duplicate JSON text and malformed syntax during endpoint setup; parsing then rewriting those files can erase the scenario. For nonfinite thresholds, distinguish a wire parser rejection from direct admission's semantic check; neither proves the other. A diagnostic for an unrelated invalid rule or missing file does not establish the intended refusal.

Keep the clean and negative stdout/stderr, process commands/exits and per-case manifest plus loopback capture counts. The driver brackets every child with target checks, holds stdin open and awaits completion; it owns and joins its server and responder. Missing paths, target drift or a harness exception are setup failures, not passing refusal cases. Preserve a failed run before any correction/rerun.

## Zero content acquisition and no backend

The binary journey observes zero HTTP and no credential requirement. Actual zero acquisition and zero backend construction require the complementary `crates/sunstack-lint/tests/rulebook_lint_gates.rs` recording-source test on the same target, with the lint pipeline real. Inspect its negative and clean cases and its assertions; a hard-coded usage count or an unreadable-file success would miss an acquisition that was discarded. Reuse that test's applicable real-path evidence instead of writing a second product test in the driver.

Return U9's conclusion with executed cases/counts/exits, clean acceptance, diagnostic identity, exact target, evidence and the recording-source/backend join or its gap. Until the built target and join are observed this recipe remains draft. No hosted, release, model-quality or acceptance claim follows.

`usage.checked_files_acquired` counts distinct checked/target files acquired by change capture or checked_file_source. Frozen after-side text is reused; snapshot-verification rereads are excluded. Join zero acquisitions/no capture before the key gate or in check, plus one acquisition per changed file. This is not a physical-read counter or claim.
