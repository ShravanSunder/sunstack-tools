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
- [ ] Supply an approved scoped CI credential, enable bot PR permission, and select/bootstrap the first supported release separately.

Unchecked rows remain unverified; source, hosted CI, deployed delivery and installed skill availability are separate claims.

## Current evidence and remaining setup

[Release instructions](../../../packages/demistify/release/README.md). [Local protocol receipt](evidence/2026-10-16/cloudflare-release-local.json). The actual local Assets binding served 24 files for a v1 fixture and 47 for retained v1 + v2, with exact bytes/MIME/CORS/cache checks. Conditional GET returned 304 with empty body/CORS, HEAD 200/empty body, OPTIONS 204, POST 405. The fixture version labels do not change the source package or claim a published release.

Real Changesets fixture: a declared compatible feature passed parser/status; versioning consumed it and produced 0.2.0; typed version-only validation passed while pending-status exited 1. CI preserves pending-status for non-version PRs and validates the consumed version-only phase separately. This is a diagnosed false failure, not a disabled feature declaration check.

GitHub readback after setup: Actions enabled; default workflow permission read; bot PR permission disabled. The new `demistify-cdn` environment exists, with exactly one allowed deployment ref: branch `main`. Repository secrets are empty; no environment credentials have been installed. No default token permission or Cloudflare resource has changed. First supported version selection remains separate.

Final local Node suite: 37 tests in 9 files, exit 0. Quality gates and actionlint pass. The actual Changesets parser rejects a body-text fake declaration (exit 1); valid feature and consumed version-only flows pass. No browser tests were run and no Cloudflare resources were changed by this fork.

History origin is derived from the protected Cloudflare account and the Worker configuration using Wrangler’s documented Workers-subdomain endpoint. It is not a separately maintained repository variable. Authentication of that CI read remains unverified until an approved credential is installed. The release protocol fixtures used no authentication. A later local `wrangler whoami --json` succeeded, and the read-only configured-Worker lookup returned provider Worker-not-found code 10007. Identity, account and credential values were suppressed; no Cloudflare resource was created or deployed.

## Source review and integration checkpoint

[PR #2](https://github.com/ShravanSunder/sunstack-tools/pull/2) is the separate deployment workflow on `feat/demistify-cloudflare-release-workflow`. Its code head `21f696124bbef044531fb173e46f675b0231bb41` passed hosted checks in 51 seconds and Cursor Bugbot in 11m49s with no findings. Version-PR and deploy jobs correctly skipped on the pull-request/non-main event. This handoff update changes documentation only; use the live PR checks for the current head. The PR remains draft and has not been merged.

The fixes workstream is separately pushed on `fix/demistify-mermaid-scroll`, latest handback `bda497218b23a6c46d6e9a4ad8903bb76a3b82f0`. Its new components, authoring guide and browser tests are not included in this workflow PR. Integrate and verify that product branch before choosing the first supported release; build runtime, theme, diagram worker, chunks and guide together. Never place a newer guide under the old experimental asset pin. Reported browser proof belongs to that workstream, not this deployment branch.

The remaining credential-selection question is pending with the owner. Existing local CLI login does not establish a durable GitHub CI credential. Do not copy local OAuth credentials or an unidentified broad API credential into CI. The workflow can be reviewed independently; authenticated hosted deployment and release selection remain open.
