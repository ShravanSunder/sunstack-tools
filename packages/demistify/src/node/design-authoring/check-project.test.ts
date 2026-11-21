import { execFile } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { expect, it } from 'vitest';
import { z } from 'zod';

import { deliverablesViewSchema } from '../../core/deliverables-view.ts';

const run = promisify(execFile);
const cli = fileURLToPath(new URL('../../../bin/demistify.ts', import.meta.url));
const projectView = z.object({ deliverables: deliverablesViewSchema.shape.deliverables });
const fixturePage = `<!doctype html><html lang="en"><head><title>Design set</title>
<link rel="stylesheet" href="https://demistify.getagentstudio.dev/v1/theme.css">
<script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script></head><body>
<ex-nav for="content" src="./navigation.generated.json"></ex-nav><ex-document id="content">
<h1>Design set</h1><p id="U1">Need</p><p id="R1">Rule</p><p id="V1">Check</p><p id="DEL-01">Implementation plan</p>
<ex-deliverables src="./deliverables.generated.json"></ex-deliverables></ex-document></body></html>`;

function destination(fragment: string): { page: string; fragment: string } {
	return { page: 'index.html', fragment };
}

function designSource(shared: boolean, receipt?: { build: string; scopeRevision: string }): string {
	const sources = [{ href: 'evidence.txt', excerpt: 'Observed target evidence.' }];
	return `import type { DesignCore } from './design-schema.d.ts';
export default ${JSON.stringify({
		pages: [{ id: 'home', title: 'Home', href: 'index.html', icon: 'file' }],
		mapGroups: [],
		records: [
			{
				id: 'U1',
				title: 'Need',
				kind: 'need',
				statement: 'Caller sees the current result.',
				destination: destination('U1'),
				sources,
			},
			{
				id: 'R1',
				title: 'Rule',
				kind: 'rule',
				statement: 'Return the current result.',
				destination: destination('R1'),
				sources,
				relationships: [
					{ relation: 'serves', targetId: 'U1' },
					...(shared
						? [
								{ relation: 'governed-by', targetId: 'project/D1' },
								{ relation: 'relies-on', targetId: 'project/A1' },
							]
						: []),
				],
			},
			{
				id: 'V1',
				title: 'Check',
				kind: 'planned-check',
				status: 'planned',
				destination: destination('V1'),
				sources,
				relationships: [{ relation: 'checks', targetId: 'R1' }],
				cases: [
					{ given: 'Active', expect: 'Current' },
					{ given: 'Closed', expect: 'Closed' },
					{ given: 'Missing', expect: 'Absent' },
				],
			},
		],
		...(receipt
			? { acceptances: [{ deliverableId: 'DEL-01', ...receipt, evidence: 'evidence.txt' }] }
			: {}),
	})} as const satisfies DesignCore;`;
}

function deliverableSource(receipt?: { build: string; scopeRevision: string }): string {
	const progress = receipt
		? {
				implementation: {
					completedTasks: ['task'],
					candidate: receipt.build,
					evidence: ['evidence.txt'],
				},
				verification: {
					...receipt,
					outcomes: {
						['V1']: { kind: 'passed', observation: 'Observed.', evidence: 'evidence.txt' },
					},
				},
			}
		: {};
	return `import { defineDeliverable } from '../design-sdk.generated.ts';
export default defineDeliverable(${JSON.stringify({
		id: 'DEL-01',
		title: 'Deliverable',
		destination: { page: 'index.html', fragment: 'DEL-01' },
		scope: { rules: ['R1'], implementationTasks: ['task'], verificationChecks: ['V1'] },
		...progress,
	})});`;
}

function deliverySource(dependency?: string): string {
	return `import design from './design.ts';
import unit from './deliverables/unit.ts';
import { defineDelivery } from './design-sdk.generated.ts';
export default defineDelivery(design, [{ deliverable: unit${dependency ? `, dependsOn: ['${dependency}']` : ''} }]${dependency ? ", { lead: 'messages-lead' }" : ''});`;
}

