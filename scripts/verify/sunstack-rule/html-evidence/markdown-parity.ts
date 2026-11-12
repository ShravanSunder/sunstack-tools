/** Same frozen consumer config/skills through 0.10.x and the declared candidate, without dispatch. */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { requireDebugSignature } from '../rulebook/rulebook-amendment.ts';
import {
	arrayValue,
	baselineGit,
	objectValue,
	readObject,
	runSequence,
	writeJson,
} from '../rulebook/rulebook-fixture-support.ts';
import { requireComparableBytes } from '../rulebook/rulebook-output-comparison.ts';
import { declaredTarget, releasedTarget, RulebookProcess } from '../rulebook/rulebook-process.ts';
import { requireNoDispatch } from './html-checks.ts';

export const consumerConfig =
	'plugins/skill-authoring/skills/domain-skill-crafting/rules/skill-rules.config.json';
export const consumerSkills = [
	'plugins/sunstack-dev/skills/practices-how/SKILL.md',
	'plugins/sunstack-dev/skills/debug-investigation/SKILL.md',
];
const consumerDependency = 'plugins/sunstack-workflow-v1/skills/practices-manage-agents/SKILL.md';
const schema = z.object({
	binary: z.string(),
	'source-root': z.string(),
	commit: z.string().regex(/^[0-9a-f]{40}$/),
	sha256: z.string().regex(/^[0-9a-f]{64}$/),
	'baseline-binary': z.string(),
	'baseline-sha256': z.string().regex(/^[0-9a-f]{64}$/),
	'baseline-version': z.string().regex(/^0\.10\.[0-9]+$/),
	'consumer-root': z.string(),
	'consumer-commit': z.string().regex(/^[0-9a-f]{40}$/),
	evidence: z.string(),
});
function snapshotConsumer(source: string, revision: string, root: string): Record<string, string> {
	filesystem.mkdirSync(root);
	const archive = spawnSync('git', ['archive', revision, 'plugins'], {
		cwd: source,
		timeout: 15000,
		maxBuffer: 100_000_000,
	});
	assert.equal(archive.status, 0, 'Cannot archive pinned consumer');
	const unpacked = spawnSync('tar', ['-xf', '-', '-C', root], {
		input: archive.stdout,
		timeout: 15000,
	});
	assert.equal(unpacked.status, 0, 'Cannot unpack owned consumer snapshot');
	const hashes: Record<string, string> = {};
	for (const file of [consumerConfig, ...consumerSkills, consumerDependency]) {
		const pinned = spawnSync('git', ['show', `${revision}:${file}`], {
			cwd: source,
			timeout: 10000,
		});
		assert.equal(pinned.status, 0);
		const actual = filesystem.readFileSync(path.join(root, file));
		assert.deepEqual(actual, pinned.stdout, `Snapshot bytes changed: ${file}`);
		hashes[file] = createHash('sha256').update(actual).digest('hex');
	}
	baselineGit(root);
	return hashes;
}
export async function observeMarkdown(options: z.infer<typeof schema>): Promise<void> {
	const target = declaredTarget({
		binary: options.binary,
		source: options['source-root'],
		commit: options.commit,
		digest: options.sha256,
	});
	requireDebugSignature(target.binary);
	const baseline = releasedTarget(
		options['baseline-binary'],
		options['baseline-sha256'],
		options['baseline-version'],
	);
	const version = spawnSync(target.binary, ['--version'], { encoding: 'utf8', timeout: 10000 });
	assert.equal(version.status, 0);
	assert.equal(version.stdout.trim(), 'sunstack 0.11.0');
	const evidence = path.resolve(options.evidence);
	filesystem.mkdirSync(evidence);
	const root = path.join(evidence, 'consumer');
	const hashes = snapshotConsumer(options['consumer-root'], options['consumer-commit'], root);
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	try {
		// Retain the original config byte-for-byte. No endpoint/selection/filter override.
		assert.ok(
			!JSON.stringify(
				objectValue(readObject(path.join(root, consumerConfig))['evidence']),
			).includes('"select"'),
			'Consumer gained selection; return to commissioner',
		);
		await runSequence(
			[
				['baseline', baseline],
				['candidate', target],
			] as const,
			async ([name, processTarget]) => {
				const check = await observer.run({
					target: processTarget,
					root,
					name: `${name}-check`,
					arguments: ['lint', '--config', consumerConfig, '--check'],
					expectedExit: 0,
				});
				assert.equal(check.captures.length, 0);
				const result = await observer.run({
					target: processTarget,
					root,
					name: `${name}-prepare`,
					arguments: [
						'lint',
						'--config',
						consumerConfig,
						'--prepare',
						path.join(evidence, `${name}.prepared.json`),
						...consumerSkills,
					],
					expectedExit: 6,
				});
				assert.equal(result.captures.length, 0);
				const report = readObject(path.join(evidence, `${name}.prepared.json`));
				requireNoDispatch(report);
				assert.ok(
					arrayValue(report['requests']).length > 0,
					'Markdown comparison prepared no requests',
				);
				assert.ok(
					arrayValue(report['results']).length > 0,
					'Markdown comparison checked no pieces',
				);
			},
		);
		requireComparableBytes({
			before: filesystem.readFileSync(path.join(evidence, 'baseline.prepared.json')),
			after: filesystem.readFileSync(path.join(evidence, 'candidate.prepared.json')),
			format: 'report',
			baselineVersion: baseline.release,
			candidateVersion: '0.11.0',
		});
		const candidate = readObject(path.join(evidence, 'candidate.prepared.json'));
		writeJson(path.join(evidence, 'pass.json'), {
			target,
			baseline,
			consumer_revision: options['consumer-commit'],
			input_sha256: hashes,
			consumer_config_modified: false,
			prepared_requests: arrayValue(candidate['requests']).length,
			results: arrayValue(candidate['results']).length,
			exact_byte_exceptions: ['Root engine version only'],
			limits: [
				'Prepared request and mechanical-result parity only; no provider judgments or model quality.',
				'Original config uses its original endpoint; owned listener is not an endpoint egress witness for this companion run.',
			],
		});
	} finally {
		await observer.close();
	}
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
					'baseline-version',
					'consumer-root',
					'consumer-commit',
					'evidence',
				].map((name) => [name, { type: 'string' as const }]),
			),
		},
	});
	if (values.help)
		process.stdout.write(
			'Compare unchanged frozen ai-tools config and two named skills with an immutable 0.10.x comparator.\n--binary PATH --source-root CHECKOUT --commit FULL_SHA --sha256 DIGEST --baseline-binary PATH --baseline-sha256 DIGEST --baseline-version 0.10.x --consumer-root CHECKOUT --consumer-commit FULL_SHA --evidence NEW_DIRECTORY\nKeyless --prepare only. Compare every saved byte except root product version. Confirm comparator provenance/signature before use; no distributed-release claim follows from a version probe.\n',
		);
	else await observeMarkdown(schema.parse(values));
}
