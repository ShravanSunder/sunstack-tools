/** Run the declared immutable public-API test consumer; never compile product code here. */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';

import { z } from 'zod';

import {
	arrayValue,
	numberValue,
	objectValue,
	readObject,
	textValue,
	writeJson,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import {
	declaredTarget,
	confirmTarget,
	type DeclaredTarget,
} from '../rulebook/rulebook-process.ts';
import { requireOutsideInput, type FrozenInput } from './frozen-inputs.ts';

export function requireSignedArtifact(binary: string): void {
	if (process.platform !== 'darwin') return;
	const verified = spawnSync('codesign', ['--verify', '--strict', binary], {
		encoding: 'utf8',
		timeout: 10000,
	});
	const metadata = spawnSync('codesign', ['-dvv', binary], { encoding: 'utf8', timeout: 10000 });
	assert.equal(verified.status, 0, verified.stderr);
	assert.equal(metadata.status, 0, metadata.stderr);
	assert.match(metadata.stderr, /Authority=Developer ID Application:/);
	assert.match(metadata.stderr, /TeamIdentifier=(?!not set)\S+/);
}
export function requireCaptureParityReceipt(
	report: JsonObject,
	input: FrozenInput,
): { changedFiles: number; fullFiles: number; paths: string[] } {
	assert.equal(report['version'], '0.12.0');
	assert.equal(report['root'], input.root);
	assert.equal(report['base_revision'], input.base);
	assert.equal(report['raw_changes_equal'], true);
	assert.equal(report['input_unchanged'], true);
	const guard = objectValue(report['git_guard']);
	assert.equal(guard['head'], input.head);
	assert.equal(guard['index_sha256'], input.gitAdmin['index']);
	const expected: Record<string, string> = {};
	for (const [name, encoded] of Object.entries(input.files)) {
		if (name === '.git') continue;
		const parts = encoded.split(':');
		const payload = parts.slice(2).join(':');
		if (parts[0] === 'file') expected[name] = `file:${payload}`;
		else {
			assert.equal(parts[0], 'link');
			const target = z.string().parse(JSON.parse(payload));
			expected[name] = `link:${createHash('sha256').update(target).digest('hex')}`;
		}
	}
	assert.deepEqual(
		report['input_fingerprints'],
		expected,
		'Native consumer did not bind the same file/link bytes',
	);
	const changed = objectValue(report['changed_only']);
	const full = objectValue(report['full']);
	const paths = arrayValue(changed['paths']).map(textValue);
	assert.ok(paths.length > 0, 'Raw-path oracle is inert');
	assert.deepEqual(changed['paths'], full['paths'], 'Native raw changed-path sets differ');
	const raw = arrayValue(report['raw_changes'])
		.flatMap((member) => arrayValue(member))
		.map(objectValue);
	assert.deepEqual(
		raw.map((entry) => textValue(entry['path'])),
		paths,
	);
	for (const entry of raw) {
		for (const key of ['before_sha256', 'after_sha256'])
			assert.ok(entry[key] === null || /^[a-f0-9]{64}$/.test(textValue(entry[key])));
		arrayValue(entry['before_spans']);
		arrayValue(entry['after_spans']);
	}
	const changedFiles = numberValue(changed['acquired_files']);
	const fullFiles = numberValue(full['acquired_files']);
	assert.ok(changedFiles >= 0 && Number.isInteger(changedFiles));
	assert.ok(fullFiles > changedFiles && Number.isInteger(fullFiles));
	assert.ok(
		changedFiles <= 2 * paths.length,
		'Candidate-only capture still acquired unrelated context',
	);
	assert.equal(objectValue(report['limits'])['max_files'], 2000);
	assert.equal(report['oracle_max_files'], 10000);
	return { changedFiles, fullFiles, paths };
}
export function runCaptureOracle(options: {
	target: DeclaredTarget;
	input: FrozenInput;
	directory: string;
}): { report: JsonObject; changedFiles: number; fullFiles: number; paths: string[] } {
	const target = declaredTarget({
		binary: options.target.binary,
		source: options.target.source,
		commit: options.target.commit,
		digest: options.target.digest,
	});
	requireSignedArtifact(target.binary);
	requireOutsideInput(options.input.root, options.directory);
	filesystem.mkdirSync(options.directory);
	const receipt = path.join(options.directory, 'capture.json');
	assert.ok(!filesystem.existsSync(receipt));
	const home = path.join(options.directory, 'home');
	filesystem.mkdirSync(home);
	const args = [
		'changed_only::capture_parity::public_capture_modes_produce_equal_raw_changes_with_fewer_acquisitions',
		'--exact',
		'--nocapture',
	];
	const result = spawnSync(target.binary, args, {
		cwd: target.source,
		encoding: 'utf8',
		timeout: 30000,
		maxBuffer: 8 * 1024 * 1024,
		env: {
			PATH: '/usr/bin:/bin',
			HOME: home,
			LANG: 'C.UTF-8',
			GIT_CONFIG_NOSYSTEM: '1',
			GIT_CONFIG_GLOBAL: '/dev/null',
			GIT_OPTIONAL_LOCKS: '0',
			SUNSTACK_CREDENTIAL_STORE: 'off',
			SUNSTACK_CAPTURE_PARITY_ROOT: options.input.root,
			SUNSTACK_CAPTURE_PARITY_BASE: options.input.base,
			SUNSTACK_CAPTURE_PARITY_RECEIPT: receipt,
		},
	});
	filesystem.writeFileSync(path.join(options.directory, 'stdout.log'), result.stdout);
	filesystem.writeFileSync(path.join(options.directory, 'stderr.log'), result.stderr);
	writeJson(path.join(options.directory, 'process.json'), {
		target,
		command: [target.binary, ...args],
		cwd: target.source,
		pid: result.pid,
		exit_code: result.status,
		signal: result.signal,
		spawn_error: result.error?.message ?? null,
		credential: 'absent',
		input_root: options.input.root,
		input_base: options.input.base,
		receipt,
	});
	confirmTarget(target);
	assert.equal(
		result.error,
		undefined,
		'Native consumer spawn/bound failure; preserve as setup evidence',
	);
	assert.equal(result.status, 0, result.stderr);
	assert.match(
		result.stdout,
		/test result: ok\. 1 passed; 0 failed/,
		'Selected native consumer did not execute exactly once',
	);
	const report = readObject(receipt);
	return { report, ...requireCaptureParityReceipt(report, options.input) };
}
