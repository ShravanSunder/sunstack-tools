# Develop the full Rust-authoritative TypeScript SDK

Planning result: ready. Originating planner: plan-implementation (Lead-authored). Supersedes: none.
Requested terminal: pr-ready-unmerged. Delivery grouping: single:typescript-sdk. PR topology: one-pr. Repository: ShravanSunder/sunstack-tools.
Planned-at: feat/typescript-sdk, b8bd05bd6a0462cac4f6a215ac55d6b25f01c5bc (engine0.12.0).

## Governing basis

Reviewed four-document design, same folder's Requirements, Specification, Program Design and Verification Design HTML files. Read every document including closed proof disclosures. Prior complete independent review and three correction rounds: `/private/tmp/2026-11-sunstack-sdk-lead/review-{1,2,3}.md`; engine-refresh closure is `tmp/sdk-delivery/design-refresh-closure.md`. Independent semantic review is ready with every finding resolved; final matching HTML check observed5 files,0 errors/warnings. Prior page pixels are partial and final document visual acceptance remains a Lead obligation; it is not evidence of SDK runtime behavior or a reason to withhold keyless product work whose contracts are closed. Current applicability: new language/Document/TargetAnchor/mechanical variants, root/containment/no-match behavior and mandatory internal ContextCapture are current Rust source. Canonical upstream authored language names differ from result snake_case names. CLI remains protected against the integrated current main baseline.

## Outcome and protected boundaries

Deliver a normal npm package `@sunstack/sdk` with generated concrete TypeScript config/result types and typed load/check/run. A developer or model can author the entire config/rule/evidence/filter/question/decision graph in TS, load and edit existing configs without rereading captured definitions, execute native Rust and use actual typed outcomes. No mandatory Zod, no TS enums, no weakened Rust domain types/newtypes/constructors. No hidden JSON/YAML definition project or fixture import to impersonate inline authoring.

Preserve existing CLI formats, commands, reports, roots/scopes and validation at integrated main. Add native SDK artifact build/CI paths; no registry publication, merge, tag, credential storage change, service, hosted sandbox or extra inspected languages beyond current engine. No rewriting history of the old experiment. Main changes may merge into this new branch; do not rebase.

## Workstreams and safe ordering

One accountable Implementer owns the product end to end, with a paired independent Verifier. They may use bounded native helpers within eligible models and disjoint paths. Shared Cargo compilation remains serial; format/TS checks may run independently. The Verifier owns apparatus only, never product tests/repairs or the oracle. This is one PR and one final independent-observation batch, with local integration gates at every slice. Contract/acquisition slices name their immediate native/SDK consumers below, not standalone infrastructure.

### S1 · Load/check from captured Rust definitions

Reads: Specification R1–R6,R11,R12,R13 and load-use/source-contract/matcher-contract; Program composition/current-paths/preserved-root/shapes/ast-boundary/engine-integration; Verification O1,O3 and U1/U2/U4/U5/U8.

Change the existing acquisition owners into content-based entry points while retaining file adapters and the same admission/compiler. Implement the design's captured data/origins and strict DTOs. Expose a shared host library entry in existing sunstack package; lower domain crates never depend on NAPI. Generate Rust-owned declarations using feature-gated Specta and the exact pinned upstream schema/wrapper exception. Do not copy the old seven-file filesystem trait draft. The native/TS package in S2 immediately consumes these seams.

Write surfaces: existing Rust contracts/acquisition/admission and package manifests; shared host library; workspace Cargo.toml/lock only for named dependencies. Keep CLI adapters as consumers of shared logic. Product unit/integration tests at existing conventions.

Earns O1/O3 portions via real shared Rust entry tests: discovery/relative sources, immutable A/copy/reload B, source metadata tampering, exact wide numeric boundaries and valid/invalid admission parity. Wrong build controls: disk reread, marker/ref bypass, unknown nested-rule stripping, unsafe integer rounding. Oracle: the specified R/V clauses and existing strict decoder/compiler, not generated output snapshots alone. Keep existing Rust suites; repair callers for new explicit context without weakening assertions. No new global CWD/environment mutation.

