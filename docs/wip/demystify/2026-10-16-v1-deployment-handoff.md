# Demistify experimental v1 deployment handoff

Public runtime: `https://demistify.getagentstudio.dev/v1/runtime.js`. Theme: `https://demistify.getagentstudio.dev/v1/theme.css`. Matching [agent guide](https://demistify.getagentstudio.dev/v1/authoring/SKILL.md), [component metadata](https://demistify.getagentstudio.dev/v1/custom-elements.json), [release manifest](https://demistify.getagentstudio.dev/v1/release.json) and [major catalog](https://demistify.getagentstudio.dev/release-catalog.json) are served from the same release.

Stage: post-review deployment handoff. Plan identity: none; the governing authority is the owner’s direct deployment, version-policy, public-hosting and merge instructions. No canonical implementation-plan acceptance is inferred. Repository: `ShravanSunder/sunstack-tools`, default branch `main`.

## Intent and ownership

Agents author compact HTML composition with Markdown, Mermaid and other reusable islands. The shared package owns rendering, styles, icons and interaction. Usage-token measurements exclude library implementation and generated assets; referenced content and author context remain separate measurements. The library is client-only. The lab/app server is optional and does not belong in every generated explainer.

The owner selected Workers Static Assets, public shared assets and hostname, historical major paths only, and initial 1.0.0. Experimental `/v1` may receive breaking changes until the owner explicitly declares strict major compatibility. Patch/minor updates replace `/v1`; the checker refuses majors above 1 while that policy applies. Generic history staging is already capable of preserving older majors when the later policy is approved.

## Source and release path

[Workflow PR2](https://github.com/ShravanSunder/sunstack-tools/pull/2) and [product PR3](https://github.com/ShravanSunder/sunstack-tools/pull/3) merged atomically at `c96b9763db5a61a8df5d3fae1a8ed2d480e419c8`. Product sources adopt frozen fixes head `bda497218b23a6c46d6e9a4ad8903bb76a3b82f0`. [Generated version PR5](https://github.com/ShravanSunder/sunstack-tools/pull/5) merged at `9490b7a42998f3f14146d27ad92bda19a03abdd3` and records the approved initial 1.0.0 separately from library source changes. Initial bootstrap used its exact passing CI artifact, source `fb7e2c42ba3dd3988cf655928c267dfd70983571`, rather than a locally rebuilt artifact.

Source change → patch/minor Changeset → read-only PR checks → merge → automatic version/changelog PR → current candidate checks/review-state verification → version merge → serialized protected main deployment → public body/hash/header verification. No npm publishing or Git tags. Credentials stay in the main-only `demistify-cdn` environment. Account and token metadata are not public handoff inputs.

## Proof

- Integrated and candidate CI: lint/format/strict types, full library/lab build, 51 Node tests across 12 files and 12 Chromium scenarios across 3 files passed. [Candidate run](https://github.com/ShravanSunder/sunstack-tools/actions/runs/38940088991).
- Domain-response regression failed before correction and seven cases passed afterward. Ownership requires exact unique hostname/Worker; missing/reassigned/ambiguous/explicit staging bindings fail. The provider's optional/deprecated environment field may be absent.
- Initial CLI upload exited 0 after verifying a new Worker and unbound hostname. Actual HTTPS verification passed all 28 public files (2,626,292 staged bytes): exact bytes, MIME, CORS, cache policy and missing-path 404/no-store. The live authenticated domain resolver accepted the bound hostname.
- Live protocol checks returned conditional GET 304/empty body/CORS, HEAD 200/empty body, OPTIONS 204 and POST 405.
- [Protected main CI run](https://github.com/ShravanSunder/sunstack-tools/actions/runs/38941869166) passed checks, version-bot no-pending-change handling and the real deployment. It authenticated through the main-only environment, resolved actual domain ownership, retained the existing v1 assets, uploaded and verified all 28 public files again. The deployed bytes retain the initial candidate manifest because the version is unchanged and the rebuild matches exactly.
- Fresh cross-origin browser interaction/screenshot proof is unverified: this session’s in-app browser is unavailable, and the other fork retains the desktop browser for active owner design work. Built-runtime Chromium regressions and actual HTTPS/CORS proof are established; they are separate claims.

## Agent use now

Read the matching public guide, then write one HTML file with the two shared asset tags. Use stable island IDs, useful headings and the guide's navigation/page slots. Check authored HTML with the matching source CLI from a dependency-installed checkout: `node packages/demistify/bin/demistify.ts check page.html`. Browser rendering/content and interactions need their own observation; a checker pass alone does not prove them. Use ordinary static HTTP for referenced Markdown/JSON, `embed` for local Markdown snapshots, and the standalone bundle for fully offline runtime/assets. Keep all companion chunks/fonts/worker files together when bundling locally.

## Open design and limits

The separate fixes/design fork now defines a presentation as a named/described ordered path over existing islands or inner anchors, including one-stop paths. This release still has the earlier content-owning wrapper. The redesign is not claimed implemented. Do not teach that wrapper as the newly agreed model.

Bugbot reviewed the deployment and version PRs; it declined the accumulated product PR because of its size. The named suites prove their exercised scenarios, not a full visual/accessibility/mobile audit, generic model reliability, field performance or complete browser/WebView/file-protocol coverage. Earlier experiment statistics remain historical named runs, not fresh benchmarks. The existing Cloudflare zone returns error 1010 for Python’s default urllib user agent; Node fetch and a browser-style request return 200. No zone security settings were changed. Use the supplied Node tools for release verification.

## Next-agent blurb

Use Demistify's public experimental v1 runtime/theme and read its matching `/v1/authoring/SKILL.md`. Work from `ShravanSunder/sunstack-tools` main in an isolated worktree. Keep library rendering separate from authored content, declare patch/minor Changesets, run the scoped quality/build/Node/Chromium gates, and let the version PR deploy through the protected main workflow. Do not edit existing experimental pins or copy credentials into source. For presentation-path work, first read the other fork's current agreed design; the shipped wrapper is historical API.