it('checks an empty project without discovering files beside the manifest', async (): Promise<void> => {
	const directory = await mkdtemp(join(tmpdir(), 'demistify-project-empty-'));
	try {
		await Promise.all([
			writeFile(join(directory, 'project.ts'), 'export default {};'),
			writeFile(join(directory, 'project-decisions.ts'), 'throw new Error("not registered");'),
			writeFile(join(directory, 'agents-identity.ts'), 'throw new Error("not registered");'),
		]);
		await run(process.execPath, [cli, 'project', 'check', directory]);
		const output = join(directory, 'project.generated.json');
		const empty = await readFile(output, 'utf8');
		expect(JSON.parse(empty)).toEqual({
			version: 1,
			designs: {},
			records: [],
			deliverables: {},
		});
		await writeFile(join(directory, 'project.ts'), "export default { identities: '' };");
		await expect(run(process.execPath, [cli, 'project', 'check', directory])).rejects.toThrow(
			/identities/,
		);
		expect(await readFile(output, 'utf8')).toBe(empty);
	} finally {
		await rm(directory, { recursive: true, force: true });
	}
});

it('checks explicit project records, qualified prerequisites, and Lead aliases across two designs', async (): Promise<void> => {
	const sandbox = await mkdtemp(join(tmpdir(), 'demistify-project-check-'));
	const root = join(sandbox, 'docs/projects/2026-11-messaging');
	const messages = join(root, 'milestones/2026-11-messages');
	const sessions = join(root, 'milestones/2026-11-sessions#archive');
	const outputPath = join(root, 'project.generated.json');
	const check = (): ReturnType<typeof run> =>
		run(process.execPath, [cli, 'project', 'check', root]);
	const readView = async (): Promise<z.infer<typeof projectView>> =>
		projectView.parse(JSON.parse(await readFile(outputPath, 'utf8')));
	try {
		await Promise.all(
			[messages, sessions].map(async (directory) => {
				await run(process.execPath, [cli, 'design', 'init', directory]);
				await mkdir(join(directory, 'deliverables'));
				await Promise.all([
					writeFile(join(directory, 'index.html'), fixturePage),
					writeFile(join(directory, 'design.ts'), designSource(directory === messages)),
					writeFile(
						join(directory, 'design-delivery.ts'),
						deliverySource(directory === messages ? 'sessions/DEL-01' : undefined),
					),
					writeFile(join(directory, 'deliverables/unit.ts'), deliverableSource()),
					writeFile(join(directory, 'evidence.txt'), 'Observed target evidence.'),
				]);
			}),
		);
		const shared = join(root, 'shared');
		await mkdir(shared);
		const owner =
			"{ state: 'owner-decided', owner: 'owner', sourceKind: 'owner-words', decision: 'Use the current result.', decidedOn: '2026-11-21' }";
		const decision = `{ id: 'D1', kind: 'choice', title: 'Result', authority: ${owner},
destination: { page: 'index.html', fragment: 'D1' }, sources: [{ href: 'evidence.txt', excerpt: 'Use the current result.' }] }`;
		await Promise.all([
			writeFile(
				join(root, 'project.ts'),
				`import messagesDesign from './milestones/2026-11-messages/design.ts';
import sessionsDesign from './milestones/2026-11-sessions#archive/design-delivery.ts';
import decisions from './shared/project-decisions.ts';
import assumptions from './shared/project-assumptions.ts';
import agents from './agents-identity.ts';
export default { designs: { messages: messagesDesign, sessions: sessionsDesign }, decisions, assumptions, identities: agents };`,
			),
			writeFile(
				join(root, 'agents-identity.ts'),
				`import { agentId } from './milestones/2026-11-messages/design-sdk.generated.ts';
const lead = agentId('private-fixture-session');
export default { owner: 'owner', agents: { 'messages-lead': lead } };`,
			),
			writeFile(
				join(shared, 'project-decisions.ts'),
				`import type { DesignDecision } from '../milestones/2026-11-messages/design-schema.d.ts';
export default [${decision}] as const satisfies readonly DesignDecision[];`,
			),
			writeFile(
				join(shared, 'project-assumptions.ts'),
				`export default [{
id: 'A1', kind: 'assumption', title: 'Target', statement: 'Target is available.', status: 'confirmed',
destination: { page: 'index.html', fragment: 'A1' }, sources: [{ href: 'evidence.txt', excerpt: 'Available.' }],
resolution: { href: 'evidence.txt', excerpt: 'Available.' }
}];`,
			),
			writeFile(
				join(shared, 'index.html'),
				'<!doctype html><title>Project evidence</title><p id="D1">Decision</p><p id="A1">Assumption</p><p id="D2">Replacement</p>',
			),
			writeFile(join(shared, 'evidence.txt'), 'Available. Use the current result.'),
		]);

		await check();
		const before = await readView();
		expect(before.deliverables['messages/DEL-01']?.state).toBe('dependency-blocked');
		expect(before.deliverables['sessions/DEL-01']?.links.plan.page).toBe(
			'milestones/2026-11-sessions%23archive/index.html',
		);
		expect(
			before.deliverables['messages/DEL-01']?.links.governingItems.map(({ id }) => id),
		).toEqual(['project/A1', 'project/D1']);
		const published = await readFile(outputPath, 'utf8');
		expect(published).toContain('messages-lead');
		expect(published).not.toContain('private-fixture-session');

		const stamp = async (directory: string, name: string, build: string): Promise<void> => {
			const scopeRevision = (await readView()).deliverables[`${name}/DEL-01`]?.scopeRevision ?? '';
			const receipt = { build, scopeRevision };
			await Promise.all([
				writeFile(join(directory, 'design.ts'), designSource(name === 'messages', receipt)),
				writeFile(join(directory, 'deliverables/unit.ts'), deliverableSource(receipt)),
			]);
			await check();
		};
		await stamp(sessions, 'sessions', 'build-1');
		expect((await readView()).deliverables['messages/DEL-01']?.state).toBe('planned');
		await stamp(messages, 'messages', 'messages-1');
		const accepted = await readView();
		expect(accepted.deliverables['messages/DEL-01']?.state).toBe('accepted');
		await stamp(sessions, 'sessions', 'build-2');
		const revised = await readView();
		expect(revised.deliverables['sessions/DEL-01']?.state).toBe('accepted');
		expect(revised.deliverables['messages/DEL-01']?.verification.checks[0]?.state).toBe('stale');

		const previous = await readFile(outputPath, 'utf8');
		const currentMessages = await readFile(join(messages, 'design.ts'), 'utf8');
		await writeFile(
			join(messages, 'design.ts'),
			currentMessages.replace('"deliverableId":"DEL-01"', '"deliverableId":"sessions/DEL-01"'),
		);
		await expect(check()).rejects.toThrow(/deliverableId/);
		expect(await readFile(outputPath, 'utf8')).toBe(previous);
		await writeFile(join(messages, 'design.ts'), currentMessages);
		await writeFile(join(sessions, 'design-delivery.ts'), deliverySource('messages/DEL-01'));
		await expect(check()).rejects.toThrow(/dependency cycle/);
		expect(await readFile(outputPath, 'utf8')).toBe(previous);
		await writeFile(join(sessions, 'design-delivery.ts'), deliverySource());

		const replacement = decision
			.replaceAll("'D1'", "'D2'")
			.replace("id: 'D2',", "id: 'D2', supersedes: 'D1',");
		const leadReplacement = replacement.replace(
			owner,
			"{ state: 'lead-decided', sourceKind: 'author', lead: 'messages-lead', rationale: 'Reuse the target.', decision: 'Reuse it.', decidedOn: '2026-11-21' }",
		);
		await writeFile(
			join(shared, 'project-decisions.ts'),
			`export default [${decision}, ${leadReplacement}];`,
		);
		await expect(check()).rejects.toThrow(/only an Owner decision can supersede project\/D1/);
		await writeFile(
			join(shared, 'project-decisions.ts'),
			`export default [${decision}, ${replacement}];`,
		);
		await expect(check()).rejects.toThrow(/uses superseded decision project\/D1/);
		expect(await readFile(outputPath, 'utf8')).toBe(previous);
	} finally {
		await rm(sandbox, { recursive: true, force: true });
	}
}, 90_000);
