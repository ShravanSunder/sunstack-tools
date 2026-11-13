/** Real config/default CLI proof: released parity, parser-located witness and standard controls. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { requireSignedArtifact } from '../git-first/capture-oracle.ts';
import {
	requireOutsideInput,
	requireSameFrozenInput,
	snapshotFrozenInput,
} from '../git-first/frozen-inputs.ts';
import {
	arrayValue,
	objectValue,
	readObject,
	runSequence,
	writeJson,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import {
	declaredTarget,
	releasedTarget,
	RulebookProcess,
	type RunCase,
	type ProcessResult,
} from '../rulebook/rulebook-process.ts';
import {
	configuredBytes,
	requireConfiguredParity,
	requireVersionOnly,
} from './configured-parity.ts';
import { requireAnchors, requireAsking, requireOutline } from './control-outcomes.ts';
import {
	corpusGroups,
	prepareCopies,
	prepareControls,
	prepareLocator,
	requireInputFiles,
	type CorpusGroup,
} from './convention-fixtures.ts';
import { requireDefaultAnchors } from './default-anchors.ts';
import { requireDefaultProjection } from './default-projection.ts';
import { labelSpans, neutralizeLabels } from './label-witness.ts';

const optionsSchema = z.object({
	phase: z.enum(['baseline', 'candidate']),
	evidence: z.string(),
	'input-root': z.string(),
	common: z.string(),
	'baseline-binary': z.string(),
	'baseline-sha256': z.string().regex(/^[a-f0-9]{64}$/),
	'source-root': z.string(),
	binary: z.string().optional(),
	commit: z.string().optional(),
	sha256: z.string().optional(),
	'baseline-run': z.string().optional(),
});

async function runCase(
	observer: RulebookProcess,
	options: RunCase,
	output?: string,
): Promise<ProcessResult> {
	const space = filesystem.statfsSync(observer.evidence);
	assert.ok(space.bavail * space.bsize > 6 * 1024 ** 3, 'Disk below6GiB hold');
	try {
		return await observer.run(options);
	} catch (error) {
		const receipt = path.join(observer.evidence, options.name, 'process.json');
		if (!filesystem.existsSync(receipt) || readObject(receipt)['timed_out'] !== true) throw error;
		if (output && filesystem.existsSync(output)) {
			filesystem.copyFileSync(
				output,
				path.join(observer.evidence, options.name, 'report.partial.json'),
			);
			filesystem.unlinkSync(output);
		}
		// C7 permits exactly one unchanged isolated retry, preserving both receipts.
		const result = await observer.run({ ...options, name: `${options.name}-isolated-retry` });
		writeJson(path.join(observer.evidence, `${options.name}-timeout-disposition.json`), {
			classification: 'host_load',
			original: options.name,
			isolated_retry: `${options.name}-isolated-retry`,
			expectation_unchanged: true,
		});
		return result;
	}
}

function setConfig(common: string, group: CorpusGroup, configured: boolean): Buffer {
	const original = filesystem.readFileSync(
		path.join(common, 'original-configs', `${group.name}.json`),
	);
	const bytes = configured ? configuredBytes(original) : original;
	filesystem.writeFileSync(path.join(common, 'projects', group.name, group.config), bytes);
	return bytes;
}

async function prepareReport(options: {
	observer: RulebookProcess;
	root: string;
	config: string;
	name: string;
	output: string;
	exit: number;
}): Promise<Buffer> {
	await runCase(options.observer, {
		root: options.root,
		name: `${options.name}-check`,
		arguments: ['lint', '--root', options.root, '--config', options.config, '--check'],
		expectedExit: 0,
	});
	if (filesystem.existsSync(options.output)) filesystem.unlinkSync(options.output);
	await runCase(
		options.observer,
		{
			root: options.root,
			name: options.name,
			arguments: [
				'lint',
				'--root',
				options.root,
				'--config',
				options.config,
				'--all',
				'--prepare',
				options.output,
			],
			expectedExit: options.exit,
		},
		options.output,
	);
	const bytes = filesystem.readFileSync(options.output);
	const report = objectValue(JSON.parse(bytes.toString('utf8')));
	assert.equal(objectValue(report['usage'])['model_requests'], 0);
	return bytes;
}

async function controls(options: {
	observer: RulebookProcess;
	common: string;
	evidence: string;
	released: boolean;
}): Promise<void> {
	const base = readObject(path.join(options.common, 'controls/original-control-configs.json'));
	await runSequence(
		options.released ? (['restored'] as const) : (['standard', 'restored', 'generic'] as const),
		async (mode) => {
			await runSequence(['outline', 'asking', 'anchors'], async (name) => {
				const root = path.join(options.common, 'controls', name);
				const config = objectValue(structuredClone(base[name]));
				if (!options.released && mode === 'restored') {
					filesystem.writeFileSync(
						path.join(root, 'sunstack.lint.json'),
						configuredBytes(Buffer.from(`${JSON.stringify(config, null, 2)}\n`)),
					);
				} else {
					if (mode === 'generic')
						objectValue(config['evidence'])['html'] = {
							skip_tags: ['custom-hidden'],
							text_attributes: { 'custom-diagram': ['label'] },
						};
					writeJson(path.join(root, 'sunstack.lint.json'), config);
				}
				if (name === 'outline') {
					await runCase(options.observer, {
						root,
						name: `${mode}-${name}-check`,
						arguments: ['lint', '--check'],
						expectedExit: 0,
					});
					const result = await runCase(options.observer, {
						root,
						name: `${mode}-${name}`,
						arguments: ['lint', '--all', '--evidence', 'section'],
						expectedExit: 0,
					});
					requireOutline(objectValue(JSON.parse(result.stdout.toString('utf8'))), mode);
				} else {
					const output = path.join(options.common, 'output', `control-${name}.json`);
					const bytes = await prepareReport({
						observer: options.observer,
						root,
						config: 'sunstack.lint.json',
						name: `${mode}-${name}`,
						output,
						exit: name === 'anchors' ? 3 : 6,
					});
					filesystem.writeFileSync(
						path.join(options.evidence, `control-${mode}-${name}.json`),
						bytes,
					);
					const report = objectValue(JSON.parse(bytes.toString('utf8')));
					if (name === 'asking') requireAsking(report, mode);
					else requireAnchors(report, options.released || mode === 'restored', options.released);
				}
			});
		},
	);
}

export async function observeConfigOnly(options: z.infer<typeof optionsSchema>): Promise<void> {
	const release = releasedTarget(options['baseline-binary'], options['baseline-sha256'], '0.14.1');
	const target =
		options.phase === 'baseline'
			? release
			: declaredTarget({
					binary: z.string().parse(options.binary),
					source: options['source-root'],
					commit: z.string().parse(options.commit),
					digest: z.string().parse(options.sha256),
				});
	requireSignedArtifact(target.binary);
	const input = path.resolve(options['input-root']);
	const common = path.resolve(options.common);
	const evidence = path.resolve(options.evidence);
	for (const root of [input, options['source-root']]) {
		requireOutsideInput(root, common);
		requireOutsideInput(root, evidence);
	}
	filesystem.mkdirSync(evidence);
	const groups = corpusGroups();
	requireInputFiles(input, groups);
	const aiSnapshot = snapshotFrozenInput(
		path.join(input, 'ai-tools'),
		'HEAD',
		'cde8d5cf5b9141a08264cc5d57e3e3a58f0f2aac',
	);
	writeJson(path.join(evidence, 'input-before.json'), aiSnapshot);
	if (options.phase === 'baseline') {
		prepareCopies(input, common, groups);
		prepareControls(path.join(common, 'controls'));
	}
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	const account: JsonObject[] = [];
	try {
		await runSequence(groups, async (group) => {
			const originalConfig = filesystem.readFileSync(
				path.join(common, 'original-configs', `${group.name}.json`),
			);
			const bytes = setConfig(common, group, options.phase === 'candidate');
			const output = path.join(common, 'output', `${group.name}.json`);
			const root = path.join(common, 'projects', group.name);
			const reportBytes = await prepareReport({
				observer,
				root,
				config: group.config,
				name: `${group.name}-restored`,
				output,
				exit: group.expected_exit,
			});
			filesystem.writeFileSync(path.join(evidence, `${group.name}-restored.json`), reportBytes);
			const report = objectValue(JSON.parse(reportBytes.toString('utf8')));
			assert.equal(arrayValue(report['requests']).length, group.expected_requests);
			if (group.name === 'anchors') {
				assert.equal(arrayValue(report['findings']).length, 3);
				assert.equal(arrayValue(report['results']).length, 127);
			}
			if (options.phase === 'candidate')
				requireConfiguredParity({
					before: filesystem.readFileSync(
						path.join(z.string().parse(options['baseline-run']), `${group.name}-restored.json`),
					),
					after: reportBytes,
					originalConfig,
					configuredConfig: bytes,
					configPath: path.join(root, group.config),
				});
			account.push({
				group: group.name,
				requests: group.expected_requests,
				configured_parity: options.phase === 'candidate',
			});
			writeJson(path.join(evidence, 'account.json'), account);
		});
		if (options.phase === 'baseline') {
			await runSequence(
				groups.filter((group) => ['j1', 'sections'].includes(group.name)),
				async (group) => {
					const root = path.join(common, 'projects', group.name);
					const locator = path.join(common, 'locators', group.name);
					const files = group.files
						.filter((file) => file.file.endsWith('.html'))
						.map((file) => file.file);
					prepareLocator(locator, files);
					await runCase(observer, {
						root,
						name: `${group.name}-locator-check`,
						arguments: [
							'lint',
							'--root',
							root,
							'--config',
							path.join(locator, 'sunstack.lint.json'),
							'--check',
						],
						expectedExit: 0,
					});
					const located = await runCase(observer, {
						root,
						name: `${group.name}-locator`,
						arguments: [
							'lint',
							'--root',
							root,
							'--config',
							path.join(locator, 'sunstack.lint.json'),
							'--all',
							'--evidence',
							'label',
						],
						expectedExit: 0,
					});
					const listing = objectValue(JSON.parse(located.stdout.toString('utf8')));
					const mutations: JsonObject[] = [];
					try {
						for (const file of files) {
							const original = filesystem.readFileSync(path.join(input, group.name, file));
							const spans = labelSpans(listing, file, original);
							filesystem.writeFileSync(path.join(root, file), neutralizeLabels(original, spans));
							mutations.push({
								file,
								spans: spans.map((span) => ({ start: span.start, end: span.end, text: span.text })),
							});
						}
						const output = path.join(common, 'output', `${group.name}.json`);
						const witness = await prepareReport({
							observer,
							root,
							config: group.config,
							name: `${group.name}-witness`,
							output,
							exit: group.expected_exit,
						});
						filesystem.writeFileSync(path.join(evidence, `${group.name}-witness.json`), witness);
						writeJson(path.join(evidence, `${group.name}-witness-mutations.json`), mutations);
					} finally {
						for (const file of files)
							filesystem.copyFileSync(path.join(input, group.name, file), path.join(root, file));
					}
				},
			);
		} else {
			await runSequence(groups, async (group) => {
				setConfig(common, group, false);
				const root = path.join(common, 'projects', group.name);
				const output = path.join(common, 'output', `${group.name}.json`);
				const bytes = await prepareReport({
					observer,
					root,
					config: group.config,
					name: `${group.name}-standard`,
					output,
					exit: group.name === 'anchors' ? 3 : group.expected_exit,
				});
				filesystem.writeFileSync(path.join(evidence, `${group.name}-standard.json`), bytes);
				const before = filesystem.readFileSync(
					path.join(z.string().parse(options['baseline-run']), `${group.name}-restored.json`),
				);
				if (group.name === 'ai-tools') requireVersionOnly(before, bytes);
				else if (group.name === 'anchors')
					writeJson(
						path.join(evidence, 'default-anchor-inventory.json'),
						requireDefaultAnchors(before, bytes, root),
					);
				else
					requireDefaultProjection(
						before,
						filesystem.readFileSync(
							path.join(z.string().parse(options['baseline-run']), `${group.name}-witness.json`),
						),
						bytes,
					);
			});
		}
		await controls({ observer, common, evidence, released: options.phase === 'baseline' });
		assert.equal(observer.captures.length, 0);
	} finally {
		try {
			await observer.close();
		} finally {
			requireInputFiles(input, groups);
			const after = snapshotFrozenInput(path.join(input, 'ai-tools'), 'HEAD', aiSnapshot.head);
			writeJson(path.join(evidence, 'input-after.json'), after);
			requireSameFrozenInput(aiSnapshot, after);
		}
	}
	writeJson(path.join(evidence, 'captured.json'), {
		target,
		phase: options.phase,
		account,
		owned_http_requests: 0,
		input_unchanged: true,
		boundaries:
			'C4 configured snapshot/version only; C5/C6 default witness; C7 isolated timeout retry',
		outstanding: ['Read native API/schema/refusal/product-name proof.'],
	});
}

if (process.argv[1] === import.meta.filename) {
	const { values } = parseArgs({
		options: {
			...Object.fromEntries(
				Object.keys(optionsSchema.shape).map((name) => [name, { type: 'string' as const }]),
			),
			help: { type: 'boolean' },
		},
	});
	if (values.help)
		process.stdout.write(
			'Observe0.15config/default contract, keyless and serial. --phase baseline|candidate --baseline-binary PATH --baseline-sha256 SHA --source-root CHECKOUT --input-root FROZEN_CORPUS --common OWNED_COMMON_PATH --evidence NEW_DIRECTORY [--binary PATH --commit FULL_HEAD --sha256 SHA --baseline-run DIRECTORY]. Original inputs/hash manifest are in real-corpus.json. C4 exact config/version exception; C5/C6 parser-located attribute-neutralized released witness; C7 one unchanged isolated timeout retry. No builds, installs, provider or release actions.\n',
		);
	else await observeConfigOnly(optionsSchema.parse(values));
}