Gate before S2 acceptance: scoped Rust tests plus cargo clippy for affected crates with generation features. No S1-only user-facing completion claim.

### S2 · Installed native TS load/check/run with complete inline authoring

Reads: Spec complete reference example, public-api, R3–R8,R11,R13,R14; Program shapes/execution/lifecycle; Verification O1–O4.

Build `packages/sdk` and its NAPI adapter (responsibility names may refine within this home) using the design's shared facade and current Rust executor. Package scripts name build/generate/typecheck/lint/test/pack verification. Implement lossless data transport, closed outcomes/diagnostics including checked path, immutable loaded values and edited provenance, release/contract handshake, per-call cancellation and explicit absolute checked root. Exact Rust refinements remain authoritative. Before-dispatch failures have no effects; recoverable after-dispatch unknown reply is explicit; fatal in-process crash cannot return a value.

Move existing binary-only credential/backend resolver to the named shared host boundary preserving current identities/lookup order. Check never touches credential/provider/checked-target acquisition. Run acquires only the existing host services and never mutates process globals. Preserve ContextCapture::ChangedOnly lint selection and Full connected context. Current main's credential timing is characterized and follows its owner-approved changes when merged.

Product tests: actual TS compiler positive/negative callers, actual addon and Rust constructors; full no-definition-files example with filter/questions/graph and a synthetic checked repository; exact semantic requests/results against an equivalent file fixture with controlled existing HTTP backend; causal two-call/abort/pre-abort/partial/unknown effects checks. No temporary tests; fixtures in permanent tests/examples. Controlled model answers are the declared repeatability stand-in, not hosted proof.

Minimum execution commands from monorepo root: `pnpm --filter @sunstack/sdk build`, `pnpm --filter @sunstack/sdk typecheck`, `pnpm --filter @sunstack/sdk lint`, `pnpm --filter @sunstack/sdk test`; actual generated script names must be documented before proof. Fresh tarball consumer must work outside the repo with no fixture/workspace imports, Rust toolchain or Zod. Negative caller tests must prove intended compiler diagnostics after a positive counterpart compiles. Examples import the real package and author full values.

Earns O1–O4 and U1–U5/U7/U8. Gate: packed installed consumer invokes real load/check/run with exact sources/typed outcomes, plus CLI parity in S3. Missing run or full authoring cannot be marked done because check succeeds.

### S3 · Preserve current CLI and prove the host connection

Reads: Spec R7–R9,R11 and root rule; Program preserved-root/execution/lifecycle; Verification O4/O6 and existing verify-sunstack-rule routes.

Characterize and retain CLI root/subdirectory, named paths, default/all/since, explicit root and no-match/outside-root outcomes, saved report/readback and exact request state. Keep existing workspace tests and declared CLI journeys. Add only regressions needed for the changed seam. No legacy report schema migration. Separate definition-origin SDK projection from real checked file paths.

Required gates: `cargo fmt --all -- --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo nextest run --locked --workspace`, feature-enabled generation checks, existing scripts TS/OXC gates touched by changes. On this Mac use command-local SDKROOT/CC/CFLAGS per verification skill and the existing signing runner. Retain raw logs/counts/exits. No broad test reruns after green without a changed dependency/failure.

One bounded real-provider installed-SDK inference is required through the existing ordinary host backend, using synthetic text and the existing credential route. This gate is pending credential availability; no key or location enters source/report. Keyless and controlled-backend work proceeds at its named independent boundary. This is an explicit prerequisite split: the keyed S3 observation is held, not treated as met; S1/S2/S4 never depend on that credential. Lack of credentials blocks this observation and final hosted acceptance, not the keyless S1/S2 build; it never becomes a pass.

### S4 · Native platform artifacts and developer experience

