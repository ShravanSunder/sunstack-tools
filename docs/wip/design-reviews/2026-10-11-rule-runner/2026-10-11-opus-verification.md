# Retained Opus reviewer: correction verification

Captured 2026-10-11 from the same independent reviewer, `the independent Claude reviewer`, configured `opus` / `high`. The initial report is preserved separately in [the original review](2026-10-11-opus-review.md).

The first correction verification is for program-design SHA256 `1f80043edeb27ec244d4c4a62d77b51a58d169396a96608592d836a1f9b49e10`. The final verification is for program-design SHA256 `d5cd4d9f85d76249110df2b1b2260bb589e496e98a9d8ed82958ee13bf739f48`, specification `107e71705affb9d504aa48eba9113963a312685d4e6aa99a87ca9724a59d36d1`, and unchanged requirements `ba83e9aa2141c26bb20c0e360f0438d763f78d04bcac7a530f96a1da5c2ecb75`.

Final reviewer result: **decision-needed**, with no open revision findings. This is independent design-source verification, not execution, typecheck or hosted-provider proof. Parent disposition and owner continuation stay on work root `private-coordination-record`.

## First correction verification

# Correction-pass check on the rule-runner design: F1–F10 closed, two minor gaps remain

All ten original findings are closed in the current files. Two new, minor gaps sit in the Program Design. The design is still not ready: G1 ("don't pass through") and your confirmation of the structure (`program-design.md:244`) are still unanswered.

## What I checked
- **Files and hashes:**
  - `specification.md` (134 lines): `107e7170…` (matches what you supplied).
  - `program-design.md` (246 lines): `1f80043e…`, hashed by me now.
  - `requirements.md`: unchanged at `ba83e9aa…`.
- **Read:** I re-read both changed files in full and checked every original finding's current anchor.
- **Diagrams:**
  - I opened four current diagrams: spec-preview-2 and design-preview-2, 3 and 4. Each matches its Mermaid source.
  - I didn't open spec-preview-1 or design-preview-1. Their source text (spec L7–14, design L7–19) is unchanged from the baseline I already inspected.
- **Outside sources checked:**
  - **SDK logging:** in the TypeSafe SDK `client.ts:157–161`, an explicit `logLevel` wins over `TYPESAFE_LOG_LEVEL`. In `logging.ts:5`, `"off"` is a valid level.
  - **Confidence range:** `docs.typesafe.ai/primitives/choice` and `/score` both returned HTTP 200 and both say "A number from 0 to 1 computed from how probabilities is spread".
- **Not done:** no repo edits and no board post. One thing outside the brief: the docs fetch saved two scratch files, `/tmp/ts-choice.html` and `/tmp/ts-score.html`.

## Status of each finding

