# Initiative migration (temporary)

One-time tooling to convert a `project.ts` tree to `initiative.ts` and re-cut
its deliverables into projects. It is not part of the published CLI or the
guide. Delete this folder, and its line in
`apps/demistify-poc/vitest.config.ts`, once the trees that need it are
converted.

## Run it

From a sunstack-tools checkout, dry run first:

```sh
node packages/demistify/scripts/initiative-migration/run.ts <initiative-dir> --plan <cut.json> --dry-run
node packages/demistify/scripts/initiative-migration/run.ts <initiative-dir> --plan <cut.json>
```

```text
Re-cut an initiative: move whole deliverables, and the records they scope, between
project folders, as the plan says. Ids change only when they collide in the target
project; every reference, Map and Walkthrough target, page binding and #ID link is
rewritten. A record that a staying deliverable also scopes stays and is named
qualified. A passage moves with its record only when its heading section holds
nothing else; every other passage is listed under "For you to place".

--dry-run   print every change and collision; write nothing
--plan      JSON: projects [{key, folder, number, title, achieves?, acceptance?, lead?}],
            moves [{from: "mac-side/DEL-07", to: "harness-contract"}],
            records [{record: "mac-side/R14", to: "<project>"}] (overrides),
            add [{project, title, dependsOn?, why?}], withdraw ["<project>/DEL-nn"],
            dependsOnChanges [{deliverable, from, to, why?}], remove ["<project>"],
            open [...] (printed, never applied)

The real run checks a staged copy first and writes nothing when that check fails.
```

## What it does, in detail

Move whole deliverables between projects with one command. Run the dry run first: it writes nothing and prints every change, collision and passage it cannot place.

```sh
demistify initiative migrate docs/initiatives/2026-11-router --plan cut.json --site-root docs --dry-run
demistify initiative migrate docs/initiatives/2026-11-router --plan cut.json --site-root docs
```

The plan is JSON. Addresses name the tree as it is before the run.

```json
{
	"projects": [
		{
			"key": "harness-contract",
			"folder": "2026-11-harness-contract",
			"number": 1,
			"title": "One contract for every harness",
			"achieves": "…",
			"acceptance": "…",
			"lead": "mac-lead"
		}
	],
	"moves": [{ "from": "mac-side/DEL-07", "to": "harness-contract" }],
	"records": [{ "record": "mac-side/R14", "to": "harness-contract" }],
	"add": [
		{
			"project": "harness-contract",
			"title": "Claude through its SDK",
			"dependsOn": [],
			"why": "…"
		}
	],
	"withdraw": ["mac-side/DEL-17"],
	"dependsOnChanges": [{ "deliverable": "mac-side/DEL-19", "from": [], "to": ["mac-side/DEL-02"] }],
	"remove": [],
	"open": []
}
```

- `projects` lists every project the initiative keeps, with its number. A project not yet on disk is created in `folder`, with the typed files, `deliverables/` and an `index.html` that declares the record sections; `title` and `lead` apply only to these. A project the plan omits stays registered and is printed; list it in `remove` to drop it from `initiative.ts` once nothing is left in it.
- A move takes the deliverable module, its delivery entry and acceptance, and every record it scopes: rules, needs, realizations, checks and the decisions and assumptions they rest on. A record that a deliverable staying behind also scopes stays, and the moved deliverable names it qualified. `records` overrides where one record goes.
- Ids change only when they collide in the target project, and never reuse an id the target already had. Every reference, `dependsOn`, Map and Walkthrough target, page binding, status and `#ID` link is rewritten to the new address.
- `add` creates a `later` deliverable with the next free id; the run prints its `dependsOn` and `why` for you to apply when you scope it. `withdraw` removes the deliverable and every `dependsOn` naming it.
- A passage moves with its record only when its heading section, or its section item, holds nothing else. Everything else stays and is listed under "For you to place". Prose is never rewritten: the run lists where a renamed item's old id still appears in text.
- The real run checks a staged copy first and writes nothing when that check fails. Afterwards, run `design types` for each new project and `initiative check --write`.

A tree still in the `project.ts` layout is converted in the same run: `initiative.ts` takes the plan's numbers (a project the plan omits gets the highest one), `project-decisions.ts` and `project-assumptions.ts` become `initiative-*`, `project/` references become `initiative/`, and `ex-project-overview` becomes `ex-initiative-overview`.
