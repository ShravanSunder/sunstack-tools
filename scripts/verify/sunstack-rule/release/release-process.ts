import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import type { ChildProcessByStdio } from 'node:child_process';
import fileSystem from 'node:fs';
import http from 'node:http';
import type { Socket } from 'node:net';
import path from 'node:path';
import type { Readable } from 'node:stream';

import { z } from 'zod';

import { sha256, save } from './release-registry.ts';
/* eslint-disable no-await-in-loop -- Each fixture and process must settle before the next owned cell begins. */
// S4 real subprocess capture. Observation is frozen before owned cleanup.

export interface ProcessMember {
	readonly pid: number;
	readonly parent: number;
	readonly group: number;
	readonly state: string;
	readonly started: string;
	readonly command: string;
}
export interface OwnershipCensus {
	readonly kind: 'established' | 'unestablished';
	readonly initial_root: ProcessMember | null;
	readonly initial_group_members: ProcessMember[];
}
interface OwnedTreeObserver {
	(): ProcessMember[];
	ownership?: () => OwnershipCensus;
}
export interface StopRequest {
	readonly signal: NodeJS.Signals;
	readonly destination: 'pid' | 'group';
	readonly expectation: string;
}
interface ProcessEnd {
	readonly code: number | null;
	readonly signal: NodeJS.Signals | null;
	readonly elapsed_ms: number;
}
interface NativeIdentity {
	readonly pid: number;
	readonly file: string;
	readonly sha256: string;
}
const heldRequestSchema = z.looseObject({
	questions: z.record(z.string(), z.json()),
	state: z.json(),
});
const lintConfigSchema = z.looseObject({ decision_model: z.looseObject({ endpoint: z.string() }) });
type HeldRequest = z.infer<typeof heldRequestSchema>;
interface StopObservation {
	readonly after_signal_ms: number;
	readonly exit: ProcessEnd | null;
	readonly close: ProcessEnd | null;
	readonly owned_tree: ProcessMember[];
	readonly sockets: number;
}
export interface ProcessObservation {
	readonly command: readonly string[];
	readonly cwd: string;
	readonly pid: number | null;
	readonly root_exit: ProcessEnd | null;
	readonly root_close: ProcessEnd | null;
	readonly error: string | null;
	readonly elapsed_ms: number;
	readonly admitted_request: HeldRequest | null;
	readonly signal_at_ms: number | null;
	readonly stop: StopRequest | null;
	readonly owned_tree_at_readiness: ProcessMember[];
	readonly native_identity_at_readiness: NativeIdentity[];
	readonly timeline: StopObservation[];
	readonly ownership: OwnershipCensus;
	readonly survivors_before_cleanup: ProcessMember[] | null;
	readonly unestablished_group_members_before_cleanup: ProcessMember[] | null;
	readonly sockets_before_cleanup: number;
}
export interface ProcessCapture {
	readonly stdout: Buffer;
	readonly stderr: Buffer;
	readonly observation: ProcessObservation;
}
export interface CaptureOptions {
	readonly command: readonly string[];
	readonly cwd: string;
	readonly environment: NodeJS.ProcessEnv;
	readonly stem: string;
	readonly stop?: StopRequest | undefined;
	readonly expectedNativeSha?: string | undefined;
	readonly timeoutMs?: number;
}
type CapturedChild = ChildProcessByStdio<null, Readable, Readable>;
const delay = (milliseconds: number): Promise<void> =>
	new Promise((resolve) => setTimeout(resolve, milliseconds));
const noOwnedProcesses: OwnedTreeObserver = (): ProcessMember[] => [];

function hasErrorCode(error: unknown, codes: readonly string[]): boolean {
	return (
		error instanceof Error &&
		'code' in error &&
		typeof error.code === 'string' &&
		codes.includes(error.code)
	);
}

