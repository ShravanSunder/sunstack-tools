# Optional provider context ceiling

This repair implements the Lead's adopted contract at design
`a32d32f2707a3fbe2e8e2cd80df890dc8687e832`, superseding `8ac4133`.
The source base is milestone 1 `9fae2cca7cc4cc87ec364f14806e4950eab7fea9`.
The execution trace is root `01a1446b-0a1b-7c2d-8e3f-4a5b6c7d8e01`
on Router service `0ff962c5-7fa3-4c18-a5ca-1bbe8db09e89`.

`BackendSelection::JevOpenrouter.context_ceiling_tokens` is independently
optional. A supplied value must be a positive integer; explicit null is not
absence. `JevOpenRouterBackend::new/prepare` take only selection and credential.
The separate accounting profile argument and its stored copy are removed.
The native `ProviderContextCeiling` enum represents `caller_declared { tokens }`
or `not_declared`, and is reused by `TokenAccounting` and `AccountingBasis`.
No provider capacity is inferred from a model name or a synthetic maximum.

## Scenarios and oracles

Each expected outcome comes from the adopted contracts' `BackendSelection`
paragraph and Specification C8, rather than a production exit/accounting helper.

| Test in `backend_ceiling.rs` | Wrong behavior it rejects | Boundary kept real |
| --- | --- | --- |
| `backend_selection_allows_an_absent_ceiling_but_declared_ceilings_are_positive` | Requiring an absent ceiling, or accepting explicit null, zero, negative or fractional declarations | Real Serde decoding of the exported selection type |
| `caller_declared_ceiling_constrains_a_higher_request_limit_without_dispatch` | Ignoring a one-token declared ceiling under a 100,000-token caller limit | Public run, actual byte estimate and actual HTTP observer; zero connections |
| `selection_is_the_only_ceiling_source_and_exact_policy_stays_unverifiable` | A second ceiling source, misleading assessment basis, or treating an estimate as exact | Public constructor/preparation/accounting/budget assessment and real zero-call HTTP observer |
| `allowed_estimation_reaches_real_http_without_exporting_local_ceiling_policy` | Refusing valid estimated execution or sending local ceiling policy to Jev | Real reqwest adapter, literal response fixture and recorded HTTP body |
| `absent_ceiling_keeps_the_caller_limit_and_records_no_provider_claim` | Treating unknown capacity as an unavailable request limit, inventing a provider capacity, or bypassing the caller's limit | Public assessment and real HTTP: first request completes; one-token second request is refused; exactly one dispatch |

The existing `request_budget.rs` tables retain exact/estimated/unavailable
accounting, inclusive boundaries, overflow, encoded-body limits and backend
identity coverage. A declared zero ceiling is now unrepresentable natively
(`NonZeroU64`) and refused at wire decode; its former native zero-sentinel
case is replaced by explicit `not_declared` estimated accounting under
`require_exact`. The generic `TokenAccounting::Unavailable` case remains.
The old injected unavailable Jev profile is gone with that API; its HTTP
refusal case now exercises a real caller-declared one-token ceiling.
Other decisions fixtures change only their backend construction and selection.

## Observed runs, in order

All Cargo commands ran from the monorepo root with this command-local prefix:

```sh
SDKROOT="$(xcrun --show-sdk-path)" CC=/usr/bin/clang CFLAGS="-isysroot $(xcrun --show-sdk-path)"
```

1. Under the superseded required-field design, run `98f20608` failed for
   accepting an absent ceiling; its later green and sensitivity are historical
   only and do not prove this optional-field repair.
2. After the Lead's correction, the revised decode test failed against the
   required-field implementation: `2ff4a88a-e6b0-4192-bc7b-0c8421a63274`,
   exit 100, one failed, three filtered. The assertion was
   "an unknown provider ceiling must not reject a caller-bounded request".
   This is removal of the superseded requirement, not a defect attributed
   to the original milestone source. Log: `target/backend-ceiling-optional-red.log`.
3. The optional implementation passed all five focused tests in `f1ad12ba`,
   exit 0. Log: `target/backend-ceiling-optional-green.log`.
4. Deliberately changing the backend's declared-ceiling branch to `not_declared`
   made the source/basis and one-token refusal cases fail: `35788848`, exit 100,
   three passed and two failed. The mutation was restored.
   Log: `target/backend-ceiling-optional-declared-sensitivity.log`.
5. Deliberately bypassing the caller limit when capacity is `not_declared`
   made the absent-ceiling second-dispatch assertion fail: `eab28efc`, exit 100,
   one failed and four filtered. The mutation was restored.
   Log: `target/backend-ceiling-optional-caller-limit-sensitivity.log`.
6. Final `cargo fmt --all -- --check`: exit 0.
   Log: `target/backend-ceiling-final-fmt.log`.
7. Final `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.
   Log: `target/backend-ceiling-final-clippy.log`.
8. Final `cargo nextest run --workspace`:
   `cd37abe5-b05d-4092-9aff-843a8449b649`, exit 0, 86 passed,
   one credential-gated hosted test skipped, no LEAK markers.
   Log: `target/backend-ceiling-final-workspace.log`.

The deliberate faults demonstrate sensitivity after green; they are not
pre-fix reds. The additional accounting scenarios first ran green under the
revised contract. Existing quality/test gates remain enabled.

## Proof limits

The real HTTP scenarios establish native public-library execution and local
admission/refusal, not fresh hosted Jev behavior. No hosted run, executable
operation, S3a composition, deployed hook or measurement is claimed here.
The ignored hosted fixture compiles with no assumed provider ceiling.
The CLI and rules still require their separate implementation and observation.
Independent observation and acceptance belong to the commissioned Verifier
and Lead. Raw logs are checkout-local build artifacts; the scenarios and
commands above are permanent and rerunnable.
