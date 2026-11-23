import { execFile } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { expect, it } from 'vitest';

import {
	parseDeliverablesView,
	type CoreDeliverableView,
	type DeliverablesView,
} from '../../core/deliverables-view.ts';

const run = promisify(execFile);
const cli = fileURLToPath(new URL('../../../bin/demistify.ts', import.meta.url));
function coreDelivery(view: DeliverablesView, id: string): CoreDeliverableView {
	const entry = view.deliverables[id];
	if (entry?.kind !== 'core') throw new Error(`Expected core deliverable ${id}.`);
	return entry;
}

const designSource = `import type { DesignCore } from './design-schema.d.ts';
export default {
  pages: [{ id: 'delivery', title: 'Queued-job cancellation', href: 'index.html', icon: 'file' }],
  mapGroups: [],
  records: [
    { id: 'U3', kind: 'need', title: 'Cancel queued work', statement: 'The caller can cancel queued work.' },
    { id: 'U4', kind: 'need', title: 'Avoid unwanted work', statement: 'Cancelled jobs must not consume worker capacity.' },
    { id: 'R3', kind: 'rule', title: 'Cancelled work never starts', statement: 'The worker never starts a cancelled job.',
      serves: ['U3', 'U4'] },
    { id: 'V3', kind: 'planned-check', title: 'Observe handler entry', statement: 'Observe handler entry after cancellation.', journey: 'Queue integration', unverified: 'Production traffic',
      checks: 'R3',
      cases: [{ given: 'Cancel first', expect: 'No handler entry' }, { given: 'Claim first', expect: 'Cancel is refused' },
        { given: 'Duplicate cancel', expect: 'The terminal state is retained' }] },
    { id: 'R12', kind: 'rule', title: 'See that the job already started', statement: 'Once claimed, cancellation reports already started.',
      serves: 'U3' },
    { id: 'V12', kind: 'planned-check', title: 'Observe the caller response', statement: 'Observe the caller response after claim.', journey: 'Caller integration', unverified: 'Production traffic',
      checks: 'R12',
      cases: [{ given: 'Claim then cancel', expect: 'Already started' }, { given: 'Cancel then claim', expect: 'Cancelled' },
        { given: 'Early success with no caller response', expect: 'Reject the build' }] },
  ],
  /* Lead acceptance */
} as const satisfies DesignCore;
`;
const rootSource = `import design from './design.ts';
import { defineDelivery } from './design-sdk.generated.ts';
import preventJob from './deliverables/del-01-prevent-cancelled-job-from-running.ts';
import reportClaim from './deliverables/del-02-report-job-already-started.ts';
export default defineDelivery(design, [
  { deliverable: preventJob },
  { deliverable: reportClaim, dependsOn: ['DEL-01'] },
]);
`;
const complete = `import { defineDeliverable } from '../design-sdk.generated.ts';
export default defineDeliverable({
  kind: 'core',
  id: 'DEL-01',
  title: 'Prevent cancelled jobs from running',
  destination: { page: 'index.html', fragment: 'DEL-01' },
  scope: { rules: ['R3'], implementationTasks: ['guard-handler'], verificationChecks: ['V3'] },
  implementation: { completedTasks: ['guard-handler'], candidate: 'build-87', evidence: ['evidence.txt'] },
  verification: { build: 'build-87', outcomes: { V3: {
    kind: 'passed', observation: 'Zero handler entries', evidence: 'evidence.txt',
  } } },
});
`;
const blocked = `import { defineDeliverable } from '../design-sdk.generated.ts';
export default defineDeliverable({
  kind: 'core',
  id: 'DEL-02',
  title: 'Report when a job has already started',
  destination: { page: 'index.html', fragment: 'DEL-02' },
  scope: { rules: ['R12'], implementationTasks: ['report-claim'], verificationChecks: ['V12'] },
  implementation: { completedTasks: ['report-claim'], candidate: 'build-87', evidence: ['evidence.txt'] },
  verification: { build: 'build-87', outcomes: { V12: { kind: 'blocked', reason: 'Caller response unavailable' } } },
});
`;
const page = `<!doctype html><html lang="en"><head><title>Queued-job cancellation</title>
<link rel="stylesheet" href="https://demistify.getagentstudio.dev/v1/theme.css">
<script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script></head>
<body><ex-nav for="content" src="./navigation.generated.json"></ex-nav><ex-document id="content">
<h1>Queued-job cancellation</h1>
<ex-deliverables src="./deliverables.generated.json"></ex-deliverables>
<p data-design-record="U3"></p><p data-design-record="R3"></p>
<p data-design-record="U4"></p>
<p data-design-record="V3"></p><p id="DEL-01">Prevent cancelled jobs from running.</p>
<p data-design-record="R12"></p><p data-design-record="V12"></p>
<p id="DEL-02">Report when a job has already started.</p>
</ex-document></body></html>`;
const outputNames = [
	'design.generated.json',
	'navigation.generated.json',
	'deliverables.generated.json',
];
const realizationNote = 'R3: No realization names this rule.';
const unservedNeedNote = 'U6: No rule serves this need.';

