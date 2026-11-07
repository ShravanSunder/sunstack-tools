# Author the shared story page from the guide

Revision r2. This is the S4 authoring subdesign for one named skill, `demistify-explainer`, packaged in `packages/demistify/skill`. The product's accepted [four-document design](../../../specs/2026-11-07-show-me-page/index.md) continues to own the component contract; this set owns only teaching it.

## Need and scope

The owner asked to formalize the approved experiment carefully, keep rich understandable HTML, reuse the component system, and see how easily a Luna agent can create it. The governing source is the 7 November commission linked from the product Requirements. Product R9/R11 and plan S4 require matching component guidance plus a fresh guide-only composition.

Class: **behavior-changing update**. It adds a conditional main-path reference call and a new teaching reference, plus conditional pointers in navigation, inspection, maps, trees, traces and cards-and-callouts. Trigger, invocation, binding, trust, ordinary authoring validation and delivery policy stay as they are. This is user-directed intent; no observed skill failure or causal improvement claim is made.

Promise: this reference skill helps an agent compose supplied work accounts with native stories, sections, records and Map when a document needs that hierarchy, without copying private presentation code or reading runtime source.

Success means a fresh author can preserve the supplied text, rich forms, IDs, links, group/order and initial disclosure states using the matching guide alone. It writes ordinary HTML plus documented islands; no custom color/type/interaction implementation is needed. The first submitted artifact is preserved and judged before feedback.

Agents reach the existing `demistify-explainer` skill when writing Demistify HTML. The new chooser condition is a page-level `ex-map` reader panel, an `ex-story` account with labelled rows/supporting record, or `ex-section` story scopes; Map-only pages also use this reference. The separate presentation-html chooser is unchanged and its end-to-end routing is not evaluated by this run; in-scope Trace/card boundary pointers help readers reach the new guide when those references are loaded. Ordered evidence steps still use Trace; generic hierarchy still uses Tree; existing cards and unrelated documents retain their rules.

## Sources and baseline

- Existing `packages/demistify/skill/SKILL.md` and references `content.md`, `layouts.md`, `trees.md`, `traces.md`, `navigation.md`, `inspection.md`, `maps.md`, `cards-and-callouts.md` were opened at integration HEAD `94896328`.
- Product Specification R1–R12 and Program Design bind the actual native markup, defaults, shared sidebar/control owner and source-preservation boundary.
- Product implementation through `56e2fe84` plus proof-only integration `94896328` supplies the API being taught. S3 local full browser: 292 passing; Node: 240 passing. The independent native report is `tmp/verification/demistify/show-me-s3/terra/final-report.md`, with its exact-case limits. These are component observations, not guide behavior evidence.
- The fixed public-safe input and manifest are under `tmp/verification/demistify/show-me-s3/`; a separate content-only author request under `show-me-s4/` removes new component syntax while retaining supplied facts.

Non-goals: change the product API/design; write workflow skills in another repository; fetch Router records; invent meaning, state, classification or a story data loader; redesign the skill's trigger; add scripts/assets/configuration or credentials; release/deploy from this subdesign; claim general author competence from one trial.

Proof posture: drafted from approved intent; guide behavior not yet evaluated. An independent first-composition trial can show this named example works, not improvement over the old guide without a fresh control.
