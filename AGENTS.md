# Sunstack tools

Demistify is an experimental library in `packages/demistify`; its demonstration host and comparative presentation lab live in `apps/demistify-poc`. Keep content and authoring examples in the app, and reusable rendering, styles and interactions in the library. HTML owns layout. Do not require an application framework or backend for a visual document.

Keep the five experiments independently inspectable and their benchmark content comparable. Count authored usage separately from library implementation, fonts, engines, packaging and generated output. Do not infer a winner from source size alone. D2 belongs to other experiments, not the Demistify island runtime.

Read `docs/wip/demystify/2026-10-14-demistify-handoff.md` before iterating. Run commands from the monorepo root and target packages with pnpm filters. Use strict TypeScript and the scoped OXC checks. These are experiments: use the minimum meaningful checks and direct browser proof, and do not grow a pedantic test suite. Preserve existing check sources and distinguish historical tests from current proof.

Use an isolated worktree for changes. Preserve unrelated design documents. No deployment, release, framework expansion or public uploads beyond the explicitly authorized source push are implied.

## Rust

Use the pinned toolchain and root Cargo workspace gates. On Apple Silicon,
Cargo and nextest use `scripts/cargo-debug-signing-runner.sh` to give debug
executables stable Developer ID identities for LuLu: `sunstack` has its
own identity; test and example executables share one. Signing is best effort;
when the identity is unavailable, the runner warns and still executes.
