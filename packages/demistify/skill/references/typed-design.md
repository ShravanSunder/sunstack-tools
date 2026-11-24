# Typed design records

Use typed records when agents refer to a milestone's needs, rules, decisions and checks by ID. A record is the only home of its text. Pages hold explanation and diagrams, and place records by ID where the reader needs them.

## Commands

```sh
demistify design init <milestone>
demistify design types [milestone]
demistify design check [milestone] [--site-root <directory>] [--write]
demistify brief <deliverable> [milestone] [--site-root <directory>]
demistify open <page.html> [--site-root <directory>]
```

Run the CLI from the selected Demistify binding: prefix `npm exec --` for a local install, or use the exact `pnpm dlx` URL from the matching entry guide. To save `brief` output without pnpm's progress line, run `pnpm --silent dlx <url> brief …`.

- `init` writes `design.ts`, `design-decisions.ts`, `design-assumptions.ts`, `design-delivery.ts` and the generated SDK, seeding Pages, Map and Walkthroughs from an existing `navigation.json`. You write the HTML pages.
- `types` regenerates `design-sdk.generated.ts` and `design-schema.d.ts`. Git ignores both, so run it after a fresh checkout and after adding or renaming an ID, for the editor. `check`, `brief` and `open` build what they need without it.
- `design check` and `project check` validate typed inputs and prepared pages in memory, then print deliverable states and design notes. Run `check <page.html>` for ordinary standalone HTML.
- After a checkout or record edit, run `demistify project check --write` (or `demistify design check --write` for a standalone milestone). Serve the authored pages with any static HTTP server and the matching runtime. Opened from disk in Chrome, pages show records from `records.generated.js`. Shared navigation and delivery tables still need HTTP. `open` runs the write step for you. One `.gitignore` covers generated files in the enclosing `projects/` folder when the project is directly under it, otherwise in the project root or standalone milestone folder. Existing ancestor rules that cover all generated files need no update. A browser may show old text until you rerun the write command. Read-only checks tell you when the generated views need refreshing.

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
| Need | `defineNeed` | `statement`; optional `governedBy`, `reliesOn` |
| Rule | `defineRule` | `statement`; optional `serves` (needs), `governedBy`, `reliesOn`, `cases` |
| Realization | `defineRealization` | `statement`, `realizes` (rules); optional `governedBy`, `reliesOn` |
| Planned check | `defineCheck` | `statement`, `checks` (one rule or several), `journey`, `unverified`, three or more `cases`; optional `governedBy`, `reliesOn` |
| Choice | `defineChoice` in `design-decisions.ts` | `question`, `options`, `recommended`, `authority` |
| Ruling | `defineRuling` in `design-decisions.ts` | `statement`, settled `authority` |
| Assumption | `defineAssumption` in `design-assumptions.ts` | `statement`, `by`, `weight`, `status` and the fields for that status |

Write a rule as one sentence and put its detail in `cases`. `title` is an optional short label for the Tree, Map and outline, which otherwise show the statement. On a need, rule, realization or check, `governedBy` names decisions and `reliesOn` names assumptions. Add a link where the record depends on that item. The check rejects links to the wrong record kind and requires design weight for any weighted decision or assumption that a need or rule depends on.

When a milestone covers more than one subject, put each subject’s needs, rules, realizations and checks in `records/<subject>.ts`. Export an array of helper calls that import from `../design-sdk.generated.ts`. In `design.ts`, import each array and spread it directly inside `records`. Keep these records in `design.ts` for a single-subject milestone. A realization or check that spans subjects goes in the file for the first rule it names. IDs and links still span the milestone.

### Decisions and assumptions

Use a **choice** when someone is weighing alternatives or weighed them to reach a decision. Give it a question, at least two options with stable IDs, and either `recommended: 'none'` or `recommended: { option, by: { alias, role } }`. Keep every option after the choice is answered, including its page summary.

Use a **ruling** for a settled decision made without weighing alternatives. Write the ruling in `statement`. An **assumption** records something the work relies on while its truth is still being established, or the result of settling it.

| Authority | Fields |
| --- | --- |
| Open owner question | `state: 'owner-open'` on a choice |
| Owner decided | `state: 'owner-decided'`, `decidedOn`, `sourceKind: 'owner-words'` for the owner's own decision or `'owner-accepted'` for explicit approval, and `source: { href, excerpt }` |
| Agent decided | `state: 'agent-decided'`, `by: { alias, role }`, `weight`, `rationale`, `decidedOn`, and `sourceKind: 'author'` or `'contract'`. A contract also requires `source: { href, excerpt }`. |

A settled choice also has `chosen` (one option ID) and `decision` (the answer in words) inside its authority. Write dates as `YYYY-MM-DD`. Record an agent's decision within that agent's authority. If a Lead adopts a decision the helper made within its remit, use the helper's alias and role for `by`.

Use the registered alias from `agents-identity.ts` in a project. Store the alias in `by.alias` and the agent's role at the time in `by.role`. Supported roles are `lead`, `implementer`, `verifier`, `reviewer` and `advisor`. Use the same actor shape for a recommendation and an assumption.

