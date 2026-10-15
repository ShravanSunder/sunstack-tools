# Presentation experiments

A local comparison lab comparing five ways to present the same complete engineering explanation. Each prototype has a separate page, inspectable usage input, real rendered output, measured source tokens, separate delivery sizes, and findings. No winner is selected.

The common input is the owner-supplied planning DAG snapshot captured on 2026-10-14: three views, 25 nodes, 24 relationships, and jobs/boundaries. The source file in its other checkout has since changed; the captured copy and provenance remain in `references/`. The Claude Layer Tree CSS is the styling reference for HTML Islands.

## Open and run

Use Node 26 or newer and pnpm 12.11.2.

```sh
pnpm install --frozen-lockfile
pnpm dev
```

The home page links to all five independent pages and the comparison report.

| Prototype           | Input                                                          | Rendering path                                                                           |
| ------------------- | -------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| Visual Plan         | `experiments/visual-plan/planning.mdx`                         | Unmodified vplan 0.24.0, React and beautiful-mermaid                                     |
| Quick JSON          | `experiments/quick-json/planning.json`                         | Pinned, unmodified visual-explainer Quick renderer                                       |
| Markdown directives | `experiments/markdown-directives/planning.md`                  | Local remark resolver and packaged D2 SVGs                                               |
| Semantic graph diff | `experiments/graph-diff/planning.md` + `planning-changes.json` | Complete authored explanation plus stable-identity changes against the provided base     |
| HTML Islands        | `experiments/html-islands/planning.html`                       | Native Markdown/Mermaid islands in HTML layouts, generic inspection and connector arrows |

`public/experiments/report.html` separates complete authoring usage from delivery costs. All five preserve the three views, 25 nodes, 24 relationships, jobs, statuses, boundaries, source references, and current/change/proposed story. HTML Islands' extra layer/timeline/file-tree examples are in `experiments/html-islands/capabilities.html`, outside the comparison.

Usage tokens count every per-example authored file with local `js-tiktoken@1.0.21`, encoding `o200k_base`. The report lists each counted file, its bytes/tokens, and SHA-256. Library implementation, CSS, fonts, engines, packaging, generated artifacts and existing source inputs are excluded. Referencing existing diagrams counts only the reference; redeclaring them counts the declaration. All formats receive the same existing inputs. These are translated-fixture source tokens, not billed model tokens or measured generation/repair/comprehension cost.

## Build and check

```sh
pnpm check
pnpm build
pnpm --filter @sunstack/demistify-poc preview --port 4174
```

Existing test sources cover graph transformations and refusals, Markdown references, source-snapshot preservation, island whitespace, and HTTP serving of all five inspectors and outputs. This experiment iteration uses scoped quality/build checks and browser smoke, without running the full suite. Browser verification drives real pages and interactions. Output is copied to `dist/experiments/`.

For a content-only HTML Islands revision after the initial build:

```sh
pnpm --filter @sunstack/demistify-poc islands:package
```

That packaging command reuses the existing component library and does not invoke Vite or compile MDX. Run the full comparison build when its source-size report also needs refreshing. Vite is the lab/library build tool; an explainer is an HTML document, not a new Vite application.

## HTML Islands

See [the authoring reference](experiments/html-islands/README.md). The live document uses local runtime assets and lazy diagram/Markdown chunks. The standalone package inlines styles, fonts and the tab/zoom runtime, and pre-renders content and diagrams. Direct browser navigation to `file://` is blocked by this host's browser policy; these artifacts are verified over local HTTP and their inline packaging is inspected.

Verified limits: Quick JSON has textual flow edges; Markdown recognizes the named corpus; graph diff handles flat graph identities; the native Mermaid adapter supports beautiful-mermaid's subset; D2 is packaged rather than executed in the browser. No paid API, backend or deployment is required. Source is published in the owner-authorized Sunstack branch; the captured corpus is a historical input. The original research report remains an input, not a verified recommendation.