Reads: Spec R10,R14,R15; Program packaging; Verification O5/U9/U10.

Produce matched wrapper/declarations/optional platform-native packages for macOS arm64/x64, Linux arm64/x64 glibc/musl. Add a scoped SDK workflow or build matrix integrated with existing CI conventions; do not change CLI release channels/caches needlessly. Packaging and native adapter may add the NAPI/build/Specta schema dependencies already selected by design. No runtime TS compiler embedded in Rust.

The same fresh consumer compiles and executes in all six cells. Cross-compilation alone is not a cell pass; use native CI runners or declared emulation/container runtime with that limit explicit. Unsupported runtime/platform, absent addon and intentional version skew return the exact closed failure before engine effects. No Rust compiler or Zod required by consumer. Need results on real cells before distribution-ready claim. gh authenticated locally; CI source push/draft PR is within grant; registry publishing is not.

DX deliverables: package README with install/import/typecheck/run, complete inline config example, existing-load/edit example actually running the edited config, typed result narrowing, invalid example/compiler evidence and why Zod is optional. Update the local experiment HTML only through Lead if a new reader walkthrough is requested; package examples are Implementer work.

## Proof map

| Observation | Earned by | Required real boundary and deciding command/route |
| --- | --- | --- |
| O1 | S1+S2 | SDK snapshot journey: installed addon, A/edit/reload B and negative acquisition/provenance; verifier-maintained permanent driver |
| O2 | S2 | SDK inline parity journey: no definition files, actual Rust executor and exact controlled backend request/results; `pnpm --filter @sunstack/sdk test` plus installed consumer driver |
| O3 | S1+S2 | Actual TypeScript compiler and native invalid inputs; all29 authored names/result union, numeric constructors, current graph/field/tools; package test and Rust tests |
| O4 | S2+S3 | Keyless check effect traps and causal independent call cancellation/partial/unknown report observations through real addon |
| O5 | S4 | Six-cell fresh-install workflow/driver; same caller, correct target/ABI, no-Zod/no-Rust, deliberate skew controls |
| O6 | S3 | Existing CLI journeys/refreshed native characterization and distinct real-provider SDK smoke; preserve current-main behavior |

Each U row uses its existing O mapping in Verification Design. Existing prototype tests are reference only and not copied as stale assertions; current workspace tests are kept, overlapping decoder/executor tests extended for the new path. Never delete/relax a gate to make the new surface pass. Concrete table/property cases are preferred for variant policies. Project proof-layer source: verify-sunstack-rule route table, current CI and existing Rust test suites.

## Integration, handback and stop conditions

S1 requires current design closure; S2 requires S1's shared admission/capture; S3 and S4 integrate S2. Verifier feasibility comes before proof apparatus creation; lasting SDK routes are added under a separate verification-skill commission before observation. The Implementer requests a paired observation only on one fixed full commit/build/package target and stops product writes for that observation batch. Corrections land as one batch and rerun affected proof once. Keep reports separate from governing documents.

Handback: exact commit, scoped diff, commands/counts/exits and raw receipt paths, installed package/addon identities, every O observed/not-run with reason, test wrong-build analysis, platform matrix and hosted boundary, language-standard departures, remaining risks. Then Lead opens deciding source/evidence, writes its acceptance, and commissions a fresh different-lineage delivery review. Draft PR remains unmerged. The owner retains merge/publication authority.

Return design breaks with assumed/found/consequence before crossing public/semantic/storage/security boundaries. Do not invent JSON fallback runtime, second policy engine, fake file origins, generic any, TS enums or temp definition directories. Routine implementation defects at named owners are repaired without asking. A failed environment/provider/platform gate stays explicit; it does not waive the full outcome or end independent work. Old dirty experiment remains untouched. Before final handback merge current origin/main (no rebase) and rerun only proof whose exercised dependencies changed, including the announced0.12.1 credential-timing correction if it lands.
