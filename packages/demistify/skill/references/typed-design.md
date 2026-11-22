# Track design records across a site

Use this optional workflow when agents need to refer to requirements, rules, decisions, and planned checks by ID. Write the HTML by hand with the components that answer the reader's questions. Give each record a real HTML ID so a handoff can link straight to the explanation.

```sh
demistify design init docs/projects/2026-11-router/milestones/2026-11-messages
demistify design check docs/projects/2026-11-router/milestones/2026-11-messages --site-root docs/projects/2026-11-router
```

Use the CLI from the selected Demistify binding. With a local npm installation, prefix each invocation with `npm exec --`. With the published archive, use the exact `pnpm dlx` URL from the matching entry guide and append the same `design` arguments.

1. `init` writes `design.ts`, `design-decisions.ts`, `design-assumptions.ts`, `design-delivery.ts`, and their generated authoring support. It takes Pages, Map, and Walkthroughs from an existing authored `navigation.json`. For a new set, use the seeded page list and author the HTML pages yourself. Keep `design.ts` a literal data object. Decisions and assumptions live in separate literal arrays. Their type imports resolve to the local declaration.
2. Add records with a stable ID, a title, a destination on a declared page, and source links. Rules cite the needs they serve. Realizations and planned checks cite the rules they address. Give each planned check at least three `given` / `expect` cases.
3. Use `<ex-nav src="./navigation.generated.json">` on owned pages, with the relative path adjusted for nested pages. Point record and plan destinations to existing reviewed passages when they already have stable HTML IDs. Add each such page to the set’s `pages` and keep its own navigation binding. Use an explicit `--site-root` containing the set and its reviewed pages. A set can declare only reviewed pages. Preview a shared navigation site over HTTP.
4. Run `check` to validate the authored inputs, local targets, and Demistify markup. It writes `design.generated.json` for record lookup, `navigation.generated.json` for navigation, and `deliverables.generated.json` for delivery progress. Edit the TypeScript or HTML sources and rerun the checker to refresh these files.

An HTML fragment check verifies that the authored ID exists. A heading ID covers the heading and its content until the next equal or higher heading in its section. A container ID covers the entire element. Editing a governing passage reopens evidence tied to its old scope revision.

Source excerpts can summarize a source. For an exact quotation from a local HTML passage, add `verbatim: true` to its source reference. The checker compares decoded text after collapsing whitespace and checks only the cited passage when the link contains a fragment.

```ts
{
  href: '../../reviewed/queue.html#cancel-before-claim',
  excerpt: 'A cancelled job cannot acquire a worker claim.',
  verbatim: true,
}
```

If a check is forcibly interrupted while writing, remove the leftover `.demistify-design.lock` only after the checking process has ended, then rerun the check to regenerate the outputs.

## Record decisions and assumptions

Give each material decision or assumption a stable ID and an owning HTML passage. Write the explanation, options and tradeoffs directly in HTML. Write decisions in `design-decisions.ts` and assumptions in `design-assumptions.ts`. Keep their stable IDs and point to them from the governing item in `design.ts`.

| Record | What to enter |
| --- | --- |
| Decision → Lead | A `choice` with `authority.state: 'lead-decided'`, the `lead`, their `decision` and `rationale`, `decidedOn` as a calendar date, and `sourceKind` of `author` or `contract`. Cite the contract when it provides the authority. |
| Decision → Owner | A `choice` with `authority.state: 'owner-decided'`, `owner: 'owner'`, the `decision`, `decidedOn`, and `sourceKind` of `owner-words` or `owner-accepted`. Cite where the owner gave the answer or approved the wording in `sources`. |
| Open owner question | A `choice` with `authority.state: 'owner-open'`, `owner: 'owner'`, and the `question`. |
| Assumption | An `assumption` with the `statement` the work relies on and `status` of `open`, `confirmed`, or `refuted`. Add a `resolution` with the evidence `href` and `excerpt` when the assumption is confirmed or refuted. |

A rule, realization or planned check uses `governed-by` to link to a decision and `relies-on` to link to an assumption. Use these links only where the work depends on the item. If the HTML passage already has a design-item or decision authority marker, it must agree with the typed record. The checker validates the shape and source links. The Lead checks who made the decision and what the source supports.

The Implementer and Verifier log observations, blockers, evidence and recommendations in Show Me Your Work. If they find something that might change the agreed design or commit substantial rework, they bring it to the Lead before proceeding with affected work. The Lead alone judges whether it breaks the shared mental model and decides what to bring to the owner. Record the call under the authority that actually made it. Carry an assumption into the design when the work depends on it.

## Compose a delivery plan

The Lead registers the deliverable modules and prerequisites in `design-delivery.ts`. Each pair works in one file under `deliverables/`. For an existing design, run `demistify design types <directory>` to refresh its generated authoring support. Update the page’s navigation binding when migrating from the earlier output filename.

```typescript
// design-delivery.ts
import design from './design.ts';
import { defineDelivery } from './design-sdk.generated.ts';
import preventCancelledJobs from './deliverables/del-01-prevent-cancelled-job-from-running.ts';

export default defineDelivery(design, [{ deliverable: preventCancelledJobs }]);
```

