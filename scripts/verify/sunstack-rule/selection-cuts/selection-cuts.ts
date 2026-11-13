/** Signed CLI proof of selection cut-field wire additions, exact text preservation and rule parity. */
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
import { cases, prepareControls, requireControlCounts } from './control-cases.ts';
import { requireCutParity } from './cut-oracle.ts';
import { corpusGroups, prepareCorpus, projectConfigPath, requireInputs } from './fixtures.ts';

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
	assert.equal(
		objectValue(objectValue(JSON.parse(bytes.toString('utf8')))['usage'])['model_requests'],
		0,
	);
	return bytes;
}
export async function observeSelectionCuts(options: z.infer<typeof optionsSchema>): Promise<void> {
	const release = releasedTarget(options['baseline-binary'], options['baseline-sha256'], '0.15.1');
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
	if (options.phase === 'baseline') prepareCorpus(input, common);
	prepareControls(common);
	const expected = objectValue(
		readObject(path.join(import.meta.dirname, 'real-corpus.json'))['expected'],
	);
	const account: JsonObject[] = [];
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	try {
		await runSequence(corpusGroups(), async (group) => {
			const root = path.join(common, 'projects', group.name);
			const bytes = await prepare({
				observer,
				root,
				config: projectConfigPath(common, group),
				output: path.join(common, 'output', `${group.name}.json`),
				name: group.name,
			});
			filesystem.writeFileSync(path.join(evidence, `${group.name}.json`), bytes);
			const report = objectValue(JSON.parse(bytes.toString('utf8')));
			assert.equal(arrayValue(report['requests']).length, group.expected_baseline_requests);
			if (options.phase === 'candidate') {
				const before = filesystem.readFileSync(
					path.join(z.string().parse(options['baseline-run']), `${group.name}.json`),
				);
				const result = requireCutParity({ before, after: bytes, markerEquivalence: true });
				if (group.name === 'html' || group.name === 'markdown') {
					const counts = objectValue(expected[group.name]);
					for (const [actual, authored] of [
						['candidate_cut', 'cut'],
						['candidate_uncut', 'uncut'],
						['shared_cut', 'shared_cut'],
						['shared_uncut', 'shared_uncut'],
					] as const)
						assert.equal(result[actual], counts[authored]);
				}
				account.push({ group: group.name, ...result });
			} else account.push({ group: group.name, requests: arrayValue(report['requests']).length });
			writeJson(path.join(evidence, 'account.json'), account);
		});
		await runSequence(cases, async (control) => {
			const bytes = await prepare({
				observer,
				root: path.join(common, control.name),
				config: 'sunstack.lint.json',
				output: path.join(common, 'output', `${control.name}.json`),
				name: control.name,
			});
			filesystem.writeFileSync(path.join(evidence, `${control.name}.json`), bytes);
			if (options.phase === 'candidate') {
				const before = filesystem.readFileSync(
					path.join(z.string().parse(options['baseline-run']), `${control.name}.json`),
				);
				const result = requireCutParity({ before, after: bytes, markerEquivalence: false });
				requireControlCounts(result, control, objectValue(JSON.parse(before.toString('utf8'))));
				account.push({ control: control.name, ...result });
			}
			writeJson(path.join(evidence, 'account.json'), account);
		});
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
			'Inspect native replay/request-boundary proof and exact source joins.',
			'Read raw receipts before handback.',
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
			'Selection cut fields0.16: exact P3/controlled selection-state additions+hashes and unchanged rule reports vs released0.15.1. --phase baseline|candidate --baseline-binary PATH --baseline-sha256 SHA --source-root CHECKOUT --input-root FROZEN_INPUTS --common NEW_OWNED_COMMON --evidence NEW_DIRECTORY [--binary PATH --commit FULL_HEAD --sha256 SHA --baseline-run DIRECTORY]. Existing Node dependencies; no build/install/provider/release. Serial bounded CLI, one unchanged isolated retry only for timeout, stop disk<6GiB. Uses headed0.15.1 P3 config and inherited frozen source manifest.\n',
		);
	else await observeSelectionCuts(optionsSchema.parse(values));
}
