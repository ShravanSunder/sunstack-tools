# S1 — lint a repository with the rulebook config

Drive the current `sunstack lint` entry and compare findings, formats and exits with independently authored fixtures. The adopted oracle is the 29 Oct skill-rules Specification/Verification Design; the earlier runner's formats, scopes and native `exec` obligations remain applicable where that amendment preserves them. RuleSet severity overrides and an off switch are superseded behavior and now belong to refusal controls.

The entry script remains `.agents/skills/verify-sunstack-rule/scripts/lint_fixtures.py`; its implementation is `scripts/verify/sunstack-rule/lint_runner_rulebook.py`. Read `--help`. Both use the new `rulebook`, `check`, `evidence` and `decision_model` config. Run from the monorepo root using the main skill's clean uv environment:

```sh
uv run --no-config --no-env-file --no-python-downloads --script \
  .agents/skills/verify-sunstack-rule/scripts/lint_fixtures.py \
  --prepare --evidence <new-authorized-directory>
```

Preparation writes fixtures, independent expected results and a manifest, reports `product_observed: false`, and runs no binary. For a declared local observation omit `--prepare` and add `--binary <absolute-local-binary> --source-root <product-checkout> --commit <full-source-commit>`. The main skill owns build/toolchain/signing mechanics and target confirmation. `--case <name>` is a bounded rerun; its receipt names the selected subset and cannot prove the full route. Before the S3 target is declared and the first live pass succeeds, the recipe is draft.

Usable readiness is a valid fixture `lint --check` followed by K002's known error at line 3 and exit 1. The driver checks source commit/product cleanliness and binary hash before and after every child. Each process has isolated HOME, no OS credential-store access or inherited provider key, and empty open stdin. Only judged loopback cases receive synthetic authorization. Model responses are controlled; the binary, engine, files and formatting remain real. A child's exit is awaited with a bound, not inferred from a ready line. Cancellation waits for that child's actual HTTP request, then signals its recorded process group.

## Current coverage

The executable manifest owns case selection: mechanical error, clean, warnings and warning limit, ignored/empty candidates, missing key, real loopback judgment, in-flight cancellation, a file-limit incomplete result, clean check listing, invalid config, named path, ancestor default-config discovery, init/create/refuse/populate, and default/GitHub/SARIF formatting. Compare marker, file, line, severity, nonempty message, status, actual exit, incomplete parts and actual HTTP calls. Check native answers and request evidence where the report exposes them. An exit alone cannot show the right finding, and missing provider cost must stay unavailable rather than become zero.

The rulebook lint journey owns the complementary Git/definition/loaded-file/untracked selection, uncertain precedence and skip coverage. Run it for those adopted rows, then reuse its exact-target evidence here; do not duplicate the same real path. The check-refusals journey owns old keys, decision/evidence validation and the R24 matrix.

## Native joins and other targets

Inspect the Implementer's current-target tests and raw runs before joining them:

- Library/CLI report equality and the current report schema, including uncertain/field/read-count additions. Keep actual producer/consumer serialization real.
- `crates/sunstack-lint/tests/rulebook_lint_gates.rs` for zero content acquisitions and no change capture and zero backend construction. The binary counter and zero network alone are insufficient.
- Every retained native JSONL/conversation `exec` test, including admission assertions extracted before deleting marker-route tests.
- Decision-body size/encoding, typed answer admission, request ordering, field states and detailed threshold/location tables from the 29 Oct design.

The older `runner_fixture_setup.py`, `runner_fixture_cases.py`, `schema_fixtures.py` and their tests describe the former RuleSet runner contract. They remain historical/native apparatus sources; their preparation or passes do not validate the new config. `schema_agreement.py` still consumes that historical shape: current released-schema agreement requires a declared new schema artifact and the product's new config/schema tests, and remains a named gap until observed. Do not silently point the old driver at a new schema or discard its negative-control obligations.

Linux/container S1 and release-install targets require their own declared artifact, configuration and route. This local S3 commission supplies none of those artifacts; a local pass proves no static ELF or installed-package behavior. Keep those targets and their existing sources separate instead of generalizing a macOS result.

## Evidence and completion

Keep fixtures/manifest, per-child stdout/stderr/process records, raw loopback bodies, target identity and safe pass/timeout receipts. The driver awaits and stops only its owned processes/server; preserve evidence before deleting owned fixture copies under the assignment's cleanup rule. Reopen the retained evidence after teardown.

Return observed/reused/unknown coverage with case count, commands/exits, exact source/binary/client, native joins and remaining target gaps. No hosted, release, model-quality or acceptance claim follows from S1. A current native real-path test may close a join without a second manual run once its scenario, assertions and target applicability have been inspected.

`usage.checked_files_acquired` counts distinct checked/target files acquired by change capture or checked_file_source. Frozen after-side text is reused; snapshot-verification rereads are excluded. Join zero acquisitions/no capture before the key gate or in check, plus one acquisition per changed file. This is not a physical-read counter or claim.
