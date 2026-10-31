/** Separately commissioned genuine K001/K003 lint in pinned arm64 Alpine. */
import { randomUUID } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { inspectStaticArm64, runCommand } from './hosted-process.ts';
import {
	arrayValue,
	configName,
	configure,
	errorMessage,
	objectValue,
	prepareFixture,
	readObject,
	requireCondition,
	selectWorkedSkill,
	writeJson,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import { binaryDigest } from '../rulebook/rulebook-process.ts';

export const hostedArgumentsSchema = z.object({
	evidence: z.string(),
	prepare: z.boolean().default(false),
	binary: z.string().optional(),
	sha256: z.string().optional(),
	commit: z.string().optional(),
	version: z.string().optional(),
	'credential-file': z.string().optional(),
});
export type HostedArguments = z.infer<typeof hostedArgumentsSchema>;
export const hostedImage =
	'alpine@sha256:260479a1cfaf304c4c20da7f8405d3ce313513dcd534bb743257bdd2fe0f3e2d';
export interface HostedControls {
	docker?: string;
	runTimeout?: number;
	cleanupTimeout?: number;
	observeEnvironment?: (environment: NodeJS.ProcessEnv) => void;
}
export function readCredential(file: string): string {
	try {
		const metadata = filesystem.lstatSync(file);
		requireCondition(
			metadata.isFile() && (metadata.mode & 0o777) === 0o600 && metadata.uid === process.getuid?.(),
			'Hosted credential must be an owner-only regular file',
		);
		const secret = filesystem.readFileSync(file, 'utf8').trim();
		requireCondition(secret && !/[\r\n]/.test(secret), 'Hosted credential is empty or multiline');
		return secret;
	} catch (error) {
		if (error instanceof Error && error.name === 'AssertionError') throw error;
		throw new Error('Cannot read the authorized hosted credential', {
			cause: error,
		});
	}
}
function dockerArguments(options: { binary: string; fixture: string; name: string }): string[] {
	requireCondition(
		!options.binary.includes(',') && !options.fixture.includes(','),
		'Docker bind paths cannot contain commas',
	);
	return [
		'run',
		'--rm',
		'--name',
		options.name,
		'--pull',
		'never',
		'--read-only',
		'--platform',
		'linux/arm64',
		'--cap-drop',
		'ALL',
		'--security-opt',
		'no-new-privileges',
		'--workdir',
		'/fixture',
		'--tmpfs',
		'/etc/ssl:rw,nosuid,nodev,noexec,size=1m',
		'--tmpfs',
		'/home/runner:rw,nosuid,nodev,size=1m',
		'--mount',
		`type=bind,src=${options.binary},dst=/sunstack,readonly`,
		'--mount',
		`type=bind,src=${options.fixture},dst=/fixture,readonly`,
		'--env',
		'OPENROUTER_API_KEY',
		hostedImage,
		'/bin/sh',
		'-c',
		`test -z "$(find /etc/ssl -type f -print)" || exit 92; printf "CA_BUNDLE_EMPTY\\n" >&2; exec /usr/bin/env -i PATH=/usr/bin:/bin HOME=/home/runner OPENROUTER_API_KEY="$OPENROUTER_API_KEY" /sunstack lint --config ${configName} --all --format json`,
	];
}
export async function observeHosted(
	options: HostedArguments,
	controls: HostedControls = {},
): Promise<void> {
	let binary = '';
	if (!options.prepare) {
		requireCondition(
			options.binary &&
				options.sha256 &&
				options.commit &&
				options.version &&
				options['credential-file'],
			'Hosted observation needs all target fields and --credential-file; --prepare is offline',
		);
		binary = path.resolve(options.binary);
		requireCondition(
			binaryDigest(binary) === options.sha256,
			'Static binary differs from declared SHA256',
		);
		requireCondition(/^[0-9a-f]{40}$/.test(options.commit), 'Declare a full source commit');
		inspectStaticArm64(filesystem.readFileSync(binary));
	}
	const evidence = path.resolve(options.evidence);
	filesystem.mkdirSync(evidence);
	const fixture = prepareFixture(path.join(evidence, 'fixture'));
	selectWorkedSkill(fixture);
	configure(fixture, (config) => {
		config['decision_model'] = { model: 'jev-latest' };
		config['limits'] = {
			max_model_requests: 16,
			max_concurrent_requests: 1,
			attempt_timeout_seconds: 60,
		};
	});
	if (options.prepare) {
		writeJson(path.join(evidence, 's3-prepared.json'), {
			product_observed: false,
			genuine_requests: 0,
		});
		process.stdout.write('Prepared S3 fixture; no credential read or product observed\n');
		return;
	}
	const executable = controls.docker ?? 'docker';
	const inspection = await runCommand({
		executable,
		arguments: ['image', 'inspect', '--platform', 'linux/arm64', hostedImage],
		timeout: 15000,
	});
	requireCondition(
		!inspection.timedOut && inspection.exit === 0,
		'Pinned image unavailable; no pull attempted',
	);
	const name = `sunstack-hosted-${randomUUID().replaceAll('-', '')}`;
	const command = dockerArguments({ binary, fixture, name });
	requireCondition(options['credential-file'], 'Missing authorized credential');
	const secret = readCredential(options['credential-file']);
	const environment: NodeJS.ProcessEnv = {
		PATH: process.env['PATH'] ?? '/usr/bin:/bin',
		HOME: process.env['HOME'] ?? '/tmp',
		OPENROUTER_API_KEY: secret,
	};
	for (const selector of ['DOCKER_HOST', 'DOCKER_CONTEXT', 'DOCKER_CONFIG'])
		if (process.env[selector]) environment[selector] = process.env[selector];
	controls.observeEnvironment?.(environment);
	let result: Awaited<ReturnType<typeof runCommand>>;
	try {
		result = await runCommand({
			executable,
			arguments: command,
			environment,
			timeout: controls.runTimeout ?? 240000,
		});
	} finally {
		delete environment['OPENROUTER_API_KEY'];
	}
	const leaked = result.stdout.includes(secret) || result.stderr.includes(secret);
	if (result.timedOut) {
		const receipt: JsonObject = {
			remote_outcome: 'unknown',
			container_cleanup_exit: null,
			container_cleanup_status: 'unknown',
			retry_performed: false,
			output_partial: true,
			raw_output_retained: !leaked,
		};
		writeJson(path.join(evidence, 's3-timeout.json'), receipt);
		if (leaked)
			writeJson(path.join(evidence, 'credential-leak.json'), {
				observed: true,
				raw_output_retained: false,
			});
		else {
			filesystem.writeFileSync(path.join(evidence, 'stdout.partial.log'), result.stdout);
			filesystem.writeFileSync(path.join(evidence, 'stderr.partial.log'), result.stderr);
		}
		try {
			const cleanup = await runCommand({
				executable,
				arguments: ['rm', '--force', name],
				timeout: controls.cleanupTimeout ?? 15000,
			});
			receipt['container_cleanup_exit'] = cleanup.exit;
			receipt['container_cleanup_status'] = cleanup.timedOut
				? 'timed_out'
				: cleanup.exit === 0
					? 'completed'
					: 'failed';
		} catch {
			receipt['container_cleanup_status'] = 'launch_failed';
		}
		writeJson(path.join(evidence, 's3-timeout.json'), receipt);
		throw new Error(
			`${leaked ? 'Credential appeared in partial output; raw output withheld; ' : ''}Hosted command timed out; remote billing/outcome unknown; do not retry`,
		);
	}
	if (leaked) {
		writeJson(path.join(evidence, 'credential-leak.json'), {
			observed: true,
			raw_output_retained: false,
		});
		throw new Error('Credential appeared in output; raw output withheld');
	}
	filesystem.writeFileSync(path.join(evidence, 'stdout.json'), result.stdout);
	filesystem.writeFileSync(path.join(evidence, 'stderr.log'), result.stderr);
	writeJson(path.join(evidence, 'process.json'), {
		command: ['docker', ...command],
		commit: options.commit,
		binary_sha256: options.sha256,
		version: options.version,
		exit_code: result.exit,
		retry_performed: false,
	});
	requireCondition(
		result.exit === 1 && result.stderr.includes('CA_BUNDLE_EMPTY'),
		'Hosted lint did not return K002 error with empty CA bundle',
	);
	const report = readObject(path.join(evidence, 'stdout.json'));
	const usage = objectValue(report['usage']);
	requireCondition(
		report['version'] === options.version &&
			typeof usage['input_tokens'] === 'number' &&
			usage['input_tokens'] > 0,
		'Wrong build version or missing real inference usage',
	);
	requireCondition(
		!arrayValue(report['incomplete']).length && !arrayValue(report['skipped_rules']).length,
		'Hosted report incomplete/skipped',
	);
	const standards = new Set(
		Object.values(objectValue(report['evidence_seen'])).map(
			(value) => objectValue(value)['standard'],
		),
	);
	requireCondition(
		[
			'Each workflow step states a checkable completion covering its obligations.',
			'A skill description names when to use the skill.',
		].every((standard) => standards.has(standard)),
		'Admitted requests did not cover K001 and K003',
	);
	requireCondition(
		binaryDigest(binary) === options.sha256,
		'Static binary changed during hosted run',
	);
	writeJson(path.join(evidence, 's3-pass.json'), {
		commit: options.commit,
		binary_sha256: options.sha256,
		genuine_fixture: 'rulebook K001 and K003 with mechanical K002 error',
		usage,
		image: hostedImage,
		gaps: [
			'one fixture/model only',
			'private CAs unverified',
			'semantic answer quality is not judged',
		],
	});
	process.stdout.write('S3 hosted lint observed; evidence retained; no retry performed\n');
}
export async function main(): Promise<void> {
	const parsed = parseArgs({
		options: {
			evidence: { type: 'string' },
			prepare: { type: 'boolean' },
			binary: { type: 'string' },
			sha256: { type: 'string' },
			commit: { type: 'string' },
			version: { type: 'string' },
			'credential-file': { type: 'string' },
			help: { type: 'boolean' },
		},
	});
	if (parsed.values.help) {
		process.stdout.write(
			'Hosted rulebook lint. Offline: --prepare --evidence <new-dir>. Separately authorized observation: --binary --sha256 --commit --version --credential-file --evidence. No automatic rerun.\n',
		);
		return;
	}
	await observeHosted(hostedArgumentsSchema.parse(parsed.values));
}
if (import.meta.main)
	try {
		await main();
	} catch (error) {
		process.stderr.write(`S3 setup/observation failed: ${errorMessage(error)}\n`);
		process.exitCode = 2;
	}
