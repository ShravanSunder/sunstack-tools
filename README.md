# Sunstack tools

Demistify is a visual-document experiment using native Web Components. Agents write HTML composition and Markdown/Mermaid content; the reusable library owns rendering, design, icons and interaction.

- `packages/demistify`: drop-in browser runtime, shared theme, optional content packaging.
- `apps/demistify-poc`: five independent presentation experiments, comparison report and composition/layout showcases.
- `docs/wip/demystify`: detailed continuation handoff and source styling study.

From the repository root, with Node 26 and pnpm 12:

```sh
pnpm install
pnpm check
pnpm build
pnpm --filter @sunstack/demistify-poc preview --port 4174
```

Open the local preview to compare all five independent pages. A library consumer needs only the built `runtime.js`, `theme.css` and their shared companion assets; the demo host is optional. See [the library reference](packages/demistify/README.md) and [the iteration handoff](docs/wip/demystify/2026-10-14-demistify-handoff.md).

The pre-existing rule-engine design documents remain under `docs/specs/` and are unrelated to this experiment.