| F | Status | Current anchor | Evidence |
| --- | --- | --- | --- |
| F1: how validated data becomes literal-typed answers | Closed | PD L133–135, L224 | The Zod base parse is followed by one private check, `answersMatchQuestions<TQuestions>`, covering every name, kind, label, index and range. The public return type is the explicit mapped `StructuredRunOutcome<TQuestions>`. No `as` cast. V2 and V3 are the joint proof, not yet run. |
| F2: how rule modules declare questions | Closed for names and labels; see residual 2 | PD L65–67, L224 | Exported `NamedQuestions`, `JudgmentAnswers<T>` and `StructuredRunOutcome<T>`. Declarations use `satisfies NamedQuestions`, and the run takes a `const TQuestions` parameter. V2 covers inline and imported declarations. No builder language added. |
| F3: backend construction and config validation | Closed; see residual 1 | PD L82–92, L74–75; Spec L78, L96, R3 L61 | `createJevBackend` returns a closed result: `backend-ready`, `invalid-configuration` or `backend-unavailable`. A failed preparation is not a run. The runner no longer claims it can inspect credentials. A throw from `execute` becomes `unexpected-provider-error`, with no second retry controller. |
| F4: Bun required beyond the runtime | Closed: an agent-added extra was dropped | Spec C5 L110, R13 L71, V8 L123, G4 L134; PD L213 | The blanket all-tools-on-Bun requirement is gone. Bun proof of library execution is kept and named (V1, V4, V5). No Node substitute is allowed for that proof. I accept the Lead's reasoning: "bun is for running" and "i meant for runtime" limit Bun to the runtime, and nothing you said asked for tools on Bun. If you meant tools too, this is easy to reverse. |
| F5: abort check after the body arrives | Closed | PD L167–168, L176–177, state table L196 | Two checkpoints: one after the awaited body, one immediately before returning `completed`. Both have `cancelled` branches, matching C3 at Spec L98. |
| F6: what V6 proves, and where its fixture lives | Closed | Spec V6 L121; PD L215, L231, L234 | It's a code-reuse proof: a scripted conforming backend, the real runner and validation, independent expected results, and an invalid-response control. The fixture lives at `packages/rule-runner` → proof-harness/consumer-fixtures. It claims nothing about semantic accuracy or migration. |
| F7: backend interface placed in two components | Closed | PD L44, L54, L80, L236 | The interface and preparation types live only in rule-contracts; the private adapter and config live in decision-backends. The trace table now names the real owners. |
| F8: backend attached to two nouns | Closed | Spec E3 L24, E4 L25, E5 L26 | E4 holds only state and questions. E5 selects the backend. E3's preparation states are separate from run state. |
| F9: environment can turn body logging back on | Closed | PD L107, L199, L232 | The SDK always gets `logLevel: "off"`, and I confirmed explicit beats environment in the SDK source. V7 includes a `TYPESAFE_LOG_LEVEL=debug` sentinel case. |
| F10: no source for the [0,1] confidence range | Closed: source found | Spec C2 L90 | I fetched the primary docs myself and they define confidence as 0 to 1. Range validation is not weakened. |

The `file:` dependency cost from my earlier observation is now named, at PD L207.

## Two remaining gaps (both minor, both for the Program Design)

**1. A backend that returns a malformed result, rather than throwing, isn't contained.**
- **Where:** PD L92 vs L80.
- **What's wrong:** L92 says the runner keeps a single clean outcome "when a backend implementation violates its declared return contract", but it only covers throws and rejections.
- **Example:** an injected or alternate backend (V9) returns a wrong `kind`, an unbounded `reason`, or a `provider-failure` with extra raw error fields. The runner would pass that object through, which breaks the clean-outcome guarantee (R7) and the no-raw-error rule (R10).
- **Fix:** parse `execute`'s result against the closed backend-result schema. Turn anything that doesn't match into `unexpected-provider-error`, and rebuild failure variants from bounded fields only.
- **Route:** `program-design`.

**2. The claim that rubric index keys stay literal is wider than the authoring path supports.**
- **Where:** PD L135 ("narrow names, choice labels and rubric index keys") vs L67 (`satisfies NamedQuestions`).
- **What's wrong:** `satisfies` against a schema-derived `string[]` doesn't turn an array literal into a tuple. So an imported score rubric loses its literal index keys.
  - The other way to get literal keys, `as const`, produces a readonly tuple, which a mutable `z.infer` array type rejects.
  - R2 only needs names and choice labels, and V2 (L224) only proves those. So this is an overclaim, not a broken requirement.
- **Fix:** either limit L135 to names and labels (rubric keys stay `number`-indexed for imported declarations), or name a tuple-preserving way to author rubrics (for example, readonly criteria types) and add it to V2.
- **Route:** `program-design`.

**Small diagram gap:** the main run diagram (PD L139–186) doesn't show the new branch where a throwing backend becomes `unexpected-provider-error`. The prose covers it. You can fix this along with residual 1.

## Result
**needs-revision**, limited to the Program Design: residuals 1 and 2, minor. One more bounded correction pass is enough; this doesn't need another full review cycle.

After that, the result becomes **decision-needed**. It is not ready for planning, and nothing here allows planning or implementation to start.

**Remaining owner decisions:**
1. **G1:** what extra guarantee you meant by "make sure they dont pass thorugh". The drafts already cover two readings: hiding provider types (U12) and keeping failures from becoming passes (R7). Any other meaning, such as the System One wire shape counting as a "provider detail", is unanswered.
2. **Structure:** do you confirm the proposed structure at PD L244? Library-owned schemas and mapped inference, an injected backend, the SDK kept inside its adapter, source exports, and consumer-owned composition.

