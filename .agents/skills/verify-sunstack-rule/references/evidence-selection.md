# Evidence selection: U13–U17

This extends the rulebook lint and check-refusal journeys for `docs/specs/2026-10-30-evidence-selection/{specification,verification-design}.md`. Selection is part of building evidence: a selected kind's mechanical matches are candidates, only `selected` candidates become pieces, and contained kinds and kind-based fields read those final pieces. The real model's classification quality and live all-skill V21 run belong to their separate commission.

The recipes are draft until driven on the declared S3 target. They do not infer a pass from prepared fixtures, a library test that supplies chosen sets directly, or the old 0.2.0 binary refusing the new key.

## Inputs and preparation

Use the main skill's source/binary identity policy. Keep the product head and binary bytes fixed through the observation. The canonical fixture is `crates/sunstack/tests/fixtures/rulebook-lint/selection/` in the product source, authored from the pin and scan oracle its `source.json` names. The driver uses `--source-root` both to confirm the target and to copy those fixture bytes; it writes only a fresh evidence directory. Never run root globs against the source fixture in the product Git tree.

From the monorepo root:

```sh
node scripts/verify/sunstack-rule/rulebook-lint.ts --case selection \
  --prepare --source-root <product-checkout> --evidence <new-authorized-directory>
node scripts/verify/sunstack-rule/rulebook-check-refusals.ts --case selection \
  --prepare --source-root <product-checkout> --evidence <another-new-directory>
```

For observation, omit `--prepare` and add `--binary <absolute-built-binary> --commit <full-head> --sha256 <declared-digest>`. Existing journey modes and expectations remain unchanged. This mode needs the new fixture and does not silently fall back to the old one.

## Selection through the CLI

The lasting `--case selection` route keeps the binary, filesystem, Git selection, JSON serialization, engine transport and decisions real. Only the remote model is replaced by owned loopback. Its scripted response is keyed by file and candidate key, with different answers for different candidates. It uses a literal synthetic authorization value, isolated HOME and disabled credential store. No credential lookup or external model call is allowed.

Readiness is valid `--check` admission followed by the worked selection listing. The raw request must carry exactly one unchanged noul question per candidate, with only `check`/`against`/`context` field names qualified as `candidates.cN.*` or `shared.*`; `state` is exactly `where`, `candidates` and one shared view. The source/scan oracle supplies candidate body/heading/outline values in the uncut worked case; the product does not generate its own expected value.

The route checks:

- Worked probabilities 0.93, 0.12 and 0.5: the research section is selected, not selected or uncertain; its contained step pieces are exactly lines 21, 22, 24, 25, 26, 27 or absent. Uncertain selection gives exit 5. The rule-list file answered no is named in default output. No-key selected evidence is invalid before acquisition; plain evidence still needs no key and sends nothing.
- Candidate identities and per-file counts: every asked outcome is retained, duplicate identities fail, zero-candidate files remain present, all configured kinds are counted, outcomes add to `asked`, `asked` does not exceed `candidates`, and selected-kind `pieces` equals `selected`. A file not asked under a change scope keeps candidate counts with `asked: 0`.
- Below/equal/above both thresholds; 33 short candidates split at 32 plus 1. A measured five-candidate encoded request splits at 3 plus 2, and an individually oversized candidate is not sent while peers complete. Input-token refusal names its own limit. Bounded shared state is independent of unequal candidate lengths. Selection carries no target opening or target-file fields, while selected pieces retain their own target fields.
- A real Git changed hunk asks only its file; a changed loaded target also triggers selection for its loading file and picks its dependent load with chosen-step context. Requests for rules appear only after every selection response has completed. One request budget covers selection and rules. Provider failure is contained to that file's selection; rules still run on surviving pieces.
- Repeated fixed-answer observations preserve selection/report order and exact bodies. Arrival order alone is not a determinism oracle; compare logical file/kind/candidate records, then correlate captured exact bodies to them.

Every child is awaited with the existing process bound, stdin held open, and target checks before/after. The owned loopback supports intentional HTTP failure and event-driven response control without counting that intentional response as a harness exception. Keep raw request bodies, response/control events, stdout/stderr, command/exit and target identity. A harness exception, timeout or missing input is not a passing product result.

## R40 refusals

`rulebook-check-refusals.ts --case selection` copies the canonical R40 mutations and checks clean admission before the negative inventory. Each must exit 2, issue a related config file/line diagnostic and send no request without a key. Review the actual retained diagnostic against each mutation: an undefined field must not accidentally satisfy a target-field prohibition, and a decoder rejection of nonfinite JSON is not the same proof as direct semantic admission.

Join the inspected check signature/call path for absence of checked-source/capture/backend handles, and the native exact-cause refusal assertions. The boundary counter or a nonblocking file alone cannot prove no checked read. Keep the original R24/FIFO suite unchanged.

## Native proof to inspect before reuse

U17's focused utility tests and regenerated bidirectional ast-grep oracle are required. Check all nine pinned `spec-program-review` stages and `##`/`###`/`####` fixtures, including `#` negatives. The expected scan output must originate from `ast-grep scan`, not the engine.

The staged-builder tests supply chosen sets directly and prove containment, `inside`/`enclosing`/`same_file` kind fields, strict-inner exclusion without self-erasure, forward-named target composition after its plain dependencies, and refusal of selection-dependent `as_kind`. They do not prove CLI wiring; the live route closes that boundary.

Inspect the selection-run and dispatcher tests for referenced malformed/non-string fields with a same-file peer, omission of unreferenced unavailable fields, independent shared reduction, exact encoded admission, concurrency across both stages and failure isolation. A count alone is insufficient: the assertion must fail for the named wrong behavior. The unchanged old rulebook tests characterize the dispatcher extraction; retain their full expectations.

## Temporary 0.2.0 compatibility observation

R42 compares the npm `@sunstack/cli@0.2.0` release executable with the declared build, using one frozen copy of the kit config and `plugins` sources named by the commission. Resolve the npm package with `pnpm dlx @sunstack/cli@0.2.0`, inspect its launcher to identify the actual executable, then record package version, native version and SHA256. The probe is TypeScript under authorized task scratch and is never committed as a product journey.

Run both binaries sequentially against the same configured loopback and identical fixed answers. Capture `--evidence` and full-run JSON plus exact request bodies. Compare pieces, fields, model views and ordered results; enumerate any extra numbered or Step heading matches at levels other than `###`. New report fields are `file_counts` and empty `selections`. Default selection lines have no application to this no-selection config. For identical results, GitHub/SARIF bytes remain unchanged.

Do not sort reports to hide order changes or drop entire kinds to make the comparison pass. A changed existing piece field or request needs an explicit source-based classification against R32, not an automatic exemption. Stop on any difference beyond the listed heading matches and return its reproduction under R42. This proves compatibility only for the named kit/config, not every possible 0.2.0 config or the release/install channels.

## Return

Return U13–U17 and compatibility separately as supported, unsupported or unknown, each with observed/reused evidence, target, commands/exits and named gaps. Keep live model answers/V21, static Linux and the kit's own count presentation outside this local commission. The Lead owns acceptance. Preserve evidence before removing only owned scratch and stop only recorded owned resources.
