# Demistify release declarations

Run `pnpm changeset` for a change to the public library or its authoring guide. The owner selected `1.0.0` as the initial release. Until the owner declares the vocabulary stable, `/v1` is experimental and may have breaking changes: use patch/minor updates within v1. The release-intent checker rejects a planned major above 1. Test-only and release-tooling-only edits need no library release.

The version PR updates package version and changelog separately from public source changes. Releases such as 1.2.3 update `/v1/`. The staging implementation can retain `/v1/` when a future `/v2/` is approved, but strict major compatibility is not yet enabled. There are no public minor/patch history URLs. This workflow PR preserves the current package version (0.1.0); the approved initial 1.0.0 will follow product integration. No npm publication or Git tags are part of the workflow.
