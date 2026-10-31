#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawn, execFile } from 'node:child_process';
import fileSystem from 'node:fs';
import http from 'node:http';
import type { Socket } from 'node:net';
import path from 'node:path';
import { promisify } from 'node:util';

import { z } from 'zod';

import type { ProcessMember } from './release-process.ts';
import { groupMembers } from './release-process.ts';
import { sha256, save as saveRaw } from './release-registry.ts';
/* eslint-disable no-await-in-loop -- Observation bounds and owned teardown are sequential. */
// S4 Actions: an admitted real request triggers exactly one self-cancellation.

const targetSchema = z.object({
	repository: z.string(),
	run: z.string(),
	head: z.string(),
	workflow: z.string(),
});
const repositorySchema = z.looseObject({ full_name: z.string(), private: z.boolean() });
const runSchema = z.looseObject({
	id: z.number(),
	repository: repositorySchema,
	head_sha: z.string(),
	name: z.string(),
	event: z.string(),
	status: z.string(),
});
const identitySchema = z.looseObject({
	tag: z.string(),
	version: z.string(),
	binary_sha256: z.string(),
	asset_sha256: z.record(z.string(), z.string()),
});
const workSchema = z.looseObject({ pid: z.number(), group: z.number(), binary: z.string() });
const heldBodySchema = z.looseObject({
	state: z.json(),
	questions: z.record(z.string(), z.json()),
});
const configSchema = z.looseObject({ decision_model: z.looseObject({ endpoint: z.string() }) });
const readySchema = z.object({ pid: z.number(), port: z.number(), base: z.string() });
const optionsSchema = targetSchema.extend({
	root: z.string(),
	assets: z.string(),
	identity: z.string(),
});
const commandErrorSchema = z.object({
	code: z.unknown().optional(),
	signal: z.string().nullish(),
	stdout: z.union([z.string(), z.instanceof(Buffer)]).nullish(),
	stderr: z.union([z.string(), z.instanceof(Buffer)]).nullish(),
});
type ControllerTarget = z.infer<typeof targetSchema>;
type WorkRecord = z.infer<typeof workSchema>;
type HeldBody = z.infer<typeof heldBodySchema>;
type ReadyRecord = z.infer<typeof readySchema>;
type ControllerOptions = z.infer<typeof optionsSchema>;
type GithubResult =
	| { code: 0; stdout: string; stderr: string }
	| { code: number | null; signal: string | null; uncertain: true; stdout: string; stderr: string };
interface ControllerSetup {
	readonly root: string;
	readonly assets: string;
	readonly identity: string;
	readonly target: ControllerTarget;
	readonly token: string;
	readonly ghCommand?: readonly string[];
	readonly bounds?: readonly number[];
}
interface TimelineRow {
	readonly after_cancel_request_ms: number;
	readonly processes: ProcessMember[];
	readonly held_sockets: number;
}
interface Admission {
	readonly elapsed_ms: number;
	readonly body: HeldBody;
	readonly native: ProcessMember[];
	readonly work_step: WorkRecord;
}
interface ObservationFacts {
	readonly timeline: TimelineRow[];
	readonly survivors_before_cleanup: ProcessMember[] | null;
	readonly sockets_before_cleanup: number;
	readonly elapsed_ms: number;
}
type ControllerObservation =
	| (ObservationFacts & { readonly admitted: Admission; readonly cancellation: GithubResult })
	| (ObservationFacts & { readonly error: string; readonly admitted: Admission | null });
interface Controller {
	readonly ready: ReadyRecord;
	readonly finish: () => Promise<void>;
	readonly observation: ControllerObservation | undefined;
	readonly pending: Promise<void> | undefined;
	readonly server: http.Server;
}
interface RequestRecord {
	readonly method: string | undefined;
	readonly path: string;
	readonly elapsed_ms: number;
	status?: number;
	sha256?: string;
	bytes?: number;
}
function hasErrorCode(error: unknown, code: string): boolean {
	return error instanceof Error && 'code' in error && error.code === code;
}
const execute = promisify(execFile);
const delay = (milliseconds: number): Promise<void> =>
	new Promise((resolve) => setTimeout(resolve, milliseconds));
