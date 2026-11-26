# Author and deliver a Demistify design

Use this when a design needs stable links from an owner decision or assumption to the rule, implementation, and verification that depend on it. Write the explanation and diagrams directly in HTML. Use the typed files for the IDs, links, decisions, assumptions, and delivery progress.

## Start with one matching binding

Use Node 26 or later. From the Demistify candidate checkout, build and install the local package in a scratch directory:

```sh
pnpm --filter @sunstack/demistify build --npm
tool_home="$(mktemp -d)"
set_home="$(mktemp -d)"
package_version="$(node --print "require('./packages/demistify/package.json').version")"
npm install --prefix "$tool_home" "$PWD/packages/demistify/npm-dist/sunstack-demistify-$package_version.tgz"
demistify="$tool_home/node_modules/.bin/demistify"
"$demistify" design init "$set_home"
cp -R "$tool_home/node_modules/@sunstack/demistify/dist" "$set_home/demistify"
```

For a hosted release, read its authoring guide, confirm it documents typed delivery, and run its exact hashed `pnpm dlx` command with the same `design` subcommands. The runtime, theme and CLI must come from the same binding. [Hosted authoring guide](https://demistify.getagentstudio.dev/v1/authoring/SKILL.md)

In the new set, author `index.html` yourself. Start with the binding's HTML shell, point the stylesheet and module at `./demistify/theme.css` and `./demistify/runtime.js`, and use `<ex-nav for="content" src="./navigation.generated.json"></ex-nav>`. Give the owning HTML passages real IDs. Add separate pages when they answer distinct reader questions. Link those pages and passages from `design.ts`. [HTML shell](../../../packages/demistify/skill/SKILL.md)

When you select a newer binding, run its `design types <set>` for each registered milestone to refresh the generated schema and SDK. This also applies to a standalone typed set. Update any older shared navigation binding to `navigation.generated.json` when it is the file your selected checker emits.

## Write each fact in its owning file

| Who           | What to write                                                                                                                                                                                                            |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Lead          | The design records in `design.ts`, their owning HTML passages, and the deliverables and prerequisites in `design-delivery.ts`. Assign each pair the module path, work log, implementation tasks and verification checks. |
| Implementer   | Completed tasks, candidate build and evidence in the assigned `deliverables/<name>.ts`. Log material observations, issues and recommendations using Show Me Your Work. Take open choices to the Lead.                    |
| Verifier      | Observed outcomes and their evidence in the same deliverable module, keyed by the assigned check IDs. Pin the actual build and scope revision. Log what the check showed and any blocker through Show Me Your Work.      |
| Lead or Owner | A dated decision. The Lead records its own call or the Owner's answer with the original source. A deliverable acceptance in `design.ts` is the separate act of accepting an exact build, scope revision and evidence.    |

Write decisions in `design-decisions.ts` with stable `D` IDs and assumptions in `design-assumptions.ts` with stable `A` IDs. Point rules, realizations and checks to decisions with `governed-by` and to assumptions with `relies-on`. Give the reader the same anchors in the HTML. Keep record files as literal data. Use the guide’s supported delivery and identity helpers where they apply. Give each planned check at least three `given` and `expect` cases. The typed-design reference has the actual field names and a worked deliverable module. [Typed-design reference](../../../packages/demistify/skill/references/typed-design.md)

The Lead alone decides whether a discovery breaks the shared mental model. Treat a one-way door as a break, including a commitment that would cost substantial time to undo. The Lead decides what to bring to the Owner. Reversible work may continue under a recorded open assumption with the Lead's direction. The affected deliverable waits for evidence before acceptance.

## Check and preview

```sh
"$demistify" design check "$set_home"
```

For a set that links to sibling material outside its folder, pass the containing `--site-root <path>`. Read the status and reason for each deliverable. An unanswered governing owner question or refuted assumption blocks the affected delivery. Confirm an open assumption with its evidence before recording acceptance against the current scope.

Edit TypeScript or HTML, then run the check again. The CLI writes `design.generated.json`, `navigation.generated.json`, and `deliverables.generated.json`. Keep them together. Serve the set over HTTP, including the generated files and matching assets. Use Pages, Map and Walkthrough to take readers to authored passages. Put `<ex-deliverables src="./deliverables.generated.json" label="Delivery progress"></ex-deliverables>` inside the document when the reader needs work state, blockers and evidence links.

The checker validates references, scope, and declared authority. Open the original decision and proof to establish who decided and what was observed.

## Connect a project

Use `docs/projects/yyyy-mm-name/project.ts` above your registered sets in `milestones/yyyy-mm-name/`. Include `project-decisions.ts` or `project-assumptions.ts` when the project has shared records. Keep owner and agent identities in `agents-identity.ts` and refer to agents by alias in the delivery plan.

Give each included set a stable name in `project.ts`. Set the milestone Lead and optional per-deliverable builder using aliases from `agents-identity.ts`. The builder implements the work. The milestone Lead records acceptance. Put a dependency such as `sessions/DEL-01` in the waiting design's delivery plan.

The following examples use a binding whose guide documents read-only project checks and explicit refresh. Run them from the repository root. Here, `docs` is the intended site root, containing both `docs/projects` and `docs/specs`.

```sh
"$demistify" project check docs/projects/2026-11-router --site-root docs
"$demistify" project check docs/projects/2026-11-router --site-root docs --write
```

The first command checks the project and reports its open owner decisions and deliverable state. The second also refreshes the generated JSON. Use the project check's scope revision when recording evidence. Keep existing reviewed passages at their current URLs. Declare their pages in the milestone and point records at exact HTML IDs. A change to a scoped passage invalidates evidence that covers the old revision. Use a relative link inside the declared site root.

On a hand-authored project page, use the binding's project overview when the reader needs open owner questions, delivery state, and waits across milestones. The overview reads `project.generated.json`. A milestone delivery panel reads `deliverables.generated.json`. Keep the generated files with the pages served over HTTP. [Project authoring example](../../../packages/demistify/skill/references/typed-design.md)
