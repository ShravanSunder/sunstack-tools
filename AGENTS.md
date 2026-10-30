# Sunstack tools

Demistify is an experimental library in `packages/demistify`; its demonstration host and comparative presentation lab live in `apps/demistify-poc`. Keep content and authoring examples in the app, and reusable rendering, styles and interactions in the library. HTML owns layout. Do not require an application framework or backend for a visual document.

Keep the five experiments independently inspectable and their benchmark content comparable. Count authored usage separately from library implementation, fonts, engines, packaging and generated output. Do not infer a winner from source size alone. D2 belongs to other experiments, not the Demistify island runtime.

Read `docs/wip/demystify/2026-10-14-demistify-handoff.md` before iterating. Run commands from the monorepo root and target packages with pnpm filters. Use strict TypeScript and the scoped OXC checks. These are experiments: use the minimum meaningful checks and direct browser proof, and do not grow a pedantic test suite. Preserve existing check sources and distinguish historical tests from current proof.

Use an isolated worktree for changes. Preserve unrelated design documents. No deployment, release, framework expansion or public uploads beyond the explicitly authorized source push are implied.

## Rust

CI runs the workspace gates on Linux and macOS with cargo-nextest 0.9.150 or
newer; older versions have a pipe-inheritance LEAK race. Keep Rust compilation
serial while independent formatting and TypeScript checks run as background
steps, and join them before saving caches. Only main runs save Rust caches;
PRs and release tags restore them. `release-cache.yml` warms each release target's
dist profile on dependency/toolchain/config changes on main or a manual main
dispatch, skipping compilation on an exact cache hit. Source-only pushes do not
start that workflow. Its toolchain, SDK environment and cache key match the
`dist-workspace.toml` build-setup hook. Keep the warmer matrix and path filters
aligned with `dist plan` and the cache inputs when changing targets or runners.

Use the pinned toolchain and root Cargo workspace gates. On Apple Silicon,
Cargo and nextest use `scripts/cargo-debug-signing-runner.sh` to give debug
executables stable Developer ID identities for LuLu: `sunstack` has its
own identity; test and example executables share one. Signing is best effort;
when the identity is unavailable, the runner warns and still executes.

## Responsive UI verification belongs to component development

When changing reusable rendering, layout or controls, verify the affected UI at desktop and 390 CSS px, in the themes the change affects. Inspect screenshots and exercise the changed interaction; fix clipping, overflow and controls covering content before handing it back. Reuse unchanged coverage instead of repeating a full matrix. A narrow desktop viewport is not physical iPhone/Safari proof; name that distinction when relevant.

Document-authoring skills own content, markup, bindings and links. Keep phone/theme regression checks here, rather than requiring every author to retest shared components. Instruction-only changes need focused text/packaging checks, not another browser run.
