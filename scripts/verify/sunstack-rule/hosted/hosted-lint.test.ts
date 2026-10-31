import { spawnSync } from 'node:child_process';
/** Actual local child timeouts and synthetic-output safety, without Docker or provider access. */
import filesystem from 'node:fs';
import operatingSystem from 'node:os';
import path from 'node:path';

import { afterEach, expect, test } from 'vitest';

import { readObject } from '../rulebook/rulebook-fixture-support.ts';
import { binaryDigest } from '../rulebook/rulebook-process.ts';
import { hostedArgumentsSchema, observeHosted, readCredential } from './hosted-lint.ts';
import { inspectStaticArm64 } from './hosted-process.ts';
const temporary: string[] = [];
function scratch(): string {
	const root = filesystem.mkdtempSync(path.join(operatingSystem.tmpdir(), 'hosted-apparatus-'));
	temporary.push(root);
	return root;
}
afterEach(() => {
	for (const root of temporary.splice(0)) filesystem.rmSync(root, { recursive: true, force: true });
});
function staticElf(root: string): string {
	const payload = Buffer.alloc(128);
	Buffer.from([127, 69, 76, 70, 2, 1]).copy(payload);
	payload.writeUInt16LE(183, 18);
	payload.writeBigUInt64LE(64n, 32);
	payload.writeUInt16LE(56, 54);
	payload.writeUInt16LE(1, 56);
	payload.writeUInt32LE(1, 64);
	payload.writeBigUInt64LE(120n, 72);
	payload.writeBigUInt64LE(8n, 96);
	const binary = path.join(root, 'fixture-elf');
	filesystem.writeFileSync(binary, payload, { mode: 0o755 });
	return binary;
}
const scenarios = [
	{ leak: 'none', timeout: true, cleanup: 'none' },
	{ leak: 'stdout', timeout: true, cleanup: 'none' },
	{ leak: 'stderr', timeout: true, cleanup: 'none' },
	{ leak: 'empty', timeout: true, cleanup: 'none' },
	{ leak: 'none', timeout: false, cleanup: 'none' },
	{ leak: 'stdout', timeout: false, cleanup: 'none' },
	{ leak: 'stderr', timeout: false, cleanup: 'none' },
	{ leak: 'none', timeout: true, cleanup: 'timeout' },
	{ leak: 'stdout', timeout: true, cleanup: 'timeout' },
	{ leak: 'none', timeout: true, cleanup: 'launch' },
	{ leak: 'stdout', timeout: true, cleanup: 'launch' },
];
test.each(scenarios)(
	'safe output $leak, timeout $timeout, cleanup $cleanup',
	async (scenario) => {
		const root = scratch();
		const binary = staticElf(root);
		const synthetic = 'SYNTHETIC_TIMEOUT_KEY_NOT_A_CREDENTIAL';
		const credential = path.join(root, 'synthetic-key');
		filesystem.writeFileSync(credential, synthetic, { mode: 0o600 });
		const docker = path.join(root, 'fake-docker');
		const cleanup = path.join(root, 'cleanup-observed');
		const calls = path.join(root, 'calls');
		const hang = "require('node:http').createServer().listen(0,'127.0.0.1');";
		filesystem.writeFileSync(
			docker,
			`#!${process.execPath}\nconst filesystem=require('node:fs');filesystem.appendFileSync(${JSON.stringify(calls)},process.argv[2]+'\\n');if(process.argv[2]==='image')process.exit(0);if(process.argv[2]==='rm'){filesystem.writeFileSync(${JSON.stringify(cleanup)},'owned cleanup');${scenario.cleanup === 'timeout' ? hang : 'process.exit(0);'}}else{${scenario.leak === 'empty' ? '' : "process.stdout.write('PARTIAL_STDOUT_SENTINEL\\n');process.stderr.write('PARTIAL_STDERR_SENTINEL\\n');"}${['stdout', 'stderr'].includes(scenario.leak) ? `process.${scenario.leak}.write(process.env.OPENROUTER_API_KEY+'\\n');` : ''}${scenario.cleanup === 'launch' ? 'filesystem.unlinkSync(process.argv[1]);' : ''}${scenario.timeout ? hang : 'process.exitCode=1;'}}`,
			{ mode: 0o755 },
		);
		const evidence = path.join(root, 'evidence');
		let runEnvironment: NodeJS.ProcessEnv | undefined;
		await expect(
			observeHosted(
				hostedArgumentsSchema.parse({
					evidence,
					binary,
					sha256: binaryDigest(binary),
					commit: 'a'.repeat(40),
					version: '0.0.6',
					'credential-file': credential,
				}),
				{
					docker,
					runTimeout: 1000,
					cleanupTimeout: 500,
					observeEnvironment: (environment) => {
						runEnvironment = environment;
					},
				},
			),
		).rejects.toThrow();
		expect(runEnvironment).toBeDefined();
		expect(runEnvironment).not.toHaveProperty('OPENROUTER_API_KEY');
		const invoked = filesystem.readFileSync(calls, 'utf8').trim().split('\n');
		expect(invoked.filter((call) => call === 'run')).toHaveLength(1);
		if (scenario.timeout) {
			const receipt = readObject(path.join(evidence, 's3-timeout.json'));
			expect(receipt).toMatchObject({
				remote_outcome: 'unknown',
				retry_performed: false,
				output_partial: true,
				container_cleanup_status:
					scenario.cleanup === 'none'
						? 'completed'
						: scenario.cleanup === 'timeout'
							? 'timed_out'
							: 'launch_failed',
				container_cleanup_exit: scenario.cleanup === 'none' ? 0 : null,
			});
			expect(invoked.filter((call) => call === 'rm')).toHaveLength(
				scenario.cleanup === 'launch' ? 0 : 1,
			);
			expect(filesystem.existsSync(cleanup)).toBe(scenario.cleanup !== 'launch');
			if (scenario.cleanup !== 'launch')
				expect(filesystem.readFileSync(cleanup, 'utf8')).toBe('owned cleanup');
			else expect(filesystem.existsSync(docker)).toBe(false);
		} else {
			expect(filesystem.existsSync(path.join(evidence, 's3-timeout.json'))).toBe(false);
			expect(filesystem.existsSync(cleanup)).toBe(false);
		}
		const stdout = path.join(evidence, scenario.timeout ? 'stdout.partial.log' : 'stdout.json');
		const stderr = path.join(evidence, scenario.timeout ? 'stderr.partial.log' : 'stderr.log');
		if (['none', 'empty'].includes(scenario.leak)) {
			expect(filesystem.readFileSync(stdout, 'utf8')).toBe(
				scenario.leak === 'empty' ? '' : 'PARTIAL_STDOUT_SENTINEL\n',
			);
			expect(filesystem.readFileSync(stderr, 'utf8')).toBe(
				scenario.leak === 'empty' ? '' : 'PARTIAL_STDERR_SENTINEL\n',
			);
			expect(filesystem.existsSync(path.join(evidence, 'credential-leak.json'))).toBe(false);
		} else {
			expect(readObject(path.join(evidence, 'credential-leak.json'))).toEqual({
				observed: true,
				raw_output_retained: false,
			});
			expect(filesystem.existsSync(stdout)).toBe(false);
			expect(filesystem.existsSync(stderr)).toBe(false);
		}
		for (const entry of filesystem.readdirSync(evidence, { recursive: true, withFileTypes: true }))
			if (entry.isFile())
				expect(
					filesystem.readFileSync(path.join(entry.parentPath, entry.name)).includes(synthetic),
				).toBe(false);
	},
	10000,
);
test('credential must be owner-only regular single-line file', () => {
	const root = scratch();
	const file = path.join(root, 'credential');
	filesystem.writeFileSync(file, 'SYNTHETIC_NOT_A_SECRET\n', { mode: 0o644 });
	expect(() => readCredential(file)).toThrow('owner-only');
	filesystem.chmodSync(file, 0o600);
	expect(readCredential(file)).toBe('SYNTHETIC_NOT_A_SECRET');
	filesystem.writeFileSync(file, 'line1\nline2');
	expect(() => readCredential(file)).toThrow('multiline');
	const link = path.join(root, 'link');
	filesystem.symlinkSync(file, link);
	expect(() => readCredential(link)).toThrow('owner-only');
});
test('static gate rejects dynamic ELF and another architecture', () => {
	const binary = staticElf(scratch());
	const bytes = filesystem.readFileSync(binary);
	inspectStaticArm64(bytes);
	bytes.writeUInt32LE(3, 64);
	expect(() => inspectStaticArm64(bytes)).toThrow('dynamic interpreter');
	bytes.writeUInt32LE(1, 64);
	bytes.writeUInt16LE(62, 18);
	expect(() => inspectStaticArm64(bytes)).toThrow('arm64');
});
test.each(['lint-runner-rulebook.ts', 'hosted-lint.ts'])(
	'help and missing target are side-effect free: %s',
	(script) => {
		const root = scratch();
		const driver = path.join(
			import.meta.dirname,
			script === 'lint-runner-rulebook.ts' ? '../rulebook' : '.',
			script,
		);
		const help = spawnSync(process.execPath, [driver, '--help'], { encoding: 'utf8' });
		expect(help.status).toBe(0);
		const evidence = path.join(root, 'missing');
		const unspecified = spawnSync(process.execPath, [driver, '--evidence', evidence], {
			encoding: 'utf8',
		});
		expect(unspecified.status).toBe(2);
		expect(filesystem.existsSync(evidence)).toBe(false);
		const missing = spawnSync(
			process.execPath,
			[
				driver,
				'--evidence',
				evidence,
				'--binary',
				path.join(root, 'absent'),
				'--commit',
				'a'.repeat(40),
				...(script === 'hosted-lint.ts'
					? [
							'--sha256',
							'b'.repeat(64),
							'--version',
							'0.1.0',
							'--credential-file',
							path.join(root, 'never-read'),
						]
					: []),
			],
			{ encoding: 'utf8' },
		);
		expect(missing.status).toBe(2);
		expect(filesystem.existsSync(evidence)).toBe(false);
	},
);
