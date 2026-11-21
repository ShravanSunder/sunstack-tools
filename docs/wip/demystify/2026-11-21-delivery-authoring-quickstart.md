# Author and deliver a Demistify design

Use this when a design needs stable links from an owner decision or assumption to the rule, implementation, and verification that depend on it. Write the explanation and diagrams directly in HTML. Use the typed files for the IDs, links, decisions, assumptions, and delivery progress.

## Start with one matching binding

Use Node 26 or later. From the Demistify candidate checkout, build and install the local package in a scratch directory:

```sh
pnpm --filter @sunstack/demistify build --npm
tool_home="$(mktemp -d)"
set_home="$(mktemp -d)"
npm install --prefix "$tool_home" "$PWD/packages/demistify/npm-dist/sunstack-demistify-1.25.3.tgz"
demistify="$tool_home/node_modules/.bin/demistify"
"$demistify" design init "$set_home"
cp -R "$tool_home/node_modules/@sunstack/demistify/dist" "$set_home/demistify"
```

The archive filename above names the local candidate. On another checkout, use the filename printed by the build. For a hosted release, read its authoring guide, confirm it documents typed delivery, and run its exact hashed `pnpm dlx` command with the same `design` subcommands. The runtime, theme and CLI must come from the same binding. [Hosted authoring guide](https://demistify.getagentstudio.dev/v1/authoring/SKILL.md)

In the new set, author `index.html` yourself. Start with the binding's HTML shell, point the stylesheet and module at `./demistify/theme.css` and `./demistify/runtime.js`, and use `<ex-nav for="content" src="./navigation.generated.json"></ex-nav>`. Give the owning HTML passages real IDs. Add separate pages when they answer distinct reader questions. Link those pages and passages from `design.ts`. [HTML shell](../../../packages/demistify/skill/SKILL.md)

For an existing typed set, run `design types <set>` to refresh editor support. Update any older shared navigation binding to `navigation.generated.json` when it is the file your selected checker emits.

## Write each fact in its owning file

| Who           | What to write                                                                                                                                                                                                         |
| ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Lead          | The design records in `design.ts`, their owning HTML passages, and the deliverables and prerequisites in `delivery.ts`. Assign each pair the module path, work log, implementation tasks and verification checks.     |
| Implementer   | Completed tasks, candidate build and evidence in the assigned `deliverables/<name>.ts`. Log material observations, issues and recommendations using Show Me Your Work. Take open choices to the Lead.                 |
| Verifier      | Observed outcomes and their evidence in the same deliverable module, keyed by the assigned check IDs. Pin the actual build and scope revision. Log what the check showed and any blocker through Show Me Your Work.   |
| Lead or Owner | A dated decision. The Lead records its own call or the Owner's answer with the original source. A deliverable acceptance in `design.ts` is the separate act of accepting an exact build, scope revision and evidence. |

Use a stable `D` ID for a decision and an `A` ID for an assumption. Point rules, realizations and checks to decisions with `governed-by` and to assumptions with `relies-on`. Give the reader the same anchors in the HTML. The typed-design reference has the actual field names and a worked deliverable module. [Typed-design reference](../../../packages/demistify/skill/references/typed-design.md)

The Lead alone decides whether a discovery breaks the shared mental model. Treat a one-way door as a break, including a commitment that would cost substantial time to undo. The Lead decides what to bring to the Owner. Reversible work may continue under a recorded open assumption with the Lead's direction. The affected deliverable waits for evidence before acceptance.

## Check and preview

```sh
"$demistify" design check "$set_home"
```

For a set that links to sibling material outside its folder, pass the containing `--site-root <path>`. Read the status and reason for each deliverable. An unanswered governing owner question or refuted assumption blocks the affected delivery. Confirm an open assumption with its evidence before recording acceptance against the current scope.

Edit TypeScript or HTML, then run the check again. The CLI writes `design.generated.json`, `navigation.generated.json`, and `deliverables.generated.json`. Keep them together. Serve the set over HTTP, including the generated files and matching assets. Use Pages, Map and Walkthrough to take readers to authored passages. Put `<ex-deliverables src="./deliverables.generated.json" label="Delivery progress"></ex-deliverables>` inside the document when the reader needs work state, blockers and evidence links.

The checker validates references, scope, and declared authority. Open the original decision and proof to establish who decided and what was observed.
