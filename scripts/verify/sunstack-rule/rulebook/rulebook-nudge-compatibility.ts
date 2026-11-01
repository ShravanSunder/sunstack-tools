/** U10: signed 0.5.0 artifact vs declared signed 0.6.0 build, identical legacy fixture/answers. */
import { spawnSync } from 'node:child_process';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { requireDebugSignature } from './rulebook-amendment.ts';
import {
	configureEndpoint,
	equal,
	errorMessage,
	lintArguments,
	prepareFixture,
	requireCondition,
	selectWorkedSkill,
	writeJson,
} from './rulebook-fixture-support.ts';
import { scriptedReply } from './rulebook-lint.ts';
import { nudgeOracle } from './rulebook-nudge-fixtures.ts';
import { requireNudgeCompatibility } from './rulebook-nudge-projection.ts';
import {
	declaredTarget,
	releasedTarget,
	RulebookProcess,
	type ProcessTarget,
} from './rulebook-process.ts';

function requireReleaseSignature(binary: string): void {
	requireCondition(
		process.platform === 'darwin' && process.arch === 'arm64',
		'U10 target is macOS arm64',
	);
	const signature = spawnSync('codesign', ['-dv', binary], { encoding: 'utf8', timeout: 10000 });
	const verified = spawnSync('codesign', ['--verify', binary], {
		encoding: 'utf8',
		timeout: 10000,
	});
	requireCondition(
		signature.status === 0 &&
			verified.status === 0 &&
			signature.stderr.includes('Identifier=dev.shravansunder.sunstack\n') &&
			signature.stderr.includes('Authority=Developer ID Application:'),
		'Baseline lacks the verified release signature',
	);
}
async function observe(
	target: ProcessTarget,
	root: string,
	evidence: string,
): Promise<{ raw: string; bodies: Buffer[] }> {
	const observer = new RulebookProcess(target, evidence);
	await observer.start();
	try {
		configureEndpoint(root, observer.endpoint);
		observer.reply = scriptedReply;
		const ready = await observer.run({
			root,
			name: 'ready',
			arguments: ['lint', '--config', 'skill-rules.config.json', '--check'],
			expectedExit: 0,
		});
		equal(ready.captures.length, 0, 'Compatibility readiness dispatched');
		const observed = await observer.run({
			root,
			name: 'legacy-report',
			arguments: lintArguments('--all'),
			expectedExit: 1,
			credential: true,
		});
		requireCondition(observed.captures.length > 0, 'Compatibility did not reach model answers');
		return {
			raw: observed.stdout.toString('utf8'),
			bodies: observed.captures.map((capture) => capture.raw),
		};
	} finally {
		await observer.close();
	}
}
const argumentsSchema = z.object({
	baseline: z.string(),
	'baseline-sha256': z.string().regex(/^[0-9a-f]{64}$/),
	binary: z.string(),
	sha256: z.string().regex(/^[0-9a-f]{64}$/),
	'source-root': z.string(),
	commit: z.string().regex(/^[0-9a-f]{40}$/),
	evidence: z.string(),
});
export async function main(): Promise<void> {
	const parsed = parseArgs({
		options: {
			baseline: { type: 'string' },
			'baseline-sha256': { type: 'string' },
			binary: { type: 'string' },
			sha256: { type: 'string' },
			'source-root': { type: 'string' },
			commit: { type: 'string' },
			evidence: { type: 'string' },
			help: { type: 'boolean' },
		},
	});
	if (parsed.values.help) {
		process.stdout.write(
			'U10: --baseline SIGNED_0.5.0 --baseline-sha256 HASH --binary SIGNED_0.6.0 --sha256 HASH --source-root CLEAN_CHECKOUT --commit FULL_SHA --evidence NEW_DIR. Real CLI, one identical legacy fixture, same scripted answers, every legacy field/value/key/array order; 0.6.0 version required. No real credential. Schema/diff/native proof remain separate.\n',
		);
		return;
	}
	const options = argumentsSchema.parse(parsed.values);
	requireReleaseSignature(options.baseline);
	requireDebugSignature(options.binary);
	const baseline = releasedTarget(options.baseline, options['baseline-sha256'], '0.5.0');
	const current = declaredTarget({
		binary: options.binary,
		digest: options.sha256,
		source: options['source-root'],
		commit: options.commit,
	});
	const evidence = path.resolve(options.evidence);
	filesystem.mkdirSync(evidence);
	const root = prepareFixture(path.join(evidence, 'fixture'), ['K001', 'K002', 'K003']);
	selectWorkedSkill(root);
	const before = await observe(baseline, root, path.join(evidence, 'baseline'));
	const after = await observe(current, root, path.join(evidence, 'current'));
	// Concurrent HTTP arrival order may vary; report order is never sorted.
	equal(
		after.bodies.map((body) => body.toString('base64')).toSorted(),
		before.bodies.map((body) => body.toString('base64')).toSorted(),
		'Legacy fixture requests/answers workload changed',
	);
	requireNudgeCompatibility(before.raw, after.raw);
	const planted = after.raw.replace(/"step"\s*:\s*"[^"]+"/, '"step":"planted_wrong_step"');
	requireCondition(planted !== after.raw, 'Compatibility control found no legacy step');
	let rejected = false;
	try {
		requireNudgeCompatibility(before.raw, planted);
	} catch {
		rejected = true;
	}
	requireCondition(rejected, 'Legacy projection accepted a planted wrong step');
	writeJson(path.join(evidence, 'pass.json'), {
		baseline,
		current,
		oracle: nudgeOracle,
		all_legacy_fields_equal: true,
		exact_request_multiset_equal: true,
		planted_step_rejected: true,
		required_joins: ['S3 report schema', 'engine diff has no new domain terms'],
	});
	process.stdout.write('U10 legacy projection passed; schema/diff joins remain separate\n');
}
if (import.meta.main)
	try {
		await main();
	} catch (error) {
		process.stderr.write(`U10 compatibility failed: ${errorMessage(error)}\n`);
		process.exitCode = 2;
	}
