# Typed design initiatives

An initiative is the whole plan. It holds projects; each project holds deliverables.

| Level | What it is | Where it lives |
| --- | --- | --- |
| Initiative | The whole plan: its projects, each with a number, and the decisions they share | `docs/initiatives/<initiative>/initiative.ts` |
| Project | One piece of work that finishes, with its own design; it says what it achieves and what the owner checks to accept it | `docs/initiatives/<initiative>/projects/<project>/`, with the files from [typed-design.md](typed-design.md) |
| Deliverable | A piece of a project that is built, checked and accepted | `deliverables/` in its project |

**Order.** Each project has a number, the order the owner wants. Every project numbered 1 runs side by side; projects numbered 2 come after them, then 3, and so on. **Dependencies** are only between deliverables: a deliverable's `dependsOn` names deliverables in its own or any other project, and it cannot be accepted before them. The check warns when a deliverable depends on one in a higher-numbered project: the cut or the number is wrong. Projects have no `dependsOn`.

```ts
// initiative.ts
import messages from './projects/2026-11-messages/design.ts';
import sessions from './projects/2026-11-sessions/design.ts';
import decisions from './initiative-decisions.ts';
import assumptions from './initiative-assumptions.ts';
import identities from './agents-identity.ts';

export default {
  projects: {
    sessions: {
      design: sessions,
      number: 1,
      achieves: 'Central knows every session.',
      acceptance: 'A Claude session registers with central on staging and is listed there.',
    },
    messages: { design: messages, number: 2, achieves: 'A message reaches any session.' },
  },
  decisions,
  assumptions,
  identities,
};
```

Every field is optional; `export default {};` is a valid empty initiative. Keep every TypeScript input inside the initiative root. The name `initiative` is reserved for shared records.

The shared files export literal arrays with an explicit `kind`, and use only type-only imports from a project's `design-schema.d.ts`:

```ts
// initiative-decisions.ts
import type { DesignDecision } from './projects/2026-11-messages/design-schema.d.ts';

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

Shared rulings use `kind: 'ruling'`, `statement` and settled authority. Shared assumptions use `kind: 'assumption'` and the `DesignAssumption` type. Follow the [decision and assumption fields](typed-design.md#decisions-and-assumptions) in both cases. Shared source links resolve from the initiative folder.

Refer to a shared record as `initiative/D1` or `initiative/A1`, and to another project's record or deliverable as `sessions/D4` or `sessions/DEL-01`. Place a shared decision or assumption with `<ex-section-item record="initiative/D1">` and its authored body on one declared page. Qualified references need `initiative check`. Use links from any other page that discusses the record.

## Agent aliases

```ts
// agents-identity.ts
import { agentId } from './projects/2026-11-messages/design-sdk.generated.ts';

const messagesLead = agentId('replace-with-the-lead-session-id');
const messagesBuilder = agentId('replace-with-the-builder-session-id');

export default { owner: 'owner', agents: { 'messages-lead': messagesLead, 'messages-builder': messagesBuilder } };
```

Declare each `agentId` as a top-level `const`. Use aliases from this file in agent decisions, recommendations and assumptions, for example `by: { alias: 'messages-lead', role: 'lead' }`. Name a project's Lead with `defineDelivery(design, entries, { lead: 'messages-lead' })` and a deliverable's builder with `builder: 'messages-builder'`. The checker validates aliases used for delivery assignments against the identity file. Generated views show aliases. Session IDs stay in the identity file.

## Check and read the initiative

```sh
demistify design types docs/initiatives/2026-11-router/projects/2026-11-messages
demistify initiative check docs/initiatives/2026-11-router --site-root docs
demistify initiative status docs/initiatives/2026-11-router --site-root docs
demistify brief messages/DEL-01 docs/initiatives/2026-11-router --site-root docs
```

Run `design types` for each project after a fresh checkout so the editor and `agents-identity.ts` can resolve the generated SDK. `initiative check` is read-only. It lists the projects by number, any order warnings, open owner questions under "Waiting on owner", then each deliverable's state; `--write` saves the ignored generated views. `initiative status` prints the plan: each project by number with its state, what it achieves and what the owner checks, every deliverable with its state, tasks done, checks passed and what it waits on, then what is ready now, what blocks the most open work, and what is built but not accepted; `--json` gives the same as data. `brief DEL-01` run inside a project and `brief messages/DEL-01` run at the root select the same deliverable; a bare ID that matches in several projects returns the qualified choices.

For an initiative-wide view, place this in the initiative's `index.html` and serve it with `open`:

```html
<ex-initiative-overview src="./initiative.generated.json"></ex-initiative-overview>
```

It opens with the projects grouped by number, each with what it achieves and what the owner checks, linked to its page; then owner questions, the work lists, deliverables and records. Every state and id prefix carries a plain explanation. Use `ex-deliverables` for one project's detail.

## Migrate an initiative from 1.32.1

From the Demistify source checkout, run the migration against a committed copy of the docs tree. The dry run validates a temporary copy when all required inputs are present. If the JSON report has `blocked: true`, fill the missing values in its `recordInput` object and save that object in `record-input.json`. Where the report lists `sourceCandidates`, copy the source that establishes the decision into the record's `source` field.

```sh
node packages/demistify/scripts/migrate-lean-design-records.ts \
  --released-lean --source /path/to/copied/docs --project initiatives/2026-11-router --dry-run
node packages/demistify/scripts/migrate-lean-design-records.ts \
  --released-lean --source /path/to/copied/docs --project initiatives/2026-11-router \
  --record-input /path/to/record-input.json --output /path/to/new/docs
```

The migration uses the original record IDs, page bindings and links. It reads a contiguous block of migration comments immediately above a record. Use `// weight: design`, `// weight: process` or `// weight: minor` for weight and `// by: messages-lead lead` for an assumption's author. Use `// settles: <sentence>` for an open assumption's settling condition and `// settledBy: <alias>` for a confirmed or refuted assumption. A `settles: unknown` comment leaves the settling condition missing. Explicit input takes precedence over comments and the report lists conflicts.

If a bound Choice already has options in HTML, the migration reads them from that page and preserves the original option and Details fragment IDs. Supply the recommendation and any other missing decision fields in the input file. The migration moves a section's authored contents into a sibling item and publishes the requested output directory only after the initiative checker passes. Regenerate views with `initiative check --write` using the matching CLI, then open the resulting pages.
