# Experimental v1 integration

The owner approved initial version 1.0.0 and public assets at `https://demistify.getagentstudio.dev/v1/`. Breaking changes remain permitted within experimental v1 until the owner explicitly declares strict major compatibility. The deployment workflow preserves historical major folders once additional majors are approved; it exposes no minor/patch history URLs.

This product integration combines frozen `fix/demistify-mermaid-scroll` head `bda497218b23a6c46d6e9a4ad8903bb76a3b82f0` with the separate Cloudflare workflow. The package version remains 0.1.0 here; the real major Changeset requests the approved initial 1.0.0 in a separate version-only PR. Public source and its consumed version update stay separate.

The built plain-HTML browser regressions now run in CI against pinned Chromium, using the existing Vitest Browser Mode harness. Root scripts and the lockfile retain both browser-testing and release dependencies. No desktop browser or the other agent's worktree is used.

## Presentation design boundary

The owner now defines a presentation as a named/described ordered path over existing islands or inner anchors, including a one-stop path. The frozen implementation still has the earlier content-owning wrapper. The fixes/design fork owns that redesign; this initial experimental build does not claim its new model is implemented. Existing guide text describes the frozen API only.

## Launch sequence

Merge the workflow and this product integration after checks and current review-state gates. Let Changesets open its version-only PR. Build and verify that reviewed 1.0.0 candidate, verify the new hostname has no conflicting DNS or Worker binding, and explicitly bootstrap the first release. Merge the version PR and verify the real protected main deployment, including public byte/hash/header checks. Bootstrap is never a fallback for missing history. Cloudflare credentials and account metadata remain private.

## Proof still required

Current-source quality, full library/lab build, Node and browser regression suites; hosted CI on the integrated source; generated version PR and its checks; actual DNS/TLS and protected GitHub deployment. Earlier source experiments and named browser runs remain historical evidence, not fresh acceptance of this integrated build.
