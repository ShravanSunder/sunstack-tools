# Typed design records

Use typed records when agents refer to a milestone's needs, rules, decisions and checks by ID. A record is the only home of its text. Pages hold explanation and diagrams, and place records by ID where the reader needs them.

## Commands

```sh
demistify design init <milestone>
demistify design types [milestone]
demistify design check [milestone] [--site-root <directory>]
demistify brief <deliverable> [milestone] [--site-root <directory>]
demistify open <page.html> [--site-root <directory>]
```

Run the CLI from the selected Demistify binding: prefix `npm exec --` for a local install, or use the exact `pnpm dlx` URL from the matching entry guide. To save `brief` output without pnpm's progress line, run `pnpm --silent dlx <url> brief …`.

- `init` writes `design.ts`, `design-decisions.ts`, `design-assumptions.ts`, `design-delivery.ts` and the generated SDK, seeding Pages, Map and Walkthroughs from an existing `navigation.json`. You write the HTML pages.
- `types` regenerates `design-sdk.generated.ts` and `design-schema.d.ts`. Git ignores both, so run it after a fresh checkout and after adding or renaming an ID, for the editor. `check`, `brief` and `open` build what they need without it.
- `design check` and `project check` validate typed inputs and prepared pages in memory, then print deliverable states and design notes. Run `check <page.html>` for ordinary standalone HTML. Use `--write` only to save ignored `*.generated.json` files for a static server.
- `open` serves pages with records filled in and data built. Restart it after editing records.

Pass `--site-root` with the enclosing docs directory when pages or evidence link outside the milestone folder.

IF the milestone belongs to a project with several milestones or shared decisions, load `typed-design-projects.md` and return the project root, its registered milestones and the `project check` result.

## Write records

Import the helpers from the milestone's generated SDK. Each helper supplies the record's `kind`. The CLI reads these files as literal data and never executes them.

```ts
// design.ts
import { defineCheck, defineDesign, defineNeed, defineRealization, defineRule } from './design-sdk.generated.ts';

export default defineDesign({
  pages: [{ id: 'home', title: 'Messages', href: 'index.html', icon: 'envelope' }],
  mapGroups: [{ description: 'Why a row leaves the inbox', icon: 'clock', target: { record: 'R1' } }],
  records: [
    defineNeed({ id: 'U1', statement: 'A reader never sees a message after central has withdrawn it.' }),
    defineRule({
      id: 'R1',
      title: 'Inbox expiry',
      statement: 'An inbox row expires at its arrival time plus the time left that central sent.',
      serves: 'U1',
      governedBy: ['D2'],
      reliesOn: ['A1'],
      cases: [{ given: 'time left 0 or below', expect: 'reject the row' }],
    }),
    defineRealization({ id: 'M1', statement: 'The Mac stores the expiry time with each inbox row.', realizes: 'R1' }),
    defineCheck({
      id: 'V1',
      statement: 'An expired row is gone from the inbox query.',
      checks: 'R1',
      journey: 'inbox-expiry integration test',
      unverified: 'Clock skew between central and the Mac',
      cases: [
        { given: 'time left 60 s, read at 61 s', expect: 'no row' },
        { given: 'time left 60 s, read at 59 s', expect: 'one row' },
        { given: 'time left 0', expect: 'row rejected on arrival' },
      ],
    }),
  ],
});
```

| Record | Helper | Fields |
| --- | --- | --- |
| Need | `defineNeed` | `statement` |
| Rule | `defineRule` | `statement`; optional `serves` (needs), `governedBy`, `reliesOn`, `cases` |
| Realization | `defineRealization` | `statement`, `realizes` (rules); optional `governedBy`, `reliesOn` |
| Planned check | `defineCheck` | `statement`, `checks` (one rule or several), `journey`, `unverified`, three or more `cases`; optional `governedBy`, `reliesOn` |
| Decision | `defineDecision` in `design-decisions.ts` | `authority`; optional `supersedes`, `sources` |
| Assumption | `defineAssumption` in `design-assumptions.ts` | `statement`, `status`; optional `sources`, `resolution` |