```typescript
// deliverables/del-01-prevent-cancelled-job-from-running.ts
import { defineDeliverable } from '../design-sdk.generated.ts';

export default defineDeliverable({
  id: 'DEL-01',
  title: 'Prevent cancelled jobs from running',
  destination: { page: 'index.html', fragment: 'DEL-01' },
  scope: {
    rules: ['R3'],
    implementationTasks: ['guard-handler'],
    verificationChecks: ['V3'],
  },
});
```

Give the pair the deliverable ID and file. The Implementer records completed tasks, the candidate, and evidence. The Verifier checks that the observed build matches `implementation.candidate` and records `verification: { build, scopeRevision, outcomes }`. Read `scopeRevision` directly from the deliverable in fresh generated data. Key outcomes by check ID and link evidence or state the blocker.

An unanswered governing owner question or a refuted assumption blocks the affected deliverable. The Lead can authorize reversible work under an open assumption. That deliverable stays open until the assumption is confirmed, even when its checks passed. The reader links to the governing items so the pair can open the right passage.

When a deliverable is ready for acceptance, record `acceptances: [{ deliverableId, build, scopeRevision, evidence }]` in `design.ts`. Use a script to fill the build and scope from the checked deliverable. When assigned checks change, keep earlier verification in `verificationHistory`.

In the authored HTML page, place the delivery overview where the reader needs the current state:

```html
<ex-deliverables src="./deliverables.generated.json" label="Delivery progress"></ex-deliverables>
```

The view shows state, open checks, evidence and links back to the design. Serve the page and generated files over HTTP.

## Check a project across Leads

Keep a project root above its named design milestones. Put each set's four authoring files in its own spec directory. The project manifest registers the sets and the shared records it needs.

```text
docs/projects/2026-11-router/
  project.ts
  project-decisions.ts
  project-assumptions.ts
  agents-identity.ts
  milestones/
    2026-11-messages/
      design.ts
      design-decisions.ts
      design-assumptions.ts
      design-delivery.ts
      deliverables/
    2026-11-sessions/
      design.ts
      design-decisions.ts
      design-assumptions.ts
      design-delivery.ts
      deliverables/
```

```typescript
// project.ts
import messages from './milestones/2026-11-messages/design.ts';
import sessions from './milestones/2026-11-sessions/design.ts';
import decisions from './project-decisions.ts';
import assumptions from './project-assumptions.ts';
import identities from './agents-identity.ts';

export default {
  designs: { messages, sessions },
  decisions,
  assumptions,
  identities,
};
```

Each shared records file exports an array of its own record kind. Use type-only imports from a generated `design-schema.d.ts` inside the project for editor checking. Keep a project's decision and assumption files optional. To move a file inside the project, retain its name and update its explicit import and relative links. A project may register no designs or shared inputs. For example, `export default {};` generates an empty project view. The checker reads the registered inputs.

```typescript
// agents-identity.ts
import { agentId } from './milestones/2026-11-messages/design-sdk.generated.ts';

const messagesLead = agentId('replace-with-the-lead-session-id');
const messagesBuilder = agentId('replace-with-the-builder-session-id');

export default {
  owner: 'owner',
  agents: { 'messages-lead': messagesLead, 'messages-builder': messagesBuilder },
};
```

Declare each `agentId` in a top-level `const` and use its alias in the delivery plan. Keep the session ID in the identity file.

```typescript
export default defineDelivery(
  design,
  [{ deliverable: cancelMessage, dependsOn: ['sessions/DEL-01'] }],
  { lead: 'messages-lead' },
);
```

Set `builder: 'messages-builder'` in the deliverable module to show its Implementer. In a project, register both role aliases in `agents-identity.ts`. The generated views show aliases and keep session IDs in the authoring input. Builder changes leave scope revisions intact. The milestone Lead still records acceptance. In a standalone set, use an authored role label.

Link a design item to a shared decision as `project/D1`, a shared assumption as `project/A1`, or a record in another registered design as `sessions/D4`. A new decision can declare `supersedes: 'D4'` and keep the previous record in the file. Update any affected governing link after checking what the replacement changes. The checker catches live links to a superseded decision and cycles in supersession or delivery prerequisites.

```sh
demistify project check docs/projects/2026-11-router --site-root docs
demistify project check docs/projects/2026-11-router --site-root docs --write
```

The first command checks the project and lists open owner questions and delivery status. Add `--write` to refresh `project.generated.json` and the three generated files in each milestone. The site root can include reviewed HTML and evidence outside the project. Keep the manifest, identity file, and other TypeScript inputs inside the project.

For receipt automation, use `project check --write` to refresh the data. Use `design check` for self-contained sets.

When readers need a project-wide view, place this binding in a hand-authored page served alongside the generated JSON.

```html
<ex-project-overview src="./project.generated.json"></ex-project-overview>
```

The overview links owner questions, milestones, deliverables, and records to their authored pages. Use `ex-deliverables` for the delivery details of a single milestone.

After updating the CLI for an existing project, run `demistify design types <milestone>` for every registered milestone. The command replaces the generated declarations and SDK, including the SDK imported by `agents-identity.ts`. Then run the read-only project check and add `--write` when ready to refresh the view.

For a draft that still embeds choices or assumptions in `design.ts`, move them to the separate record files and rename `delivery.ts` to `design-delivery.ts`. Preserve record IDs, HTML destinations, and sources.