function beforeVerification(source: string): string {
	return `${source.slice(0, source.indexOf('  verification:'))}});\n`;
}

it('holds acceptance for an explicitly scoped need and keeps missing realizations informational', async (): Promise<void> => {
	const directory = await mkdtemp(join(tmpdir(), 'demistify-need-coverage-'));
	try {
		const deliverables = join(directory, 'deliverables');
		const designFile = join(directory, 'design.ts');
		const coreFile = join(deliverables, 'del-01-prevent-cancelled-job-from-running.ts');
		const authored = designSource
			.replace(
				'  records: [',
				`  records: [
    { id: 'U5', kind: 'need', statement: 'Operators know whether a job has started.' },
						{ id: 'U6', kind: 'need', statement: 'An unscoped need has an informational note.' },
    { id: 'M12', kind: 'realization', statement: 'A committed claim is reported.', realizes: 'R12' },`,
			)
			.replace("serves: 'U3' },", "serves: ['U3', 'U5'] },")
			.replace(
				'/* Lead acceptance */',
				`acceptances: [{ deliverableId: 'DEL-01', build: 'build-87', evidence: 'evidence.txt' }],`,
			);
		const core = complete.replace('scope: { rules:', "scope: { needs: ['U5'], rules:");
		await mkdir(deliverables);
		await Promise.all([
			writeFile(designFile, authored),
			writeFile(coreFile, core),
			writeFile(join(directory, 'index.html'), page),
			writeFile(join(directory, 'evidence.txt'), 'The fixture exercised handler entry.'),
			writeFile(join(deliverables, 'del-02-report-job-already-started.ts'), blocked),
			writeFile(
				join(deliverables, 'later.ts'),
				`import { defineDeliverable } from '../design-sdk.generated.ts';
export default defineDeliverable({ kind: 'later', id: 'DEL-03', title: 'Review further work' });`,
			),
			writeFile(
				join(directory, 'design-delivery.ts'),
				rootSource
					.replace(
						'export default defineDelivery',
						"import later from './deliverables/later.ts';\nexport default defineDelivery",
					)
					.replace(
						"{ deliverable: reportClaim, dependsOn: ['DEL-01'] },",
						'{ deliverable: reportClaim },\n  { deliverable: later },',
					),
			),
		]);
		const check = [cli, 'design', 'check', directory, '--write'];
		const brief = [cli, 'brief', 'DEL-01', directory];
		const initial = (await run(process.execPath, check)).stdout;
		expect(initial).toContain('DEL-01 · awaiting-verification');
		expect(initial).toContain('Link scoped rules to scoped needs: U5.');
		expect(initial).toContain('DEL-02 · verification-blocked');
		expect(initial).toContain('DEL-03 · later');
		expect(initial).toContain(`Design notes\n  ${unservedNeedNote}`);
		expect(initial.split(unservedNeedNote)).toHaveLength(2);
		expect(initial).not.toMatch(/(?:U3|U4|U5): No rule serves|R12: No realization/);
		expect(initial.split(realizationNote)).toHaveLength(2);
		const initialBrief = (await run(process.execPath, brief)).stdout;
		expect(initialBrief).toContain('**U5** Operators know whether a job has started.');
		expect(initialBrief).toContain('Link scoped rules to scoped needs: U5.');
		expect(initialBrief).toContain(realizationNote);
		expect(initialBrief).not.toMatch(/U6|R12|V12|DEL-03/);

		const coveredDesign = authored.replace("serves: ['U3', 'U4']", "serves: ['U3', 'U4', 'U5']");
		await writeFile(designFile, coveredDesign);
		const covered = (await run(process.execPath, check)).stdout;
		expect(covered).toContain('DEL-01 · accepted');
		expect(covered).toContain(unservedNeedNote);
		expect(covered).toContain(realizationNote);
		expect(covered).not.toContain('Link scoped rules to scoped needs: U5.');
		const view = await readFile(join(directory, 'deliverables.generated.json'), 'utf8');
		expect(view).not.toMatch(/No realization|No rule serves|U6/);
		const accepted = coreDelivery(parseDeliverablesView(view), 'DEL-01');
		expect(accepted.state).toBe('accepted');
		expect(accepted.reasons).toEqual(['The Lead accepted candidate build-87.']);
		const coveredBrief = (await run(process.execPath, brief)).stdout;
		expect(coveredBrief).toContain(realizationNote);
		expect(coveredBrief).not.toMatch(/U6|No rule serves/);
		expect(coveredBrief).not.toContain('Link scoped rules to scoped needs: U5.');

		await writeFile(coreFile, core.replace("needs: ['U5']", "needs: ['R3']"));
		await expect(run(process.execPath, check)).rejects.toThrow(
			/R3 must resolve to a design record of kind need/,
		);
		await writeFile(coreFile, core.replace("needs: ['U5']", "needs: ['U5', 'U5']"));
		await expect(run(process.execPath, check)).rejects.toThrow(/duplicate need U5/);

		const withRealization = coveredDesign.replace(
			'  records: [',
			`  records: [
    { id: 'M3', kind: 'realization', statement: 'The worker reads cancellation before handler entry.', realizes: 'R3' },`,
		);
		const coreWithRealization = core.replace("rules: ['R3']", "rules: ['R3'], designItems: ['M3']");
		await Promise.all([
			writeFile(designFile, withRealization),
			writeFile(coreFile, coreWithRealization),
			writeFile(join(directory, 'index.html'), page.replace(/<link rel="stylesheet"[^>]+>/, '')),
		]);
		const realizationBrief = (await run(process.execPath, brief)).stdout;
		expect(realizationBrief).toContain(
			'## Realizations\n\n- **M3** The worker reads cancellation before handler entry.',
		);
		expect(realizationBrief).not.toMatch(/M12|No realization|stylesheet/);
		await expect(run(process.execPath, check)).rejects.toThrow(/theme\.css/);
		const invalidTarget = (target: string): string =>
			withRealization.replace(
				'mapGroups: [],',
				`mapGroups: [{ title: 'Review', description: 'Review the scope', icon: 'file', target: ${target} }],`,
			);
		await writeFile(designFile, invalidTarget("{ page: 'missing-page' }"));
		await expect(run(process.execPath, brief)).rejects.toThrow(/Unknown page id: missing-page/);
		await writeFile(
			designFile,
			withRealization.replace(
				'mapGroups: [],',
				`mapGroups: [], walkthroughs: [{ id: 'review', title: 'Review', description: 'Review the scope',
steps: [{ narration: 'Review the rule', target: { record: 'R-NOT-THERE' as 'R3' } }] }],`,
			),
		);
		await expect(run(process.execPath, brief)).rejects.toThrow(/unknown design record R-NOT-THERE/);
		await writeFile(designFile, withRealization);
		await writeFile(
			coreFile,
			coreWithRealization.replace("designItems: ['M3']", "designItems: ['M-BAD']"),
		);
		await expect(run(process.execPath, brief)).rejects.toThrow(
			/M-BAD must resolve to a design record of kind realization/,
		);
	} finally {
		await rm(directory, { recursive: true, force: true });
	}
}, 30_000);

