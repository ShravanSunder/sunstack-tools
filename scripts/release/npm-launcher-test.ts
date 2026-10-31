/** Exercise the owned npm entrypoint against a real child process and Unix signals. */
import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import filesystem from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';
import type { TestContext } from 'node:test';

import { z } from 'zod';

const childSource = `if(process.argv[2]==='exit'){
process.stdout.write('exact stdout\\n');process.stderr.write('exact stderr\\n');process.exitCode=Number(process.argv[3]);
}else{
const cancelled=()=>{process.removeAllListeners('SIGINT');process.removeAllListeners('SIGTERM');process.on('SIGINT',()=>{});process.on('SIGTERM',()=>{});process.stdout.write(JSON.stringify({status:'cancelled',exit:4})+'\\n',()=>process.exit(4));};
process.on('SIGINT',cancelled);process.on('SIGTERM',cancelled);process.stdout.write('READY '+process.pid+'\\n');setInterval(()=>{},1000);
}`;
interface LauncherFixture {
	readonly launcher: string;
	readonly child: string;
}
interface ProcessEnding {
	readonly code: number | null;
	readonly signal: NodeJS.Signals | null;
}
const cancelledSchema = z.object({ status: z.literal('cancelled'), exit: z.literal(4) });

function installedLauncher(context: TestContext): LauncherFixture {
	const directory = filesystem.mkdtempSync(path.join(tmpdir(), 'sunstack-npm-launcher-'));
	context.after(() => filesystem.rmSync(directory, { recursive: true, force: true }));
	const launcher = path.join(directory, 'run.cjs');
	filesystem.copyFileSync(path.join(import.meta.dirname, 'npm-launcher/run.js'), launcher);
	const child = path.join(directory, 'child.ts');
	filesystem.writeFileSync(child, childSource);
	// Download selection stays outside this process-lifetime test, as before.
	filesystem.writeFileSync(
		path.join(directory, 'binary.js'),
		`exports.getPackage=()=>({exists:()=>true,install:()=>{throw Error('unexpected install')},installDirectory:${JSON.stringify(path.dirname(process.execPath))},binaries:{sunstack:${JSON.stringify(path.basename(process.execPath))}}});`,
	);
	return { launcher, child };
}
function withCompletionDeadline<TOutcome>(
	promise: Promise<TOutcome>,
	message: string,
): Promise<TOutcome> {
	let deadline: NodeJS.Timeout | undefined;
	const timeout = new Promise<never>((_resolve, reject) => {
		deadline = setTimeout(() => reject(new Error(message)), 10000);
	});
	return Promise.race([promise, timeout]).finally(() => {
		clearTimeout(deadline);
	});
}
function ignoreMissingProcess(error: unknown): void {
	if (!(error instanceof Error && 'code' in error && error.code === 'ESRCH')) throw error;
}

for (const exitCode of [0, 1, 2, 3]) {
	void test(`native exit ${exitCode} and both streams pass through unchanged`, (context): void => {
		const fixture = installedLauncher(context);
		const result = spawnSync(
			process.execPath,
			[fixture.launcher, fixture.child, 'exit', String(exitCode)],
			{ encoding: 'utf8', timeout: 10000 },
		);
		assert.equal(result.status, exitCode);
		assert.equal(result.stdout, 'exact stdout\n');
		assert.equal(result.stderr, 'exact stderr\n');
	});
}
const signalCases: readonly {
	readonly signal: NodeJS.Signals;
	readonly wholeGroup: boolean;
	readonly expectedCode: number | null;
	readonly expectedSignal: NodeJS.Signals | null;
}[] = [
	{ signal: 'SIGINT', wholeGroup: true, expectedCode: 4, expectedSignal: null },
	{ signal: 'SIGINT', wholeGroup: false, expectedCode: 4, expectedSignal: null },
	{ signal: 'SIGTERM', wholeGroup: false, expectedCode: 4, expectedSignal: null },
	{ signal: 'SIGHUP', wholeGroup: false, expectedCode: null, expectedSignal: 'SIGHUP' },
];
for (const scenario of signalCases) {
	void test(`${scenario.signal} to ${scenario.wholeGroup ? 'group' : 'launcher'} reaches the child and preserves its outcome`, async (context): Promise<void> => {
		const fixture = installedLauncher(context);
		const launcher = spawn(process.execPath, [fixture.launcher, fixture.child, 'signal'], {
			detached: true,
			stdio: ['ignore', 'pipe', 'pipe'],
		});
		assert.ok(launcher.pid);
		const launcherPid = launcher.pid;
		const closed = new Promise<ProcessEnding>((resolve, reject) => {
			launcher.once('close', (code, signal) => resolve({ code, signal }));
			launcher.once('error', reject);
		});
		context.after(async (): Promise<void> => {
			try {
				process.kill(-launcherPid, 'SIGKILL');
			} catch (error) {
				ignoreMissingProcess(error);
			}
			await withCompletionDeadline(closed, 'owned launcher cleanup did not settle');
		});
		const ready = await withCompletionDeadline(
			new Promise<Buffer>((resolve, reject) => {
				launcher.stdout.once('data', (chunk: Buffer) => resolve(chunk));
				launcher.once('error', reject);
			}),
			'real child did not declare signal readiness',
		);
		assert.match(ready.toString(), /^READY [0-9]+\n$/);
		const childPid = z.coerce
			.number()
			.int()
			.gt(1)
			.parse(ready.toString().match(/^READY ([0-9]+)/)?.[1]);
		const stdout: Buffer[] = [],
			stderr: Buffer[] = [];
		launcher.stdout.on('data', (chunk: Buffer) => stdout.push(chunk));
		launcher.stderr.on('data', (chunk: Buffer) => stderr.push(chunk));
		process.kill(scenario.wholeGroup ? -launcherPid : launcherPid, scenario.signal);
		assert.deepEqual(
			await withCompletionDeadline(closed, 'launcher did not settle its native outcome'),
			{
				code: scenario.expectedCode,
				signal: scenario.expectedSignal,
			},
		);
		if (scenario.signal === 'SIGHUP') assert.equal(Buffer.concat(stdout).length, 0);
		else
			assert.deepEqual(cancelledSchema.parse(JSON.parse(Buffer.concat(stdout).toString('utf8'))), {
				status: 'cancelled',
				exit: 4,
			});
		assert.equal(Buffer.concat(stderr).length, 0);
		assert.throws(
			() => process.kill(childPid, 0),
			(error: unknown): boolean =>
				error instanceof Error && 'code' in error && error.code === 'ESRCH',
		);
	});
}
