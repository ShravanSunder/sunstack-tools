import { execFile } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, stat, symlink, writeFile } from 'node:fs/promises';
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
function coreView(
	view: z.infer<typeof projectView>,
	id: string,
): Extract<z.infer<typeof deliverablesViewSchema.shape.deliverables>[string], { kind: 'core' }> {
	const entry = view.deliverables[id];
	if (entry?.kind !== 'core') throw new Error(`Expected core deliverable ${id}.`);
	return entry;
}

const fixturePage = `<!doctype html><html lang="en"><head><title>Design set</title>
<link rel="stylesheet" href="https://demistify.getagentstudio.dev/v1/theme.css">
<script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script></head><body>
<ex-nav for="content" src="./navigation.generated.json"></ex-nav><ex-document id="content">
<h1>Design set</h1><p data-design-record="U1"></p><p data-design-record="R1"></p><p data-design-record="V1"></p><p id="DEL-01">Implementation plan</p>
<ex-deliverables src="./deliverables.generated.json"></ex-deliverables></ex-document></body></html>`;

function designSource(shared: boolean, receipt?: { build: string }): string {
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
			},
			{
				id: 'R1',
				title: 'Rule',
				kind: 'rule',
				statement: 'Return the current result.',

				serves: 'U1',
				...(shared ? { governedBy: ['project/D1'], reliesOn: ['project/A1'] } : {}),
			},
			{
				id: 'V1',
				title: 'Check',
				kind: 'planned-check',

				checks: 'R1',
				statement: 'Compare the observed result with the expected state.',
				journey: 'Result integration',
				unverified: 'Remote delivery',
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

function deliverableSource(receipt?: { build: string }, builder?: string): string {
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
		kind: 'core',
		id: 'DEL-01',
		title: 'Deliverable',
		...(builder ? { builder } : {}),
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
		const checked = await run(process.execPath, [cli, 'project', 'check', directory]);
		expect(checked.stdout).toContain('Read-only check');
		const output = join(directory, 'project.generated.json');
		await expect(stat(output)).rejects.toMatchObject({ code: 'ENOENT' });
		await run(process.execPath, [cli, 'project', 'check', directory, '--write']);
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
}, 20_000);

