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
  pages: ['decisions.html'],
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

The check reads three kinds of page: each project's pages declared in its `design.ts`, the initiative's `index.html`, and the initiative pages listed in `pages`, relative to `initiative.ts`. Any of them may place `initiative/` records and other projects' records by qualified ID; an initiative page has no namespace of its own, so every ID on it is qualified. Give each initiative page `<meta name="demistify-records" content="records.generated.js">` with the path to the initiative folder. The check warns about any HTML page under the initiative that binds records while nothing registers it, because its records would stay empty.

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

`demistify check <page.html | folder …>` does the same for any mix of pages: pages and folders of the initiative run `initiative check` once, and standalone pages get the page check.

Run `design types` for each project after a fresh checkout so the editor and `agents-identity.ts` can resolve the generated SDK. `initiative check` is read-only. It lists the projects by number, any order warnings, open owner questions under "Waiting on owner", then each deliverable's state; `--write` saves the ignored generated views. `initiative status` prints the plan: each project by number with its state, what it achieves and what the owner checks, every deliverable with its state, tasks done, checks passed and what it waits on, then what is ready now, what blocks the most open work, and what is built but not accepted; `--json` gives the same as data. `brief DEL-01` run inside a project and `brief messages/DEL-01` run at the root select the same deliverable; a bare ID that matches in several projects returns the qualified choices.

## Record the owner's answers

When the owner answers open choices on a page and copies the review, save the copied text (or pipe it on stdin with `-`) and apply it:

```sh
demistify initiative answer review.md docs/initiatives/2026-11-router --site-root docs
demistify initiative answer review.md docs/initiatives/2026-11-router --site-root docs --write
```

The first run is a preview that writes nothing. With `--write`, each answer with a Decision ID settles its open choice:

- The record's `authority` becomes `owner-decided`, with `chosen` the selected option and `decision` its title, dated today (or `--decided-on YYYY-MM-DD`).
- When the owner commented on the question or the selected option, the comments are the owner's words. They go into a `<p id="D4-answer">` before `</ex-document>` on the page that places the record, and the record cites them verbatim with `sourceKind: 'owner-words'`.
- A bare selection is `owner-accepted`, citing the record's placement and the option title.

The item moves to its settled section on its own. The command then runs `initiative check --write`; if that check fails, every file it changed is restored. It refuses the whole review when a record is not an open choice, or when the review selects anything but one of its options. Answers without a Decision ID are skipped. Rewrite `decision` afterwards when the option title does not read as the answer.

## Add a decision or assumption

Scaffold a new record, its section item and the owner's words in one step:

```sh
demistify initiative add choice docs/initiatives/2026-11-router --namespace initiative --page docs/initiatives/2026-11-router/decisions.html --words "the owner's exact words" --write
demistify initiative add ruling docs/initiatives/2026-11-router --namespace messages --page docs/initiatives/2026-11-router/projects/2026-11-messages/index.html --author mac-lead --write
```

Without `--write` it previews. The command:

- **Takes the next free ID** in the namespace: `D` for choices and rulings, `A` for assumptions.
- **Appends the record** to the namespace's `decisions` or `assumptions` file: the one registered in `initiative.ts`, or a project's `design-decisions.ts` / `design-assumptions.ts`.
- **Places an `ex-section-item`** right after the page's section for the record's first state. A choice gets a native `ex-choice` with two options.
- **With `--words`, cites the owner verbatim** from a `<p id="<ref>-source">` before `</ex-document>`. A worded ruling is `owner-decided`; an agent's ruling and every assumption need `--author`.

Every placeholder reads `TODO:`. The scaffold passes the check as written, so fill each placeholder in place and run `initiative check` again. Like `answer`, it runs `initiative check --write` after writing and restores every file if that fails. It refuses a page without the record's first section. In a file written with record helpers, the new record is a helper call such as `defineRuling({ … })`, and the helper is added to the file's SDK import when missing.

For an initiative-wide view, place this in the initiative's `index.html` and serve it with `open`:

```html
<ex-initiative-overview src="./initiative.generated.json"></ex-initiative-overview>
```

The overview opens with projects grouped by number. Select a project to open its summary beside the overview, or in a sheet on a phone. The summary shows what the project achieves, what the owner checks, its state, open owner decisions, and deliverables. Select **Open design page** to visit the project’s page. The project groups are followed by **Work now**, initiative-wide owner questions, and other records. States and ID prefixes carry plain explanations. Use `ex-deliverables` on a project page to show its delivery evidence.

## Trees on an older layout

A tree still on 1.32.1 records, or on `project.ts`, is converted from a sunstack-tools source checkout, not with this CLI: first the 1.32.1 record migration from the 1.33.0 release commit (`git checkout a9e9a34c`, then `node packages/demistify/scripts/migrate-lean-design-records.ts --released-lean …` as that commit's guide describes), then `packages/demistify/scripts/initiative-migration/` at the 1.36.1 release commit (`git checkout da3363b6`), whose README gives the command. Run each with its dry run first.
