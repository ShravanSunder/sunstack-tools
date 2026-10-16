# Cloudflare deployment workflow — scope and todo

16 October 2026. Owner-selected behavior: public history by major only (`/v1`, `/v2`, `/v3`), compatible patch/minor releases update the current major, breaking changes get a new major, and Workers Static Assets is the easier selected backend. This work is a separate workflow PR; the existing package version 0.1.0 remains unchanged. No first supported release is asserted or authorized by this document.

## Ownership

This fork owns deployment on feat/demistify-cloudflare-release-workflow, based on main 49686d9. The fixes fork owns diagram/layout bugs, cross-page/presentation navigation, skill guidance and the later Vitest browser-test PR. Do not edit that worktree or the skill integration branch here.

## Intended workflow for review

Changeset declaration → read-only PR CI → automated version/changelog PR → merge → serialized Cloudflare deployment → body/hash/header verification. No npm publishing, automatic Git tags, R2, application backend or explainer-content publication. A dedicated sunstack-demistify Worker will serve major paths; the existing experimental Worker and its pinned guides remain untouched. A first public `/v1` requires selecting a 1.x package version in a separate reviewed release PR. Pre-1.0 versions do not deploy to a public major path.

Staging verifies and preserves all prior major directories before deployment. Incomplete or unavailable history must abort instead of producing a partial replacement. Stable entry files revalidate. Content-hashed chunks remain available for already-open pages, without exposing minor-release URLs. Credentials belong only in the protected CI environment; local OAuth credentials are never copied into Git or CI.

## Todo and proof

- [x] Confirm major-only history and Static Assets with owner.
- [x] Isolate deployment work from fixes and skill integration.
- [x] Implement and verify history-preserving staging and cache headers.
- [x] Configure Changesets version PR without npm publishing/tags.
- [x] Add PR CI and gated main deployment with matched guide assets.
- [x] Exercise release/retention/error scenarios and real local workerd.
- [x] Check lint/types/format/build and inspect public diff.
- [x] Push separate PR [#2](https://github.com/ShravanSunder/sunstack-tools/pull/2); source head `21f6961` passed hosted checks and Bugbot, with no comments/threads and conflict-free mergeability.
- [x] Create `demistify-cdn` and verify its custom deployment policy permits only branch `main`. No credentials were copied.
- [x] Create, verify and privately install an approved project CI credential and account secret in the main-only environment.
- [x] Configure the owner-selected custom hostname and verify ownership/refusal cases.
- [ ] Enable bot PR permission and select/bootstrap the first supported release separately.

Unchecked rows remain unverified; source, hosted CI, deployed delivery and installed skill availability are separate claims.

## Current evidence and remaining setup

[Release instructions](../../../packages/demistify/release/README.md). [Local protocol receipt](evidence/2026-10-16/cloudflare-release-local.json). The actual local Assets binding served 24 files for a v1 fixture and 47 for retained v1 + v2, with exact bytes/MIME/CORS/cache checks. Conditional GET returned 304 with empty body/CORS, HEAD 200/empty body, OPTIONS 204, POST 405. The fixture version labels do not change the source package or claim a published release.

Real Changesets fixture: a declared compatible feature passed parser/status; versioning consumed it and produced 0.2.0; typed version-only validation passed while pending-status exited 1. CI preserves pending-status for non-version PRs and validates the consumed version-only phase separately. This is a diagnosed false failure, not a disabled feature declaration check.

GitHub readback after setup: Actions enabled; default workflow permission read; bot PR permission disabled. The new `demistify-cdn` environment exists, with exactly one allowed deployment ref: branch `main`. Repository-level secrets remain empty; the two Cloudflare environment secrets are installed in `demistify-cdn`. A new project account-owned CI token was created; no Worker or DNS resource was changed. The default GitHub token permission is unchanged. First supported version selection remains separate.

Final local Node suite: 37 tests in 9 files, exit 0. Quality gates and actionlint pass. The actual Changesets parser rejects a body-text fake declaration (exit 1); valid feature and consumed version-only flows pass. No browser tests were run and no Worker/DNS resources were changed by this fork.

History origin now uses the owner-selected `demistify.getagentstudio.dev` Custom Domain from the Wrangler configuration. Cloudflare’s authenticated domain lookup must confirm the exact production Worker before any historical download. The real new CI token passed account, selected-zone and domain reads; running the actual resolver against the currently unbound hostname correctly refused release without writing an origin. The release protocol fixtures remain unauthenticated local proof. Existing local Wrangler login also works; the new Worker lookup returned provider Worker-not-found code 10007. No Worker or DNS resource was created or deployed.

## Source review and integration checkpoint

[PR #2](https://github.com/ShravanSunder/sunstack-tools/pull/2) is the separate deployment workflow on `feat/demistify-cloudflare-release-workflow`. Its code head `21f696124bbef044531fb173e46f675b0231bb41` passed hosted checks in 51 seconds and Cursor Bugbot in 11m49s with no findings. Version-PR and deploy jobs correctly skipped on the pull-request/non-main event. This handoff update changes documentation only; use the live PR checks for the current head. The PR remains draft and has not been merged.

The fixes workstream is separately pushed on `fix/demistify-mermaid-scroll`, latest handback `bda497218b23a6c46d6e9a4ad8903bb76a3b82f0`. Its new components, authoring guide and browser tests are not included in this workflow PR. Integrate and verify that product branch before choosing the first supported release; build runtime, theme, diagram worker, chunks and guide together. Never place a newer guide under the old experimental asset pin. Reported browser proof belongs to that workstream, not this deployment branch.

The owner authorized creating a project-specific token through the Cloudflare CLI, then approved provider consent. Installed cf0.11 lacked the new token-creation scope, so a pinned current CLI was used temporarily without replacing the global installation. Cloudflare correctly rejected delegating permissions missing from the initial bootstrap grant; the corrected grant was approved and a single project token was created. Its exact live policy and API access were verified before encrypted GitHub secret installation. No unidentified existing credential or local OAuth token was copied into CI. Hosted CI authentication, first release selection and actual deployment remain open.

## Custom-domain and credential checkpoint

The owner selected `demistify.getagentstudio.dev`. The zone is active in the supplied account, and the exact hostname had no DNS record or Worker binding. The Wrangler route declares it as a Custom Domain; first bootstrap will create its binding/DNS/certificate after a supported release is selected. There is no public `/v1` deployment at this hostname yet.

The CI token has exactly four permissions: Workers Scripts Write and Account Settings Read for the selected account; Workers Routes Write and Zone Read for the selected zone. Actual provider token policy, active status and scoped reads passed. GitHub secret-name readback confirmed both required secrets in the protected environment. Secret values, account/zone/token identifiers, OAuth URLs and credential paths are absent from this repository.

Current local checks: 43 Node tests in 10 files, lint/format/types exit 0, custom-domain Wrangler dry-run exit 0. Six focused target cases cover the valid production binding, missing/reassigned/staging/ambiguous bindings, and invalid route configuration. The initial typed test table was corrected so invalid fixture shapes cannot satisfy the ownership-rejection assertions. [Sanitized control-plane receipt](evidence/2026-10-16/cloudflare-cdn-setup.json). Normal CI correctly refuses an unbound hostname rather than treating it as bootstrap.
