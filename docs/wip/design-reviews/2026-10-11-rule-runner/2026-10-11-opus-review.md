# Captured independent rule-runner review

Captured 2026-10-11 by the Lead from Agent Router operation `private-coordination-record`, terminal `end_turn`, output available. Reviewer: `the independent Claude reviewer`; settings `opus`, effort `high`. The provider exposes the alias, not a resolved full model ID; Claude Code documentation maps the normal `opus` route to Opus 5.5.

This is the review report for the pre-correction snapshots, not a current design-ready claim:

| Artifact | Reviewed SHA256 |
| --- | --- |
| requirements.md | `ba83e9aa2141c26bb20c0e360f0438d763f78d04bcac7a530f96a1da5c2ecb75` |
| specification.md | `f103e4f9fe347b31874c89e6e2a1ca96c13c187dcb87be659eb04b5af0664ff2` |
| program-design.md | `d6f92d74adde2d96f64d0f8aa125adb02d608a6da9a17d6fba2d9de112970bd6` |

Parent dispositions, corrections and current verification belong to work thread `private-coordination-record`. The report's line numbers refer to those reviewed snapshots. Its upstream-main SDK reads are distinct from the separately inspected published SDK 0.6.0 declarations. The raw report is preserved below, including its own stated limitations.

---

# Three-artifact design review of the rule-runner: needs revision

**Verdict: needs revision.** The design is not ready for planning. Next skill: `spec-design`, with all ten findings below; then `program-design`. Readiness is also held up by three open owner decisions, including your confirmation of the structure, which you haven't given yet.

The review was read-only, on Opus 5.5 at high effort as the host reports. I made no repo edits, installs, inference calls, commits or delegations. One file was written outside the repo: plan mode required a summary at `~/.claude/plans/you-are-a-witty-cloud.md`.

## Coverage
- **Documents reviewed:**
  - `requirements.md` (76 lines), `specification.md` (132) and `program-design.md` (193). Their SHA256 values match the commission.
  - The handoff and copy-paste prompt.
  - The TS rules.
  - Repo HEAD `4e55750`; only `.gitignore` and `LICENSE` are tracked.
- **Rows walked:** U1–U13, R1–R14, E1–E8, C1–C5, V1–V9 and G1–G4. All 14 R rows trace back to U rows.
- **Diagrams:** I opened all five rendered diagram PNGs and they match their Mermaid source.
- **Checks:**
  - Completeness check: done.
  - Scope check: done. Every design element maps to a requirement; none is missing.
  - Proof-command check: skipped, because the design cites no commands.
  - Public API contract and Bun runtime/test-harness checks: done.
- **External sources:**
  - `@typesafe-ai/sdk` 0.6.0 source on main: `systemOne`, `validateQuestions`, `parseBody`, retries, abort and logging.
  - Docs for pnpm `file:`, Vitest's module runner, and tsdown's runtime requirements.

## Findings

### Fix in the Specification

**F4 — Bun is required beyond the runtime the owner asked for.** Important; contested until you decide.
- **Anchors:**
  - `specification.md:108` (C5)
  - `specification.md:121` (V8)
  - `specification.md:132` (G4)
  - `program-design.md:162`
- **What's wrong:**
  - The drafts require tool launch, test workers and builds to run under Bun, and say a Bun-launched wrapper isn't enough.
  - Your words limit Bun to the runtime: "bun is for running", "i meant for runtime", "not as othe rhtings".
  - Vitest's docs say `vi.mock` and `vi.hoisted` don't work on Bun.
  - tsdown on Bun is experimental.
- **Consequence:** the plan inherits a test and build gate that may not be achievable and that nobody asked for.
- **Basis:** U8, U13, R13.
- **Fix:**
  - Require Bun for library runtime paths and for the tests counted as runtime proof (V1, V4, V5).
  - Don't require Oxc, tsc, tsdown or the Vitest orchestrator to run on Bun unless you say so.
  - Keep "no Node runtime substitution" for the library.
- **Route:** `spec-design`, then `program-design`.

**F6 — V6's kind of proof is undefined, and its fixture has no home.** Important; accepted.
- **Anchors:** `specification.md:119`, `program-design.md:178`, `program-design.md:181`.
- **What's wrong:**
  - "Detects a planted defect" could mean live inference. That would be a semantic eval, which is out of scope.
  - It could also mean a scripted backend, which proves only the wiring.
  - The proof's owner is the Sunstack rule module, which lives in another repo.
- **Consequence:** the planner either runs an out-of-scope eval or writes a proof with no home.
- **Basis:** R9, R12, and the exclusion of an eval rebuild.
- **Fix:** state that V6 uses a scripted, conforming backend to prove both consumers run the same code, and name where the fixture lives in this repo.
- **Route:** `spec-design`, then `program-design`.

**F8 — The backend is attached to two different nouns.** Minor; accepted.
- **Anchors:** `specification.md:25` includes the backend in E4 (the request), but `specification.md:24` and the diagram at `specification.md:44` bind it to E5 (the run).
- **What's wrong:** the design passes the backend as a run argument, so E5 is the consistent home.
- **Fix:** remove the backend from E4.
- **Basis:** E3/E4/E5, R14.
- **Route:** `spec-design`.

**F10 — The [0,1] confidence range has no primary source.** Minor; unverified.
- **Anchor:** `specification.md:86`.
- **What's wrong:**
  - The SDK documents a range only for noul.
  - The Specification's own rule at `specification.md:88` requires primary evidence before adopting a provider constraint.
  - Earlier in this work, the OpenRouter Jev doc returned a 403, so I couldn't confirm the range there.
