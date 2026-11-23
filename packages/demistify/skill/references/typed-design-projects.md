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
  { id: 'D1', kind: 'choice', authority: { state: 'owner-open', owner: 'owner', question: 'Which end state should the reader show?' } },
] as const satisfies readonly DesignDecision[];
```

Refer to a shared record as `project/D1` or `project/A1`, and to another milestone's record or deliverable as `sessions/D4` or `sessions/DEL-01`. Qualified references need `project check`; `design check` rejects them.

## Agent aliases

```ts
// agents-identity.ts
import { agentId } from './milestones/2026-11-messages/design-sdk.generated.ts';

const messagesLead = agentId('replace-with-the-lead-session-id');
const messagesBuilder = agentId('replace-with-the-builder-session-id');

export default { owner: 'owner', agents: { 'messages-lead': messagesLead, 'messages-builder': messagesBuilder } };
```

Declare each `agentId` as a top-level `const`. Name a milestone's Lead with `defineDelivery(design, entries, { lead: 'messages-lead' })` and a deliverable's builder with `builder: 'messages-builder'`. The check rejects an alias missing from `agents-identity.ts`. Generated views show aliases; session IDs stay in the identity file.

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