**Still unverified evidence (not decisions):**
- G2: a real alternate provider.
- G3: a distribution destination.
- G4: proof that the tooling and Bun runtime actually work together (V1, V4, V5, V8). These are not run.

I've kept this review's context for further corrections.

## Final residual verification

# Residual check: both remaining Program Design gaps are closed

Both gaps from my last report are closed, and the diagram gap is closed too. With nothing left to revise, the result moves to **decision-needed**. Two owner items still block readiness: the extra meaning of G1 ("don't pass through") and your confirmation of the structure.

**What I checked**
- **Files.** `program-design.md` is 254 lines, hash `d5cd4d9f…`, matching what you supplied. `requirements.md` (`ba83e9aa…`) and `specification.md` (`107e7170…`) are unchanged.
- **Scope.** I read L60–253 of the design, which covers every changed region. I also inspected the rendered `design-preview-4.png`; the render matches its Mermaid source (L139–194).
- **Limits.** I made no writes, board posts or new full review.

## Residual 1: unchecked backend results (governed by R7 and R10)
**Closed.**
- **L92:** the entire `execute` result is checked against the closed backend-result schema in rule-contracts before the runner branches on it. The check covers the discriminant, a bounded reason, an optional finite integer status, and the permitted fields.
- **L92, failure handling:**
  - A malformed result, a throw or a rejected promise becomes `unexpected-provider-error`. Nothing raw is returned and there is no retry.
  - Failure and cancellation outcomes are rebuilt from checked fields, not passed through.
  - A received body is still checked once, separately, against the request.
- **L92, proof:** V9 includes wrong-kind, wrong-reason, extra-raw-error and rejection cases. V7 checks that none of that data escapes in returned diagnostics.
- **L203:** the state table now checks the result before branching.
- **L237:** the R7 trace row includes containing malformed results and V9's wrong-result cases.
- **L240:** R10 still holds; no raw errors or values are returned.

**Optional note (doesn't change the status).** The rebuild path takes a backend's `cancelled` result at face value. A misbehaving backend could report `cancelled` (reason `caller-aborted`) when the caller never aborted. A one-line rule would close it:
- Accept a backend `cancelled` only when `signal.aborted` is true.
- Otherwise, treat it as `unexpected-provider-error`.

This is optional hardening and does not block the design.

## Residual 2: promised literal rubric indexes (governed by R2)
**Closed.**
- **L135:** the type narrowing now covers question names and choice labels only.
  - Imported score rubrics may stay wide arrays, so the legend and probability maps stay numeric-indexed, and lookups may come back empty.
  - Actual rubric index sets and score bounds are checked at runtime.
  - V2 proves only the required name and label types.
- **L67, L123, L248:** these lines are consistent and make no rubric-index promise.
- **What R2 still requires** (name and choice-label inference, plus strict rejection of wrong names and labels) is intact at L232.
- **No new owner decision.** The dropped promise was something an agent added and R2 never required.
- **No new helper or language** was added.

## Diagram gap
**Closed** (L154 and L160–192, plus the render).
- **Throws and rejections:** a throw or rejected promise has its own return path from the execution owner to `provider-failure (unexpected-provider-error)`.
- **Returned results:** a returned result goes to the contract-validation owner and branches three ways:
  - a malformed result is refused;
  - a bounded failure or cancellation is rebuilt;
  - a received body goes on to the abort checkpoint, request-relative validation and the final admission check.
- **Scope of the diagram:** the Jev path is shown as the representative one. It makes no claim about SDK or HTTP behavior for alternate backends that haven't been chosen yet.

The nesting indentation in the source is uneven around L163–192, but it renders correctly.

## Result
**decision-needed.** I accept no open revision findings. All of F1–F10 and both residuals are closed against the current hashes.

Two decisions still need you:
1. **G1:** what extra guarantee you meant by "make sure they dont pass thorugh". The drafts already cover hiding provider types (U12) and keeping failures from turning into passes (R7). Any other meaning is still unanswered.
2. **Structure:** confirm the structural proposal at `program-design.md:252` or change it.

Nothing is admitted for implementation, and no runtime or type proof has been run. These remain unverified evidence (V1–V9 not run), not decisions:
- **G2:** a real alternate provider.
- **G3:** a distribution destination.
- **G4:** the toolchain and Bun runtime actually working together.

I've kept this review's context in case there are further corrections.
