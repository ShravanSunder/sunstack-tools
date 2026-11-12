/** Real rule-author usability rows on an untouched detached input and keyless negatives. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { requireSignedArtifact } from '../git-first/capture-oracle.ts';
import {
	snapshotFrozenInput,
	requireSameFrozenInput,
	requireOutsideInput,
} from '../git-first/frozen-inputs.ts';
import { requireDebugSignature } from '../rulebook/rulebook-amendment.ts';
import {
	arrayValue,
	objectValue,
	readObject,
	writeJson,
	runSequence,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import { requireComparableBytes } from '../rulebook/rulebook-output-comparison.ts';
import { declaredTarget, releasedTarget, RulebookProcess } from '../rulebook/rulebook-process.ts';
import {
	requireScope,
	requireProblemSummary,
	requirePreparedSummary,
	requireCredentialRefusal,
	requireNoSelection,
	requireConfigReason,
	requireHelp,
} from './checks.ts';
import { askingFixture, requireConfigFree } from './inputs.ts';

const schema = z.object({
	binary: z.string(),
	'source-root': z.string(),
	commit: z.string().regex(/^[a-f0-9]{40}$/),
	sha256: z.string().regex(/^[a-f0-9]{64}$/),
	'baseline-binary': z.string(),
	'baseline-sha256': z.string().regex(/^[a-f0-9]{64}$/),
	'input-root': z.string(),
	'input-head': z.string().regex(/^[a-f0-9]{40}$/),
	config: z.string(),
	'one-file': z.string(),
	evidence: z.string(),
});
export async function observeUsability(options: z.infer<typeof schema>): Promise<void> {
	const target = declaredTarget({
		binary: options.binary,
		source: options['source-root'],
		commit: options.commit,
		digest: options.sha256,
	});
	const baseline = releasedTarget(options['baseline-binary'], options['baseline-sha256'], '0.12.0');
	requireDebugSignature(target.binary);
	requireSignedArtifact(baseline.binary);
	const input = snapshotFrozenInput(options['input-root'], 'HEAD', options['input-head']);
	const evidence = path.resolve(options.evidence);
	requireOutsideInput(input.root, evidence);
	filesystem.mkdirSync(evidence);
	writeJson(path.join(evidence, 'input-before.json'), input);
	const empty = path.join(evidence, 'no-config');
	filesystem.mkdirSync(empty);
	requireConfigFree(empty);
	assert.ok(
		!filesystem.existsSync(path.join(input.root, 'nope.json')),
		'Missing-config row is not missing',
	);
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	let pass: JsonObject | undefined;
	try {
		await observer.run({
			root: input.root,
			name: 'readiness-check',
			arguments: ['lint', '--config', options.config, '--check'],
			expectedExit: 0,
		});
		const common = ['lint', '--config', options.config];
		await runSequence(
			[
				{ name: 'default', extra: [] },
				{ name: 'since-head', extra: ['--since', 'HEAD'] },
			],
			async (row) => {
				await runSequence(
					[
						{ name: 'released', target: baseline, exit: 2 },
						{ name: 'candidate', target, exit: 0 },
					],
					async (subject) => {
						const file = path.join(evidence, `${row.name}-${subject.name}.json`);
						const result = await observer.run({
							root: input.root,
							target: subject.target,
							name: `${row.name}-${subject.name}`,
							arguments: [...common, ...row.extra, '--report', file],
							expectedExit: subject.exit,
						});
						const report = readObject(file);
						assert.equal(report['version'], subject.name === 'released' ? '0.12.0' : '0.12.1');
						assert.equal(result.captures.length, 0);
						if (subject.name === 'candidate') {
							requireNoSelection(report);
							requireScope(result.stdout.toString('utf8'), false);
							requireProblemSummary(result.stdout.toString('utf8'), 'No checks selected', 0);
						} else requireCredentialRefusal(report);
					},
				);
			},
		);
		await runSequence(
			[
				{ name: 'since-head-prepare', extra: ['--since', 'HEAD'], requests: 0, results: null },
				{ name: 'named-prepare', extra: [options['one-file']], requests: 1, results: 3 },
				{ name: 'default-prepare-parity', extra: [], requests: 0, results: null },
				{ name: 'all-prepare-parity', extra: ['--all'], requests: null, results: null },
			],
			async (row) => {
				const saved: Buffer[] = [];
				const outputs: Buffer[] = [];
				await runSequence(
					[
						{ name: 'released', target: baseline, version: '0.12.0' },
						{ name: 'candidate', target, version: '0.12.1' },
					],
					async (subject) => {
						const file = path.join(evidence, `${row.name}-${subject.name}.json`);
						const args = [
							...common,
							...row.extra,
							'--prepare',
							file,
							...(row.name === 'all-prepare-parity' ? ['--format', 'json'] : []),
						];
						const result = await observer.run({
							root: input.root,
							target: subject.target,
							name: `${row.name}-${subject.name}`,
							arguments: args,
							expectedExit: 6,
						});
						const report = readObject(file);
						assert.equal(report['version'], subject.version);
						assert.equal(objectValue(report['status'])['kind'], 'prepared');
						assert.equal(objectValue(report['usage'])['model_requests'], 0);
						if (row.requests !== null)
							assert.equal(arrayValue(report['requests']).length, row.requests);
						if (row.results !== null)
							assert.equal(arrayValue(report['results']).length, row.results);
						if (subject.name === 'candidate' && row.name !== 'all-prepare-parity')
							requirePreparedSummary(
								result.stdout.toString('utf8'),
								arrayValue(report['requests']).length,
								file,
							);
						if (row.name === 'named-prepare') requireScope(result.stdout.toString('utf8'), true);
						saved.push(filesystem.readFileSync(file));
						outputs.push(result.stdout);
					},
				);
				const before = saved[0];
				const after = saved[1];
				assert.ok(before && after);
				const versions = { baselineVersion: '0.12.0', candidateVersion: '0.12.1' };
				requireComparableBytes({ ...versions, before, after, format: 'report' });
				if (row.name === 'all-prepare-parity') {
					const beforeOutput = outputs[0];
					const afterOutput = outputs[1];
					assert.ok(beforeOutput && afterOutput);
					requireComparableBytes({
						...versions,
						before: beforeOutput,
						after: afterOutput,
						format: 'json',
					});
				}
			},
		);
		await runSequence(
			[
				{ name: 'released', target: baseline },
				{ name: 'candidate', target },
			],
			async (subject) => {
				const file = path.join(evidence, `named-${subject.name}.json`);
				const result = await observer.run({
					root: input.root,
					target: subject.target,
					name: `named-${subject.name}`,
					arguments: [...common, options['one-file'], '--report', file],
					expectedExit: 2,
				});
				const report = readObject(file);
				requireCredentialRefusal(report);
				if (subject.name === 'candidate') {
					requireScope(result.stdout.toString('utf8'), true);
					assert.equal(objectValue(report['scope'])['kind'], 'all');
					requireProblemSummary(
						result.stdout.toString('utf8'),
						'Invalid input',
						arrayValue(report['diagnostics']).length,
					);
				}
			},
		);
		await runSequence(
			[
				{ name: 'released', target: baseline },
				{ name: 'candidate', target },
			],
			async (subject) => {
				const result = await observer.run({
					root: input.root,
					target: subject.target,
					name: `missing-config-${subject.name}`,
					arguments: ['lint', '--config', 'nope.json', '--check'],
					expectedExit: 2,
				});
				if (subject.name === 'candidate')
					requireConfigReason(
						Buffer.concat([result.stdout, result.stderr]).toString('utf8'),
						'nope.json',
						/not found|no such file/i,
					);
				const noConfigFile = path.join(evidence, `no-config-${subject.name}.json`);
				const noConfig = await observer.run({
					root: empty,
					target: subject.target,
					name: `no-config-${subject.name}`,
					arguments: ['lint', '--report', noConfigFile],
					expectedExit: 2,
				});
				const report = readObject(noConfigFile);
				assert.ok(
					arrayValue(report['diagnostics'])
						.map(objectValue)
						.some((entry) => entry['code'] === 'no_config_found'),
				);
				if (subject.name === 'candidate')
					requireProblemSummary(
						noConfig.stdout.toString('utf8'),
						'Invalid input',
						arrayValue(report['diagnostics']).length,
					);
				const help = await observer.run({
					root: empty,
					target: subject.target,
					name: `help-${subject.name}`,
					arguments: ['lint', '--help'],
					expectedExit: 0,
				});
				if (subject.name === 'candidate') requireHelp(help.stdout.toString('utf8'));
			},
		);
		const directory = path.join(evidence, 'config-directory');
		filesystem.mkdirSync(directory);
		const badDirectory = await observer.run({
			root: empty,
			name: 'config-is-directory',
			arguments: ['lint', '--config', directory, '--check'],
			expectedExit: 2,
		});
		requireConfigReason(
			Buffer.concat([badDirectory.stdout, badDirectory.stderr]).toString('utf8'),
			directory,
			/not (?:a )?(?:regular )?file|directory/i,
		);
		const denied = path.join(evidence, 'unreadable-config.json');
		filesystem.writeFileSync(denied, '{}\n');
		filesystem.chmodSync(denied, 0);
		let unreadableObserved = false;
		try {
			let accessible = true;
			try {
				filesystem.readFileSync(denied);
			} catch {
				accessible = false;
			}
			if (!accessible) {
				const result = await observer.run({
					root: empty,
					name: 'config-unreadable',
					arguments: ['lint', '--config', denied, '--check'],
					expectedExit: 2,
				});
				requireConfigReason(
					Buffer.concat([result.stdout, result.stderr]).toString('utf8'),
					denied,
					/permission|unreadable|access.*denied/i,
				);
				unreadableObserved = true;
			}
		} finally {
			filesystem.chmodSync(denied, 0o600);
		}
		const asking = path.join(evidence, 'asking-fixture');
		const base = askingFixture(asking, observer.endpoint);
		await observer.run({
			root: asking,
			name: 'asking-check',
			arguments: ['lint', '--check'],
			expectedExit: 0,
		});
		const preparedFile = path.join(evidence, 'asking-prepared.json');
		await observer.run({
			root: asking,
			name: 'asking-prepare-positive',
			arguments: ['lint', '--all', '--prepare', preparedFile],
			expectedExit: 6,
		});
		assert.equal(arrayValue(readObject(preparedFile)['requests']).length, 1);
		filesystem.writeFileSync(path.join(asking, 'docs/checked.md'), 'BODY changed asking input.\n');
		const negativeFile = path.join(evidence, 'changed-asking.json');
		await observer.run({
			root: asking,
			name: 'changed-asking-negative',
			arguments: ['lint', '--since', base, '--format', 'json', '--report', negativeFile],
			expectedExit: 2,
		});
		requireCredentialRefusal(readObject(negativeFile));
		const configFile = path.join(asking, 'sunstack.lint.json');
		const config = readObject(configFile);
		objectValue(config['check'])['ignore'] = ['docs/checked.md'];
		writeJson(configFile, config);
		const namedEmptyFile = path.join(evidence, 'named-empty.json');
		await observer.run({
			root: asking,
			name: 'named-empty-preserved',
			arguments: ['lint', 'docs/checked.md', '--format', 'json', '--report', namedEmptyFile],
			expectedExit: 3,
		});
		assert.equal(objectValue(readObject(namedEmptyFile)['status'])['kind'], 'incomplete');
		const emptyFile = path.join(evidence, 'excluded-unnamed.json');
		await observer.run({
			root: asking,
			name: 'excluded-unnamed-empty',
			arguments: ['lint', '--format', 'json', '--report', emptyFile],
			expectedExit: 0,
		});
		requireNoSelection(readObject(emptyFile));
		assert.equal(observer.captures.length, 0);
		pass = {
			target: { ...target },
			baseline: { ...baseline },
			input_head: input.head,
			one_file: options['one-file'],
			probe_rows: 8,
			prepare_parity: ['since HEAD', 'named file', 'default', 'all JSON stdout/saved report'],
			extra_controls: [
				'directory config',
				...(unreadableObserved ? ['unreadable config'] : []),
				'asking rule admitted/prepared',
				'changed asking refusal',
				'named-empty3',
				'excluded unnamed0',
			],
			unreadable_observed: unreadableObserved,
			http_requests: 0,
			native_joins_required: [
				'Common credential guard before EvidenceBuild; named/all CheckedFileSource spy, frozen-source caveat.',
				'C2 referenced/unrelated outside changes and model-selection/gated empty configs.',
				'Status formats including Cancelled, config error codes and unchanged inspection modes.',
			],
			limits: [
				'Keychain lookup is unchanged under C3; all children use credential store off.',
				'Late usage0 and empty report arrays alone are not proof of no evidence work.',
				'Loopback request count is not a general outbound-network witness.',
			],
		};
	} finally {
		try {
			await observer.close();
		} finally {
			const after = snapshotFrozenInput(input.root, 'HEAD', input.head);
			writeJson(path.join(evidence, 'input-after.json'), after);
			requireSameFrozenInput(input, after);
		}
	}
	assert.ok(pass);
	writeJson(path.join(evidence, 'pass.json'), pass);
}
if (process.argv[1] === import.meta.filename) {
	const { values } = parseArgs({
		options: {
			help: { type: 'boolean' },
			...Object.fromEntries(
				[
					'binary',
					'source-root',
					'commit',
					'sha256',
					'baseline-binary',
					'baseline-sha256',
					'input-root',
					'input-head',
					'config',
					'one-file',
					'evidence',
				].map((name) => [name, { type: 'string' as const }]),
			),
		},
	});
	if (values.help)
		process.stdout.write(
			'Keyless0.12.1 rule-author usability on a clean detached input.\n--binary PATH --source-root CHECKOUT --commit FULL_SHA --sha256 DIGEST --baseline-binary RELEASED_0.12.0 --baseline-sha256 DIGEST --input-root DETACHED_CHECKOUT --input-head FULL_SHA --config RELATIVE_CONFIG --one-file RELATIVE_SKILL --evidence NEW_OUTSIDE_DIRECTORY\nDrives8Leadprobe rows, changed/named asking negatives and named-empty3; counts saved requests/output path, status diagnostics, effective scope, real help footer, and version-only JSON/saved prepare parity. Owns bounded children/listener and separate mutable fixtures; preserves original file/link/Git guards. C3 credential lookup and C2 dirty irrelevant-change limits remain. Native joins prove before-evidence ordering.\n',
		);
	else await observeUsability(schema.parse(values));
}