| Weight | When to use it |
| --- | --- |
| `design` | Changes what the system does, stores or exposes, or changes its structure, including ownership, boundaries, contracts or dependencies. |
| `process` | Changes only how the work is done. |
| `minor` | A detail that leaves both the design and the working process as they were. |

An assumption takes the weight of the work that rests on it. If a design decision or rule depends on it, use `design`.

```ts
// design-decisions.ts
import { defineChoice } from './design-sdk.generated.ts';

export default [
  defineChoice({
    id: 'D2',
    question: 'Should an expired row stay visible until the reader scrolls?',
    options: [
      { id: 'hide-at-once', title: 'Hide at once' },
      { id: 'grey-until-scroll', title: 'Grey until scroll' },
    ],
    recommended: 'none',
    authority: { state: 'owner-open' },
  }),
];
```

Write a ruling and an assumption in their respective sidecars.

```ts
// design-decisions.ts
import { defineRuling } from './design-sdk.generated.ts';

export default [
  defineRuling({
    id: 'D3',
    statement: 'Keep the five-minute mark in the restart fixture name.',
    authority: {
      state: 'agent-decided',
      by: { alias: 'messages-lead', role: 'lead' },
      weight: 'minor',
      rationale: 'The mark makes the fixture easy to find.',
      decidedOn: '2026-11-24',
      sourceKind: 'author',
    },
  }),
];
```

```ts
// design-assumptions.ts
import { defineAssumption } from './design-sdk.generated.ts';

export default [
  defineAssumption({
    id: 'A1',
    statement: 'The fixture can reproduce a restart between two marks.',
    by: { alias: 'messages-lead', role: 'lead' },
    weight: 'design',
    status: 'open',
    settles: 'Run the restart fixture and observe the two marks in its trace.',
  }),
];
```

When an assumption is `confirmed` or `refuted`, replace `settles` with `settledBy` (a name or alias) and `resolution: { href, excerpt }`. Choices, rulings and assumptions may also carry `sources`. Source links are relative to the owning design. Set `verbatim: true` when the excerpt must occur in the linked page.

When the owner explicitly approves an agent's decision, keep the record ID and answer. Replace the authority with `{ state: 'owner-decided', sourceKind: 'owner-accepted', decidedOn, source: { href, excerpt } }`, keeping `chosen` and `decision` for a choice. Link to and quote the approval. A reversal is a new settled decision with `supersedes: 'D3'`. Settle an open owner choice before using it to supersede a decision. Keep the earlier record for history and move active dependencies to the replacement. An owner decision can only be superseded by another owner decision.

## Place records on pages

Place needs, rules, realizations and planned checks with empty `data-design-record` elements beside their explanation. The placement supplies the anchor. After generating the views, the runtime adds `id="R1"` and the statement when the page loads, so any page can link to `#R1`. It finds the record data beside the page’s generated navigation source, or beside the project overview source if the page has no generated navigation. A declared external page with neither binding can use `<meta name="demistify-records" content="../milestone/records.generated.js">`.

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

Give each choice, ruling and assumption one `ex-section-item record="ID"`. Declare each needed section once with `holds`. Keep every `ex-section` empty and write the items beside it. Declare all reachable destinations so an answer can move a record to its new section without moving its authored body.

| Record on this page | Declare one section for each of |
| --- | --- |
| Open owner choice | `needs-owner`, `decided-by-agent`, `settled` |
| Agent-decided choice or ruling | `decided-by-agent`, `settled` |
| Owner-decided choice or ruling | `settled` |
| Any assumption | `assumed` |

Place one native `ex-choice` inside the item for a choice. The record supplies the question, anchor and option titles. Put each option's Markdown in a `template slot="summary"`. Add a `template slot="details"` with HTML only when it adds something to the summary. An `ex-markdown` inside Details can render richer Markdown. Put shared explanation or a diagram in `template data-record-body`. Use direct authored prose or rich content inside a ruling or assumption item.

```html
<ex-section id="open" title="For you to decide" holds="needs-owner"></ex-section>
<ex-section id="agent" title="Decided without you" holds="decided-by-agent"></ex-section>
<ex-section id="settled" title="You decided" holds="settled"></ex-section>
<ex-section id="assumptions" title="Assumed" holds="assumed"></ex-section>
<ex-section-item record="D2">
  <ex-choice>
    <ex-option data-option="hide-at-once">
      <template slot="summary">The row disappears when it expires.</template>
    </ex-option>
    <ex-option data-option="grey-until-scroll">
      <template slot="summary">The row stays, greyed, until the reader scrolls.</template>
      <template slot="details"><p>Moves DEL-02 into the core plan.</p></template>
    </ex-option>
  </ex-choice>
</ex-section-item>
<ex-section-item record="D3"><p>The test author can search for the mark in the fixture list.</p></ex-section-item>
<ex-section-item record="A1"><p>The restart must fall between two alarm marks.</p></ex-section-item>
```

The library derives the destination section, record and option anchors, state and weight chips, chosen and recommended markers, and the visible decision date. Author the decision, evidence, record links and explanatory content once. See [Sections and Map](stories.md) for the section and navigation rules.

To show the state next to another paragraph, use an empty `<span data-design-status="D2"></span>`. It needs the record's existing page placement and becomes a linked status chip.

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
