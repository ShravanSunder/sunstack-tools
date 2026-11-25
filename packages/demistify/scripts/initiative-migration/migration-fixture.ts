import { mkdir, mkdtemp, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';

/**
 * A small initiative for move tests. alpha holds three core deliverables and one
 * later one; beta already uses R1, R3, DEL-01 and DEL-03, so arrivals collide.
 */
const head = `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Page</title>
<link rel="stylesheet" href="https://demistify.getagentstudio.dev/v1/theme.css">
<script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script></head><body>
<ex-nav for="content" src="./navigation.generated.json"></ex-nav>`;

const checkCases = `cases: [
      { given: 'one', expect: 'first' },
      { given: 'two', expect: 'second' },
      { given: 'three', expect: 'third' },
    ]`;

const files: Readonly<Record<string, string>> = {
	'initiative.ts': `import alpha from './projects/alpha/design.ts';
import beta from './projects/beta/design.ts';

export default {
  projects: {
    alpha: { design: alpha, number: 1 },
    beta: { design: beta, number: 2 },
  },
};
`,
	'projects/alpha/design.ts': `import { defineCheck, defineDesign, defineNeed, defineRealization, defineRule } from './design-sdk.generated.ts';

export default defineDesign({
  pages: [
    { id: 'home', title: 'Alpha', href: 'index.html', icon: 'file' },
    { id: 'other', title: 'Alpha notes', href: 'other.html', icon: 'file' },
  ],
  mapGroups: [{ description: 'Expiry', icon: 'clock', target: { record: 'R1' } }],
  acceptances: [{ deliverableId: 'DEL-01', build: 'build-1', evidence: 'index.html#DEL-01' }],
  records: [
    defineNeed({ id: 'N1', statement: 'A reader sees only live rows.' }),
    defineNeed({ id: 'N2', statement: 'A reader can trust old rows.' }),
    // The rule the first deliverable builds.
    defineRule({ id: 'R1', statement: 'A row expires at its deadline.', serves: 'N1', governedBy: ['D1'] }),
    defineRule({ id: 'R2', statement: 'An expired row is hidden.', serves: 'N1' }),
    defineRule({ id: 'R3', statement: 'An old row keeps its stamp.', serves: 'N2', reliesOn: ['A1'] }),
    defineRealization({ id: 'M1', statement: 'The row stores its deadline.', realizes: 'R1' }),
    defineCheck({ id: 'V1', statement: 'An expired row is gone.', checks: 'R1', journey: 'expiry test', unverified: 'clock skew', ${checkCases} }),
    defineCheck({ id: 'V2', statement: 'A hidden row stays hidden.', checks: 'R2', journey: 'hide test', unverified: 'none', ${checkCases} }),
    defineCheck({ id: 'V3', statement: 'The stamp survives.', checks: 'R3', journey: 'stamp test', unverified: 'none', ${checkCases} }),
  ],
});
`,
	'projects/alpha/design-decisions.ts': `import { defineRuling } from './design-sdk.generated.ts';

export default [
  defineRuling({
    id: 'D1',
    statement: 'Deadlines come from central.',
    sources: [{ href: 'index.html#DEL-01', excerpt: 'Expire' }],
    authority: {
      state: 'agent-decided',
      by: { alias: 'alpha-lead', role: 'lead' },
      weight: 'design',
      rationale: 'Central owns time.',
      decidedOn: '2026-11-20',
      sourceKind: 'author',
    },
  }),
];
`,
	'projects/alpha/design-assumptions.ts': `import { defineAssumption } from './design-sdk.generated.ts';

export default [
  defineAssumption({
    id: 'A1',
    statement: 'Stamps are monotonic.',
    by: { alias: 'alpha-lead', role: 'lead' },
    weight: 'design',
    status: 'open',
    settles: 'Read the stamp source.',
  }),
];
`,
	'projects/alpha/design-delivery.ts': `import design from './design.ts';
import { defineDelivery } from './design-sdk.generated.ts';
import expire from './deliverables/del-01-expire.ts';
import hide from './deliverables/del-02-hide.ts';
import stamp from './deliverables/del-03-stamp.ts';
import someday from './deliverables/del-04-someday.ts';

export default defineDelivery(design, [
  { deliverable: expire },
  { deliverable: hide, dependsOn: ['DEL-01'] },
  { deliverable: stamp },
  { deliverable: someday },
]);
`,
	'projects/alpha/deliverables/del-01-expire.ts': `import { defineDeliverable } from '../design-sdk.generated.ts';

export default defineDeliverable({
  kind: 'core',
  id: 'DEL-01',
  title: 'Expire rows',
  destination: { page: 'index.html', fragment: 'DEL-01' },
  scope: { rules: ['R1'], designItems: ['M1'], implementationTasks: ['store'], verificationChecks: ['V1'] },
  implementation: { completedTasks: ['store'], candidate: 'build-1', evidence: ['index.html#DEL-01'] },
  verification: {
    build: 'build-1',
    outcomes: { V1: { kind: 'passed', observation: 'Gone.', evidence: 'other.html#notes' } },
  },
});
`,
	'projects/alpha/deliverables/del-02-hide.ts': `import { defineDeliverable } from '../design-sdk.generated.ts';

export default defineDeliverable({
  kind: 'core',
  id: 'DEL-02',
  title: 'Hide rows',
  destination: { page: 'index.html', fragment: 'DEL-02' },
  scope: { rules: ['R2'], implementationTasks: ['hide'], verificationChecks: ['V2'] },
});
`,
	'projects/alpha/deliverables/del-03-stamp.ts': `import { defineDeliverable } from '../design-sdk.generated.ts';

export default defineDeliverable({
  kind: 'core',
  id: 'DEL-03',
  title: 'Keep stamps',
  destination: { page: 'index.html', fragment: 'DEL-03' },
  scope: { rules: ['R3'], implementationTasks: ['stamp'], verificationChecks: ['V3'] },
});
`,
	'projects/alpha/deliverables/del-04-someday.ts': `import { defineDeliverable } from '../design-sdk.generated.ts';

export default defineDeliverable({ kind: 'later', id: 'DEL-04', title: 'Someday' });
`,
	'projects/alpha/index.html': `${head}
<ex-document id="content">
  <h1>Alpha</h1>
  <h2 id="needs">Needs</h2>
  <p data-design-record="N1"></p>
  <p data-design-record="N2"></p>
  <h2 id="DEL-01">DEL-01 · Expire rows</h2>
  <p data-design-record="R1"></p>
  <p>It hides with <a href="#R2">R2</a> and keeps <a href="#DEL-03">stamps</a>.</p>
  <h3 id="expire-check">Checking it</h3>
  <p data-design-record="V1"></p>
  <p data-design-record="M1"></p>
  <h2 id="DEL-02">DEL-02 · Hide rows</h2>
  <p data-design-record="R2"></p>
  <p data-design-record="V2"></p>
  <section id="stamps">
  <h2 id="DEL-03">DEL-03 · Keep stamps</h2>
  <p data-design-record="R3"></p>
  <p data-design-record="V3"></p>
  </section>
  <ex-section id="agent" title="Decided without you" holds="decided-by-agent"></ex-section>
  <ex-section id="settled" title="You decided" holds="settled"></ex-section>
  <ex-section id="assumed" title="Assumed" holds="assumed"></ex-section>
  <ex-section-item record="D1"><p>Central's clock is the only clock.</p></ex-section-item>
  <ex-section-item record="A1"><p>We read it once.</p></ex-section-item>
  <ex-deliverables src="./deliverables.generated.json"></ex-deliverables>
</ex-document></body></html>
`,
	'projects/alpha/other.html': `${head}
<ex-document id="content">
  <h1>Alpha notes</h1>
  <p id="notes">See <a href="index.html#R1">the expiry rule</a> and <a href="index.html#R3">the stamp rule</a> (<a href="index.html#stamps">all of it</a>).</p>
</ex-document></body></html>
`,
	'projects/beta/design.ts': `import { defineCheck, defineDesign, defineNeed, defineRule } from './design-sdk.generated.ts';

export default defineDesign({
  pages: [{ id: 'home', title: 'Beta', href: 'index.html', icon: 'file' }],
  mapGroups: [],
  records: [
    defineNeed({ id: 'N1', statement: 'A sender learns the outcome.' }),
    defineRule({ id: 'R1', statement: 'Each send gets one outcome.', serves: 'N1' }),
    defineRule({ id: 'R3', statement: 'An outcome names its send.', serves: 'N1' }),
    defineCheck({ id: 'V1', statement: 'Outcomes match sends.', checks: ['R1', 'R3'], journey: 'outcome test', unverified: 'none', ${checkCases} }),
  ],
});
`,
	'projects/beta/design-delivery.ts': `import design from './design.ts';
import { defineDelivery } from './design-sdk.generated.ts';
import outcomes from './deliverables/del-01-outcomes.ts';
import later from './deliverables/del-03-later.ts';

export default defineDelivery(design, [
  { deliverable: outcomes, dependsOn: ['alpha/DEL-02'] },
  { deliverable: later },
]);
`,
	'projects/beta/deliverables/del-01-outcomes.ts': `import { defineDeliverable } from '../design-sdk.generated.ts';

export default defineDeliverable({
  kind: 'core',
  id: 'DEL-01',
  title: 'Outcomes',
  destination: { page: 'index.html', fragment: 'DEL-01' },
  scope: { rules: ['R1', 'R3'], implementationTasks: ['outcome'], verificationChecks: ['V1'] },
});
`,
	'projects/beta/deliverables/del-03-later.ts': `import { defineDeliverable } from '../design-sdk.generated.ts';

export default defineDeliverable({ kind: 'later', id: 'DEL-03', title: 'Later outcomes' });
`,
	'projects/beta/index.html': `${head}
<ex-document id="content">
  <h1>Beta</h1>
  <h2 id="DEL-01">DEL-01 · Outcomes</h2>
  <p data-design-record="N1"></p>
  <p data-design-record="R1"></p>
  <p data-design-record="R3"></p>
  <p data-design-record="V1"></p>
  <p>The stamp rule is <a href="../alpha/index.html#R3">alpha's R3</a>.</p>
  <ex-deliverables src="./deliverables.generated.json"></ex-deliverables>
</ex-document></body></html>
`,
};

export const fixturePlan = {
	projects: [
		{ key: 'gamma', folder: 'gamma', number: 1, title: 'Gamma', achieves: 'Rows expire.' },
		{ key: 'alpha', folder: 'alpha', number: 1, title: 'Alpha' },
		{ key: 'beta', folder: 'beta', number: 2, title: 'Beta' },
	],
	moves: [
		{ from: 'alpha/DEL-01', to: 'gamma' },
		{ from: 'alpha/DEL-03', to: 'beta' },
	],
	add: [{ project: 'gamma', title: 'Grey out rows', dependsOn: ['alpha/DEL-01'], why: 'asked' }],
	withdraw: ['alpha/DEL-04'],
	dependsOnChanges: [{ deliverable: 'beta/DEL-01', from: ['alpha/DEL-02'], to: ['alpha/DEL-01'] }],
};

/**
 * The same tree in the 1.33 layout: project.ts with designs, a shared decision in
 * project-decisions.ts, named project/D9 by a rule and placed on alpha's page.
 */
const legacyFiles: Readonly<Record<string, string | undefined>> = {
	'initiative.ts': undefined,
	'project.ts': `import alpha from './projects/alpha/design.ts';
import beta from './projects/beta/design.ts';
import decisions from './project-decisions.ts';

export default { designs: { alpha, beta }, decisions };
`,
	'project-decisions.ts': `import type { DesignDecision } from './projects/alpha/design-schema.d.ts';

export default [
  {
    id: 'D9',
    kind: 'ruling',
    statement: 'Rows are hidden, never deleted.',
    authority: {
      state: 'agent-decided',
      by: { alias: 'alpha-lead', role: 'lead' },
      weight: 'design',
      rationale: 'History stays readable.',
      decidedOn: '2026-11-20',
      sourceKind: 'author',
    },
  },
] as const satisfies readonly DesignDecision[];
`,
	'projects/alpha/design.ts': files['projects/alpha/design.ts']?.replace(
		"serves: 'N1' }),\n    defineRule({ id: 'R3'",
		"serves: 'N1', governedBy: ['project/D9'] }),\n    defineRule({ id: 'R3'",
	),
	'projects/alpha/index.html': files['projects/alpha/index.html']?.replace(
		'<ex-section-item record="A1">',
		'<ex-section-item record="project/D9"><p>Kept for history.</p></ex-section-item>\n  <ex-section-item record="A1">',
	),
};

async function writeTree(tree: Readonly<Record<string, string | undefined>>): Promise<string> {
	const root = await mkdtemp(join(tmpdir(), 'demistify-migration-'));
	const initiative = join(root, 'initiative');
	await Promise.all(
		Object.entries(tree).map(async ([path, text]) => {
			if (text === undefined) return;
			const file = join(initiative, path);
			await mkdir(dirname(file), { recursive: true });
			await writeFile(file, text);
		}),
	);
	return initiative;
}

/** Write the fixture into a fresh directory; returns the initiative root. */
export function writeMigrationFixture(): Promise<string> {
	return writeTree(files);
}

/** The fixture in the project.ts layout. */
export function writeLegacyMigrationFixture(): Promise<string> {
	return writeTree({ ...files, ...legacyFiles });
}
