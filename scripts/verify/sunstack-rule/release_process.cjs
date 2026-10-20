/* eslint-disable no-await-in-loop -- Each fixture and process must settle before the next owned cell begins. */
// S4 real subprocess capture. Observation is frozen before owned cleanup.
const assert = require('node:assert/strict');
const fileSystem = require('node:fs');
const http = require('node:http');
const path = require('node:path');
const { spawn, spawnSync } = require('node:child_process');
const { sha256, save } = require('./release_registry.cjs');
const delay = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));

function groupMembers(group) {
	if (process.platform === 'darwin') {
		const result = spawnSync('/bin/ps', ['-axo', 'pid=,ppid=,pgid=,stat=,command='], {
			encoding: 'utf8',
		});
		assert.equal(
			result.status,
			0,
			'owned-group inspection unavailable: ' + (result.error?.message || result.stderr),
		);
		return result.stdout.split('\n').flatMap((line) => {
			const fields = line.trim().split(/\s+/);
			return +fields[2] === group
				? [
						{
							pid: +fields[0],
							parent: +fields[1],
							group: +fields[2],
							state: fields[3],
							command: fields.slice(4).join(' '),
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
				return +fields[2] === group
					? [
							{
								pid: +name,
								parent: +fields[1],
								group: +fields[2],
								state: fields[0],
								command: fileSystem
									.readFileSync('/proc/' + name + '/cmdline', 'utf8')
									.replaceAll('\0', ' '),
							},
						]
					: [];
			} catch (error) {
				if (error.code === 'ENOENT' || error.code === 'ESRCH') return [];
				throw error;
			}
		});
}

async function settleGroup(group) {
	let members = groupMembers(group);
	for (let attempt = 0; attempt < 100 && members.length; attempt++) {
		await delay(30);
		members = groupMembers(group);
	}
	return members;
}

function ownedKill(group, signal) {
	try {
		process.kill(-group, signal);
	} catch (error) {
		if (error.code !== 'ESRCH') throw error;
	}
}

async function captureProcess({
	command,
	cwd,
	environment,
	stem,
	stop,
	expectedNativeSha,
	timeoutMs = 120000,
}) {
	// Refuse before spawning a product or held request when the host cannot
	// supply process-group observations. A denied inspection is not no survivors.
	groupMembers(process.pid);
	fileSystem.mkdirSync(path.dirname(stem), { recursive: true });
	const started = performance.now();
	const elapsed = () => Math.round(performance.now() - started);
	let child,
		server,
		admitted,
		signalAt,
		readyGroup,
		nativeIdentity,
		rootExit = null,
		rootClose = null,
		errorText;
	const stdout = [],
		stderr = [],
		sockets = new Set(),
		timeline = [];
	const deadlines = [];
	const deadline = (milliseconds, value) =>
		new Promise((resolve) => {
			deadlines.push(setTimeout(() => resolve(value), milliseconds));
		});
	let readyResolve;
	let socketsClosedResolve;
	const socketsClosed = new Promise((resolve) => {
		socketsClosedResolve = resolve;
	});
	const ready = new Promise((resolve) => {
		readyResolve = resolve;
	});
	let configBefore;
	const configPath = path.join(cwd, 'sunstack.lint.json');
	if (stop) {
		server = http.createServer((request) => {
			const chunks = [];
			let bytes = 0;
			request.on('data', (chunk) => {
				bytes += chunk.length;
				if (bytes > 1024 * 1024) request.destroy();
				else chunks.push(chunk);
			});
			request.on('end', () => {
				try {
					admitted = JSON.parse(Buffer.concat(chunks));
					assert.ok(
						admitted.questions &&
							!Array.isArray(admitted.questions) &&
							typeof admitted.questions === 'object' &&
							Object.keys(admitted.questions).length > 0,
					);
					assert.ok(admitted.state);
					readyGroup = groupMembers(child.pid);
					nativeIdentity = [];
					for (const member of readyGroup) {
						for (const token of member.command.split(/\s+/)) {
							if (!token.startsWith('/') || path.basename(token) !== 'sunstack') continue;
							try {
								const digest = sha256(fileSystem.readFileSync(token));
								if (digest === expectedNativeSha)
									nativeIdentity.push({ pid: member.pid, file: token, sha256: digest });
							} catch (error) {
								if (!['ENOENT', 'EACCES'].includes(error.code)) throw error;
							}
						}
					}
					assert.ok(
						nativeIdentity.length,
						'held request must be owned by the installed release native process',
					);
					readyResolve(true);
				} catch (error) {
					errorText = error.stack;
					readyResolve(false);
				}
			});
		});
		server.on('connection', (socket) => {
			sockets.add(socket);
			socket.on('close', () => {
				sockets.delete(socket);
				if (sockets.size === 0) socketsClosedResolve(true);
			});
		});
		await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
		configBefore = fileSystem.readFileSync(configPath);
		const config = JSON.parse(configBefore);
		config.backend.endpoint = 'http://127.0.0.1:' + server.address().port + '/systemone';
		save(configPath, config);
	}
	let observation, frozenStdout, frozenStderr;
	try {
		const actualEnvironment = stop
			? { ...environment, OPENROUTER_API_KEY: 'SYNTHETIC_RUNNER_CONTROL_NOT_A_SECRET' }
			: environment;
		save(stem + '.command.json', {
			command,
			cwd,
			environment: actualEnvironment,
			stop: stop || null,
			timeout_ms: timeoutMs,
		});
		child = spawn(command[0], command.slice(1), {
			cwd,
			env: actualEnvironment,
			detached: true,
			stdio: ['ignore', 'pipe', 'pipe'],
		});
		child.stdout.on('data', (chunk) => stdout.push(chunk));
		child.stderr.on('data', (chunk) => stderr.push(chunk));
		let closedResolve, failedResolve;
		const closed = new Promise((resolve) => {
			closedResolve = resolve;
		});
		const failed = new Promise((resolve) => {
			failedResolve = resolve;
		});
		child.on('error', (error) => {
			errorText = error.stack;
			failedResolve('spawn-error');
		});
		child.on('exit', (code, signal) => {
			rootExit = { code, signal, elapsed_ms: elapsed() };
		});
		child.on('close', (code, signal) => {
			rootClose = { code, signal, elapsed_ms: elapsed() };
			closedResolve('closed');
		});
		if (stop) {
			const readiness = await Promise.race([
				ready,
				failed,
				closed.then(() => false),
				deadline(30000, false),
			]);
			assert.equal(readiness, true, errorText || 'no admitted request before stop');
			signalAt = elapsed();
			if (stop.destination === 'group') ownedKill(child.pid, stop.signal);
			else process.kill(child.pid, stop.signal);
			if (stop.expectation === 'no-cancelled-report') {
				for (const bound of [100, 500, 1500]) {
					await delay(Math.max(0, signalAt + bound - elapsed()));
					timeline.push({
						after_signal_ms: elapsed() - signalAt,
						exit: rootExit,
						close: rootClose,
						group: groupMembers(child.pid),
						sockets: sockets.size,
					});
				}
			} else {
				const result = await Promise.race([closed, failed, deadline(15000, 'timeout')]);
				if (result !== 'closed') errorText = errorText || 'supported stop did not close within15s';
			}
		} else {
			const result = await Promise.race([closed, failed, deadline(timeoutMs, 'timeout')]);
			if (result !== 'closed')
				errorText = errorText || 'normal invocation did not close within bound';
		}
	} catch (error) {
		errorText = error.stack;
	} finally {
		const before = child?.pid
			? stop?.expectation === 'no-cancelled-report'
				? groupMembers(child.pid)
				: await settleGroup(child.pid)
			: [];
		if (stop && stop.expectation !== 'no-cancelled-report' && sockets.size > 0) {
			await Promise.race([socketsClosed, deadline(3000, false)]);
		}
		frozenStdout = Buffer.concat(stdout);
		frozenStderr = Buffer.concat(stderr);
		observation = {
			command,
			cwd,
			pid: child?.pid || null,
			root_exit: rootExit,
			root_close: rootClose,
			error: errorText || null,
			elapsed_ms: elapsed(),
			admitted_request: admitted || null,
			signal_at_ms: signalAt ?? null,
			stop: stop || null,
			group_at_readiness: readyGroup || [],
			native_identity_at_readiness: nativeIdentity || [],
			timeline,
			survivors_before_cleanup: before,
			sockets_before_cleanup: sockets.size,
		};
		fileSystem.writeFileSync(stem + '.stdout.log', frozenStdout);
		fileSystem.writeFileSync(stem + '.stderr.log', frozenStderr);
		save(stem + '.process.json', observation);
		let after = [];
		if (child?.pid) {
			if (groupMembers(child.pid).length) {
				ownedKill(child.pid, 'SIGINT');
				after = await settleGroup(child.pid);
			}
			if (after.length) {
				ownedKill(child.pid, 'SIGKILL');
				after = await settleGroup(child.pid);
			}
		}
		for (const socket of sockets) socket.destroy();
		if (server) {
			server.closeAllConnections();
			await new Promise((resolve) => server.close(resolve));
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
		for (const timer of deadlines) clearTimeout(timer);
	}
	return { stdout: frozenStdout, stderr: frozenStderr, observation };
}

module.exports = { captureProcess, groupMembers, settleGroup };
