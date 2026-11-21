# Track design records across a site

Use this optional workflow when agents need to refer to requirements, rules, decisions, and planned checks by ID. Write the HTML by hand with the components that answer the reader's questions. Give each record a real HTML ID so a handoff can link straight to the explanation.

```sh
demistify design init docs/specs/main/deliverables/jobs
demistify design check docs/specs/main/deliverables/jobs --site-root docs/specs
```

Use the CLI from the selected Demistify binding. With a local npm installation, prefix each invocation with `npm exec --`. With the published archive, use the exact `pnpm dlx` URL from the matching entry guide and append the same `design` arguments.

1. `init` writes `design.ts`, its local `design-schema.d.ts` declaration, and `design-sdk.generated.ts`. It takes Pages, Map, and Walkthroughs from an existing authored `navigation.json`. For a new set, use the seeded page list and author the HTML pages yourself. Keep `design.ts` a literal data object. Its type import resolves to the local declaration.
2. Add records with a stable ID, a title, a destination on a page in the set, and source links. Rules cite the needs they serve. Realizations and planned checks cite the rules they address.
3. Use `<ex-nav src="./navigation.generated.json">` on owned pages, with the relative path adjusted for nested pages. Page links and source links may leave the set when their destinations stay inside the explicit site root. For example, `../../overview.html#scope` can link from the jobs set to the main overview. Preview a shared navigation site over HTTP.
4. Run `check` to validate the authored inputs, local targets, and Demistify markup. It writes `design.generated.json` for record lookup, `navigation.generated.json` for navigation, and `deliverables.generated.json` for delivery progress. Edit the TypeScript or HTML sources and rerun the checker to refresh these files.

An HTML fragment check verifies that the authored ID exists. Source excerpts and whether a planned check actually proves its rule still need engineering review.

If a check is forcibly interrupted while writing, remove the leftover `.demistify-design.lock` only after the checking process has ended, then rerun the check to regenerate the outputs.

## Record decisions and assumptions

Give each material decision or assumption a stable ID and an owning HTML passage. Write the explanation, options and tradeoffs directly in HTML. Edit the structured record in `design.ts`.

| Record | What to enter |
| --- | --- |
| Decision → Lead | A `choice` with `authority.state: 'lead-decided'`, the `lead`, their `decision` and `rationale`, `decidedOn` as a calendar date, and `sourceKind` of `author` or `contract`. Cite the contract when it provides the authority. |
| Decision → Owner | A `choice` with `authority.state: 'owner-decided'`, `owner: 'owner'`, the `decision`, `decidedOn`, and `sourceKind` of `owner-words` or `owner-accepted`. Cite where the owner gave the answer or approved the wording in `sources`. |
| Open owner question | A `choice` with `authority.state: 'owner-open'`, `owner: 'owner'`, and the `question`. |
| Assumption | An `assumption` with the `statement` the work relies on and `status` of `open`, `confirmed`, or `refuted`. Add a `resolution` with the evidence `href` and `excerpt` when the assumption is confirmed or refuted. |

A rule, realization or planned check uses `governed-by` to link to a decision and `relies-on` to link to an assumption. Use these links only where the work depends on the item. If the HTML passage already has a design-item or decision authority marker, it must agree with the typed record. The checker validates the shape and source links. The Lead checks who made the decision and what the source supports.

The Implementer and Verifier log observations, blockers, evidence and recommendations in Show Me Your Work. If they find something that might change the agreed design or commit substantial rework, they bring it to the Lead before proceeding with affected work. The Lead alone judges whether it breaks the shared mental model and decides what to bring to the owner. Record the call under the authority that actually made it. Carry an assumption into the design when the work depends on it.

## Compose a delivery plan

The Lead registers the deliverable modules and prerequisites in `delivery.ts`. Each pair works in one file under `deliverables/`. For an existing design, run `demistify design types <directory>` to refresh its generated authoring support. Update the page’s navigation binding when migrating from the earlier output filename.

```typescript
// delivery.ts
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

Give the pair the deliverable ID and file. Link scope to existing rule, design, and planned-check IDs. The Implementer records `implementation.completedTasks`, `candidate`, and local evidence. The Verifier records `verification.build`, the checker’s displayed `scopeRevision`, and `outcomes` keyed by check ID. A passing or failing outcome links its observation and evidence. A blocked outcome names the reason.

An unanswered governing owner question or a refuted assumption blocks the affected deliverable. The Lead can authorize reversible work under an open assumption. That deliverable stays open until the assumption is confirmed, even when its checks passed. The reader links to the governing items so the pair can open the right passage.

Once the required evidence matches the current scope and candidate, the Lead can record `acceptances` in `design.ts`, with a `deliverableId`, `build`, `scopeRevision`, and evidence link. Dependent work shows its unmet prerequisites. A build or governing passage change keeps the older outcome visible as stale. When the Lead changes the assigned checks, move the earlier verification block to the deliverable’s `verificationHistory` so readers can still inspect it.

In the authored HTML page, place the delivery overview where the reader needs the current state:

```html
<ex-deliverables src="./deliverables.generated.json" label="Delivery progress"></ex-deliverables>
```

The view shows state, open checks, evidence and links back to the design. Serve the page and generated files over HTTP.
