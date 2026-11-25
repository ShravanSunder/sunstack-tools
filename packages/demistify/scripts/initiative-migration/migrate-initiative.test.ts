import { mkdir, readdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';

import { expect, it } from 'vitest';

import { checkInitiative } from '../../src/node/design-authoring/check-initiative.ts';
import { AnchorRelocations, rewriteReference } from './address-rewrite.ts';
import { parseCutPlan } from './cut-plan.ts';
import { migrateInitiative } from './migrate-initiative.ts';
import {
	fixturePlan,
	writeLegacyMigrationFixture,
	writeMigrationFixture,
} from './migration-fixture.ts';
import { nextFreeId } from './move-resolution.ts';

async function treeContents(root: string): Promise<Map<string, string>> {
	const contents = new Map<string, string>();
	const walk = async (directory: string): Promise<void> => {
		for (const entry of await readdir(directory, { withFileTypes: true })) {
			const path = join(directory, entry.name);
			// oxlint-disable-next-line no-await-in-loop -- a small fixture tree, read in order.
			if (entry.isDirectory()) await walk(path);
			// oxlint-disable-next-line no-await-in-loop -- a small fixture tree, read in order.
			else contents.set(relative(root, path), await readFile(path, 'utf8'));
		}
	};
	await walk(root);
	return contents;
}

async function withFixture(
	plan: unknown,
	run: (root: string, planFile: string) => Promise<void>,
	write: () => Promise<string> = writeMigrationFixture,
): Promise<void> {
	const root = await write();
	const planFile = join(dirname(root), 'plan.json');
	await writeFile(planFile, JSON.stringify(plan));
	try {
		await run(root, planFile);
	} finally {
		await rm(dirname(root), { recursive: true, force: true });
	}
}

it('names the next free id after the highest one with the same prefix', (): void => {
	expect(nextFreeId('R3', new Set(['R1', 'R3']))).toBe('R4');
	expect(nextFreeId('DEL-03', new Set(['DEL-01', 'DEL-03']))).toBe('DEL-04');
	expect(nextFreeId('DEL-07', new Set(['DEL-07', 'DEL-12']))).toBe('DEL-13');
	expect(nextFreeId('home', new Set(['home']))).toBe('home-2');
});

it('writes a reference bare in its own project and qualified elsewhere', (): void => {
	const maps = {
		records: new Map([['alpha/R3', 'beta/R4']]),
		deliverables: new Map([['alpha/DEL-01', 'gamma/DEL-01']]),
	};
	expect(rewriteReference('R3', 'record', { before: 'alpha', after: 'beta' }, maps)).toBe('R4');
	expect(rewriteReference('R3', 'record', { before: 'alpha', after: 'alpha' }, maps)).toBe(
		'beta/R4',
	);
	expect(rewriteReference('alpha/R3', 'record', { before: 'beta', after: 'beta' }, maps)).toBe(
		'R4',
	);
	expect(rewriteReference('R2', 'record', { before: 'alpha', after: 'gamma' }, maps)).toBe(
		'alpha/R2',
	);
	expect(
		rewriteReference('initiative/D1', 'record', { before: 'alpha', after: 'gamma' }, maps),
	).toBe('initiative/D1');
	expect(rewriteReference('DEL-01', 'deliverable', { before: 'alpha', after: 'alpha' }, maps)).toBe(
		'gamma/DEL-01',
	);
});

it('follows a moved anchor and keeps unchanged links exactly as written', (): void => {
	const anchors = new AnchorRelocations();
	anchors.movePlacement('/i/p/alpha/index.html', 'R3', {
		file: '/i/p/beta/index.html',
		fragment: 'R4',
	});
	expect(anchors.relocate('index.html#R3', '/i/p/alpha/other.html', '/i/p/alpha/other.html')).toBe(
		'../beta/index.html#R4',
	);
	expect(
		anchors.relocate(
			'../alpha/index.html#R3--details',
			'/i/p/beta/index.html',
			'/i/p/beta/index.html',
		),
	).toBe('#R4--details');
	expect(anchors.relocate('index.html#R2', '/i/p/alpha/other.html', '/i/p/alpha/other.html')).toBe(
		'index.html#R2',
	);
	expect(anchors.relocate('https://example.com/#R3', '/i/p/alpha/a.html', '/i/p/beta/a.html')).toBe(
		'https://example.com/#R3',
	);
});

it('prints the whole move in a dry run and writes nothing', async (): Promise<void> => {
	await withFixture(fixturePlan, async (root, planFile) => {
		const before = await treeContents(dirname(root));
		const { report, applied } = await migrateInitiative(root, { plan: planFile, dryRun: true });
		expect(applied).toBe(false);
		expect(await treeContents(dirname(root))).toEqual(before);
		expect(report).toContain('create 1 gamma (projects/gamma)');
		expect(report).toContain('alpha/DEL-01 -> gamma/DEL-01');
		expect(report).toContain('deliverable alpha/DEL-03 -> beta/DEL-04');
		expect(report).toContain('record alpha/R3 -> beta/R4');
		expect(report).toContain('alpha/N1 stays: alpha/DEL-02 still scopes it');
		expect(report).toContain(
			'beta/N2 in projects/alpha/index.html: its section "Needs" places records that do not all go to one project: alpha/N1 (stays), beta/N2',
		);
		expect(report).toContain('gamma/DEL-02 (later) · Grey out rows');
		expect(report).toContain('when you scope it: dependsOn gamma/DEL-01; why: asked');
		expect(report).toContain('Initiative check: not run in a dry run.');
	});
}, 30_000);

it('moves deliverables with their records, renames on collision and rewrites every reference', async (): Promise<void> => {
	await withFixture(fixturePlan, async (root, planFile) => {
		const { report, applied, checkPassed } = await migrateInitiative(root, {
			plan: planFile,
			dryRun: false,
		});
		expect({ applied, checkPassed }).toEqual({ applied: true, checkPassed: true });
		expect(report).toContain('Initiative check on the staged tree: passed');
		const read = (path: string): Promise<string> => readFile(join(root, path), 'utf8');

		// Without a collision: same ids, the shared need named qualified.
		const expire = await read('projects/gamma/deliverables/del-01-expire.ts');
		expect(expire).toContain("id: 'DEL-01'");
		expect(expire).toContain("evidence: '../alpha/other.html#notes'");
		await expect(
			stat(join(root, 'projects/alpha/deliverables/del-01-expire.ts')),
		).rejects.toHaveProperty('code', 'ENOENT');
		const gammaDesign = await read('projects/gamma/design.ts');
		expect(gammaDesign).toContain("serves: 'alpha/N1', governedBy: ['D1']");
		expect(gammaDesign).toContain('// The rule the first deliverable builds.');
		expect(gammaDesign).toContain("deliverableId: 'DEL-01'");
		expect(await read('projects/gamma/design-decisions.ts')).toContain("id: 'D1'");
		const alphaDesign = await read('projects/alpha/design.ts');
		expect(alphaDesign).toContain("defineNeed({ id: 'N1'");
		expect(alphaDesign).not.toContain("id: 'R1'");
		expect(alphaDesign).toContain("target: { record: 'gamma/R1' }");

		// With a collision: DEL-03 becomes DEL-04 and R3 becomes R4 in beta.
		const stamp = await read('projects/beta/deliverables/del-04-stamp.ts');
		expect(stamp).toContain("id: 'DEL-04'");
		expect(stamp).toContain("rules: ['R4']");
		expect(stamp).toContain("destination: { page: 'index.html', fragment: 'DEL-04' }");
		const betaDesign = await read('projects/beta/design.ts');
		expect(betaDesign).toContain(
			"id: 'R4', statement: 'An old row keeps its stamp.', serves: 'N2'",
		);
		expect(betaDesign).toContain("checks: 'R4'");

		// Pages: the clean section moved; links across pages follow it.
		expect(await read('projects/alpha/other.html')).toContain(
			'<a href="../gamma/index.html#R1">the expiry rule</a> and <a href="../beta/index.html#R4">',
		);
		expect(await read('projects/alpha/other.html')).toContain(
			'<a href="../beta/index.html#stamps">',
		);
		const betaPage = await read('projects/beta/index.html');
		expect(betaPage).toContain('<section id="stamps">');
		expect(betaPage).toContain('<a href="#R4">');
		expect(betaPage).toContain('<p data-design-record="R4"></p>');
		expect(betaPage).toContain('<h2 id="DEL-04">DEL-03 · Keep stamps</h2>');
		const alphaPage = await read('projects/alpha/index.html');
		expect(alphaPage).not.toContain('DEL-01 · Expire rows');
		expect(alphaPage).not.toContain('<section id="stamps">');
		expect(alphaPage).toContain('<p data-design-record="beta/N2"></p>');
		expect(await read('projects/gamma/index.html')).toContain(
			'<a href="../alpha/index.html#R2">R2</a> and keeps <a href="../beta/index.html#DEL-04">',
		);

		// Withdrawal, the added row and dependsOn.
		await expect(
			stat(join(root, 'projects/alpha/deliverables/del-04-someday.ts')),
		).rejects.toHaveProperty('code', 'ENOENT');
		const alphaDelivery = await read('projects/alpha/design-delivery.ts');
		expect(alphaDelivery).not.toContain('someday');
		expect(alphaDelivery).toContain("dependsOn: ['gamma/DEL-01']");
		expect(await read('projects/gamma/deliverables/del-02-grey-out-rows.ts')).toContain(
			"kind: 'later'",
		);
		expect(await read('projects/beta/design-delivery.ts')).toContain(
			"{ deliverable: outcomes, dependsOn: ['gamma/DEL-01'] }",
		);
		expect(await read('initiative.ts')).toContain(
			"import gamma from './projects/gamma/design.ts';",
		);

		expect(await checkInitiative(root)).toContain('gamma/DEL-01 · accepted · Expire rows');
	});
}, 60_000);

it('writes nothing when the staged tree fails the check', async (): Promise<void> => {
	const cycle = {
		...fixturePlan,
		dependsOnChanges: [{ deliverable: 'alpha/DEL-02', from: [], to: ['beta/DEL-01'] }],
	};
	await withFixture(cycle, async (root, planFile) => {
		const before = await treeContents(dirname(root));
		const { report, applied, checkPassed } = await migrateInitiative(root, {
			plan: planFile,
			dryRun: false,
		});
		expect({ applied, checkPassed }).toEqual({ applied: false, checkPassed: false });
		expect(report).toContain('Delivery dependency cycle');
		expect(await treeContents(dirname(root))).toEqual(before);
	});
}, 60_000);

it('refuses a plan that names a deliverable the initiative does not have', async (): Promise<void> => {
	const unknown = { ...fixturePlan, moves: [{ from: 'alpha/DEL-09', to: 'gamma' }] };
	await withFixture(unknown, async (root, planFile) => {
		await expect(migrateInitiative(root, { plan: planFile, dryRun: true })).rejects.toThrow(
			'Move alpha/DEL-09: no such deliverable.',
		);
	});
}, 30_000);

it('converts a project.ts tree to initiative.ts in the same run', async (): Promise<void> => {
	const plan = {
		...fixturePlan,
		projects: [
			{ key: 'gamma', folder: 'gamma', number: 0, title: 'Gamma', achieves: 'Rows expire.' },
			{ key: 'alpha', folder: 'alpha', number: 1, title: 'Alpha' },
		],
		moves: [{ from: 'alpha/DEL-01', to: 'gamma' }],
		dependsOnChanges: [],
	};
	await withFixture(
		plan,
		async (root, planFile) => {
			const before = await treeContents(dirname(root));
			const dry = await migrateInitiative(root, { plan: planFile, dryRun: true });
			expect(await treeContents(dirname(root))).toEqual(before);
			expect(dry.report).toContain('project.ts -> initiative.ts');
			expect(dry.report).toContain('project-decisions.ts -> initiative-decisions.ts');
			expect(dry.report).toContain('beta: not in the plan; keeps its folder, given number 1');

			const { applied } = await migrateInitiative(root, { plan: planFile, dryRun: false });
			expect(applied).toBe(true);
			const read = (path: string): Promise<string> => readFile(join(root, path), 'utf8');
			await expect(stat(join(root, 'project.ts'))).rejects.toHaveProperty('code', 'ENOENT');
			await expect(stat(join(root, 'project-decisions.ts'))).rejects.toHaveProperty(
				'code',
				'ENOENT',
			);
			const manifest = await read('initiative.ts');
			expect(manifest).toContain("import decisions from './initiative-decisions.ts';");
			expect(manifest).toMatch(/gamma: \{\n\s+design: gamma,\n\s+number: 0,/u);
			expect(manifest).toMatch(/beta: \{\n\s+design: beta,\n\s+number: 1,/u);
			expect(await read('projects/alpha/design.ts')).toContain("governedBy: ['initiative/D9']");
			expect(await read('projects/alpha/index.html')).toContain(
				'<ex-section-item record="initiative/D9">',
			);
			expect(await checkInitiative(root)).toContain('Checked 3 project(s) by number: 0 gamma');
		},
		writeLegacyMigrationFixture,
	);
}, 60_000);

it('refuses a move to the project the deliverable is already in', (): void => {
	const selfMove = { ...fixturePlan, moves: [{ from: 'alpha/DEL-02', to: 'alpha' }] };
	expect(() => parseCutPlan('plan.json', JSON.stringify(selfMove))).toThrow(
		'plan.json: the move of alpha/DEL-02 names its own project alpha; leave it out of moves.',
	);
});

it('refuses two dependsOn changes for one deliverable', (): void => {
	const twice = {
		...fixturePlan,
		dependsOnChanges: [
			{ deliverable: 'beta/DEL-01', from: ['alpha/DEL-02'], to: [] },
			{ deliverable: 'beta/DEL-01', from: [], to: ['alpha/DEL-01'] },
		],
	};
	expect(() => parseCutPlan('plan.json', JSON.stringify(twice))).toThrow(
		'plan.json: beta/DEL-01 has two dependsOn changes; write them as one row.',
	);
});

const adoptPlan = {
	projects: [
		{ key: 'gamma', folder: 'gamma', number: 1, title: 'Gamma' },
		{ key: 'alpha', folder: 'alpha', number: 1, title: 'Alpha' },
		{ key: 'beta', folder: 'beta', number: 2, title: 'Beta' },
	],
	moves: [{ from: 'alpha/DEL-01', to: 'gamma' }],
};
const handMadePage = `<!doctype html><html lang="en"><head><title>Gamma by hand</title></head><body>
<h1>Gamma</h1><p>The rule is <a href="../alpha/index.html#R1">alpha's R1</a>, see project/D1.</p>
</body></html>
`;

it('adopts a planned folder without design.ts and leaves its own files alone', async (): Promise<void> => {
	await withFixture(adoptPlan, async (root, planFile) => {
		const gamma = join(root, 'projects/gamma');
		await mkdir(gamma, { recursive: true });
		await writeFile(join(gamma, 'index.html'), handMadePage);
		await writeFile(join(gamma, 'requirements.html'), handMadePage);

		const dry = await migrateInitiative(root, { plan: planFile, dryRun: true });
		expect(dry.report).toContain('adopt  1 gamma (projects/gamma)');
		expect(dry.report).toContain(
			'left alone (byte for byte, not registered; register them in design.ts later): projects/gamma/index.html, projects/gamma/requirements.html',
		);

		const { applied, report } = await migrateInitiative(root, { plan: planFile, dryRun: false });
		expect(report).toContain('Initiative check on the staged tree: passed');
		expect(applied).toBe(true);
		expect(await readFile(join(gamma, 'index.html'), 'utf8')).toBe(handMadePage);
		expect(await readFile(join(gamma, 'requirements.html'), 'utf8')).toBe(handMadePage);
		expect(await readFile(join(gamma, 'design.ts'), 'utf8')).toContain(
			'href: "moved-records.html"',
		);
		const moved = await readFile(join(gamma, 'moved-records.html'), 'utf8');
		expect(moved).toContain('<p data-design-record="R1"></p>');
		expect(moved).toContain('<ex-section-item record="D1">');
		expect(await readFile(join(gamma, 'deliverables/del-01-expire.ts'), 'utf8')).toContain(
			"destination: { page: 'moved-records.html', fragment: 'DEL-01' }",
		);
		expect(await checkInitiative(root)).toContain('gamma/DEL-01 · accepted · Expire rows');
	});
}, 60_000);

it('refuses a planned folder that has a design.ts the initiative does not register', async (): Promise<void> => {
	await withFixture(adoptPlan, async (root, planFile) => {
		const gamma = join(root, 'projects/gamma');
		await mkdir(gamma, { recursive: true });
		await writeFile(join(gamma, 'design.ts'), 'export default {};\n');
		await expect(migrateInitiative(root, { plan: planFile, dryRun: true })).rejects.toThrow(
			'gamma: projects/gamma already exists and is not a registered project.',
		);
	});
}, 30_000);
