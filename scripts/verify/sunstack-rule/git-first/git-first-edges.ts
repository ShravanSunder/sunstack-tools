/** Real CLI raw-identity edge controls and --all Markdown parity with released0.11.1. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { requireJsonRefusal } from '../cli-reports/report-checks.ts';
import { configFile } from '../cli-reports/scope-fixtures.ts';
import { requireDebugSignature } from '../rulebook/rulebook-amendment.ts';
import {
	arrayValue,
	objectValue,
	readObject,
	parseObject,
	writeJson,
	runSequence,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import { requireComparableBytes } from '../rulebook/rulebook-output-comparison.ts';
import { declaredTarget, releasedTarget, RulebookProcess } from '../rulebook/rulebook-process.ts';
import { requireSignedArtifact } from './capture-oracle.ts';
import { edgeCases, prepareEdgeFixture, type EdgeFixture } from './git-first-fixtures.ts';
import { preparedSelectedFiles } from './prepared-oracle.ts';

const schema = z.object({
	binary: z.string(),
	'source-root': z.string(),
	commit: z.string().regex(/^[a-f0-9]{40}$/),
	sha256: z.string().regex(/^[a-f0-9]{64}$/),
	'baseline-binary': z.string(),
	'baseline-sha256': z.string().regex(/^[a-f0-9]{64}$/),
	evidence: z.string(),
	case: z.array(z.enum(edgeCases)).default([...edgeCases]),
});
function requireEdge(report: JsonObject, fixture: EdgeFixture, version: string): void {
	assert.equal(report['version'], version);
	assert.equal(report['mode'], 'prepare');
	assert.equal(objectValue(report['usage'])['model_requests'], 0);
	if (fixture.encodingRefusalPath) {
		assert.equal(objectValue(report['status'])['kind'], 'invalid');
		requireJsonRefusal(report, fixture.root, {
			path: fixture.encodingRefusalPath,
			reason: /UnsupportedEncoding/,
		});
		return;
	}
	assert.equal(objectValue(report['status'])['kind'], 'prepared');
	assert.deepEqual(
		preparedSelectedFiles(report),
		fixture.expectedSelectedFiles.toSorted(),
		'Changed checked-file selection differs from authored edge oracle',
	);
	assert.equal(
		arrayValue(report['requests']).length,
		0,
		'Mechanical edge invented a model request',
	);
	for (const result of arrayValue(report['results']).map(objectValue))
		assert.equal(result['outcome'], 'pass');
	assert.deepEqual(
		objectValue(report['scope'])['widened'],
		fixture.name === 'changed-symlink' ? { kind: 'config_changed' } : null,
		'Raw rulebook-link change did not preserve config-dependency widening',
	);
}
export async function observeEdges(options: z.infer<typeof schema>): Promise<void> {
	const target = declaredTarget({
		binary: options.binary,
		source: options['source-root'],
		commit: options.commit,
		digest: options.sha256,
	});
	const baseline = releasedTarget(options['baseline-binary'], options['baseline-sha256'], '0.11.1');
	requireDebugSignature(target.binary);
	requireSignedArtifact(baseline.binary);
	const evidence = path.resolve(options.evidence);
	filesystem.mkdirSync(evidence);
	const fixturesRoot = path.join(evidence, 'fixtures');
	filesystem.mkdirSync(fixturesRoot);
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	let pass: JsonObject | undefined;
	try {
		await runSequence(options.case, async (name) => {
			const fixture = prepareEdgeFixture({
				root: path.join(fixturesRoot, name),
				endpoint: observer.endpoint,
				name,
			});
			writeJson(path.join(evidence, `${name}.input.json`), fixture);
			await observer.run({
				root: fixture.root,
				name: `${name}-check`,
				arguments: ['lint', '--config', configFile, '--check'],
				expectedExit: 0,
			});
			const reports: JsonObject[] = [];
			await runSequence(
				[
					{ name: 'current', target, version: '0.12.0' },
					{ name: 'released', target: baseline, version: '0.11.1' },
				],
				async (subject) => {
					const file = path.join(evidence, `${name}-${subject.name}.json`);
					const result = await observer.run({
						root: fixture.root,
						target: subject.target,
						name: `${name}-${subject.name}`,
						arguments: [
							'lint',
							'--config',
							configFile,
							'--since',
							fixture.base,
							'--format',
							'json',
							'--prepare',
							file,
						],
						expectedExit: fixture.encodingRefusalPath ? 2 : 6,
					});
					assert.equal(result.captures.length, 0);
					const report = readObject(file);
					requireEdge(report, fixture, subject.version);
					reports.push(report);
				},
			);
			const current = reports[0];
			const released = reports[1];
			assert.ok(current && released);
			if (!fixture.encodingRefusalPath) {
				assert.deepEqual(current['results'], released['results']);
				assert.deepEqual(current['requests'], released['requests']);
			}
			if (['staged-only', 'unstaged-only', 'untracked'].includes(name)) {
				const file = path.join(evidence, `${name}-default.json`);
				await observer.run({
					root: fixture.root,
					name: `${name}-default`,
					arguments: ['lint', '--config', configFile, '--format', 'json', '--prepare', file],
					expectedExit: 6,
				});
				requireEdge(readObject(file), fixture, '0.12.0');
			}
			if (name === 'unstaged-only') {
				await runSequence(['ordinary', 'prepare'] as const, async (mode) => {
					const outputs: Buffer[] = [];
					const saved: Buffer[] = [];
					await runSequence(
						[
							{ name: 'released', target: baseline },
							{ name: 'current', target },
						],
						async (subject) => {
							const file = path.join(evidence, `all-${mode}-${subject.name}.json`);
							const result = await observer.run({
								root: fixture.root,
								target: subject.target,
								name: `all-${mode}-${subject.name}`,
								arguments: [
									'lint',
									'--config',
									configFile,
									'--all',
									'--format',
									'json',
									mode === 'prepare' ? '--prepare' : '--report',
									file,
								],
								expectedExit: mode === 'prepare' ? 6 : 0,
							});
							const report = parseObject(result.stdout.toString('utf8'));
							assert.equal(arrayValue(report['results']).length, 2);
							outputs.push(result.stdout);
							saved.push(filesystem.readFileSync(file));
						},
					);
					const before = outputs[0];
					const after = outputs[1];
					const savedBefore = saved[0];
					const savedAfter = saved[1];
					assert.ok(before && after && savedBefore && savedAfter);
					const comparison = { baselineVersion: '0.11.1', candidateVersion: '0.12.0' };
					requireComparableBytes({ ...comparison, before, after, format: 'json' });
					requireComparableBytes({
						...comparison,
						before: savedBefore,
						after: savedAfter,
						format: 'report',
					});
				});
			}
		});
		assert.equal(observer.captures.length, 0);
		pass = {
			target: { ...target },
			baseline: { ...baseline },
			edges: options.case,
			default_scope_edges: options.case.filter((name) =>
				['staged-only', 'unstaged-only', 'untracked'].includes(name),
			),
			all_parity_observed: options.case.includes('unstaged-only'),
			http_requests: 0,
			native_joins_required: [
				'Actual capture acquisitions and raw changed sets via declared public API consumer.',
				'Changed/unchanged mutable source verification, hashing byte budgets, Full-context default and gitlink semantics.',
			],
			limits: [
				'The CLI reports selected checked files and exact raw linked-piece evidence; API joins decide acquisition counts/deleted raw paths.',
				'Markdown --all parity here is the two-document mechanical fixture, not general inference or release proof.',
			],
		};
	} finally {
		await observer.close();
	}
	assert.ok(pass);
	writeJson(path.join(evidence, 'pass.json'), pass);
}
if (process.argv[1] === import.meta.filename) {
	const { values } = parseArgs({
		options: {
			help: { type: 'boolean' },
			prepare: { type: 'boolean' },
			case: { type: 'string', multiple: true },
			...Object.fromEntries(
				[
					'binary',
					'source-root',
					'commit',
					'sha256',
					'baseline-binary',
					'baseline-sha256',
					'evidence',
				].map((name) => [name, { type: 'string' as const }]),
			),
		},
	});
	if (values.help)
		process.stdout.write(
			'Raw-identity capture edges through the real signed CLI.\n--binary PATH --source-root CHECKOUT --commit FULL_SHA --sha256 DIGEST --baseline-binary RELEASED_0.11.1 --baseline-sha256 DIGEST --evidence NEW_DIRECTORY [--case NAME ...]\n--prepare --evidence NEW_DIRECTORY only prepares all owned Git states, with product_observed:false.\nChecks staged/unstaged/untracked/deleted/renamed/mode/binary/symlink/ignored and five Git-diff blind spots, preserved default scope and exact --all JSON/report/prepare parity except root version. Native joins still own mutation/byte-budget/capture-count proof. Keyless isolated HOME; owned bounded children/listener.\n',
		);
	else if (values.prepare) {
		const evidence = path.resolve(z.object({ evidence: z.string() }).parse(values).evidence);
		filesystem.mkdirSync(evidence);
		const names = values.case ? z.array(z.enum(edgeCases)).parse(values.case) : edgeCases;
		const fixtures = names.map((name) =>
			prepareEdgeFixture({
				root: path.join(evidence, name),
				endpoint: 'http://127.0.0.1:9/systemone',
				name,
			}),
		);
		writeJson(path.join(evidence, 'prepared-fixtures.json'), { product_observed: false, fixtures });
	} else await observeEdges(schema.parse(values));
}