function processMembers(): ProcessMember[] {
	if (process.platform === 'darwin') {
		const result = spawnSync('/bin/ps', ['-axo', 'pid=,ppid=,pgid=,stat=,lstart=,command='], {
			encoding: 'utf8',
		});
		assert.equal(
			result.status,
			0,
			'owned-group inspection unavailable: ' +
				(result.error?.message?.length ? result.error.message : result.stderr),
		);
		return result.stdout.split('\n').flatMap((line): ProcessMember[] => {
			const fields = line.trim().split(/\s+/);
			const [processId, parentId, processGroup, processState] = fields;
			return fields.length >= 10 &&
				processId &&
				parentId &&
				processGroup &&
				processState &&
				/^\d+$/.test(processId)
				? [
						{
							pid: +processId,
							parent: +parentId,
							group: +processGroup,
							state: processState,
							started: fields.slice(4, 9).join(' '),
							command: fields.slice(9).join(' '),
						},
					]
				: [];
		});
	}
	return fileSystem
		.readdirSync('/proc')
		.filter((name) => /^\d+$/.test(name))
		.flatMap((name) => {
			try {
				const stat = fileSystem.readFileSync('/proc/' + name + '/stat', 'utf8');
				const fields = stat.slice(stat.lastIndexOf(')') + 2).split(' ');
				const [processState, parentId, processGroup] = fields;
				const started = fields[19];
				assert.ok(processState && parentId && processGroup && started);
				return [
					{
						pid: +name,
						parent: +parentId,
						group: +processGroup,
						state: processState,
						started,
						command: fileSystem
							.readFileSync('/proc/' + name + '/cmdline', 'utf8')
							.replaceAll('\0', ' '),
					},
				];
			} catch (error) {
				if (hasErrorCode(error, ['ENOENT', 'ESRCH'])) return [];
				throw error;
			}
		});
}

function groupMembers(group: number): ProcessMember[] {
	return processMembers().filter((member) => member.group === group);
}

function trackOwnedTree(rootPid: number): OwnedTreeObserver {
	const knownStarts = new Map<number, string>();
	let initialCensus: OwnershipCensus | undefined;
	const observe: OwnedTreeObserver = (): ProcessMember[] => {
		const members = processMembers();
		const root = members.find((member) => member.pid === rootPid);
		if (!initialCensus) {
			const established = root?.parent === process.pid;
			const seeds =
				established && root ? [root] : members.filter((member) => member.group === rootPid);
			initialCensus = {
				kind: established ? 'established' : 'unestablished',
				initial_root: root ?? null,
				initial_group_members: members.filter((member) => member.group === rootPid),
			};
			// A missed root is never proof of absence. Its still-existing group
			// supplies cleanup candidates, but does not establish the missing tree.
			for (const member of seeds) knownStarts.set(member.pid, member.started);
		}
		const owned = new Set(
			members
				.filter(
					(member) => knownStarts.has(member.pid) && knownStarts.get(member.pid) === member.started,
				)
				.map((member) => member.pid),
		);
		for (let index = 0; index < members.length; index++) {
			for (const member of members) {
				if (owned.has(member.parent)) {
					knownStarts.set(member.pid, member.started);
					owned.add(member.pid);
				}
			}
		}
		return members.filter((member) => owned.has(member.pid));
	};
	observe.ownership = () =>
		initialCensus ?? { kind: 'unestablished', initial_root: null, initial_group_members: [] };
	return observe;
}

async function settleOwnedTree(observe: OwnedTreeObserver): Promise<ProcessMember[]> {
	let members = observe();
	for (let attempt = 0; attempt < 100 && members.length; attempt++) {
		await delay(30);
		members = observe();
	}
	return members;
}

async function settleGroup(group: number): Promise<ProcessMember[]> {
	let members = groupMembers(group);
	for (let attempt = 0; attempt < 100 && members.length; attempt++) {
		await delay(30);
		members = groupMembers(group);
	}
	return members;
}

function signalOwnedTarget(targetPid: number, signal: NodeJS.Signals): void {
	try {
		process.kill(targetPid, signal);
	} catch (error) {
		if (!hasErrorCode(error, ['ESRCH'])) throw error;
	}
}

function ownedKill(group: number, signal: NodeJS.Signals): void {
	signalOwnedTarget(-group, signal);
}

