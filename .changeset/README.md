# Demistify release declarations

Run `pnpm changeset` for a change to the public library or its authoring guide. Choose patch for compatible fixes, minor for compatible additions, and major for removed/renamed APIs or changed meanings. Reviewers judge behavioral compatibility; CI cannot infer it from a diff. Test-only and release-tooling-only edits need no library release.

The version PR updates package version and changelog. Once a supported major is selected, 1.2.3 updates `/v1/`; 2.0.0 creates `/v2/` and keeps `/v1/`. There are no public minor/patch history URLs. This workflow PR preserves the current package version (0.1.0); selecting the first supported release belongs to a separate release PR. No npm publication or Git tags are part of the workflow.
