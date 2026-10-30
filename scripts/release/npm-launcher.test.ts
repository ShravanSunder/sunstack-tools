// Exercise the shipped JavaScript launcher with real processes and Unix signals.
import assert from 'node:assert/strict';
import { execFileSync, spawn, spawnSync } from 'node:child_process';
import { once } from 'node:events';
import { copyFileSync, mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { beforeAll, test } from 'vitest';
import { z } from 'zod';

const compiledLauncher = fileURLToPath(
	new URL('../../target/npm-launcher/run-sunstack.js', import.meta.url),
);

beforeAll((): void => {
	execFileSync('bun', ['run', 'build:npm-launcher'], { stdio: 'pipe' });
});

const childSource = `
if (process.argv[2] === 'exit') {
 process.stdout.write('exact stdout\\n');
 process.stderr.write('exact stderr\\n');
 process.exitCode = Number(process.argv[3]);
} else {
 const cancelled = () => {
  process.removeAllListeners('SIGINT'); process.removeAllListeners('SIGTERM');
  process.on('SIGINT', () => {}); process.on('SIGTERM', () => {});
  process.stdout.write(JSON.stringify({status:'cancelled',exit:4})+'\\n', () => process.exit(4));
 };
 process.on('SIGINT', cancelled); process.on('SIGTERM', cancelled);
 process.stdout.write('READY '+process.pid+'\\n');
 setInterval(() => {}, 1000000);
}
`;

function installedLauncher(): { launcher: string; child: string } {
	const root = mkdtempSync(path.join(tmpdir(), 'sunstack-launcher-test-'));
	const directory = path.join(root, 'node_modules/@sunstack/cli');
	mkdirSync(directory, { recursive: true });
	writeFileSync(
		path.join(directory, 'package.json'),
		JSON.stringify({
			name: '@sunstack/cli',
			version: '0.0.0',
			bin: { sunstack: 'run-sunstack.js' },
		}),
	);
	const launcher = path.join(directory, 'run-sunstack.js');
	copyFileSync(compiledLauncher, launcher);
	const child = path.join(directory, 'child.cjs');
	writeFileSync(child, childSource);
	// Selection/download is outside the process-lifetime boundary, as in the old tests.
	writeFileSync(
		path.join(directory, 'binary.js'),
		`exports.getPackage=()=>({exists:()=>true,install:()=>{throw Error('unexpected install')},installDirectory:${JSON.stringify(path.dirname(process.execPath))},binaries:{sunstack:${JSON.stringify(path.basename(process.execPath))}}});`,
	);
	return { launcher, child };
}

for (const exitCode of [0, 1, 2, 3])
	test(`native exit ${exitCode} and both streams pass through unchanged`, (): void => {
		const { launcher, child } = installedLauncher();
		const result = spawnSync(process.execPath, [launcher, child, 'exit', String(exitCode)], {
			timeout: 10000,
		});
		assert.equal(result.status, exitCode);
		assert.deepEqual(result.stdout, Buffer.from('exact stdout\n'));
		assert.deepEqual(result.stderr, Buffer.from('exact stderr\n'));
	});

for (const [signal, wholeGroup, expectedCode, expectedSignal] of [
	['SIGINT', true, 4, null],
	['SIGINT', false, 4, null],
	['SIGTERM', false, 4, null],
	['SIGHUP', false, null, 'SIGHUP'],
] satisfies [NodeJS.Signals, boolean, number | null, NodeJS.Signals | null][])
	test(`${signal} ${wholeGroup ? 'group' : 'pid'} reaches the child and launcher waits`, async (): Promise<void> => {
		const { launcher, child } = installedLauncher();
		const launcherProcess = spawn(process.execPath, [launcher, child, 'signal'], {
			detached: true,
			stdio: ['ignore', 'pipe', 'pipe'],
		});
		const output: Buffer[] = [];
		const errors: Buffer[] = [];
		launcherProcess.stdout.on('data', (bytes: Buffer): void => {
			output.push(bytes);
		});
		launcherProcess.stderr.on('data', (bytes: Buffer): void => {
			errors.push(bytes);
		});
		const closed = once(launcherProcess, 'close');
		let childPid: number | undefined;
		try {
			await once(launcherProcess.stdout, 'data');
			const ready = Buffer.concat(output).toString();
			assert.match(ready, /^READY \d+\n$/);
			childPid = Number(ready.trim().split(' ')[1]);
			output.length = 0;
			assert.ok(launcherProcess.pid);
			if (wholeGroup) process.kill(-launcherProcess.pid, signal);
			else launcherProcess.kill(signal);
			const [code, endingSignal] = z
				.tuple([z.number().nullable(), z.string().nullable()])
				.parse(await closed);
			assert.equal(code, expectedCode);
			assert.equal(endingSignal, expectedSignal);
			if (signal !== 'SIGHUP')
				assert.deepEqual(
					z
						.object({ status: z.string(), exit: z.number() })
						.parse(JSON.parse(Buffer.concat(output).toString())),
					{ status: 'cancelled', exit: 4 },
				);
			else assert.deepEqual(Buffer.concat(output), Buffer.alloc(0));
			assert.deepEqual(Buffer.concat(errors), Buffer.alloc(0));
			assert.throws(
				() => process.kill(childPid ?? 0, 0),
				(error: unknown): boolean =>
					error instanceof Error && 'code' in error && error.code === 'ESRCH',
			);
		} finally {
			if (childPid)
				try {
					process.kill(childPid, 'SIGTERM');
				} catch (error) {
					assert.ok(error instanceof Error && 'code' in error && error.code === 'ESRCH');
				}
			if (launcherProcess.exitCode === null && launcherProcess.signalCode === null)
				launcherProcess.kill('SIGKILL');
			await closed;
		}
	});