it('writes records under the shared projects ignore scope and clears obsolete page data', async (): Promise<void> => {
	const root = await mkdtemp(join(tmpdir(), 'demistify-record-output-'));
	const projects = join(root, 'docs', 'projects');
	const project = join(projects, 'example');
	const phase = join(project, 'phase');
	const page = join(phase, 'index.html');
	const output = join(phase, 'records.generated.js');
	const ignore = join(projects, '.gitignore');
	const patterns =
		'*.generated.json\ndesign-sdk.generated.ts\ndesign-schema.d.ts\nrecords.generated.js\n';
	const original = fixturePage.replace(
		'</ex-document>',
		'<ex-tree data-design-need="U1"></ex-tree></ex-document>',
	);
	const check = [cli, 'project', 'check', project];
	const git = (args: string[]): ReturnType<typeof run> => run('git', ['-C', root, ...args]);
	try {
		await mkdir(phase, { recursive: true });
		await git(['init', '--quiet', '--template=']);
		await mkdir(join(root, '.git/info'), { recursive: true });
		await Promise.all([
			writeFile(join(root, '.git/info/exclude'), patterns),
			writeFile(join(project, '.gitignore'), patterns),
			writeFile(
				join(project, 'project.ts'),
				"import phase from './phase/design.ts'; export default { designs: { phase } };",
			),
			writeFile(join(phase, 'design.ts'), designSource(false)),
			writeFile(page, original),
		]);
		const advisory =
			'Generated views are missing or stale. Run demistify project check --write to refresh them.';
		const missing = await run(process.execPath, check);
		expect(missing.stdout.split(advisory)).toHaveLength(2);
		await expect(stat(ignore)).rejects.toHaveProperty('code', 'ENOENT');
		await expect(stat(output)).rejects.toHaveProperty('code', 'ENOENT');
		await run(process.execPath, [...check, '--write']);
		expect(await readFile(ignore, 'utf8')).toBe(patterns);
		expect(await readFile(join(project, '.gitignore'), 'utf8')).toBe(patterns);
		expect(await readFile(page, 'utf8')).toBe(original);
		expect(await readFile(output, 'utf8')).toContain('Return the current result.');
		expect(await readFile(output, 'utf8')).toContain('[phase/R1 · Rule](<#R1>)');

		expect((await run(process.execPath, check)).stdout).not.toContain(advisory);
		const freshRecords = await readFile(output, 'utf8');
		await writeFile(
			join(phase, 'design.ts'),
			designSource(false).replace('Return the current result.', 'Return the latest result.'),
		);
		expect((await run(process.execPath, check)).stdout.split(advisory)).toHaveLength(2);
		expect(await readFile(output, 'utf8')).toBe(freshRecords);
		await run(process.execPath, [...check, '--write']);
		expect(await readFile(output, 'utf8')).toContain('Return the latest result.');
		expect((await run(process.execPath, check)).stdout).not.toContain(advisory);

		const unchangedIgnore = await stat(ignore);
		await run(process.execPath, [...check, '--write']);
		expect((await stat(ignore)).mtimeMs).toBe(unchangedIgnore.mtimeMs);

		const incomplete = '# maintained by the author\n';
		await writeFile(ignore, incomplete);
		await git(['add', 'docs/projects/.gitignore']);
		const populated = await readFile(output, 'utf8');
		await writeFile(page, fixturePage.replace(/<p data-design-record="[^"]+"><\/p>/g, ''));
		await expect(run(process.execPath, [...check, '--write'])).rejects.toThrow(
			/Add the generated design ignore patterns to tracked .*projects\/\.gitignore/,
		);
		expect(await readFile(ignore, 'utf8')).toBe(incomplete);
		expect(await readFile(output, 'utf8')).toBe(populated);

		await writeFile(join(root, '.gitignore'), patterns);
		await run(process.execPath, [...check, '--write']);
		expect(await readFile(ignore, 'utf8')).toBe(incomplete);
		expect(await readFile(output, 'utf8')).toBe(
			'document.currentScript.dataset.demistifyRecords = "{\\"version\\":1,\\"pages\\":{}}";\n',
		);
		await git(['add', '-f', 'docs/projects/example/phase/records.generated.js']);
		await expect(run(process.execPath, [...check, '--write'])).rejects.toThrow(
			/Generated output is tracked: .*records\.generated\.js/,
		);
	} finally {
		await rm(root, { recursive: true, force: true });
	}
}, 30_000);

it('initializes a design under the shared projects ignore scope without editing a tracked ignore', async (): Promise<void> => {
	const root = await mkdtemp(join(tmpdir(), 'demistify-init-ignore-'));
	const projects = join(root, 'docs', 'projects');
	const project = join(projects, 'example');
	const phase = join(project, 'phase');
	const ignore = join(projects, '.gitignore');
	const tracked = '# maintained by the author\n';
	const git = (args: string[]): ReturnType<typeof run> => run('git', ['-C', root, ...args]);
	try {
		await mkdir(phase, { recursive: true });
		await git(['init', '--quiet', '--template=']);
		await writeFile(
			join(project, 'project.ts'),
			"import phase from './phase/design.ts'; export default { designs: { phase } };",
		);
		await writeFile(join(phase, 'project.ts'), 'export const settings = { app: true };');
		await writeFile(ignore, tracked);
		await git(['add', 'docs/projects/.gitignore']);
		const init = [cli, 'design', 'init', phase];
		await expect(run(process.execPath, init)).rejects.toThrow(
			/Add the generated design ignore patterns to tracked .*projects\/\.gitignore/,
		);
		await expect(stat(join(phase, 'design.ts'))).rejects.toHaveProperty('code', 'ENOENT');
		await writeFile(
			join(root, '.gitignore'),
			'*.generated.json\ndesign-sdk.generated.ts\ndesign-schema.d.ts\nrecords.generated.js\n',
		);
		await run(process.execPath, init);
		expect(await readFile(ignore, 'utf8')).toBe(tracked);
		await expect(stat(join(phase, '.gitignore'))).rejects.toHaveProperty('code', 'ENOENT');
		expect(
			await git(['check-ignore', 'docs/projects/example/phase/design-sdk.generated.ts']),
		).toMatchObject({ stdout: 'docs/projects/example/phase/design-sdk.generated.ts\n' });
	} finally {
		await rm(root, { recursive: true, force: true });
	}
}, 30_000);

it.each(['absent', 'untracked'] as const)(
	'refuses to change a repository-root ignore with %s coverage',
	async (initial): Promise<void> => {
		const root = await mkdtemp(join(tmpdir(), 'demistify-root-ignore-'));
		const ignore = join(root, '.gitignore');
		const project = join(root, 'project.ts');
		const output = join(root, 'project.generated.json');
		const original = '# unrelated untracked rule\n*.bak\n';
		const coverage =
			'*.generated.json\ndesign-sdk.generated.ts\ndesign-schema.d.ts\nrecords.generated.js\n';
		const cliCheck = [cli, 'project', 'check', root, '--write'];
		try {
			await run('git', ['-C', root, 'init', '--quiet', '--template=']);
			await writeFile(project, 'export default {};');
			if (initial === 'untracked') await writeFile(ignore, original);
			await expect(run(process.execPath, cliCheck)).rejects.toThrow(/repository-root \.gitignore/);
			await expect(run(process.execPath, [cli, 'design', 'init', root])).rejects.toThrow(
				/repository-root \.gitignore/,
			);
			if (initial === 'absent') await expect(stat(ignore)).rejects.toHaveProperty('code', 'ENOENT');
			else expect(await readFile(ignore, 'utf8')).toBe(original);
			await expect(stat(output)).rejects.toHaveProperty('code', 'ENOENT');
			await expect(stat(join(root, 'design.ts'))).rejects.toHaveProperty('code', 'ENOENT');

			await writeFile(ignore, coverage);
			const before = await stat(ignore);
			await run(process.execPath, cliCheck);
			await run(process.execPath, [cli, 'design', 'init', root]);
			expect(await readFile(ignore, 'utf8')).toBe(coverage);
			expect((await stat(ignore)).mtimeMs).toBe(before.mtimeMs);
		} finally {
			await rm(root, { recursive: true, force: true });
		}
	},
	30_000,
);

it.each([
	['application code', 'export const settings = { app: true };'],
	['unregistered design', 'export default {};'],
])(
	'keeps standalone ignore scope under a project.ts containing %s',
	async (_name, manifest): Promise<void> => {
		const root = await mkdtemp(join(tmpdir(), 'demistify-standalone-ignore-'));
		const projects = join(root, 'docs', 'projects');
		const project = join(projects, 'app');
		const initialized = join(project, 'initialized');
		const existing = join(project, 'existing');
		try {
			await mkdir(existing, { recursive: true });
			await run('git', ['-C', root, 'init', '--quiet', '--template=']);
			await writeFile(join(project, 'project.ts'), manifest ?? '');
			await run(process.execPath, [cli, 'design', 'init', initialized]);
			expect(await readFile(join(initialized, '.gitignore'), 'utf8')).toContain(
				'records.generated.js',
			);
			await Promise.all([
				writeFile(join(existing, 'design.ts'), designSource(false)),
				writeFile(join(existing, 'index.html'), fixturePage),
			]);
			await run(process.execPath, [cli, 'design', 'check', existing, '--write']);
			expect(await readFile(join(existing, '.gitignore'), 'utf8')).toContain(
				'records.generated.js',
			);
			expect(await readFile(join(existing, 'records.generated.js'), 'utf8')).toContain(
				'Return the current result.',
			);
			await expect(stat(join(projects, '.gitignore'))).rejects.toHaveProperty('code', 'ENOENT');
		} finally {
			await rm(root, { recursive: true, force: true });
		}
	},
	30_000,
);

it('qualifies need coverage and deduplicates scoped rule notes across a checked project', async (): Promise<void> => {
	const root = await mkdtemp(join(tmpdir(), 'demistify-qualified-coverage-'));
	const queue = join(root, 'queue');
	const history = join(root, 'history');
	const queueFile = join(queue, 'design.ts');
	const historyFile = join(history, 'design.ts');
	const receipt = { build: 'build-01' };
	const historyRealization = {
		id: 'M1',
		kind: 'realization',
		statement: 'History returns the stored result. It retains the latest value.',
		realizes: 'R1',
	};
	const realizedHistory = designSource(false, receipt).replace(
		'"records":[',
		`"records":[${JSON.stringify(historyRealization)},${JSON.stringify({
			id: 'U2',
			kind: 'need',
			statement: 'Queue operators see the history record.',
		})},`,
	);
	const note = 'queue/R1: No realization names this rule.';
	const unservedNeedNote = 'history/U2: No rule serves this need.';
	try {
		await Promise.all([
			mkdir(join(queue, 'deliverables'), { recursive: true }),
			mkdir(join(history, 'deliverables'), { recursive: true }),
		]);
		const queueDeliverable = deliverableSource(receipt);
		await Promise.all([
			writeFile(
				join(root, 'project.ts'),
				`import queue from './queue/design.ts'; import history from './history/design.ts'; export default { designs: { queue, history } };`,
			),
			writeFile(queueFile, designSource(false, receipt)),
			writeFile(historyFile, realizedHistory),
			writeFile(
				join(queue, 'index.html'),
				fixturePage.replace('</ex-document>', '<p id="DEL-02">The shared rule</p></ex-document>'),
			),
			writeFile(join(history, 'index.html'), fixturePage),
			writeFile(join(queue, 'evidence.txt'), 'Queue evidence.'),
			writeFile(join(history, 'evidence.txt'), 'History evidence.'),
			writeFile(join(history, 'design-delivery.ts'), deliverySource()),
			writeFile(join(history, 'deliverables/unit.ts'), deliverableSource(receipt)),
			writeFile(
				join(queue, 'deliverables/unit.ts'),
				queueDeliverable.replace('"scope":{', '"scope":{"needs":["U1","history/U1"],'),
			),
			writeFile(
				join(queue, 'deliverables/other.ts'),
				queueDeliverable.replaceAll('DEL-01', 'DEL-02'),
			),
			writeFile(
				join(queue, 'design-delivery.ts'),
				deliverySource()
					.replace(
						'import { defineDelivery }',
						"import other from './deliverables/other.ts';\nimport { defineDelivery }",
					)
					.replace('{ deliverable: unit }]', '{ deliverable: unit }, { deliverable: other }]'),
			),
		]);
		const check = [cli, 'project', 'check', root];
		const first = (await run(process.execPath, [...check, '--write'])).stdout;
		expect(first).toContain('queue/DEL-01 · awaiting-verification');
		expect(first).toContain('Link scoped rules to scoped needs: history/U1.');
		expect(first).toContain('queue/DEL-02 · ready-for-acceptance');
		expect(first).toContain('history/DEL-01 · accepted');
		expect(first).toContain(`Design notes\n  ${unservedNeedNote}`);
		expect(first).toContain('history/M1: statement has 2 sentences; keep one sentence.');
		expect(first.split(unservedNeedNote)).toHaveLength(2);
		expect(first).not.toMatch(/(?:queue|history)\/U1: No rule serves/);
		expect(first.split(note)).toHaveLength(2);
		expect(first).not.toContain('history/R1: No realization');
		expect(await readFile(join(root, 'project.generated.json'), 'utf8')).not.toMatch(
			/No rule serves|No realization|statement (?:is|has)/,
		);
		const brief = [cli, 'brief', 'queue/DEL-01', root];
		const firstBrief = (await run(process.execPath, brief)).stdout;
		expect(firstBrief).toContain('**queue/U1** Caller sees the current result.');
		expect(firstBrief).toContain('**history/U1** Caller sees the current result.');
		expect(firstBrief).toContain(note);
		expect(firstBrief).not.toMatch(
			/history\/R1|history\/M1|queue\/DEL-02|history\/U2|No rule serves|statement (?:is|has)/,
		);

		await writeFile(
			queueFile,
			designSource(false, receipt).replace(
				'"serves":"U1"',
				'"serves":["U1","history/U1","history/U2"]',
			),
		);
		await writeFile(
			historyFile,
			realizedHistory.replace(
				'"records":[',
				`"records":[${JSON.stringify({
					...historyRealization,
					id: 'M2',
					realizes: 'queue/R1',
				})},`,
			),
		);
		const complete = (await run(process.execPath, check)).stdout;
		expect(complete).toContain('queue/DEL-01 · accepted');
		expect(complete).not.toMatch(/No realization|No rule serves|Link scoped rules/);
		await writeFile(
			join(history, 'index.html'),
			fixturePage.replace(/<script type="module"[^>]+><\/script>/, ''),
		);
		const completeBrief = (await run(process.execPath, brief)).stdout;
		expect(completeBrief).not.toMatch(/No realization|Link scoped rules|history\/M2/);
		await expect(run(process.execPath, check)).rejects.toThrow(/runtime\.js/);
	} finally {
		await rm(root, { recursive: true, force: true });
	}
}, 30_000);

it('checks explicit project records, qualified prerequisites, and Lead aliases across two designs', async (): Promise<void> => {
	const sandbox = await mkdtemp(join(tmpdir(), 'demistify-project-check-'));
	const root = join(sandbox, 'docs/projects/2026-11-messaging');
	const messages = join(root, 'milestones/2026-11-messages');
	const sessions = join(root, 'milestones/2026-11-sessions#archive');
	const outputPath = join(root, 'project.generated.json');
	const check = (write = true): ReturnType<typeof run> =>
		run(process.execPath, [cli, 'project', 'check', root, ...(write ? ['--write'] : [])]);
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
					writeFile(
						join(directory, 'deliverables/unit.ts'),
						deliverableSource(undefined, directory === messages ? 'messages-builder' : undefined),
					),
					writeFile(join(directory, 'evidence.txt'), 'Observed target evidence.'),
				]);
			}),
		);
		const shared = join(root, 'shared');
		await mkdir(shared);
		const owner =
			"{ state: 'owner-decided', sourceKind: 'owner-words', decidedOn: '2026-11-21', source: { href: 'evidence.txt', excerpt: 'Use the current result.' } }";
		const decision = `{ id: 'D1', kind: 'ruling', statement: 'Use the current result.', title: 'Result', authority: ${owner},
sources: [{ href: 'evidence.txt', excerpt: 'Use the current result.' }] }`;
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
const builder = agentId('private-builder-session');
export default { owner: 'owner', agents: { 'messages-lead': lead, 'messages-builder': builder } };`,
			),
			writeFile(
				join(shared, 'project-decisions.ts'),
				`import type { DesignDecision } from '../milestones/2026-11-messages/design-schema.d.ts';
