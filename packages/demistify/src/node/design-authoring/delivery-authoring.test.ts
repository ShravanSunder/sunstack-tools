import { execFile } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { expect, it } from 'vitest';

import { parseDeliverablesView } from '../../core/deliverables-view.ts';

const run = promisify(execFile);
const cli = fileURLToPath(new URL('../../../bin/demistify.ts', import.meta.url));
const designSource = `import type { DesignCore } from './design-schema.d.ts';
export default {
  pages: [{ id: 'delivery', title: 'Queued-job cancellation', href: 'index.html', icon: 'file' }],
  mapGroups: [],
  records: [
    { id: 'U3', kind: 'need', title: 'Cancel queued work', statement: 'The caller can cancel queued work.',
      destination: { page: 'index.html', fragment: 'U3' }, sources: [{ href: 'evidence.txt', excerpt: 'Cancel queued work.' }] },
    { id: 'U4', kind: 'need', title: 'Avoid unwanted work', statement: 'Cancelled jobs must not consume worker capacity.',
      destination: { page: 'index.html', fragment: 'U4' }, sources: [{ href: 'evidence.txt', excerpt: 'Zero handler entries.' }] },
    { id: 'R3', kind: 'rule', title: 'Cancelled work never starts', statement: 'The worker never starts a cancelled job.',
      destination: { page: 'index.html', fragment: 'R3' }, sources: [{ href: 'index.html#U3', excerpt: 'Cancel queued work.' }],
      relationships: [{ relation: 'serves', targetId: 'U3' }, { relation: 'serves', targetId: 'U4' }] },
    { id: 'V3', kind: 'planned-check', title: 'Observe handler entry', status: 'planned',
      destination: { page: 'index.html', fragment: 'V3' }, sources: [{ href: 'index.html#R3', excerpt: 'No handler entry.' }],
      relationships: [{ relation: 'checks', targetId: 'R3' }],
      cases: [{ given: 'Cancel first', expect: 'No handler entry' }, { given: 'Claim first', expect: 'Cancel is refused' },
        { given: 'Duplicate cancel', expect: 'The terminal state is retained' }] },
    { id: 'R12', kind: 'rule', title: 'See that the job already started', statement: 'Once claimed, cancellation reports already started.',
      destination: { page: 'index.html', fragment: 'R12' }, sources: [{ href: 'index.html#U3', excerpt: 'Cancel queued work.' }],
      relationships: [{ relation: 'serves', targetId: 'U3' }] },
    { id: 'V12', kind: 'planned-check', title: 'Observe the caller response', status: 'planned',
      destination: { page: 'index.html', fragment: 'V12' }, sources: [{ href: 'index.html#R12', excerpt: 'Already started.' }],
      relationships: [{ relation: 'checks', targetId: 'R12' }],
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
  id: 'DEL-01',
  title: 'Prevent cancelled jobs from running',
  destination: { page: 'index.html', fragment: 'DEL-01' },
  scope: { rules: ['R3'], implementationTasks: ['guard-handler'], verificationChecks: ['V3'] },
  implementation: { completedTasks: ['guard-handler'], candidate: 'build-87', evidence: ['evidence.txt'] },
  verification: { build: 'build-87', scopeRevision: 'DEL-01-scope', outcomes: { V3: {
    kind: 'passed', observation: 'Zero handler entries', evidence: 'evidence.txt',
  } } },
});
`;
const blocked = `import { defineDeliverable } from '../design-sdk.generated.ts';
export default defineDeliverable({
  id: 'DEL-02',
  title: 'Report when a job has already started',
  destination: { page: 'index.html', fragment: 'DEL-02' },
  scope: { rules: ['R12'], implementationTasks: ['report-claim'], verificationChecks: ['V12'] },
  implementation: { completedTasks: ['report-claim'], candidate: 'build-87', evidence: ['evidence.txt'] },
  verification: { build: 'build-87', scopeRevision: 'DEL-02-scope', outcomes: { V12: { kind: 'blocked', reason: 'Caller response unavailable' } } },
});
`;
const page = `<!doctype html><html lang="en"><head><title>Queued-job cancellation</title>
<link rel="stylesheet" href="https://demistify.getagentstudio.dev/v1/theme.css">
<script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script></head>
<body><ex-nav for="content" src="./navigation.generated.json"></ex-nav><ex-document id="content">
<h1>Queued-job cancellation</h1>
<ex-deliverables src="./deliverables.generated.json"></ex-deliverables>
<p id="U3">Cancel queued work.</p><p id="R3">No handler entry after cancellation.</p>
<p id="U4">Avoid unwanted work.</p>
<p id="V3">Observe handler entry.</p><p id="DEL-01">Prevent cancelled jobs from running.</p>
<p id="R12">Already started.</p><p id="V12">Observe the caller response.</p>
<p id="DEL-02">Report when a job has already started.</p>
</ex-document></body></html>`;
const outputNames = [
	'design.generated.json',
	'navigation.generated.json',
	'deliverables.generated.json',
];