- **Consequence:** validation could reject answers the provider considers valid.
- **Fix:** cite a source for the range, or check only that confidence is finite.
- **Basis:** R6, C2.
- **Route:** `spec-design`.

### Fix in the Program Design

**F1 — The design never says how validated data becomes literal-typed answers.** Important; accepted.
- **Anchors:** `program-design.md:92`, `program-design.md:47`, `program-design.md:171`.
- **What's wrong:**
  - Casts are ruled out.
  - A Zod schema built at runtime can't carry the request's mapped answer types.
  - L92 defers the decision instead of making it.
- **Consequence:** implementers will diverge, or add a hidden cast.
- **Fix:**
  - Name one narrowing step: a type guard or assertion function tied to the request, whose body checks every entry.
  - Allow it as the single narrowing at the boundary.
  - Prove it with V2 (types) and V3 (behavior).
- **Basis:** R2, R6, TS rules.
- **Route:** `program-design`.

**F2 — Rule modules have no public way to declare questions that keep their literal names and labels.** Important; accepted.
- **Anchors:** `program-design.md:64–66`, `program-design.md:51`.
- **What's wrong:**
  - Rule modules declare questions away from the call site, so labels survive only through `const` generic builders or `as const`/`satisfies`.
  - Because the SDK is hidden behind the backend interface, its `noul`/`choice`/`score` builders aren't exported.
  - `runStructuredJudgments` is the only named export.
- **Consequence:** the planner has to invent a public contract.
- **Fix:** define library builders or a declared-question type, and cover them in V2.
- **Basis:** R2, R9, R12, C1.
- **Route:** `program-design`. If authoring counts as observable C1 behavior, go through `spec-design` first.

**F3 — Creating a backend and validating its configuration has no result contract.** Important; accepted.
- **Anchors:** `program-design.md:76`, `program-design.md:83`, `program-design.md:105`.
- **What's wrong:** the design doesn't say:
  - when Jev config is validated, at adapter creation or at run time;
  - whether the factory throws or returns a closed result (the SDK constructor throws on a missing key);
  - how the runner can emit `invalid-configuration` for an injected backend it can't inspect.
- **Fix:** have the factory return a closed result, and say how a configuration error reaches the run's outcome.
- **Basis:** R3, C3, the TS rule on closed variants.
- **Route:** `program-design`.

**F5 — An abort after the response body arrives isn't checked.** Minor; accepted.
- **Anchors:** the state table at `program-design.md:141–146` and the sequence diagram.
- **What's wrong:**
  - C3 at `specification.md:96` says an abort observed before the answer is accepted returns `cancelled`.
  - The SDK doesn't check the signal during its final parse, and the design has no check either.
- **Fix:** check `signal.aborted` before returning `completed`.
- **Basis:** R11, C3.
- **Route:** `program-design`.

**F7 — The backend interface is placed in two components.** Minor; accepted.
- **Anchors:** `program-design.md:43` (decision-backends) versus `program-design.md:53` and `program-design.md:183` (rule-contracts).
- **What's wrong:** R14's owner, "Backend adapter", isn't one of the named components. R10's owner is only the Jev adapter, although outcome sanitization belongs to execution and validation.
- **Fix:** give the interface one home and name real owners in the trace table.
- **Route:** `program-design`.

**F9 — An environment variable can turn body logging back on.** Minor; accepted.
- **Anchor:** `program-design.md:148`.
- **What's wrong:** if the adapter doesn't pass an explicit `logLevel`, setting `TYPESAFE_LOG_LEVEL=debug` turns on body logging. The SDK redacts credential headers but not bodies.
- **Fix:** always pass an explicit, non-debug level, and prove it in V7.
- **Basis:** R10, C3.
- **Route:** `program-design`.

### Observations (not accepted as findings)
- **Usage on failed runs:** usage is dropped when a run returns `invalid-response`, which could skew the batching-cost measurement under U6. Fixing it would be a reviewer-added extension, so it's yours to decide.
- **pnpm `file:` cost:** with a `file:` dependency, pnpm installs the library's dependencies from the consumer's install and ignores the library repo's own node_modules. That cost of `program-design.md:156` isn't named.
- **Mirrored vs reused SDK types:** the handoff's line about reusing SDK types (handoff L153) conflicts with the library mirroring them. This is disclosed in `program-design.md:32` and justified by U12. It held.

## Owner decisions
1. **G1: what "don't pass through" adds.** The two readings in the Requirements already appear to be met: hiding provider types (U12) and keeping failures from becoming passes (R7). An open sub-question is whether the System One wire shape, used as the internal backend protocol (`program-design.md:76`), counts as a "provider detail".
2. **F4: what "fully bun compatible" covers.** Does it include test workers and builds, or only runtime?
3. **Structural confirmation:** your confirmation of the structure hasn't been supplied.

## What held
- Requirements, Specification and Program Design keep separate identities.
- Every U and E reference resolves.
- The closed outcome union and its reason sets hold.
- Failures stay distinct from judgments.
- There is one retry controller, delegated to the SDK.
- A non-JSON success body becomes `invalid-response`, which is consistent with the SDK's `parseBody`.
- An already-aborted signal causes no inference.
- Scope exclusions are respected.
- G1–G4 are disclosed honestly.
- This is a new codebase, which I verified, so there's no earlier implementation the design has to stay compatible with.

## Board post
The outcome is posted on root `01a12ca7…` as activity 6051, role reviewer. The router doesn't support inspecting `claude-local` sessions, so it couldn't verify my identity. I posted as `de78a696-0255-417b-bcef-81d5ae849157`, the ID in this session's own transcript path.
