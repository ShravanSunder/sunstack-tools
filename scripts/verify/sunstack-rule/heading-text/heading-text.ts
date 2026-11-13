/** Keyless signed-CLI journey: all P3 source headings, exact legacy parity and literal controls. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { requireSignedArtifact } from '../git-first/capture-oracle.ts';
import {
	requireOutsideInput,
	snapshotFrozenInput,
	requireSameFrozenInput,
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
import { prepareControls, requireControl } from './controls.ts';
import {
	groups,
	headingConfig,
	prepareCopies,
	projectConfigPath,
	requireInputs,
	savedReport,
} from './fixtures.ts';
import { headings, requireHeadingReport } from './heading-oracle.ts';
import { requireLegacyParity } from './parity.ts';

const optionsSchema = z.object({
	phase: z.enum(['baseline', 'candidate']),
	evidence: z.string(),
	common: z.string(),
	'input-root': z.string(),
	'baseline-binary': z.string(),
	'baseline-sha256': z.string(),
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
	assert.ok(space.bavail * space.bsize > 6 * 1024 ** 3, 'Disk below6GiB');
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
		const result = await observer.run({ ...options, name: `${options.name}-isolated-retry` });
		writeJson(path.join(observer.evidence, `${options.name}-timeout-disposition.json`), {
			classification: 'host_load',
			expectation_unchanged: true,
		});
		return result;
	}
}
async function prepare(options: {
	observer: RulebookProcess;
	root: string;
	config: string;
	output: string;
	name: string;
}): Promise<Buffer> {
	const { observer, root, config, output, name } = options;
	await runCase(observer, {
		root,
		name: `${name}-check`,
		arguments: ['lint', '--root', root, '--config', config, '--check'],
		expectedExit: 0,
	});
	if (filesystem.existsSync(output)) filesystem.unlinkSync(output);
	await runCase(
		observer,
		{
			root,
			name,
			arguments: ['lint', '--root', root, '--config', config, '--all', '--prepare', output],
			expectedExit: 6,
		},
		output,
	);
	const bytes = filesystem.readFileSync(output);
	assert.equal(objectValue(savedReport(bytes)['usage'])['model_requests'], 0);
	return bytes;
}
export async function observeHeadingText(options: z.infer<typeof optionsSchema>): Promise<void> {
	const release = releasedTarget(options['baseline-binary'], options['baseline-sha256'], '0.15.0');
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
	requireInputs(input);
	const snapshot = snapshotFrozenInput(
		path.join(input, 'ai-tools'),
		'HEAD',
		'cde8d5cf5b9141a08264cc5d57e3e3a58f0f2aac',
	);
	writeJson(path.join(evidence, 'input-before.json'), snapshot);
	if (options.phase === 'baseline') prepareCopies(input, common);
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	const account: JsonObject[] = [];
	await observer.start();
	try {
		await runSequence(groups(), async (group) => {
			const root = path.join(common, 'projects', group.name);
			const output = path.join(common, 'output', `${group.name}.json`);
			const originalConfig = filesystem.readFileSync(
				path.join(common, 'configs', `${group.name}.json`),
			);
			const usingHeading =
				options.phase === 'candidate' && (group.name === 'html' || group.name === 'markdown');
			filesystem.writeFileSync(
				projectConfigPath(common, group),
				usingHeading ? headingConfig(originalConfig) : originalConfig,
			);
			const bytes = await prepare({
				observer,
				root,
				config: projectConfigPath(common, group),
				output,
				name: group.name,
			});
			filesystem.writeFileSync(path.join(evidence, `${group.name}.json`), bytes);
			const report = savedReport(bytes);
			if (options.phase === 'baseline') {
				assert.equal(arrayValue(report['requests']).length, group.expected_baseline_requests);
				account.push({ group: group.name, requests: group.expected_baseline_requests });
			} else {
				const before = filesystem.readFileSync(
					path.join(z.string().parse(options['baseline-run']), `${group.name}.json`),
				);
				if (group.name === 'html' || group.name === 'markdown') {
					assert.equal(report['version'], '0.15.1');
					const html = group.name === 'html';
					const result = requireHeadingReport({
						before: savedReport(before),
						after: report,
						headings: headings(group.name),
						root,
						expectedCandidates: html ? 354 : 236,
						expectedEntries: html ? 820 : 570,
					});
					if (html) {
						assert.equal(result['changed_scalar_entries'], 190);
						assert.deepEqual(result['affected_files'], [
							'pages/page-06.html',
							'pages/page-07.html',
							'pages/page-11.html',
							'pages/page-12.html',
						]);
					}
					account.push({ group: group.name, ...result });
				} else {
					requireLegacyParity(before, bytes);
					account.push({
						group: group.name,
						version_only_parity: true,
						requests: arrayValue(report['requests']).length,
					});
				}
			}
			writeJson(path.join(evidence, 'account.json'), account);
		});
		if (options.phase === 'candidate') {
			prepareControls(common);
			await runSequence(['html', 'markdown'] as const, async (format) => {
				const root = path.join(common, `control-${format}`);
				const bytes = await prepare({
					observer,
					root,
					config: 'sunstack.lint.json',
					output: path.join(common, 'output', `control-${format}.json`),
					name: `control-${format}`,
				});
				filesystem.writeFileSync(path.join(evidence, `control-${format}.json`), bytes);
				requireControl(savedReport(bytes), format);
			});
		}
		assert.equal(observer.captures.length, 0);
	} finally {
		try {
			await observer.close();
		} finally {
			requireInputs(input);
			const after = snapshotFrozenInput(path.join(input, 'ai-tools'), 'HEAD', snapshot.head);
			writeJson(path.join(evidence, 'input-after.json'), after);
			requireSameFrozenInput(snapshot, after);
		}
	}
	writeJson(path.join(evidence, 'captured.json'), {
		target,
		phase: options.phase,
		account,
		owned_http_requests: 0,
		input_unchanged: true,
		outstanding: [
			'Read offered native/admission/schema proof and exact source joins.',
			'Read all raw child receipts before handback.',
		],
	});
}
if (process.argv[1] === import.meta.filename) {
	const { values } = parseArgs({
		options: {
			...Object.fromEntries(
				Object.keys(optionsSchema.shape).map((key) => [key, { type: 'string' as const }]),
			),
			help: { type: 'boolean' },
		},
	});
	if (values.help)
		process.stdout.write(
			'Heading-text0.15.1: source-derived HTML/Markdown P3 heading paths and own-heading controls; legacy reports version-only versus released0.15.0. --phase baseline|candidate --baseline-binary PATH --baseline-sha256 SHA --source-root CHECKOUT --input-root FROZEN_INPUTS --common NEW_OWNED_COMMON --evidence NEW_DIRECTORY [--binary PATH --commit FULL_HEAD --sha256 SHA --baseline-run DIRECTORY]. Existing Node dependencies, no install/build/provider. Serial CLI, one unchanged isolated retry only on timeout; stop disk<6GiB. Inputs/oracles beside driver, adopted commission Corrections1–5.\n',
		);
	else await observeHeadingText(optionsSchema.parse(values));
}