function beforeVerification(source: string): string {
	return `${source.slice(0, source.indexOf('  verification:'))}});\n`;
}

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
		const check = [cli, 'design', 'check', directory];
		await run(process.execPath, check);
		const draft = parseDeliverablesView(
			await readFile(join(directory, 'deliverables.generated.json'), 'utf8'),
		);
		const firstRevision = draft.deliverables['DEL-01']?.scopeRevision ?? '';
		const secondRevision = draft.deliverables['DEL-02']?.scopeRevision ?? '';
		expect(firstRevision).toMatch(/^[a-f0-9]{64}$/);
		expect(secondRevision).toMatch(/^[a-f0-9]{64}$/);
		const blockedOnScope = blocked.replace('DEL-02-scope', secondRevision);
		const acceptedDesign = designSource.replace(
			'/* Lead acceptance */',
			`acceptances: [{ deliverableId: 'DEL-01', build: 'build-87', scopeRevision: '${firstRevision}', evidence: 'acceptance.txt' }],`,
		);
		await Promise.all([
			writeFile(completeFile, complete.replace('DEL-01-scope', firstRevision)),
			writeFile(blockedFile, blockedOnScope),
			writeFile(join(directory, 'design.ts'), acceptedDesign),
		]);
		const checked = await run(process.execPath, check);
		const readOutputs = (): Promise<string[]> =>
			Promise.all(outputNames.map((file) => readFile(join(directory, file), 'utf8')));
		const produced = await readOutputs();
		expect(checked.stdout).toContain('DEL-02 · verification-blocked');
		expect(checked.stdout).toContain('V12 is blocked. Caller response unavailable');
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

		const reorderedDesign = acceptedDesign.replace(
			"{ relation: 'serves', targetId: 'U3' }, { relation: 'serves', targetId: 'U4' }",
			"{ relation: 'serves', targetId: 'U4' }, { relation: 'serves', targetId: 'U3' }",
		);
		expect(reorderedDesign).not.toBe(acceptedDesign);
		await writeFile(join(directory, 'design.ts'), reorderedDesign);
		await run(process.execPath, check);
		const [, , reorderedDelivery] = await readOutputs();
		expect(reorderedDelivery).toBe(produced[2]);
		await writeFile(join(directory, 'design.ts'), acceptedDesign);
		await run(process.execPath, check);

		await writeFile(
			join(directory, 'old-delivery.json'),
			JSON.stringify({ version: 1, snapshotRevision: '0'.repeat(64), deliverables: {} }),
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

		const completeOnScope = complete.replace('DEL-01-scope', firstRevision);
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
			join(directory, 'index.html'),
			page.replace(
				'No handler entry after cancellation.',
				'The worker must never attempt handler entry after cancellation.',
			),
		);
		await run(process.execPath, check);
		const [, , changed] = await readOutputs();
		expect(JSON.parse(changed ?? '')).toMatchObject({
			deliverables: {
				'DEL-01': {
					state: 'awaiting-verification',
					verification: { checks: [{ id: 'V3', state: 'stale' }] },
				},
				'DEL-02': { state: 'dependency-blocked', scopeRevision: secondRevision },
			},
		});
		const withHistoricalCheck = beforeVerification(complete)
			.replace("verificationChecks: ['V3']", 'verificationChecks: []')
			.replace(
				'});',
				`  verificationHistory: [{ build: 'build-87', scopeRevision: '${firstRevision}', outcomes: {
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
		const check = [cli, 'design', 'check', directory];
		const ownerQuestion =
			"{ state: 'owner-open', owner: 'owner', question: 'Which cancellation guarantee applies?' }";
		const decided =
			"{ state: 'owner-decided', owner: 'owner', sourceKind: 'owner-accepted', decision: 'Cancel before claim.', decidedOn: '2026-11-21' }";
		const pendingDecisions = `import type { DesignDecision } from './design-schema.d.ts';
export default [
    { id: 'D1', kind: 'choice', title: 'Cancellation guarantee', authority: ${ownerQuestion},
      destination: { page: 'index.html', fragment: 'D1' },
      sources: [{ href: 'evidence.txt', excerpt: 'Cancel before claim.' }] },
    { id: 'D2', kind: 'choice', title: 'Unrelated follow-up', authority: ${ownerQuestion},
      destination: { page: 'index.html', fragment: 'D2' },
      sources: [{ href: 'evidence.txt', excerpt: 'Cancel before claim.' }] },
] as const satisfies readonly DesignDecision[];
`;
		const pendingDesign = designSource.replace(
			"targetId: 'U4' }]",
			"targetId: 'U4' }, { relation: 'governed-by', targetId: 'D1' }]",
		);
		const fixturePage = page.replace(
			'</ex-document>',
			`<aside id="D1" data-design-item="decision">Cancellation guarantee.</aside>
<aside id="A1" data-design-item="assumption">Claims are atomic on the target backend.</aside>
<aside id="D2" data-design-item="decision">Unrelated follow-up.</aside></ex-document>`,
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
			const scopeRevision = (await readDelivery()).deliverables['DEL-01']?.scopeRevision ?? '';
			await Promise.all([
				writeFile(deliverableFile, complete.replace('DEL-01-scope', scopeRevision)),
				writeFile(
					designFile,
					source.replace(
						'/* Lead acceptance */',
						`acceptances: [{ deliverableId: 'DEL-01', build: 'build-87', scopeRevision: '${scopeRevision}', evidence: 'acceptance.txt' }],`,
					),
				),
			]);
			await run(process.execPath, check);
			return readDelivery();
		}

		const unanswered = (await checkWithCurrentReceipts(pendingDesign)).deliverables['DEL-01'];
		expect(unanswered?.state).toBe('design-blocked');
		expect(unanswered?.reasons).toContain('D1 requires an owner decision.');

		const decidedSource = pendingDecisions.replace(ownerQuestion, decided);
		const openAssumptions = `import type { DesignAssumption } from './design-schema.d.ts';
export default [
    { id: 'A1', kind: 'assumption', title: 'Atomic claims', status: 'open',
      statement: 'Claims are atomic on the target backend.',
      destination: { page: 'index.html', fragment: 'A1' },
      sources: [{ href: 'evidence.txt', excerpt: 'Target backend supports atomic claims.' }] },
] as const satisfies readonly DesignAssumption[];
`;
		const assumedDesign = pendingDesign.replace(
			"{ relation: 'checks', targetId: 'R3' }]",
			"{ relation: 'checks', targetId: 'R3' }, { relation: 'relies-on', targetId: 'A1' }]",
		);
		await Promise.all([
			writeFile(decisionsFile, decidedSource),
			writeFile(assumptionsFile, openAssumptions),
			writeFile(designFile, assumedDesign),
		]);
		await run(process.execPath, check);
		const working = (await readDelivery()).deliverables['DEL-01'];
		expect(working?.state).toBe('awaiting-verification');
		expect(working?.reasons).toContain('Confirm assumption A1 before acceptance.');
		const awaitingConfirmation = (await checkWithCurrentReceipts(assumedDesign)).deliverables[
			'DEL-01'
		];
		expect(awaitingConfirmation?.state).toBe('design-blocked');
		expect(awaitingConfirmation?.reasons).toContain('Confirm assumption A1 before acceptance.');
		expect(awaitingConfirmation?.links.governingItems).toContainEqual({
			id: 'A1',
			page: 'index.html',
			fragment: 'A1',
		});
		const refutedAssumptions = openAssumptions.replace(
			"status: 'open'",
			"status: 'refuted', resolution: { href: 'evidence.txt', excerpt: 'Two claims observed.' }",
		);
		await writeFile(assumptionsFile, refutedAssumptions);
		await run(process.execPath, check);
		const refuted = (await readDelivery()).deliverables['DEL-01'];
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

		await writeFile(pageFile, fixturePage.replace('id="D1"', 'id="D1" data-authority="author"'));
		await expect(run(process.execPath, check)).rejects.toThrow(
			/D1.*data-authority.*owner-accepted/,
		);
		expect(await readDelivery()).toEqual(accepted);
		await writeFile(pageFile, fixturePage);
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
		await writeFile(pageFile, fixturePage.replace('id="D1"', 'id="D1" data-authority="author"'));
		expect((await checkWithCurrentReceipts(assumedDesign)).deliverables['DEL-01']?.state).toBe(
			'accepted',
		);
		await writeFile(pageFile, fixturePage.replace('id="D1"', 'id="D1" data-authority="contract"'));
		await expect(run(process.execPath, check)).rejects.toThrow(/D1.*data-authority.*author/);
		const contractDecision = leadDecisions.replace(
			"sourceKind: 'author'",
			"sourceKind: 'contract'",
		);
		await writeFile(decisionsFile, contractDecision);
		expect((await checkWithCurrentReceipts(assumedDesign)).deliverables['DEL-01']?.state).toBe(
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