it('composes typed work files into one delivery graph and keeps the last output if inputs are invalid', async (): Promise<void> => {
	const directory = await mkdtemp(join(tmpdir(), 'demistify-delivery-check-'));
	try {
		const graphFile = join(directory, 'design-delivery.ts');
		const deliverables = join(directory, 'deliverables');
		const completeFile = join(deliverables, 'del-01-prevent-cancelled-job-from-running.ts');
		const blockedFile = join(deliverables, 'del-02-report-job-already-started.ts');
		await run(process.execPath, [cli, 'design', 'init', directory]);
		await mkdir(deliverables);
		await Promise.all([
			writeFile(join(directory, 'design.ts'), designSource),
			writeFile(graphFile, rootSource),
			writeFile(join(directory, 'index.html'), page),
			writeFile(join(directory, 'evidence.txt'), 'Cancel queued work. Zero handler entries.'),
			writeFile(join(directory, 'acceptance.txt'), 'Lead inspected current cancellation evidence.'),
			writeFile(completeFile, beforeVerification(complete)),
			writeFile(blockedFile, beforeVerification(blocked)),
		]);
		const check = [cli, 'design', 'check', directory, '--write'];
		await run(process.execPath, check);
		const blockedOnScope = blocked;
		const acceptedDesign = designSource.replace(
			'/* Lead acceptance */',
			`acceptances: [{ deliverableId: 'DEL-01', build: 'build-87', evidence: 'acceptance.txt' }],`,
		);
		await Promise.all([
			writeFile(completeFile, complete),
			writeFile(blockedFile, blockedOnScope),
			writeFile(join(directory, 'design.ts'), acceptedDesign),
		]);
		const checked = await run(process.execPath, check);
		const readOutputs = (): Promise<string[]> =>
			Promise.all(outputNames.map((file) => readFile(join(directory, file), 'utf8')));
		const produced = await readOutputs();
		expect(checked.stdout).toContain('DEL-02 · verification-blocked');
		expect(checked.stdout).toContain('V12 is blocked. Caller response unavailable');
		expect(checked.stdout).not.toMatch(/Last checked snapshot|Scope revision:/);
		expect(JSON.parse(produced[2] ?? '')).toMatchObject({
			version: 1,
			deliverables: {
				'DEL-01': { state: 'accepted', progress: { acceptance: { evidence: 'acceptance.txt' } } },
				'DEL-02': {
					title: 'Report when a job has already started',
					state: 'verification-blocked',
					dependsOn: ['DEL-01'],
					links: { plan: { page: 'index.html', fragment: 'DEL-02' } },
					verification: {
						checks: [{ id: 'V12', state: 'blocked', reason: 'Caller response unavailable' }],
					},
				},
			},
		});

		const reorderedDesign = acceptedDesign.replace("['U3', 'U4']", "['U4', 'U3']");
		expect(reorderedDesign).not.toBe(acceptedDesign);
		await writeFile(join(directory, 'design.ts'), reorderedDesign);
		await run(process.execPath, check);
		const [, , reorderedDelivery] = await readOutputs();
		expect(reorderedDelivery).toBe(produced[2]);
		await writeFile(join(directory, 'design.ts'), acceptedDesign);
		await run(process.execPath, check);

		await writeFile(
			join(directory, 'old-delivery.json'),
			JSON.stringify({ version: 1, deliverables: {} }),
		);
		await writeFile(
			join(directory, 'index.html'),
			page.replace('deliverables.generated.json', 'old-delivery.json'),
		);
		await expect(run(process.execPath, check)).rejects.toThrow(
			/ex-deliverables src .*deliverables\.generated\.json/,
		);
		expect(await readOutputs()).toEqual(produced);
		await writeFile(join(directory, 'index.html'), page);

		const completeOnScope = complete;
		await writeFile(completeFile, completeOnScope.replaceAll('V3', 'V12'));
		await expect(run(process.execPath, check)).rejects.toThrow(/V12[^\n]*(?:scope|rule|cover)/i);
		expect(await readOutputs()).toEqual(produced);
		await writeFile(completeFile, completeOnScope);

		await writeFile(
			graphFile,
			rootSource.replace(
				'{ deliverable: preventJob }',
				"{ deliverable: preventJob, dependsOn: ['DEL-02'] }",
			),
		);
		await expect(run(process.execPath, check)).rejects.toThrow(/cycle/i);
		expect(await readOutputs()).toEqual(produced);
		await writeFile(
			graphFile,
			rootSource.replace("dependsOn: ['DEL-01']", "dependsOn: ['DEL-99']"),
		);
		await expect(run(process.execPath, check)).rejects.toThrow(/DEL-99/);
		expect(await readOutputs()).toEqual(produced);
		await writeFile(graphFile, rootSource);
		await writeFile(
			blockedFile,
			blockedOnScope.replace('outcomes: { V12:', 'outcomes: { Unassigned:'),
		);
		await expect(run(process.execPath, check)).rejects.toThrow(/Unassigned/);
		expect(await readOutputs()).toEqual(produced);
		await writeFile(
			blockedFile,
			blockedOnScope.replace("candidate: 'build-87'", "candidate: 'build-88'"),
		);
		await run(process.execPath, check);
		const [, , stale] = await readOutputs();
		expect(JSON.parse(stale ?? '')).toMatchObject({
			deliverables: {
				'DEL-02': {
					state: 'awaiting-verification',
					verification: { checks: [{ id: 'V12', state: 'stale' }] },
				},
			},
		});
		await writeFile(blockedFile, blockedOnScope);
		await writeFile(
			join(directory, 'design.ts'),
			acceptedDesign.replace(
				'The worker never starts a cancelled job.',
				'A cancelled job never enters a worker.',
			),
		);
		await run(process.execPath, check);
		const [, , changed] = await readOutputs();
		expect(JSON.parse(changed ?? '')).toMatchObject({
			deliverables: {
				'DEL-01': { state: 'accepted' },
				'DEL-02': { state: 'verification-blocked' },
			},
		});
		const brief = await run(process.execPath, [cli, 'brief', 'DEL-01', directory]);
		expect(brief.stdout).toContain('A cancelled job never enters a worker.');
		expect(brief.stdout).toContain('Observation: Queue integration');
		expect(brief.stdout).toContain('Unverified: Production traffic');
		expect(brief.stdout).toContain('guard-handler');
		expect(brief.stdout).not.toMatch(/R12|V12|report-claim/);
		const withHistoricalCheck = beforeVerification(complete)
			.replace("verificationChecks: ['V3']", 'verificationChecks: []')
			.replace(
				'});',
				`  verificationHistory: [{ build: 'build-87', outcomes: {
    V3: { kind: 'passed', observation: 'Zero handler entries', evidence: 'evidence.txt' },
  } }],
});`,
			);
		await writeFile(completeFile, withHistoricalCheck);
		await run(process.execPath, check);
		const [, , archived] = await readOutputs();
		expect(JSON.parse(archived ?? '')).toMatchObject({
			deliverables: {
				'DEL-01': {
					state: 'awaiting-verification',
					verification: {
						checks: [],
						history: [{ build: 'build-87', outcomes: { ['V3']: { evidence: 'evidence.txt' } } }],
					},
				},
			},
		});
	} finally {
		await rm(directory, { recursive: true, force: true });
	}
}, 90_000);

