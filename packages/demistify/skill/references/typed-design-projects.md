# Typed design projects

A project root such as `docs/projects/2026-11-router/` sits above its milestones and registers them in `project.ts`, with the decisions and assumptions they share. Each milestone keeps its own files from [typed-design.md](typed-design.md).

```ts
// project.ts
import messages from './milestones/2026-11-messages/design.ts';
import sessions from './milestones/2026-11-sessions/design.ts';
import decisions from './project-decisions.ts';
import assumptions from './project-assumptions.ts';
import identities from './agents-identity.ts';

export default { designs: { messages, sessions }, decisions, assumptions, identities };
```

Every field is optional; `export default {};` is a valid empty project. Keep every TypeScript input inside the project root. The name `project` is reserved for shared records.

The shared files export literal arrays with an explicit `kind`, and use only type-only imports from a milestone's `design-schema.d.ts`:

```ts
// project-decisions.ts
import type { DesignDecision } from './milestones/2026-11-messages/design-schema.d.ts';

export default [
  {
    id: 'D1',
    kind: 'choice',
    question: 'Which end state should the reader show?',
    options: [
      { id: 'last-seen', title: 'Show the last observed state' },
      { id: 'unknown', title: 'Show unknown until observed again' },
    ],
    recommended: 'none',
    authority: { state: 'owner-open' },
  },
] as const satisfies readonly DesignDecision[];
```

Shared rulings use `kind: 'ruling'`, `statement` and settled authority. Shared assumptions use `kind: 'assumption'` and the `DesignAssumption` type. Follow the [decision and assumption fields](typed-design.md#decisions-and-assumptions) in both cases. Shared source links resolve from the project folder.

Refer to a shared record as `project/D1` or `project/A1`, and to another milestone's record or deliverable as `sessions/D4` or `sessions/DEL-01`. Place a shared decision or assumption with `<ex-section-item record="project/D1">` and its authored body on one declared page. Qualified references need `project check`. Use links from any other page that discusses the record.

## Agent aliases

```ts
// agents-identity.ts
import { agentId } from './milestones/2026-11-messages/design-sdk.generated.ts';

const messagesLead = agentId('replace-with-the-lead-session-id');
const messagesBuilder = agentId('replace-with-the-builder-session-id');

export default { owner: 'owner', agents: { 'messages-lead': messagesLead, 'messages-builder': messagesBuilder } };
```

Declare each `agentId` as a top-level `const`. Use aliases from this file in agent decisions, recommendations and assumptions, for example `by: { alias: 'messages-lead', role: 'lead' }`. Name a milestone's Lead with `defineDelivery(design, entries, { lead: 'messages-lead' })` and a deliverable's builder with `builder: 'messages-builder'`. The checker validates aliases used for delivery assignments against the identity file. Generated views show aliases. Session IDs stay in the identity file.

## Check and read the project

```sh
demistify design types docs/projects/2026-11-router/milestones/2026-11-messages
demistify project check docs/projects/2026-11-router --site-root docs
demistify brief messages/DEL-01 docs/projects/2026-11-router --site-root docs
```

Run `design types` for each milestone after a fresh checkout so the editor and `agents-identity.ts` can resolve the generated SDK. `project check` is read-only. It lists open owner questions under "Waiting on owner", then each milestone's delivery state; `--write` saves the ignored generated views. `brief DEL-01` run inside a milestone and `brief messages/DEL-01` run at the root select the same deliverable; a bare ID that matches in several milestones returns the qualified choices.

For a project-wide view, place this in the project's `index.html` and serve it with `open`:

```html
<ex-project-overview src="./project.generated.json"></ex-project-overview>
```

It links owner questions, milestones, deliverables and records to their pages. Use `ex-deliverables` for one milestone's detail.

## Migrate a project from 1.32.1

From the Demistify source checkout, run the migration against a committed copy of the docs tree. The first command produces a JSON report. If the report has `blocked: true`, fill the missing values in its `recordInput` object and save that object in `record-input.json`. The report includes the original question as a starting point where a choice needs author input.

```sh
node packages/demistify/scripts/migrate-lean-design-records.ts \
  --released-lean --source /path/to/copied/docs --project projects/2026-11-router --dry-run
node packages/demistify/scripts/migrate-lean-design-records.ts \
  --released-lean --source /path/to/copied/docs --project projects/2026-11-router \
  --record-input /path/to/record-input.json --output /path/to/new/docs
```

The migration uses the original record IDs, page bindings and links. It reads a contiguous block of migration comments immediately above a record. Use `// weight: design`, `// weight: process` or `// weight: minor` for weight and `// by: messages-lead lead` for an assumption's author. Use `// settles: <sentence>` for an open assumption's settling condition and `// settledBy: <alias>` for a confirmed or refuted assumption. A `settles: unknown` comment leaves the settling condition missing. Supply missing facts and the actual open-choice options and recommendation in the input file. The migration moves a section's authored contents into a sibling item and validates the output with the project checker. Regenerate views with `project check --write` using the matching CLI, then open the resulting pages.
