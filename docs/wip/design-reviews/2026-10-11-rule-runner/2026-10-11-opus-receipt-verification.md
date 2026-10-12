# Retained Opus reviewer: native receipt correction

This is a Lead-written summary of the Router-returned source review, not runtime proof or design acceptance. Reviewer: `claude-local/de78a696-0255-417b-bcef-81d5ae849157`, configured `opus` / `high`. Existing review lineage is preserved; no new reviewer.

## Cost evidence and first verification

Operation `01a12d0b-b7e4-7951-9bf8-25cd32820db9` settled `end_turn`, effect applied, reconciliation confirmed. Reviewed Requirements hash `ba83e9aa2141c26bb20c0e360f0438d763f78d04bcac7a530f96a1da5c2ecb75`, Specification hash `651df5c496700b8b5c09bffdfc0989743e90aee1b044b02165d703d657611bbf`, and Program Design hash `98110bad466c80087b8860d655dda960af3446b6c230decf65a4aa321e0072d4`.

The reviewer independently retrieved OpenRouter's OpenAPI JSON, System One reference markdown and Jev guide markdown (HTTP 200), and inspected the exact upstream TypeSafe SDK commit named in [the research ledger](../../../../tmp/practices-research/2026-10-11-decision-backend-contract/research-ledger.md). The schema defines optional numeric billing cost; the guide identifies USD and says every response includes it. Optionality follows the schema. SDK annotations omit cost, while the parsed body is returned without projection.

Verified correction: U6/R8 already authorize preserving native usage; supplied cost is retained, absence stays unavailable, malformed supplied cost is refused, and prices/zero values are not invented. Cost is admitted explicitly by the Zod-derived receipt rather than exposing arbitrary SDK metadata. V3/V4/V5 cover the cases. Native answer and inference semantics are unchanged. The reviewer found no hidden owner decision in this correction.

One minor residual: the Program Design did not state whether unknown usage/receipt fields are stripped or rejected. The concrete failure is either spurious invalid response on additive metadata or undocumented omission. Governing anchors are U6/R8, R7 and C2; route is `program-design`. The Lead selected schema stripping for unknown receipt metadata, keeping known cost and the exact answer/backend-envelope checks, and added V3 observations at Program Design line 131. The reviewer also suggested an optional trace repair; Specification R8 now lists V3 alongside V4/V5.

First result: **needs-revision**, only the unknown-field policy residual. All earlier findings remain closed. The corrected residual requires retained-reviewer verification before carrying source-review closure to the current hashes.

G1 meaning and structural-realization confirmation remain owner-controlled. G2–G4 and V1–V9 remain unverified; no implementation or inference is admitted. No diagram owner, edge or lifecycle changed; the existing diagrams are reused without regeneration.


## Final residual verification

Operation `01a12d0e-1ff9-7f46-8fe1-1fe5083cf966` settled `end_turn`, effect applied, reconciliation confirmed. Current Specification hash: `4119f81fd43928fce93383499a413ee3537a0e11e59a776a22e96351c266529e`; Program Design hash: `674a848cb8b069407e78ef622a2160bae69af5972b4b99b5aef026f2e06a6bda`; Requirements unchanged.

The following is the final reviewer output returned by the supported Router operation. An unrelated integration-authorization notice is omitted. Its optional trace wording suggestion is not selected: the binding and V3 case already preserve the proof meaning, so another correction round is unnecessary. The structural proposal remains at Program Design line 252; the reviewer refers to its concluding open-items sentence at line 254.

# Unknown receipt-field residual: closed

The residual is closed and so is the R8 trace nit. This leaves no open review findings, so the result goes back to **decision-needed**. Two owner items keep it from being ready for planning: **G1**, what "make sure they dont pass thorugh" adds, and confirmation of the structure in Program Design's open-items section, now L254. This was a source review only. I wrote no files, posted nothing to the board, changed no mode, and ran no tests or inference.

**Hashes checked:** all three match what you sent.
- `requirements.md` `ba83e9aa…` (unchanged)
- `specification.md` `4119f81f…`
- `program-design.md` `674a848c…`

## The residual: what happens to unknown receipt fields
Program Design L131 now covers each part of the fix I asked for.

| Part of the fix | Status | Evidence at PD L131 |
| --- | --- | --- |
| Policy is stated | Closed | The completed-body and nested `usage` schemas both strip unknown provider receipt and metadata fields. |
| Outcome is stated | Closed | Additive metadata "neither invalidates a run nor escapes in the public result." A new provider field therefore doesn't turn every run into `invalid-receipt`, and raw metadata doesn't escape. |
| V3 case added | Closed | V3 now includes an unknown `usage` field and an unknown top-level receipt field. It expects the run to complete and both fields to be missing from the returned receipt. |
| Answer and envelope checks stay strict | Closed | Stripping explicitly doesn't loosen exact answer-name, choice and probability-key, rubric-index, or closed backend-envelope validation. That matches Spec C2 L86 ("Missing or unexpected keys cannot silently become a complete response"), PD L92 and PD L133. |
| Supported fields kept; nothing invented | Closed | The receipt keeps model, tokens and optional USD cost. The final sentence now says "supported native System One usage fields", so it no longer promises future fields the design would drop. Other receipt shapes stay under G2 rather than getting an untyped metadata escape hatch. |

**Authority check:** stripping unknown fields doesn't conflict with Spec R8's "native usage receipt". U6 (Req L38) asks for usage that makes batching economics measurable, and tokens plus cost meet that. Keeping or rejecting future unknown fields would be a reversible design choice, not an owner decision. Nothing new was selected about providers, IDs or pass-through.

## The R8 trace nit
Closed. Spec R8 at L66 now cites V3/V4/V5, matching the Program Design trace at L238. The rest of the specification still reads as before: E7 L28, C2 L86–92 and V3–V5 L118–120 have the same meaning.

## Optional, non-blocking
The R8 trace row at PD L238 still lists only "V3 present/absent/malformed cost" and doesn't mention the new unknown-field V3 case. L131 is the binding text and already includes that case, so this is about consistency only. Appending "unknown-field strip" to that cell would keep them in step.

## Carried forward
- F1–F10, the two earlier residuals, the diagram gap and the receipt-cost correction all remain closed.
- G2, G3, G4 and V1–V9 remain unverified. They are proof obligations for later, not things this review established.


