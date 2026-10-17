# Sunstack tools

Demistify is a visual-document experiment using native Web Components. Agents write HTML composition and Markdown/Mermaid content; the reusable library owns rendering, design, icons and interaction.

- `packages/demistify`: drop-in browser runtime, shared theme, optional content packaging.
- `apps/demistify-poc`: five independent presentation experiments, comparison report and composition/layout showcases.
- `docs/wip/demystify`: detailed continuation handoff and source styling study.

For a plain HTML explainer, use the public experimental v1 assets:

```html
<link rel="stylesheet" href="https://demistify.getagentstudio.dev/v1/theme.css" />
<script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script>
<ex-document>
	<ex-markdown
		><script type="text/markdown">
			# Session architecture

			Sessions survive transport reconnections.
		</script></ex-markdown
	>
</ex-document>
```

Give the page ordinary HTML document metadata and a title. Read the [matching authoring guide](https://demistify.getagentstudio.dev/v1/authoring/SKILL.md) for components and relative references. Inline or embedded content needs no server; referenced Markdown/JSON files need HTTP serving or the supplied embedding/standalone tools. CDN links need network access. Experimental v1 may have breaking changes until the owner declares stable major compatibility.

Library changes use a Changeset, then a separate version PR and protected main deployment. See the [release instructions](packages/demistify/release/README.md) and [v1 deployment handoff](docs/wip/demystify/2026-10-16-v1-deployment-handoff.md).

From the repository root, with Node 26 and pnpm 12:

```sh
pnpm install
pnpm check
pnpm build
pnpm --filter @sunstack/demistify-poc preview --port 4174
```

Open the local preview to compare all five independent pages. A library consumer needs only the built `runtime.js`, `theme.css` and their shared companion assets; the demo host is optional. See [the library reference](packages/demistify/README.md) and [the iteration handoff](docs/wip/demystify/2026-10-14-demistify-handoff.md).

The pre-existing rule-engine design documents remain under `docs/specs/` and are unrelated to this experiment.

## Authoring measurements

Six exploratory Sol/Luna runs compared Demistify with ordinary HTML/CSS artifacts. Demistify produced smaller reader source in each matched pair; author-session usage and first-handback quality varied. See the [measured counts, browser findings and limitations](docs/benchmarks/demistify-authoring-2026-10-17.md). Source size alone does not establish billed savings or reliability.