const privateRepository = 'ShravanSunder/sunstack-release-rehearsal';
const workflowName = 'S4 cancellation observation';

function validateTarget(
	target: ControllerTarget,
	repository: z.infer<typeof repositorySchema>,
	run: z.infer<typeof runSchema>,
): void {
	assert.equal(target.repository, privateRepository, 'wrong private repository');
	assert.match(target.run, /^[1-9][0-9]*$/);
	assert.match(target.head, /^[0-9a-f]{40}$/);
	assert.equal(target.workflow, workflowName);
	assert.equal(repository.full_name, target.repository);
	assert.equal(repository.private, true, 'repository must remain private');
	assert.equal(String(run.id), target.run, 'wrong run');
	assert.equal(run.repository.full_name, target.repository);
	assert.equal(run.repository.private, true);
	assert.equal(run.head_sha, target.head, 'wrong run head');
	assert.equal(run.name, target.workflow, 'wrong workflow');
	assert.equal(run.event, 'workflow_dispatch');
	assert.equal(run.status, 'in_progress');
}

function safeOutput(value: string | Buffer | null | undefined, token: string): string {
	const text = String(value ?? '');
	assert.ok(
		!token || !text.includes(token),
		'credential appeared in command output; raw output withheld',
	);
	return text;
}

async function startController({
	root,
	assets,
	identity,
	target,
	token,
	ghCommand = ['gh'],
	bounds = [0, 100, 500, 1500, 4000, 8000, 12000, 20000],
}: ControllerSetup): Promise<Controller> {
	assert.ok(token, 'controller token required');
	const save = (file: string, value: unknown): void => {
		safeOutput(JSON.stringify(value), token);
		saveRaw(file, value);
	};
	assert.equal(target.repository, privateRepository);
	assert.match(target.run, /^[1-9][0-9]*$/);
	assert.match(target.head, /^[0-9a-f]{40}$/);
	assert.equal(target.workflow, workflowName);
	assert.ok(!fileSystem.existsSync(path.join(root, 'ready.json')), 'controller already prepared');
	const expected = identitySchema.parse(JSON.parse(fileSystem.readFileSync(identity, 'utf8')));
	assert.equal(expected.tag, 'v0.0.7');
	assert.equal(expected.version, '0.0.7');
	assert.match(expected.binary_sha256, /^[0-9a-f]{64}$/);
	fileSystem.mkdirSync(root, { recursive: true });
	const started = performance.now();
	const elapsed = (): number => Math.round(performance.now() - started);
	const heldSockets = new Set<Socket>();
	const requests: RequestRecord[] = [];
	const timeline: TimelineRow[] = [];
	const admittedNative = new Map<number, string>();
	let admitted: Admission | undefined;
	let work: WorkRecord | undefined;
	let workIdentity: string | undefined;
	let frozen: ControllerObservation | undefined;
	let cancelStarted = false;
	let observationPromise: Promise<void> | undefined;
	let closed = false;
	async function github(words: readonly string[]): Promise<GithubResult> {
		const executable = ghCommand[0];
		assert.ok(executable);
		try {
			const result = await execute(executable, [...ghCommand.slice(1), ...words], {
				cwd: root,
				timeout: 20000,
				maxBuffer: 1024 * 1024,
				env: {
					PATH: process.env['PATH'],
					HOME: root,
					GH_TOKEN: token,
					GH_HOST: 'github.com',
					GH_PROMPT_DISABLED: '1',
				},
			});
			return {
				code: 0,
				stdout: safeOutput(result.stdout, token),
				stderr: safeOutput(result.stderr, token),
			};
		} catch (error) {
			const failure = commandErrorSchema.parse(error);
			return {
				code: typeof failure.code === 'number' ? failure.code : null,
				signal: failure.signal?.length ? failure.signal : null,
				uncertain: true,
				stdout: safeOutput(failure.stdout, token),
				stderr: safeOutput(failure.stderr, token),
			};
		}
	}
	async function admitTarget(): Promise<void> {
		const repository = await github(['api', 'repos/' + target.repository]);
		const run = await github(['api', 'repos/' + target.repository + '/actions/runs/' + target.run]);
		assert.equal(repository.code, 0, 'private metadata read failed');
		assert.equal(run.code, 0, 'own run read failed');
		validateTarget(
			target,
			repositorySchema.parse(JSON.parse(repository.stdout)),
			runSchema.parse(JSON.parse(run.stdout)),
		);
		save(path.join(root, 'target.json'), { ...target, private: true, validated_at_ms: elapsed() });
	}
	await admitTarget();
	function ownedFacts(): ProcessMember[] {
		const currentWork = workSchema.parse(
			JSON.parse(fileSystem.readFileSync(path.join(root, 'work-step.json'), 'utf8')),
		);
		work = currentWork;
		assert.ok(Number.isSafeInteger(work.pid) && work.pid > 1);
		assert.equal(work.binary, path.join(root, 'installed/sunstack'));
		const members = groupMembers(currentWork.group);
		const ownedIds = new Set(
			members
				.filter(
					(member) =>
						(member.pid === currentWork.pid &&
							(!workIdentity || member.command === workIdentity)) ||
						admittedNative.get(member.pid) === member.command,
				)
				.map((member) => member.pid),
		);
		for (let index = 0; index < members.length; index++)
			for (const member of members) if (ownedIds.has(member.parent)) ownedIds.add(member.pid);
		return members.filter((member) => ownedIds.has(member.pid));
	}
	async function cancelAfterAdmission(requestBody: HeldBody): Promise<void> {
		assert.equal(cancelStarted, false, 'duplicate cancel forbidden');
		safeOutput(JSON.stringify(requestBody), token);
		const members = ownedFacts();
		assert.ok(work);
		const admittedWork = work;
		workIdentity = members.find((member) => member.pid === admittedWork.pid)?.command;
		assert.equal(
			sha256(fileSystem.readFileSync(admittedWork.binary)),
			expected.binary_sha256,
			'installed bytes differ',
		);
		const native = members.filter((member) => {
			try {
				if (process.platform === 'linux')
					return (
						fileSystem.realpathSync('/proc/' + member.pid + '/exe') ===
						fileSystem.realpathSync(admittedWork.binary)
					);
				return member.command.split(/\s+/).includes(admittedWork.binary);
			} catch (error) {
				if (hasErrorCode(error, 'ENOENT') || hasErrorCode(error, 'ESRCH')) return false;
				throw error;
			}
		});
		assert.ok(native.length > 0, 'admitted request lacks installed native owner');
		for (const member of native) admittedNative.set(member.pid, member.command);
		admitted = { elapsed_ms: elapsed(), body: requestBody, native, work_step: admittedWork };
		save(path.join(root, 'admitted.json'), admitted);
		await admitTarget();
		cancelStarted = true;
		const cancelAt = elapsed();
		const command = [...ghCommand, 'run', 'cancel', target.run, '--repo', target.repository];
		save(path.join(root, 'cancel-command.json'), {
			command,
			elapsed_ms: cancelAt,
			after_real_admission: true,
		});
		const cancellation = github(command.slice(ghCommand.length)).then((result) => {
			fileSystem.writeFileSync(path.join(root, 'cancel.stdout.log'), result.stdout);
			fileSystem.writeFileSync(path.join(root, 'cancel.stderr.log'), result.stderr);
			save(path.join(root, 'cancel-result.json'), { ...result, elapsed_ms: elapsed() });
			return result;
		});
		for (const bound of bounds) {
			await delay(Math.max(0, cancelAt + bound - elapsed()));
			timeline.push({
				after_cancel_request_ms: elapsed() - cancelAt,
				processes: ownedFacts(),
				held_sockets: heldSockets.size,
			});
			save(path.join(root, 'timeline.json'), timeline);
		}
		const cancelResult = await cancellation;
		frozen = {
			admitted,
			cancellation: cancelResult,
			timeline,
			survivors_before_cleanup: ownedFacts(),
			sockets_before_cleanup: heldSockets.size,
			elapsed_ms: elapsed(),
		};
		save(path.join(root, 'observation.json'), frozen);
		save(path.join(root, 'complete.json'), {
			observation_frozen: true,
			cancellation_exit: cancelResult.code,
		});
	}
	function failObservation(error: unknown): void {
		const message = safeOutput(error instanceof Error ? error.message : String(error), token);
		let processes: ProcessMember[] | null = null;
		try {
			processes = ownedFacts();
		} catch {
			/* No owned work record is an explicit setup gap. */
		}
		frozen = {
			error: message,
			admitted: admitted ?? null,
			timeline,
			survivors_before_cleanup: processes,
			sockets_before_cleanup: heldSockets.size,
			elapsed_ms: elapsed(),
		};
		save(path.join(root, 'controller-error.json'), { message, elapsed_ms: elapsed() });
		save(path.join(root, 'observation.json'), frozen);
		save(path.join(root, 'complete.json'), { observation_frozen: true, status: 'unknown' });
	}
	const server = http.createServer((request, response) => {
		const route = new URL(request.url ?? '/', 'http://127.0.0.1').pathname;
		requests.push({ method: request.method, path: route, elapsed_ms: elapsed() });
		save(path.join(root, 'requests.json'), requests);
		if (request.method === 'GET' && route === '/health') {
			response.end('ready');
			return;
		}
		if (
			request.method === 'GET' &&
			/^\/archives\/(sunstack-x86_64-unknown-linux-musl\.tar\.gz(?:\.sha256)?|sha256.sum)$/.test(
				route,
			)
		) {
			const name = path.basename(route),
				bytes = fileSystem.readFileSync(path.join(assets, name));
			assert.equal(sha256(bytes), expected.asset_sha256[name], 'served archive identity');
			const currentRequest = requests.at(-1);
			assert.ok(currentRequest);
			Object.assign(currentRequest, { status: 200, sha256: sha256(bytes), bytes: bytes.length });
			save(path.join(root, 'requests.json'), requests);
			response.end(bytes);
			return;
		}
		if (
			request.method === 'POST' &&
			route === '/systemone' &&
			!admitted &&
			!observationPromise &&
			!frozen
		) {
			let size = 0;
			const chunks: Buffer[] = [];
			request.on('data', (chunk: Buffer) => {
				size += chunk.length;
				if (size > 262144) request.destroy();
				else chunks.push(chunk);
			});
			request.on('end', () => {
				if (frozen) {
					response.writeHead(409).end('admission closed');
					return;
				}
				try {
					const body = heldBodySchema.parse(JSON.parse(Buffer.concat(chunks).toString('utf8')));
					assert.ok(
						body.state &&
							body.questions &&
							!Array.isArray(body.questions) &&
							Object.keys(body.questions).length > 0,
					);
					heldSockets.add(request.socket);
					request.socket.on('close', () => heldSockets.delete(request.socket));
					observationPromise = cancelAfterAdmission(body).catch(failObservation);
				} catch {
					response.writeHead(400).end('invalid held request');
				}
			});
			return;
		}
		if (request.method === 'POST' && route === '/finish' && frozen) {
			response.end('cleanup');
			void finish();
			return;
		}
		response.writeHead(404).end();
	});
	async function finish(): Promise<void> {
		assert.ok(frozen, 'cleanup forbidden before frozen observation');
		if (closed) return;
		closed = true;
		const survivors = work ? ownedFacts() : [];
		for (const member of survivors) {
			// Do not kill a reused PID or a command outside the captured work tree.
			if (member.pid === work?.pid || admittedNative.get(member.pid) === member.command) {
				try {
					process.kill(member.pid, 'SIGKILL');
				} catch (error) {
					if (!hasErrorCode(error, 'ESRCH')) throw error;
				}
			}
		}
		for (const socket of heldSockets) socket.destroy();
		server.closeAllConnections();
		await new Promise<void>((resolve) => server.close(() => resolve()));
		await delay(100);
		save(path.join(root, 'cleanup.json'), {
			survivors_after_cleanup: work ? ownedFacts() : [],
			listening: server.listening,
			observation_unchanged:
				JSON.stringify(frozen) ===
				JSON.stringify(
					JSON.parse(fileSystem.readFileSync(path.join(root, 'observation.json'), 'utf8')),
				),
		});
	}
	await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
	const address = server.address();
	assert.ok(address && typeof address !== 'string');
	const ready = {
		pid: process.pid,
		port: address.port,
		base: 'http://127.0.0.1:' + address.port,
	};
	const configPath = path.join(root, 'fixture/sunstack.lint.json');
	const config = configSchema.parse(JSON.parse(fileSystem.readFileSync(configPath, 'utf8')));
	config.decision_model.endpoint = ready.base + '/systemone';
	save(configPath, config);
	save(path.join(root, 'ready.json'), ready);
	const admissionDeadline = setTimeout(() => {
		if (!admitted && !frozen) failObservation(new Error('no admitted real request within120s'));
	}, 120000);
	admissionDeadline.unref();
	return {
		ready,
		finish,
		get observation(): ControllerObservation | undefined {
			return frozen;
		},
		get pending(): Promise<void> | undefined {
			return observationPromise;
		},
		server,
	};
}