Write a rule as one sentence and put its detail in `cases`. `title` is an optional short label for the Tree, Map and outline, which otherwise show the statement. On a rule, realization or check, `governedBy` names decisions and `reliesOn` names assumptions; add them only where the record depends on that item. The check rejects a link to the wrong kind of record.

A large milestone can move a subject's records to `records/<subject>.ts`, exporting an array of helper calls that import from `../design-sdk.generated.ts`. Spread the default import directly inside `records`. IDs and links still span the milestone.

### Decisions and assumptions

A decision holds the owner's words in its `authority`:

| Authority | Fields |
| --- | --- |
| Owner decided | `state: 'owner-decided'`, `owner: 'owner'`, `decision`, `decidedOn` (a date), `sourceKind: 'owner-words'` or `'owner-accepted'` |
| Open owner question | `state: 'owner-open'`, `owner: 'owner'`, `question` |
| Lead decided | `state: 'lead-decided'`, `lead`, `decision`, `rationale`, `decidedOn`, `sourceKind: 'author'` or `'contract'` |

```ts
// design-decisions.ts
import { defineDecision } from './design-sdk.generated.ts';

export default [
  defineDecision({
    id: 'D2',
    title: 'Expired row display',
    authority: { state: 'owner-open', owner: 'owner', question: 'Should an expired row stay visible, greyed out, until the reader scrolls?' },
  }),
];
```

`design-assumptions.ts` has the same shape: `defineAssumption({ id: 'A1', statement: 'Central and the Mac clocks differ by under five seconds.', status: 'open' })`. A `confirmed` or `refuted` assumption needs `resolution: { href, excerpt }` pointing at local evidence. `sources` take the same shape; add `verbatim: true` and the check confirms the excerpt appears in the cited HTML passage.

A new decision can declare `supersedes: 'D1'` and keep the old one in the file. Only an owner decision can replace an owner decision, and the check rejects a live link to a superseded one.

## Place records on pages

Place a record with an empty element. The placement is the anchor: `open` adds `id="R1"` and the statement to the served HTML, so any page can link to `#R1`. The source stays as written.

```html
<head>
  <link rel="stylesheet" href="https://demistify.getagentstudio.dev/v1/theme.css">
  <script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script>
</head>
<ex-nav for="content" src="./navigation.generated.json"></ex-nav>
<ex-document id="content">
  <h2>Inbox expiry</h2>
  <p data-design-record="R1"></p>
  <p>The sweep runs on one timer per inbox, so a burst of arrivals does not multiply wakeups.</p>
  <ex-tree data-design-need="U1"></ex-tree>
</ex-document>
```

- Use `p`, `div`, `span`, `li`, `dt`, `dd`, `td`, `th`, `blockquote` or a heading. A long statement in a heading fills the outline and wraps badly on a phone, so put a full rule in a `p` under an ordinary heading.
- Write no text, `id` or `title` on the placement. The check fails on copied text, a hand-written anchor, an unknown ID or a second placement of one record.
- A record need not be placed. Briefs and deliverables still list it, without a link.
- `<ex-tree data-design-need="U1"></ex-tree>` draws the need and the rules that serve it, linked to their placements.
- Each page in the milestone keeps one `ex-nav` whose `src` resolves to its `navigation.generated.json`.

Bind a Choice to an open owner decision. The record supplies the question, anchor and decision identity; you write the options. The check rejects a bound Choice on a settled decision, or with its own `title` or `data-decision`.

```html
<ex-choice data-design-record="D2">
  <ex-option id="hide-at-once" title="Hide at once">
    <template slot="summary">The row disappears when it expires.</template>
  </ex-option>
  <ex-option id="grey-until-scroll" title="Grey until scroll">
    <template slot="summary">The row stays, greyed, until the reader scrolls.</template>
    <div slot="details"><p>Moves DEL-02 into the core plan.</p></div>
  </ex-option>
</ex-choice>
```