function briefRecordIds(markdown: string): (string | undefined)[] {
	return [...markdown.matchAll(/^- \*\*([^*]+)\*\*/gm)].map((match) => match[1]);
}

it('keeps the shared check gates in each brief and scopes rules and acceptance to each deliverable', async (): Promise<void> => {
	const directory = await mkdtemp(join(tmpdir(), 'demistify-shared-check-'));
	try {
		await mkdir(join(directory, 'deliverables'));
		await Promise.all([
			writeFile(
				join(directory, 'design.ts'),
				designSource
					.replace("serves: 'U3' },", "serves: 'U3', governedBy: ['D1'] },")
					.replace("checks: 'R3',", "checks: ['R3', 'R12'], reliesOn: ['A1'],")
					.replace(
						'/* Lead acceptance */',
						`acceptances: [{ deliverableId: 'DEL-01', build: 'build-87', evidence: 'evidence.txt' }],`,
					),
			),
			writeFile(join(directory, 'index.html'), page),
			writeFile(
				join(directory, 'design-delivery.ts'),
				rootSource.replace(", dependsOn: ['DEL-01']", ''),
			),
			writeFile(
				join(directory, 'deliverables/del-01-prevent-cancelled-job-from-running.ts'),
				complete,
			),
			writeFile(
				join(directory, 'deliverables/del-02-report-job-already-started.ts'),
				blocked.replaceAll('V12', 'V3'),
			),
			writeFile(
				join(directory, 'design-decisions.ts'),
				`export default [{ kind: 'choice', id: 'D1', authority: {
				state: 'owner-open', owner: 'owner', question: 'What should a claimed job report?' } }];`,
			),
			writeFile(
				join(directory, 'design-assumptions.ts'),
				`export default [{ kind: 'assumption', id: 'A1', status: 'open', statement: 'The queue fixture supports both cases.' }];`,
			),
			writeFile(join(directory, 'evidence.txt'), 'The fixture exercised both rules.'),
		]);
		const brief = async (id: string): Promise<string> =>
			(await run(process.execPath, [cli, 'brief', id, directory])).stdout;
		const first = await brief('DEL-01');
		const second = await brief('DEL-02');

		expect(briefRecordIds(first)).toEqual(['R3', 'V3', 'A1']);
		expect(briefRecordIds(second)).toEqual(['R12', 'V3', 'D1', 'A1']);

		const check = [cli, 'design', 'check', directory, '--write'];
		await run(process.execPath, check);
		const readView = async (): Promise<DeliverablesView> =>
			parseDeliverablesView(await readFile(join(directory, 'deliverables.generated.json'), 'utf8'));
		const open = await readView();
		expect(coreDelivery(open, 'DEL-01')).toMatchObject({
			state: 'design-blocked',
			reasons: ['Confirm assumption A1 before acceptance.'],
		});
		expect(coreDelivery(open, 'DEL-02').reasons).toContain('D1 requires an owner decision.');
		await writeFile(
			join(directory, 'design-assumptions.ts'),
			`export default [{ kind: 'assumption', id: 'A1', status: 'confirmed', statement: 'The queue fixture supports both cases.', resolution: { href: 'evidence.txt', excerpt: 'Both rules ran.' } }];`,
		);
		await run(process.execPath, check);
		const confirmed = await readView();
		expect(confirmed.deliverables['DEL-01']?.state).toBe('accepted');
		expect(confirmed.deliverables['DEL-02']?.state).toBe('design-blocked');
	} finally {
		await rm(directory, { recursive: true, force: true });
	}
}, 30_000);