export default [${decision}, {
id: 'D2', kind: 'choice', title: 'Unassigned question',
question: 'Which end state should the reader show?', recommended: 'none', options: [{ id: 'end-state', title: 'Show the final result' }, { id: 'timeline', title: 'Show the timeline' }], authority: { state: 'owner-open' },
sources: [{ href: 'evidence.txt', excerpt: 'Context.' }]
}] as const satisfies readonly DesignDecision[];`,
			),
			writeFile(
				join(shared, 'project-assumptions.ts'),
				`export default [{
id: 'A1', kind: 'assumption', title: 'Target', statement: 'Target is available.', status: 'confirmed', by: { alias: 'messages-lead', role: 'lead' }, weight: 'design', settledBy: 'messages-lead',
sources: [{ href: 'evidence.txt', excerpt: 'Available.' }],
resolution: { href: 'evidence.txt', excerpt: 'Available.' }
}];`,
			),
			writeFile(
				join(shared, 'index.html'),
				'<!doctype html><title>Project evidence</title><meta name="demistify-records" content="../records.generated.js"><p data-design-record="project/D1"></p><p data-design-record="project/A1"></p><p data-design-record="project/D2"></p>',
			),
			writeFile(join(shared, 'evidence.txt'), 'Available. Use the current result.'),
		]);

		const sdkFile = join(messages, 'design-sdk.generated.ts');
		await writeFile(sdkFile, `${await readFile(sdkFile, 'utf8')}\n// Older SDK bytes.\n`);
		await check(false);
		expect(await readFile(sdkFile, 'utf8')).toContain('Older SDK bytes.');
		const messagesDesign = join(messages, 'design.ts');
		await writeFile(
			messagesDesign,
			designSource(true)
				.replace(
					'"mapGroups":[]',
					`"mapGroups":[{ title: 'Owner call', description: 'Open the shared question.',
					icon: 'file', target: { record: 'project/D2' } }]`,
				)
				.replace(
					'"href":"index.html","icon":"file"}]',
					'"href":"index.html","icon":"file"},{ id: "shared", title: "Shared decisions", href: "../../shared/index.html", icon: "file" }]',
				),
		);
		const readOnly = await check(false);
		expect(readOnly.stdout).toMatch(/Waiting on owner[\s\S]*project\/D2[\s\S]*messages\/DEL-01/);
		await expect(stat(outputPath)).rejects.toMatchObject({ code: 'ENOENT' });
		const milestonePath = join(messages, 'deliverables.generated.json');
		await expect(stat(milestonePath)).rejects.toMatchObject({ code: 'ENOENT' });
		await check();
		await run(process.execPath, [cli, 'design', 'types', messages]);
		const generatedTypes = await readFile(join(messages, 'design-schema.d.ts'), 'utf8');
		expect(generatedTypes).toContain('"project/D2"');
		expect(generatedTypes).toContain('"sessions/R1"');
		const typedDesign = await readFile(messagesDesign, 'utf8');
		await writeFile(messagesDesign, typedDesign.replace('project/D2', 'project/DX'));
		await expect(check(false)).rejects.toThrow(/TS(?:2322|2820).*project\/DX/s);
		await writeFile(
			messagesDesign,
			typedDesign.replace("'project/D2'", "'project/DX' as 'project/D2'"),
		);
		await expect(check(false)).rejects.toThrow(/unknown design record project\/DX/);
		await expect(run(process.execPath, [cli, 'brief', 'messages/DEL-01', root])).rejects.toThrow(
			/unknown design record project\/DX/,
		);
		await writeFile(messagesDesign, typedDesign);
		const generatedNavigation: unknown = JSON.parse(
			await readFile(join(messages, 'navigation.generated.json'), 'utf8'),
		);
		expect(generatedNavigation).toMatchObject({
			map: [{ target: { page: 'shared', fragment: 'project/D2' } }],
		});
		const sharedPage = join(shared, 'index.html');
		const sharedMarkup = await readFile(sharedPage, 'utf8');
		await writeFile(
			sharedPage,
			sharedMarkup.replace(
				'<p data-design-record="project/D2"></p>',
				'<ex-choice data-design-record="project/D2"><ex-option data-option="end-state"><template slot="summary">Keep the completed state visible.</template></ex-option><ex-option data-option="timeline"><template slot="summary">Show every step.</template></ex-option></ex-choice>',
			),
		);
		await check(false);
		const sharedChoiceMarkup = await readFile(sharedPage, 'utf8');
		const sharedChoice = sharedChoiceMarkup.slice(sharedChoiceMarkup.indexOf('<ex-choice'));
		await writeFile(
			sharedPage,
			sharedChoiceMarkup + sharedChoice.replace('id="end-state"', 'id="other-end-state"'),
		);
		await expect(check(false)).rejects.toThrow(
			/Duplicate HTML id or record placement.*project\/D2/,
		);
		await writeFile(sharedPage, sharedMarkup);
		const before = await readView();
		expect(before.deliverables['messages/DEL-01']?.state).toBe('dependency-blocked');
		expect(coreView(before, 'sessions/DEL-01').links.plan.page).toBe(
			'milestones/2026-11-sessions%23archive/index.html',
		);
		expect(coreView(before, 'messages/DEL-01').links.governingItems.map(({ id }) => id)).toEqual([
			'project/D1',
			'project/A1',
		]);
		const published = await readFile(outputPath, 'utf8');
		expect(published).toContain('messages-lead');
		expect(published).toContain('messages-builder');
		expect(published).not.toContain('private-fixture-session');
		expect(published).not.toContain('private-builder-session');
		expect(await readFile(milestonePath, 'utf8')).toContain('messages-builder');
		const written = await stat(outputPath);
		await check();
		expect((await stat(outputPath)).mtimeMs).toBe(written.mtimeMs);

		const stamp = async (directory: string, name: string, build: string): Promise<void> => {
			const receipt = { build };
			await Promise.all([
				writeFile(join(directory, 'design.ts'), designSource(name === 'messages', receipt)),
				writeFile(
					join(directory, 'deliverables/unit.ts'),
					deliverableSource(receipt, name === 'messages' ? 'messages-builder' : undefined),
				),
			]);
			await check();
		};
		await stamp(sessions, 'sessions', 'build-1');
		expect((await readView()).deliverables['messages/DEL-01']?.state).toBe('planned');
		await stamp(messages, 'messages', 'messages-1');
		const accepted = await readView();
		expect(accepted.deliverables['messages/DEL-01']?.state).toBe('accepted');
		const beforeAssignment = await readFile(milestonePath, 'utf8');
		const messagesFile = join(messages, 'deliverables/unit.ts');
		const withBuilder = await readFile(messagesFile, 'utf8');
		await writeFile(messagesFile, withBuilder.replace('messages-builder', 'messages-lead'));
		await check(false);
		expect(await readFile(milestonePath, 'utf8')).toBe(beforeAssignment);
		await check();
		expect((await readView()).deliverables['messages/DEL-01']).toMatchObject({
			builder: 'messages-lead',
			state: 'accepted',
		});
		await writeFile(messagesFile, withBuilder.replace('messages-builder', 'unknown-builder'));
		await expect(check(false)).rejects.toThrow(/unknown Builder alias unknown-builder/);
		await writeFile(messagesFile, withBuilder);
		await stamp(sessions, 'sessions', 'build-2');
		const revised = await readView();
		expect(revised.deliverables['sessions/DEL-01']?.state).toBe('accepted');
		const local = await run(process.execPath, [cli, 'brief', 'DEL-01'], { cwd: messages });
		const scoped = await run(process.execPath, [cli, 'brief', 'messages/DEL-01', root]);
		expect(local.stdout).toBe(scoped.stdout);
		expect(local.stdout).toContain('sessions/DEL-01');
		await expect(run(process.execPath, [cli, 'brief', 'DEL-01', root])).rejects.toThrow(
			/ambiguous.*messages\/DEL-01.*sessions\/DEL-01/,
		);
		expect(coreView(revised, 'messages/DEL-01').verification.checks[0]?.state).toBe('passed');

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
			"{ state: 'agent-decided', sourceKind: 'author', by: { alias: 'messages-lead', role: 'lead' }, weight: 'design', rationale: 'Reuse the target.', decidedOn: '2026-11-21' }",
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

