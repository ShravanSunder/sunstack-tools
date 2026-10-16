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
- [ ] Push separate PR; verify CI/comments/mergeability.
- [ ] Configure CI environment and select/bootstrap the first supported release separately.

Unchecked rows remain unverified; source, hosted CI, deployed delivery and installed skill availability are separate claims.

## Current evidence and remaining setup

[Release instructions](../../../packages/demistify/release/README.md). [Local protocol receipt](evidence/2026-10-16/cloudflare-release-local.json). The actual local Assets binding served 24 files for a v1 fixture and 47 for retained v1 + v2, with exact bytes/MIME/CORS/cache checks. Conditional GET returned 304 with empty body/CORS, HEAD 200/empty body, OPTIONS 204, POST 405. The fixture version labels do not change the source package or claim a published release.

Real Changesets fixture: a declared compatible feature passed parser/status; versioning consumed it and produced 0.2.0; typed version-only validation passed while pending-status exited 1. CI preserves pending-status for non-version PRs and validates the consumed version-only phase separately. This is a diagnosed false failure, not a disabled feature declaration check.

GitHub readback: Actions enabled; default workflow permission read; bot PR creation disabled; demistify-cdn environment not found. These are outstanding operator setup, not a claim that CI authentication works. No repository setting or credential has been changed by this branch. First supported version selection remains separate.

Final local Node suite: 37 tests in 9 files, exit 0. Quality gates and actionlint pass. The actual Changesets parser rejects a body-text fake declaration (exit 1); valid feature and consumed version-only flows pass. No browser tests or remote Cloudflare resources were touched by this fork.

History origin is derived from the protected Cloudflare account and the Worker configuration using Wrangler’s documented Workers-subdomain endpoint. It is not a separately maintained repository variable. Authentication of that read remains unverified until the protected environment is configured; no token has been read or sent by local proof.