function parseOptions(words: readonly string[]): ControllerOptions {
	const allowed = ['root', 'assets', 'identity', 'repository', 'run', 'head', 'workflow'];
	const options: Record<string, string> = {};
	for (let index = 0; index < words.length; index += 2) {
		const word = words[index];
		assert.ok(word);
		const name = word.replace(/^--/, '');
		const value = words[index + 1];
		assert.ok(allowed.includes(name) && value && !options[name], 'invalid controller argument');
		options[name] = value;
	}
	for (const name of allowed) assert.ok(options[name], '--' + name + ' required');
	return optionsSchema.parse(options);
}

async function main(words: string[]): Promise<void> {
	const action = words.shift();
	if (action === 'finish') {
		const directory = words[0];
		assert.ok(directory);
		const root = path.resolve(directory);
		const until = Date.now() + 45000;
		while (!fileSystem.existsSync(path.join(root, 'complete.json')) && Date.now() < until)
			await delay(100);
		assert.ok(
			fileSystem.existsSync(path.join(root, 'complete.json')),
			'controller observation missing; retain logs as unknown',
		);
		const ready = readySchema.parse(
			JSON.parse(fileSystem.readFileSync(path.join(root, 'ready.json'), 'utf8')),
		);
		const result = await fetch(ready.base + '/finish', { method: 'POST' });
		assert.equal(result.status, 200);
		while (!fileSystem.existsSync(path.join(root, 'cleanup.json')) && Date.now() < until)
			await delay(100);
		assert.ok(
			fileSystem.existsSync(path.join(root, 'cleanup.json')),
			'owned cleanup result missing',
		);
		return;
	}
	assert.ok(action && ['start', 'serve'].includes(action));
	const options = parseOptions(words);
	if (action === 'start') {
		assert.ok(process.env['GH_TOKEN'], 'controller token required');
		fileSystem.mkdirSync(options.root, { recursive: true });
		const output = fileSystem.openSync(path.join(options.root, 'controller.log'), 'wx', 0o600);
		const child = spawn(process.execPath, [import.meta.filename, 'serve', ...words], {
			detached: true,
			stdio: ['ignore', output, output],
			env: { PATH: process.env['PATH'], GH_TOKEN: process.env['GH_TOKEN'] },
		});
		child.unref();
		fileSystem.closeSync(output);
		const until = Date.now() + 30000;
		while (!fileSystem.existsSync(path.join(options.root, 'ready.json')) && Date.now() < until)
			await delay(100);
		assert.ok(
			fileSystem.existsSync(path.join(options.root, 'ready.json')),
			'controller did not become ready',
		);
		process.stdout.write('controller ready\n');
		return;
	}
	const target = targetSchema.parse(options);
	const token = process.env['GH_TOKEN'];
	assert.ok(token, 'controller token required');
	await startController({ ...options, target, token });
}

if (import.meta.main)
	main(process.argv.slice(2)).catch((error) => {
		process.stderr.write((error instanceof Error ? error.message : String(error)) + '\n');
		process.exitCode = 2;
	});
export { startController, validateTarget, safeOutput };
