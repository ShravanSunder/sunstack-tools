#!/usr/bin/env node
import assert from 'node:assert/strict';
import fileSystem from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import { spawn, execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { sha256, save as  saveRaw } from './release-registry.ts';
import { groupMembers } from './release-process.ts';
/* eslint-disable no-await-in-loop -- Observation bounds and owned teardown are sequential. */
// S4 Actions: an admitted real request triggers exactly one self-cancellation.








const execute = promisify(execFile);
const delay = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));
const privateRepository = 'ShravanSunder/sunstack-release-rehearsal';
const workflowName = 'S4 cancellation observation';

function validateTarget(target, repository, run) {
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

function safeOutput(value, token) {
	const text = String(value || '');
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
}) {
	assert.ok(token, 'controller token required');
	const save = (file, value) => {
		safeOutput(JSON.stringify(value), token);
		saveRaw(file, value);
	};
	assert.equal(target.repository, privateRepository);
	assert.match(target.run, /^[1-9][0-9]*$/);
	assert.match(target.head, /^[0-9a-f]{40}$/);
	assert.equal(target.workflow, workflowName);
	assert.ok(!fileSystem.existsSync(path.join(root, 'ready.json')), 'controller already prepared');
	const expected = JSON.parse(fileSystem.readFileSync(identity));
	assert.equal(expected.tag, 'v0.0.7');
	assert.equal(expected.version, '0.0.7');
	assert.match(expected.binary_sha256, /^[0-9a-f]{64}$/);
	fileSystem.mkdirSync(root, { recursive: true });
	const started = performance.now();
	const elapsed = () => Math.round(performance.now() - started);
	const heldSockets = new Set(),
		requests = [],
		timeline = [],
		admittedNative = new Map();
	let admitted,
		work,
		workIdentity,
		frozen,
		cancelStarted = false,
		observationPromise,
		closed = false;
	async function github(words) {
		try {
			const result = await execute(ghCommand[0], [...ghCommand.slice(1), ...words], {
				cwd: root,
				timeout: 20000,
				maxBuffer: 1024 * 1024,
				env: {
					PATH: process.env.PATH,
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
			return {
				code: typeof error.code === 'number' ? error.code : null,
				signal: error.signal || null,
				uncertain: true,
				stdout: safeOutput(error.stdout, token),
				stderr: safeOutput(error.stderr, token),
			};
		}
	}
	async function admitTarget() {
		const repository = await github(['api', 'repos/' + target.repository]);
		const run = await github(['api', 'repos/' + target.repository + '/actions/runs/' + target.run]);
		assert.equal(repository.code, 0, 'private metadata read failed');
		assert.equal(run.code, 0, 'own run read failed');
		validateTarget(target, JSON.parse(repository.stdout), JSON.parse(run.stdout));
		save(path.join(root, 'target.json'), { ...target, private: true, validated_at_ms: elapsed() });
	}
	await admitTarget();
	function ownedFacts() {
		work = JSON.parse(fileSystem.readFileSync(path.join(root, 'work-step.json')));
		assert.ok(Number.isSafeInteger(work.pid) && work.pid > 1);
		assert.equal(work.binary, path.join(root, 'installed/sunstack'));
		const members = groupMembers(work.group);
		const ownedIds = new Set(
			members
				.filter(
					(member) =>
						(member.pid === work.pid && (!workIdentity || member.command === workIdentity)) ||
						admittedNative.get(member.pid) === member.command,
				)
				.map((member) => member.pid),
		);
		for (let index = 0; index < members.length; index++)
			for (const member of members) if (ownedIds.has(member.parent)) ownedIds.add(member.pid);
		return members.filter((member) => ownedIds.has(member.pid));
	}
	async function cancelAfterAdmission(requestBody) {
		assert.equal(cancelStarted, false, 'duplicate cancel forbidden');
		safeOutput(JSON.stringify(requestBody), token);
		const members = ownedFacts();
		workIdentity = members.find((member) => member.pid === work.pid)?.command;
		assert.equal(
			sha256(fileSystem.readFileSync(work.binary)),
			expected.binary_sha256,
			'installed bytes differ',
		);
		const native = members.filter((member) => {
			try {
				if (process.platform === 'linux')
					return (
						fileSystem.realpathSync('/proc/' + member.pid + '/exe') ===
						fileSystem.realpathSync(work.binary)
					);
				return member.command.split(/\s+/).includes(work.binary);
			} catch (error) {
				if (['ENOENT', 'ESRCH'].includes(error.code)) return false;
				throw error;
			}
		});
		assert.ok(native.length > 0, 'admitted request lacks installed native owner');
		for (const member of native) admittedNative.set(member.pid, member.command);
		admitted = { elapsed_ms: elapsed(), body: requestBody, native, work_step: work };
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
	function failObservation(error) {
		const message = safeOutput(error.message, token);
		let processes = null;
		try {
			processes = ownedFacts();
		} catch {
			/* No owned work record is an explicit setup gap. */
		}
		frozen = {
			error: message,
			admitted: admitted || null,
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
		const route = new URL(request.url, 'http://127.0.0.1').pathname;
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
			Object.assign(requests.at(-1), { status: 200, sha256: sha256(bytes), bytes: bytes.length });
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
			const chunks = [];
			request.on('data', (chunk) => {
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
					const body = JSON.parse(Buffer.concat(chunks));
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
	async function finish() {
		assert.ok(frozen, 'cleanup forbidden before frozen observation');
		if (closed) return;
		closed = true;
		const survivors = work ? ownedFacts() : [];
		for (const member of survivors) {
			// Do not kill a reused PID or a command outside the captured work tree.
			if (member.pid === work.pid || admittedNative.get(member.pid) === member.command) {
				try {
					process.kill(member.pid, 'SIGKILL');
				} catch (error) {
					if (error.code !== 'ESRCH') throw error;
				}
			}
		}
		for (const socket of heldSockets) socket.destroy();
		server.closeAllConnections();
		await new Promise((resolve) => server.close(resolve));
		await delay(100);
		save(path.join(root, 'cleanup.json'), {
			survivors_after_cleanup: work ? ownedFacts() : [],
			listening: server.listening,
			observation_unchanged:
				JSON.stringify(frozen) ===
				JSON.stringify(JSON.parse(fileSystem.readFileSync(path.join(root, 'observation.json')))),
		});
	}
	await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
	const ready = {
		pid: process.pid,
		port: server.address().port,
		base: 'http://127.0.0.1:' + server.address().port,
	};
	const configPath = path.join(root, 'fixture/sunstack.lint.json');
	const config = JSON.parse(fileSystem.readFileSync(configPath));
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
		get observation() {
			return frozen;
		},
		get pending() {
			return observationPromise;
		},
		server,
	};
}

function parseOptions(words) {
	const allowed = ['root', 'assets', 'identity', 'repository', 'run', 'head', 'workflow'];
	const options = {};
	for (let index = 0; index < words.length; index += 2) {
		const name = words[index].replace(/^--/, '');
		assert.ok(
			allowed.includes(name) && words[index + 1] && !options[name],
			'invalid controller argument',
		);
		options[name] = words[index + 1];
	}
	for (const name of allowed) assert.ok(options[name], '--' + name + ' required');
	return options;
}

async function main(words) {
	const action = words.shift();
	if (action === 'finish') {
		const root = path.resolve(words[0]);
		const until = Date.now() + 45000;
		while (!fileSystem.existsSync(path.join(root, 'complete.json')) && Date.now() < until)
			await delay(100);
		assert.ok(
			fileSystem.existsSync(path.join(root, 'complete.json')),
			'controller observation missing; retain logs as unknown',
		);
		const ready = JSON.parse(fileSystem.readFileSync(path.join(root, 'ready.json')));
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
	assert.ok(['start', 'serve'].includes(action));
	const options = parseOptions(words);
	if (action === 'start') {
		assert.ok(process.env.GH_TOKEN, 'controller token required');
		fileSystem.mkdirSync(options.root, { recursive: true });
		const output = fileSystem.openSync(path.join(options.root, 'controller.log'), 'wx', 0o600);
		const child = spawn(process.execPath, [import.meta.filename, 'serve', ...words], {
			detached: true,
			stdio: ['ignore', output, output],
			env: { PATH: process.env.PATH, GH_TOKEN: process.env.GH_TOKEN },
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
	const target = Object.fromEntries(
		['repository', 'run', 'head', 'workflow'].map((name) => [name, options[name]]),
	);
	await startController({ ...options, target, token: process.env.GH_TOKEN });
}

if (import.meta.main)
	main(process.argv.slice(2)).catch((error) => {
		process.stderr.write(error.message + '\n');
		process.exitCode = 2;
	});
export { startController, validateTarget, safeOutput };
