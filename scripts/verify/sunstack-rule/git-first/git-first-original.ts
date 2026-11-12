/** Original read-only --since preparation, exact full-capture oracle and raw API parity. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { requireDebugSignature } from '../rulebook/rulebook-amendment.ts';
import {
	arrayValue,
	objectValue,
	readObject,
	writeJson,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import { declaredTarget, releasedTarget, RulebookProcess } from '../rulebook/rulebook-process.ts';
import { runCaptureOracle, requireSignedArtifact } from './capture-oracle.ts';
import {
	snapshotFrozenInput,
	requireSameFrozenInput,
	requireOutsideInput,
} from './frozen-inputs.ts';
import {
	fullOracleConfig,
	requireSamePreparedInputs,
	preparedSelectedFiles,
	preparedOwnerFiles,
} from './prepared-oracle.ts';

const schema = z.object({
	binary: z.string(),
	'source-root': z.string(),
	commit: z.string().regex(/^[a-f0-9]{40}$/),
	sha256: z.string().regex(/^[a-f0-9]{64}$/),
	'capture-binary': z.string(),
	'capture-sha256': z.string().regex(/^[a-f0-9]{64}$/),
	'baseline-binary': z.string(),
	'baseline-sha256': z.string().regex(/^[a-f0-9]{64}$/),
	'input-root': z.string(),
	'input-head': z.string().regex(/^[a-f0-9]{40}$/),
	base: z.string(),
	config: z.string(),
	'tracked-count': z.coerce.number().int().positive().default(1397),
	evidence: z.string(),
});
export async function observeOriginal(options: z.infer<typeof schema>): Promise<void> {
	const target = declaredTarget({
		binary: options.binary,
		source: options['source-root'],
		commit: options.commit,
		digest: options.sha256,
	});
	const native = declaredTarget({
		binary: options['capture-binary'],
		source: target.source,
		commit: target.commit,
		digest: options['capture-sha256'],
	});
	const baseline = releasedTarget(options['baseline-binary'], options['baseline-sha256'], '0.11.1');
	requireDebugSignature(target.binary);
	requireSignedArtifact(baseline.binary);
	const input = snapshotFrozenInput(options['input-root'], options.base, options['input-head']);
	assert.equal(input.trackedCount, options['tracked-count']);
	const evidence = path.resolve(options.evidence);
	requireOutsideInput(input.root, evidence);
	filesystem.mkdirSync(evidence);
	writeJson(path.join(evidence, 'input-before.json'), input);
	const configFile = path.resolve(input.root, options.config);
	const config = readObject(configFile);
	const fullFile = path.join(evidence, 'full-oracle.config.json');
	requireOutsideInput(input.root, fullFile);
	writeJson(fullFile, fullOracleConfig(config, configFile, 10000));
	writeJson(path.join(evidence, 'oracle-construction.json'), {
		root: input.root,
		original_config: configFile,
		copy: fullFile,
		raised_limit: { max_files: 10000 },
		path_rebasing:
			'Only rulebook markdown/rules and ast_grep paths made absolute from original config directory; all other values retained, no endpoint rewrite.',
		versions: { candidate: '0.12.0', full: '0.11.1' },
	});
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	let pass: JsonObject | undefined;
	try {
		await observer.run({
			root: input.root,
			name: 'candidate-check',
			arguments: ['lint', '--config', options.config, '--check'],
			expectedExit: 0,
		});
		const api = runCaptureOracle({
			target: native,
			input,
			directory: path.join(evidence, 'native'),
		});
		assert.ok(
			api.changedFiles < input.trackedCount,
			'Original capture did not avoid unchanged acquisitions',
		);
		assert.ok(api.fullFiles > 2000, 'Raised-cap oracle did not cover the original limit boundary');
		const oldFile = path.join(evidence, 'baseline-default-cap.json');
		await observer.run({
			root: input.root,
			target: baseline,
			name: 'baseline-default-cap',
			arguments: [
				'lint',
				'--config',
				options.config,
				'--since',
				options.base,
				'--prepare',
				oldFile,
			],
			expectedExit: 3,
		});
		const old = readObject(oldFile);
		assert.equal(old['version'], '0.11.1');
		assert.equal(objectValue(old['status'])['kind'], 'incomplete');
		assert.ok(
			arrayValue(old['diagnostics'])
				.map(objectValue)
				.some(
					(entry) =>
						typeof entry['message'] === 'string' && entry['message'].includes('max_files=2000'),
				),
			'Comparator did not reach the intended file-acquisition limit',
		);
		const fullOutput = path.join(evidence, 'full-prepared.json');
		await observer.run({
			root: input.root,
			target: baseline,
			name: 'released-full-oracle',
			arguments: [
				'lint',
				'--root',
				input.root,
				'--config',
				fullFile,
				'--since',
				options.base,
				'--prepare',
				fullOutput,
			],
			expectedExit: 6,
		});
		const currentOutput = path.join(evidence, 'candidate-prepared.json');
		await observer.run({
			root: input.root,
			name: 'candidate-original-command',
			arguments: [
				'lint',
				'--config',
				options.config,
				'--since',
				options.base,
				'--prepare',
				currentOutput,
			],
			expectedExit: 6,
		});
		const current = readObject(currentOutput);
		const full = readObject(fullOutput);
		requireSamePreparedInputs({
			candidate: current,
			full,
			candidateVersion: '0.12.0',
			fullVersion: '0.11.1',
		});
		assert.equal(
			objectValue(current['scope'])['widened'],
			null,
			'Original config dependencies changed; revisit oracle rebasing',
		);
		assert.equal(objectValue(full['scope'])['widened'], null);
		assert.equal(observer.captures.length, 0);
		pass = {
			target: { ...target },
			native_target: { ...native },
			baseline: { ...baseline },
			input_head: input.head,
			input_base: input.base,
			tracked_files: input.trackedCount,
			raw_changed_paths: api.paths,
			candidate_capture_acquisitions: api.changedFiles,
			full_capture_acquisitions: api.fullFiles,
			selected_files: preparedSelectedFiles(current),
			represented_owner_files: preparedOwnerFiles(current),
			prepared_requests: arrayValue(current['requests']).length,
			http_requests: 0,
			proof:
				'Current CLI and released full CLI requests/owners match; raw paths/payloads/spans and counts independently exercised through the declared immutable public-API consumer.',
			limits: [
				'No new model dispatch or inference-quality claim.',
				'Loopback is not an egress witness for original saved endpoint settings.',
				'The comparator and native artifact provenance must be joined with their declared source/digest receipts.',
			],
		};
	} finally {
		try {
			await observer.close();
		} finally {
			const after = snapshotFrozenInput(input.root, options.base, input.head);
			writeJson(path.join(evidence, 'input-after.json'), after);
			requireSameFrozenInput(input, after);
		}
	}
	assert.ok(pass);
	writeJson(path.join(evidence, 'pass.json'), pass);
}
if (process.argv[1] === import.meta.filename) {
	const names = [
		'binary',
		'source-root',
		'commit',
		'sha256',
		'capture-binary',
		'capture-sha256',
		'baseline-binary',
		'baseline-sha256',
		'input-root',
		'input-head',
		'base',
		'config',
		'tracked-count',
		'evidence',
	];
	const { values } = parseArgs({
		options: {
			help: { type: 'boolean' },
			...Object.fromEntries(names.map((name) => [name, { type: 'string' as const }])),
		},
	});
	if (values.help)
		process.stdout.write(
			'Read-only original --since preparation and same-input full-capture oracle.\n--binary PATH --source-root CHECKOUT --commit FULL_SHA --sha256 DIGEST --capture-binary IMMUTABLE_TEST_EXECUTABLE --capture-sha256 DIGEST --baseline-binary RELEASED_0.11.1 --baseline-sha256 DIGEST --input-root FROZEN_CHECKOUT --input-head FULL_SHA --base REV --config RELATIVE_CONFIG --evidence NEW_OUTSIDE_DIRECTORY [--tracked-count N]\nInvokes exactly one permanent public-API parity test without compiling, records actual capture acquisitions/raw paths, checks old default-cap exit3 then raised10000-file full oracle versus current prepare exit6. Hashes every original file/link and Git identity before/after; never edits the frozen input. Keyless, bounded children/listener; no model or release acceptance.\n',
		);
	else await observeOriginal(schema.parse(values));
}