async function captureProcess({
	command,
	cwd,
	environment,
	stem,
	stop,
	expectedNativeSha,
	timeoutMs = 120000,
}: CaptureOptions): Promise<ProcessCapture> {
	// Refuse before spawning a product or held request when the host cannot
	// supply process-group observations. A denied inspection is not no survivors.
	groupMembers(process.pid);
	fileSystem.mkdirSync(path.dirname(stem), { recursive: true });
	const started = performance.now();
	const elapsed = (): number => Math.round(performance.now() - started);
	let child: CapturedChild | undefined;
	let server: http.Server | undefined;
	let admitted: HeldRequest | undefined;
	let signalAt: number | undefined;
	let readyGroup: ProcessMember[] | undefined;
	let nativeIdentity: NativeIdentity[] | undefined;
	let rootExit: ProcessEnd | null = null;
	let rootClose: ProcessEnd | null = null;
	let errorText: string | undefined;
	const stdout: Buffer[] = [],
		stderr: Buffer[] = [];
	const sockets = new Set<Socket>();
	const timeline: StopObservation[] = [];
	const deadlines: NodeJS.Timeout[] = [];
	const deadline = <TOutcome>(milliseconds: number, value: TOutcome): Promise<TOutcome> =>
		new Promise<TOutcome>((resolve) => {
			deadlines.push(setTimeout(() => resolve(value), milliseconds));
		});
	let readyResolve: ((value: boolean) => void) | undefined;
	let socketsClosedResolve: ((value: boolean) => void) | undefined;
	const socketsClosed = new Promise<boolean>((resolve) => {
		socketsClosedResolve = resolve;
	});
	const ready = new Promise<boolean>((resolve) => {
		readyResolve = resolve;
	});
	let configBefore: Buffer | undefined;
	let observeOwned: OwnedTreeObserver = noOwnedProcesses;
	const configPath = path.join(cwd, 'sunstack.lint.json');
	if (stop) {
		server = http.createServer((request) => {
			const chunks: Buffer[] = [];
			let bytes = 0;
			request.on('data', (chunk) => {
				bytes += chunk.length;
				if (bytes > 1024 * 1024) request.destroy();
				else chunks.push(chunk);
			});
			request.on('end', () => {
				try {
					admitted = heldRequestSchema.parse(JSON.parse(Buffer.concat(chunks).toString('utf8')));
					assert.ok(
						admitted.questions &&
							!Array.isArray(admitted.questions) &&
							typeof admitted.questions === 'object' &&
							Object.keys(admitted.questions).length > 0,
					);
					assert.ok(admitted.state);
					readyGroup = observeOwned();
					nativeIdentity = [];
					for (const member of readyGroup) {
						for (const token of member.command.split(/\s+/)) {
							if (!token.startsWith('/') || path.basename(token) !== 'sunstack') continue;
							try {
								const digest = sha256(fileSystem.readFileSync(token));
								if (digest === expectedNativeSha)
									nativeIdentity.push({ pid: member.pid, file: token, sha256: digest });
							} catch (error) {
								if (!hasErrorCode(error, ['ENOENT', 'EACCES'])) throw error;
							}
						}
					}
					assert.ok(
						nativeIdentity.length,
						'held request must be owned by the installed release native process',
					);
					assert.ok(readyResolve);
					readyResolve(true);
				} catch (error) {
					errorText = error instanceof Error ? error.stack : undefined;
					assert.ok(readyResolve);
					readyResolve(false);
				}
			});
		});
		server.on('connection', (socket) => {
			sockets.add(socket);
			socket.on('close', () => {
				sockets.delete(socket);
				if (sockets.size === 0) {
					assert.ok(socketsClosedResolve);
					socketsClosedResolve(true);
				}
			});
		});
		const heldServer = server;
		await new Promise<void>((resolve) => heldServer.listen(0, '127.0.0.1', resolve));
		configBefore = fileSystem.readFileSync(configPath);
		const config = lintConfigSchema.parse(JSON.parse(configBefore.toString('utf8')));
		const address = server.address();
		assert.ok(address && typeof address === 'object');
		config.decision_model.endpoint = 'http://127.0.0.1:' + address.port + '/systemone';
		save(configPath, config);
	}
	let observation: ProcessObservation, frozenStdout: Buffer, frozenStderr: Buffer;
	try {
		const actualEnvironment = stop
			? { ...environment, OPENROUTER_API_KEY: 'SYNTHETIC_RUNNER_CONTROL_NOT_A_SECRET' }
			: environment;
		save(stem + '.command.json', {
			command,
			cwd,
			environment: actualEnvironment,
			stop: stop ?? null,
			timeout_ms: timeoutMs,
		});
		const executable = command[0];
		assert.ok(executable);
		child = spawn(executable, command.slice(1), {
			cwd,
			env: actualEnvironment,
			detached: true,
			stdio: ['ignore', 'pipe', 'pipe'],
		});
		if (child.pid) {
			observeOwned = trackOwnedTree(child.pid);
			observeOwned();
		}
		child.stdout.on('data', (chunk) => stdout.push(chunk));
		child.stderr.on('data', (chunk) => stderr.push(chunk));
		let closedResolve: ((value: 'closed') => void) | undefined;
		let failedResolve: ((value: 'spawn-error') => void) | undefined;
		const closed = new Promise<'closed'>((resolve) => {
			closedResolve = resolve;
		});
		const failed = new Promise<'spawn-error'>((resolve) => {
			failedResolve = resolve;
		});
		child.on('error', (error) => {
			errorText = error.stack;
			assert.ok(failedResolve);
			failedResolve('spawn-error');
		});
		child.on('exit', (code, signal) => {
			rootExit = { code, signal, elapsed_ms: elapsed() };
		});
		child.on('close', (code, signal) => {
			rootClose = { code, signal, elapsed_ms: elapsed() };
			assert.ok(closedResolve);
			closedResolve('closed');
		});
		if (stop) {
			const readiness = await Promise.race([
				ready,
				failed,
				closed.then(() => false),
				deadline(30000, false),
			]);
			assert.equal(
				readiness,
				true,
				errorText?.length ? errorText : 'no admitted request before stop',
			);
			signalAt = elapsed();
			assert.ok(child.pid);
			if (stop.destination === 'group') ownedKill(child.pid, stop.signal);
			else process.kill(child.pid, stop.signal);
			if (stop.expectation === 'no-cancelled-report') {
				for (const bound of [100, 500, 1500]) {
					await delay(Math.max(0, signalAt + bound - elapsed()));
					timeline.push({
						after_signal_ms: elapsed() - signalAt,
						exit: rootExit,
						close: rootClose,
						owned_tree: observeOwned(),
						sockets: sockets.size,
					});
				}
			} else {
				const result = await Promise.race([closed, failed, deadline(15000, 'timeout')]);
				if (result !== 'closed')
					errorText = errorText?.length ? errorText : 'supported stop did not close within15s';
			}
		} else {
			const result = await Promise.race([closed, failed, deadline(timeoutMs, 'timeout')]);
			if (result !== 'closed')
				errorText = errorText?.length ? errorText : 'normal invocation did not close within bound';
		}
	} catch (error) {
		errorText = error instanceof Error ? error.stack : undefined;
	} finally {
		const before = child?.pid
			? stop?.expectation === 'no-cancelled-report'
				? observeOwned()
				: await settleOwnedTree(observeOwned)
			: [];
		if (stop && stop.expectation !== 'no-cancelled-report' && sockets.size > 0) {
			await Promise.race([socketsClosed, deadline(3000, false)]);
		}
		const ownership = observeOwned.ownership?.() ?? {
			kind: 'unestablished',
			initial_root: null,
			initial_group_members: [],
		};
		if (ownership.kind !== 'established')
			errorText =
				'ownership unestablished: root absent from initial census' +
				(errorText ? '\n' + errorText : '');
		frozenStdout = Buffer.concat(stdout);
		frozenStderr = Buffer.concat(stderr);
		observation = {
			command,
			cwd,
			pid: child?.pid ?? null,
			root_exit: rootExit,
			root_close: rootClose,
			error: errorText?.length ? errorText : null,
			elapsed_ms: elapsed(),
			admitted_request: admitted ?? null,
			signal_at_ms: signalAt ?? null,
			stop: stop ?? null,
			owned_tree_at_readiness: readyGroup ?? [],
			native_identity_at_readiness: nativeIdentity ?? [],
			timeline,
			ownership,
			survivors_before_cleanup: ownership.kind === 'established' ? before : null,
			unestablished_group_members_before_cleanup: ownership.kind === 'established' ? null : before,
			sockets_before_cleanup: sockets.size,
		};
		fileSystem.writeFileSync(stem + '.stdout.log', frozenStdout);
		fileSystem.writeFileSync(stem + '.stderr.log', frozenStderr);
		save(stem + '.process.json', observation);
		let after: ProcessMember[] = [];
		if (child?.pid) {
			if (observeOwned().length) {
				ownedKill(child.pid, 'SIGINT');
				after = await settleOwnedTree(observeOwned);
			}
			if (after.length) {
				for (const member of observeOwned()) {
					signalOwnedTarget(member.pid, 'SIGKILL');
				}
				after = await settleOwnedTree(observeOwned);
			}
		}
		for (const socket of sockets) socket.destroy();
		if (sockets.size > 0) await Promise.race([socketsClosed, deadline(3000, false)]);
		if (server) {
			server.closeAllConnections();
			const heldServer = server;
			await new Promise<void>((resolve) => heldServer.close(() => resolve()));
		}
		if (configBefore) fileSystem.writeFileSync(configPath, configBefore);
		fileSystem.writeFileSync(stem + '.after-cleanup.stdout.log', Buffer.concat(stdout));
		fileSystem.writeFileSync(stem + '.after-cleanup.stderr.log', Buffer.concat(stderr));
		save(stem + '.cleanup.json', {
			survivors_after_cleanup: after,
			registry_control_stopped: !server?.listening,
			sockets_after_cleanup: sockets.size,
		});
		assert.deepEqual(after, [], 'owned process cleanup failed');
		assert.equal(sockets.size, 0, 'owned socket cleanup failed');
		for (const timer of deadlines) clearTimeout(timer);
	}
	return { stdout: frozenStdout, stderr: frozenStderr, observation };
}

export { captureProcess, groupMembers, settleGroup, trackOwnedTree };