it('keeps scoped acceptance open until the owner answers and the governing assumption is confirmed', async (): Promise<void> => {
	const directory = await mkdtemp(join(tmpdir(), 'demistify-design-authority-'));
	try {
		const designFile = join(directory, 'design.ts');
		const decisionsFile = join(directory, 'design-decisions.ts');
		const assumptionsFile = join(directory, 'design-assumptions.ts');
		const pageFile = join(directory, 'index.html');
		const deliverableFile = join(
			directory,
			'deliverables',
			'del-01-prevent-cancelled-job-from-running.ts',
		);
		const check = [cli, 'design', 'check', directory, '--write'];
		const ownerQuestion =
			"{ state: 'owner-open', owner: 'owner', question: 'Which cancellation guarantee applies?' }";
		const decided =
			"{ state: 'owner-decided', owner: 'owner', sourceKind: 'owner-accepted', decision: 'Cancel before claim.', decidedOn: '2026-11-21' }";
		const pendingDecisions = `import type { DesignDecision } from './design-schema.d.ts';
export default [
    { id: 'D1', kind: 'choice', title: 'Cancellation guarantee', authority: ${ownerQuestion},
      sources: [{ href: 'evidence.txt', excerpt: 'Cancel before claim.' }] },
    { id: 'D2', kind: 'choice', title: 'Unrelated follow-up', authority: ${ownerQuestion},
      sources: [{ href: 'evidence.txt', excerpt: 'Cancel before claim.' }] },
] as const satisfies readonly DesignDecision[];
`;
		const pendingDesign = designSource.replace(
			"serves: ['U3', 'U4']",
			"serves: ['U3', 'U4'], governedBy: ['D1']",
		);
		const fixturePage = page.replace(
			'</ex-document>',
			'<p data-design-record="D1"></p><p data-design-record="D2"></p></ex-document>',
		);
		const readDelivery = async (): Promise<ReturnType<typeof parseDeliverablesView>> =>
			parseDeliverablesView(await readFile(join(directory, 'deliverables.generated.json'), 'utf8'));

		await run(process.execPath, [cli, 'design', 'init', directory]);
		await mkdir(join(directory, 'deliverables'));
		await Promise.all([
			writeFile(decisionsFile, pendingDecisions),
			writeFile(
				join(directory, 'design-delivery.ts'),
				rootSource.replace(/import reportClaim[^\n]+\n|  \{ deliverable: reportClaim[^\n]+\n/g, ''),
			),
			writeFile(pageFile, fixturePage),
			writeFile(
				join(directory, 'evidence.txt'),
				'Cancel before claim. Target backend supports atomic claims.',
			),
			writeFile(join(directory, 'acceptance.txt'), 'Lead reviewed build-87.'),
			writeFile(deliverableFile, beforeVerification(complete)),
		]);

		async function checkWithCurrentReceipts(
			source: string,
		): Promise<ReturnType<typeof parseDeliverablesView>> {
			await writeFile(designFile, source);
			await run(process.execPath, check);

			await Promise.all([
				writeFile(deliverableFile, complete),
				writeFile(
					designFile,
					source.replace(
						'/* Lead acceptance */',
						`acceptances: [{ deliverableId: 'DEL-01', build: 'build-87', evidence: 'acceptance.txt' }],`,
					),
				),
			]);
			await run(process.execPath, check);
			return readDelivery();
		}

		const unanswered = coreDelivery(await checkWithCurrentReceipts(pendingDesign), 'DEL-01');
		expect(unanswered?.state).toBe('design-blocked');
		expect(unanswered?.reasons).toContain('D1 requires an owner decision.');

		const decidedSource = pendingDecisions.replace(ownerQuestion, decided);
		const openAssumptions = `import type { DesignAssumption } from './design-schema.d.ts';
export default [
    { id: 'A1', kind: 'assumption', title: 'Atomic claims', status: 'open',
      statement: 'Claims are atomic on the target backend.',
      sources: [{ href: 'evidence.txt', excerpt: 'Target backend supports atomic claims.' }] },
] as const satisfies readonly DesignAssumption[];
`;
		const assumedDesign = pendingDesign.replace("checks: 'R3'", "checks: 'R3', reliesOn: ['A1']");
		await Promise.all([
			writeFile(decisionsFile, decidedSource),
			writeFile(assumptionsFile, openAssumptions),
			writeFile(designFile, assumedDesign),
		]);
		await run(process.execPath, check);
		const working = coreDelivery(await readDelivery(), 'DEL-01');
		expect(working.state).toBe('design-blocked');
		expect(working?.reasons).toContain('Confirm assumption A1 before acceptance.');
		const awaitingConfirmation = coreDelivery(
			await checkWithCurrentReceipts(assumedDesign),
			'DEL-01',
		);
		expect(awaitingConfirmation?.state).toBe('design-blocked');
		expect(awaitingConfirmation?.reasons).toContain('Confirm assumption A1 before acceptance.');
		expect(awaitingConfirmation?.links.governingItems).toContainEqual({
			id: 'A1',
			label: 'Atomic claims',
		});
		const refutedAssumptions = openAssumptions.replace(
			"status: 'open'",
			"status: 'refuted', resolution: { href: 'evidence.txt', excerpt: 'Two claims observed.' }",
		);
		await writeFile(assumptionsFile, refutedAssumptions);
		await run(process.execPath, check);
		const refuted = coreDelivery(await readDelivery(), 'DEL-01');
		expect(refuted?.state).toBe('design-blocked');
		expect(refuted?.reasons).toContain('Assumption A1 was refuted. Await Lead review.');
		await writeFile(
			assumptionsFile,
			openAssumptions.replace("status: 'open'", "status: 'confirmed'"),
		);
		await expect(run(process.execPath, check)).rejects.toThrow(/require a resolution source/);
		await writeFile(
			assumptionsFile,
			refutedAssumptions.replace("status: 'refuted'", "status: 'open'"),
		);
		await expect(run(process.execPath, check)).rejects.toThrow(/remove the resolution source/);
		const settledAssumptions = openAssumptions.replace(
			"status: 'open'",
			"status: 'confirmed', resolution: { href: 'evidence.txt', excerpt: 'Target backend supports atomic claims.' }",
		);
		await writeFile(assumptionsFile, settledAssumptions);
		const accepted = await checkWithCurrentReceipts(assumedDesign);
		expect(accepted.deliverables['DEL-01']?.state).toBe('accepted');

		await writeFile(
			decisionsFile,
			decidedSource.replace("decidedOn: '2026-11-21'", "decidedOn: '2026-02-30'"),
		);
		await expect(run(process.execPath, check)).rejects.toThrow(/decidedOn/);
		expect(await readDelivery()).toEqual(accepted);

		const leadDecision =
			"{ state: 'lead-decided', lead: 'Queue Lead', sourceKind: 'author', decision: 'Use existing claim operation.', rationale: 'Keeps the agreed behavior.', decidedOn: '2026-11-21' }";
		const leadDecisions = decidedSource.replace(decided, leadDecision);
		await writeFile(decisionsFile, leadDecisions);
		expect(coreDelivery(await checkWithCurrentReceipts(assumedDesign), 'DEL-01').state).toBe(
			'accepted',
		);
		await writeFile(
			decisionsFile,
			leadDecisions.replace("'lead-decided'", "'implementer-decided'"),
		);
		await expect(run(process.execPath, check)).rejects.toThrow(
			/Invalid discriminator value.*owner-decided.*lead-decided/,
		);
	} finally {
		await rm(directory, { recursive: true, force: true });
	}
}, 90_000);
