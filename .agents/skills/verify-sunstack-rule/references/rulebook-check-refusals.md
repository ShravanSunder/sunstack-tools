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

## Zero checked-content acquisition and no backend

U9 uses two complementary observations from the amended Verification Design. Inspect `sunstack-lint::check(&RulesConfig, CancellationToken)` and its call path: it holds no checked-file source or capture handle and reaches only definition/project admission and rule listing, with no backend construction path. Then run the binary witness: a valid config whose check.files selects a FIFO with no writer, outside Git, with no key. It must complete and list K001–K003 within the existing process bound; a timeout fails the journey. The driver prepares that FIFO and checks that its fixture is outside any Git worktree before the run.

The recording-source tests remain for run-path gates: missing key, malformed config, skipped peers and one logical acquisition per changed file. They do not stand in for this check-only structural/FIFO proof. `usage.checked_files_acquired` counts distinct checked/target files acquired through capture or checked_file_source, excludes snapshot-verification rereads, and makes no physical-read claim.

Return U9 with clean acceptance, executed refusal diagnostics/exits, the completed FIFO witness and the inspected structural call path, exact target and evidence. Until those are observed this recipe remains draft. No hosted, release, model-quality or acceptance claim follows.