## Map and Walkthrough by record ID

A Map node or Walkthrough step in `design.ts` targets `{ record: 'R1' }` and links to that record's placement. A record-bound Map node may omit `title` but keeps its `description` and `icon`. A Walkthrough step keeps its narration: `steps: [{ target: { record: 'U1' }, narration: 'Start with the need.' }]`. A mistyped ID fails the check with `TS2322`.

## Plan deliverables

`design-delivery.ts` lists one module per deliverable under `deliverables/`, each with optional `dependsOn` prerequisites. A `core` deliverable has a scope and is checked. A `later` one is only `defineDeliverable({ kind: 'later', id: 'DEL-02', title: 'Grey out expired rows' })`: listed, not checked, and without a brief.

```ts
// design-delivery.ts
import design from './design.ts';
import { defineDelivery } from './design-sdk.generated.ts';
import expireRows from './deliverables/del-01-expire-inbox-rows.ts';
import greyRows from './deliverables/del-02-grey-expired-rows.ts';

export default defineDelivery(design, [{ deliverable: expireRows }, { deliverable: greyRows }]);
```

```ts
// deliverables/del-01-expire-inbox-rows.ts
import { defineDeliverable } from '../design-sdk.generated.ts';

export default defineDeliverable({
  kind: 'core',
  id: 'DEL-01',
  title: 'Expire inbox rows',
  destination: { page: 'index.html', fragment: 'DEL-01' },
  scope: {
    needs: ['U1'],
    rules: ['R1'],
    designItems: ['M1'],
    implementationTasks: ['store-expiry', 'sweep-expired'],
    verificationChecks: ['V1'],
  },
});
```

`destination` names a real `id` on a declared page, such as `<h2 id="DEL-01">`. Optional `needs` lists the needs this deliverable covers. `designItems` lists scoped realizations. Each scoped check must cover a scoped rule. Progress goes in the same module:

- `implementation: { completedTasks, candidate, evidence }`: finished tasks, the build, evidence links.
- `verification: { build, outcomes }`, keyed by check ID. An outcome is `{ kind: 'passed' | 'failed', observation, evidence }` or `{ kind: 'blocked', reason }`. When the assigned checks change, move the old receipt to `verificationHistory`.

Record acceptance in `design.ts` as `acceptances: [{ deliverableId: 'DEL-01', build: 'build-87', evidence: 'acceptance.md' }]`, where `build` equals the implementation candidate. The check holds a core deliverable short of acceptance while:

- its scope has no rules or no tasks, a task is open, or no candidate is recorded;
- a scoped rule serves no need, or no assigned check covers it;
- no scoped rule serves one of its explicitly scoped needs;
- a check outcome is missing, failed, blocked, or recorded on another build;
- a governing open owner question or refuted assumption blocks the work, or a governing assumption is still open;
- a prerequisite is not accepted.

Known loss: the check does not flag a rule that changed after acceptance. Treat such an edit as new scope and verify it again.

Design notes include each authored need that no rule in the checked design or project serves, even if the need is outside delivery scope. The check also notes scoped rules without a realization. These notes are informational and stay visible after acceptance.
The check adds one informational note per need, rule, realization or planned check whose statement has more than one sentence or more than 40 whitespace-delimited words.

Show progress with `<ex-deliverables src="./deliverables.generated.json" label="Delivery progress"></ex-deliverables>`: each deliverable's state, open checks and evidence, linked to the governing records.

## Brief a deliverable

`demistify brief DEL-01 <milestone>` prints a Markdown brief with the title, prerequisites, scoped needs, rules, realizations and checks, tasks, open owner questions and assumptions, and coverage gaps and notes. It validates the typed inputs without preparing HTML or writing files. Give the helper this brief and run the typed check to validate the pages.