it('uses one declared external page in two milestone scopes and confines authoring inputs', async (): Promise<void> => {
	const sandbox = await mkdtemp(join(tmpdir(), 'demistify-canonical-project-'));
	const site = join(sandbox, 'site');
	const project = join(site, 'project');
	const canonicalPage = join(site, 'Reviewed Guide.html');
	const canonicalHref = '../../Reviewed%20Guide.html';
	const names = ['north', 'south'];
	const viewFile = join(project, 'project.generated.json');
	const check = (options: readonly string[] = []): ReturnType<typeof run> =>
		run(process.execPath, [cli, 'project', 'check', project, '--site-root', site, ...options]);
	const readView = async (): Promise<z.infer<typeof projectView>> =>
		projectView.parse(JSON.parse(await readFile(viewFile, 'utf8')));
	const authoredDesign = (receipt?: { build: string }): string =>
		designSource(false, receipt).replaceAll('index.html', canonicalHref);
	const authoredDelivery = (receipt?: { build: string }): string =>
		deliverableSource(receipt).replaceAll('index.html', canonicalHref);
	const html = fixturePage
		.replace(
			'</head>',
			'<meta name="demistify-records" content="./project/records.generated.js"></head>',
		)
		.replace('./navigation.generated.json', './reviewed-navigation.json')
		.replace(
			'<p data-design-record="U1"></p><p data-design-record="R1"></p><p data-design-record="V1"></p>',
			'<h2 data-design-record="north/R1"></h2><h2 data-design-record="south/R1"></h2><p>Shared context.</p>',
		);

	try {
		await Promise.all(
			names.map(async (name) => {
				const milestone = join(project, name);
				await run(process.execPath, [cli, 'design', 'init', milestone]);
				await mkdir(join(milestone, 'deliverables'));
				await Promise.all([
					writeFile(join(milestone, 'design.ts'), authoredDesign()),
					writeFile(join(milestone, 'design-delivery.ts'), deliverySource()),
					writeFile(join(milestone, 'deliverables/unit.ts'), authoredDelivery()),
					writeFile(join(milestone, 'evidence.txt'), 'Observed the shared passage.'),
				]);
			}),
		);
		const projectSource = `import north from './north/design.ts';
import south from './south/design.ts';
export default { designs: { north, south } };`;
		await Promise.all([
			writeFile(canonicalPage, html),
			writeFile(join(project, 'project.ts'), projectSource),
		]);
		await check();
		await expect(stat(viewFile)).rejects.toMatchObject({ code: 'ENOENT' });
		await check(['--write']);
		await Promise.all(
			names.map(async (name) => {
				const receipt = { build: `${name}-build` };
				await Promise.all([
					writeFile(join(project, name, 'design.ts'), authoredDesign(receipt)),
					writeFile(join(project, name, 'deliverables/unit.ts'), authoredDelivery(receipt)),
				]);
			}),
		);
		await check(['--write']);
		const accepted = await readView();
		for (const name of names) {
			const view = coreView(accepted, `${name}/DEL-01`);
			expect(view?.state).toBe('accepted');
			expect(new URL(view.links.plan.page, 'https://example.test/site/project/').pathname).toBe(
				'/site/Reviewed%20Guide.html',
			);
		}
		const acceptedBytes = await readFile(viewFile, 'utf8');
		await writeFile(canonicalPage, html.replace('Shared context.', 'Revised explanation.'));
		await check(['--write']);
		expect(await readFile(viewFile, 'utf8')).toBe(acceptedBytes);
		await writeFile(canonicalPage, html.replace('south/R1', 'north/R1'));
		await expect(check()).rejects.toThrow(/Duplicate HTML id or record placement.*north/);
		await writeFile(canonicalPage, html.replace('id="DEL-01"', 'id="other-plan"'));
		await expect(check()).rejects.toThrow(/Missing HTML target.*#DEL-01/);
		await writeFile(canonicalPage, html);
		const unlistedDesign = authoredDesign().replace(
			'"href":"' + canonicalHref + '"',
			'"href":"other.html"',
		);
		await writeFile(
			join(project, 'north', 'other.html'),
			fixturePage.replace(/<p data-design-record="[^"]+"><\/p>/g, ''),
		);
		await writeFile(join(project, 'north', 'design.ts'), unlistedDesign);
		await expect(check()).rejects.toThrow(/Deliverable destination must be a declared HTML page/);
		await writeFile(join(project, 'north', 'design.ts'), authoredDesign());

		const outside = join(sandbox, 'outside.html');
		await writeFile(outside, html);
		await symlink(outside, join(site, 'escaped.html'));
		await writeFile(
			join(project, 'north', 'design.ts'),
			authoredDesign().replaceAll(canonicalHref, '../../escaped.html'),
		);
		await expect(check()).rejects.toThrow(/resolves outside the site root/);
		await writeFile(join(project, 'north', 'design.ts'), authoredDesign());

		await writeFile(
			join(site, 'agents-identity.ts'),
			'export default { owner: "owner", agents: {} };',
		);
		await symlink(join(site, 'agents-identity.ts'), join(project, 'agents-identity.ts'));
		await writeFile(
			join(project, 'project.ts'),
			projectSource
				.replace('export default', "import identities from './agents-identity.ts';\nexport default")
				.replace('{ designs:', '{ identities, designs:'),
		);
		await expect(check()).rejects.toThrow(/resolves outside the project root/);
	} finally {
		await rm(sandbox, { recursive: true, force: true });
	}
}, 50_000);
