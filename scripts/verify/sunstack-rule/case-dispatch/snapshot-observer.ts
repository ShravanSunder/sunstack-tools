#!/usr/bin/env node
/** Child wrapper for the native case runner's existing executable injection seam. */
import assert from 'node:assert/strict';
import { spawnSync, type SpawnSyncReturns } from 'node:child_process';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';

import { ordinarySnapshotArguments } from './snapshot-arguments.ts';
import { requireSnapshotParity } from './snapshot-parity.ts';

function declared(name: string): string {
	const value = process.env[name];
	assert.ok(value && path.isAbsolute(value), `${name} must be absolute`);
	return value;
}
function sha256(file: string): string {
	return createHash('sha256').update(filesystem.readFileSync(file)).digest('hex');
}
function main(): void {
	const binary = declared('CASE_DISPATCH_REAL_CLI');
	const evidence = declared('CASE_DISPATCH_EVIDENCE');
	const expectedHash = process.env['CASE_DISPATCH_EXPECTED_SHA256'];
	assert.match(expectedHash ?? '', /^[0-9a-f]{64}$/u, 'Pinned CLI digest required');
	assert.equal(sha256(binary), expectedHash, 'Pinned CLI bytes changed');
	assert.ok(!process.env['OPENROUTER_API_KEY'], 'Snapshot preparation cannot inherit a model key');
	assert.equal(process.env['SUNSTACK_CREDENTIAL_STORE'], 'off');
	const cwd = filesystem.realpathSync(process.cwd());
	const evidenceRoot = filesystem.realpathSync(evidence);
	assert.ok(
		cwd !== evidenceRoot &&
			!cwd.startsWith(`${evidenceRoot}${path.sep}`) &&
			!evidenceRoot.startsWith(`${cwd}${path.sep}`),
		'Evidence must be outside the snapshot',
	);
	const invocation = filesystem.mkdtempSync(path.join(evidence, 'snapshot-'));
	const ordinaryReport = path.join(invocation, 'ordinary.json');
	const original = process.argv.slice(2);
	const ordinaryArgs = ordinarySnapshotArguments(original, ordinaryReport);
	const selectedIndex = original.indexOf('--prepare');
	const selectedPath = original[selectedIndex + 1];
	assert.ok(selectedPath && path.isAbsolute(selectedPath), 'Selected report path must be absolute');
	const bindingsIndex = original.indexOf('--case-selection');
	const bindingsPath = original[bindingsIndex + 1];
	assert.ok(bindingsPath && path.isAbsolute(bindingsPath), 'Selection input must be absolute');
	filesystem.copyFileSync(
		bindingsPath,
		path.join(invocation, 'bindings.json'),
		filesystem.constants.COPYFILE_EXCL,
	);
	const invoke = (arguments_: string[]): SpawnSyncReturns<Buffer> =>
		spawnSync(binary, arguments_, {
			cwd,
			env: process.env,
			encoding: 'buffer',
			timeout: 120000,
			maxBuffer: 128_000_000,
		});
	const ordinary = invoke(ordinaryArgs);
	filesystem.writeFileSync(
		path.join(invocation, 'ordinary.stdout'),
		ordinary.stdout ?? Buffer.alloc(0),
	);
	filesystem.writeFileSync(
		path.join(invocation, 'ordinary.stderr'),
		ordinary.stderr ?? Buffer.alloc(0),
	);
	const selected = invoke(original);
	filesystem.writeFileSync(
		path.join(invocation, 'selected.stdout'),
		selected.stdout ?? Buffer.alloc(0),
	);
	filesystem.writeFileSync(
		path.join(invocation, 'selected.stderr'),
		selected.stderr ?? Buffer.alloc(0),
	);
	if (filesystem.existsSync(selectedPath))
		filesystem.copyFileSync(
			selectedPath,
			path.join(invocation, 'selected.json'),
			filesystem.constants.COPYFILE_EXCL,
		);
	filesystem.writeFileSync(
		path.join(invocation, 'process.json'),
		`${JSON.stringify({ cwd, binary, binary_sha256: expectedHash, original, ordinary: ordinaryArgs, ordinaryExit: ordinary.status, ordinarySignal: ordinary.signal, ordinaryError: ordinary.error?.message, selectedExit: selected.status, selectedSignal: selected.signal, selectedError: selected.error?.message }, null, 2)}\n`,
		{ flag: 'wx' },
	);
	assert.equal(sha256(binary), expectedHash, 'Pinned CLI bytes changed after execution');
	assert.equal(ordinary.error, undefined, 'Ordinary prepare did not execute');
	assert.equal(selected.error, undefined, 'Selected prepare did not execute');
	assert.equal(ordinary.status, 6, 'Ordinary prepare did not produce prepared exit');
	assert.equal(selected.status, 6, 'Selected prepare did not produce prepared exit');
	assert.ok(
		filesystem.existsSync(ordinaryReport) &&
			filesystem.existsSync(path.join(invocation, 'selected.json')),
		'Saved report missing',
	);
	const parity = requireSnapshotParity(
		filesystem.readFileSync(ordinaryReport, 'utf8'),
		filesystem.readFileSync(path.join(invocation, 'selected.json'), 'utf8'),
		filesystem.readFileSync(path.join(invocation, 'bindings.json'), 'utf8'),
	);
	filesystem.writeFileSync(
		path.join(invocation, 'parity.json'),
		`${JSON.stringify(parity, null, 2)}\n`,
		{ flag: 'wx' },
	);
	assert.ok(selected.stdout, 'Selected stdout missing');
	process.stdout.write(selected.stdout);
	process.stderr.write(selected.stderr ?? Buffer.alloc(0));
	process.exitCode = selected.status;
}
main();
