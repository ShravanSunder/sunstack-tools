import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import fileSystem from 'node:fs';
import operatingSystem from 'node:os';
import path from 'node:path';
import test from 'node:test';

import { z } from 'zod';

import { captureProcess, groupMembers, trackOwnedTree } from './release-process.ts';
import { sha256, save } from './release-registry.ts';
import { gradeStop } from './release-report-checks.ts';
// Real child, signal, ownership and HTTP controls; synthetic children are not product proof.

void test('real child capture retains streams and exit; timeout records live group before cleanup', async () => {
	const root = fileSystem.mkdtempSync(
		path.join(operatingSystem.tmpdir(), 'sunstack-s4-process-control-'),
	);
	const environment = { PATH: '/usr/bin:/bin', HOME: root };
	const completed = await captureProcess({
		command: [
			process.execPath,
			'-e',
			"console.log('exact stdout');console.error('exact stderr');process.exit(3)",
		],
		cwd: root,
		environment,
		stem: path.join(root, 'completed'),
	});
	assert.equal(completed.stdout.toString(), 'exact stdout\n');
	assert.equal(completed.stderr.toString(), 'exact stderr\n');
	assert.ok(completed.observation.root_close);
	assert.equal(completed.observation.root_close.code, 3);
	assert.equal(completed.observation.ownership.kind, 'established');
	assert.deepEqual(completed.observation.survivors_before_cleanup, []);
	const timedOut = await captureProcess({
		command: [process.execPath, '-e', "console.log('partial sentinel');setInterval(()=>{},1000)"],
		cwd: root,
		environment,
		stem: path.join(root, 'timeout'),
		timeoutMs: 500,
	});
	assert.ok(timedOut.observation.error);
	assert.match(timedOut.observation.error, /within bound/);
	assert.equal(timedOut.stdout.toString(), 'partial sentinel\n');
	assert.ok(timedOut.observation.survivors_before_cleanup);
	assert.ok(timedOut.observation.survivors_before_cleanup.length > 0);
	assert.deepEqual(
		z
			.object({ survivors_after_cleanup: z.array(z.json()) })
			.parse(JSON.parse(fileSystem.readFileSync(path.join(root, 'timeout.cleanup.json'), 'utf8')))
			.survivors_after_cleanup,
		[],
	);
});

void test('a real held request with a question map admits cancellation and retains its exact state', async () => {
	const root = fileSystem.mkdtempSync(
		path.join(operatingSystem.tmpdir(), 'sunstack-s4-held-control-'),
	);
	const configPath = path.join(fileSystem.realpathSync(root), 'sunstack.lint.json');
	save(configPath, { decision_model: { endpoint: 'http://127.0.0.1:1' } });
	const state = { contents: { content1: 'synthetic held state' }, evidence: {}, state_version: 1 };
	const reference = { status: { kind: 'cancelled' }, evidence_seen: { 0: state } };
	const source = `const fileSystem =require('node:fs'),http=require('node:http');
const state=${JSON.stringify(state)},reference=${JSON.stringify(reference)};
const config=JSON.parse(fileSystem.readFileSync(${JSON.stringify(configPath)}));
const cancelled=()=>{process.removeAllListeners('SIGINT');process.stdout.write(JSON.stringify({...reference,version:'0.0.6',config:${JSON.stringify(configPath)}})+'\\n',()=>process.exit(4));};
process.on('SIGINT',cancelled);process.on('SIGTERM',cancelled);
const body=JSON.stringify({state,questions:{q000001:{question:'synthetic'}}});
const request=http.request(config.decision_model.endpoint,{method:'POST',headers:{'content-length':Buffer.byteLength(body)}},()=>{});request.on('error',()=>{});request.end(body);`;
	const binary = path.join(root, 'sunstack');
	fileSystem.writeFileSync(binary, source);
	const result = await captureProcess({
		command: [process.execPath, binary],
		cwd: root,
		environment: { PATH: '/usr/bin:/bin', HOME: root },
		stem: path.join(root, 'cancel'),
		stop: { signal: 'SIGINT', destination: 'pid', expectation: 'cancel4' },
		expectedNativeSha: sha256(Buffer.from(source)),
	});
	assert.equal(
		gradeStop({
			result,
			expected: 'cancel4',
			reference,
			version: '0.0.6',
			configPath,
		}),
		'pass',
	);
	assert.ok(result.observation.admitted_request);
	assert.deepEqual(result.observation.admitted_request.state, state);
});

void test('an owned native in a separate process group is admitted without widening the signal destination', async () => {
	const root = fileSystem.mkdtempSync(
		path.join(operatingSystem.tmpdir(), 'sunstack-s4-detached-control-'),
	);
	const configPath = path.join(fileSystem.realpathSync(root), 'sunstack.lint.json');
	save(configPath, { decision_model: { endpoint: 'http://127.0.0.1:1' } });
	const state = { contents: { content1: 'detached native held state' } };
	const reference = { status: { kind: 'cancelled' }, evidence_seen: { 0: state } };
	const binary = path.join(root, 'sunstack');
	const source = `const fs=require('node:fs'),http=require('node:http');
process.on('SIGINT',()=>process.stdout.write(JSON.stringify({...${JSON.stringify(reference)},version:'0.0.6',config:${JSON.stringify(configPath)}})+'\\n',()=>process.exit(4)));
const endpoint=JSON.parse(fs.readFileSync(${JSON.stringify(configPath)})).decision_model.endpoint;
const request=http.request(endpoint,{method:'POST'});request.on('error',()=>{});request.end(JSON.stringify({state:${JSON.stringify(state)},questions:{question1:{question:'held'}}}));`;
	fileSystem.writeFileSync(binary, source);
	const wrapper = `const {spawn}=require('node:child_process');
const child=spawn(process.execPath,[${JSON.stringify(binary)}],{detached:true,stdio:['ignore',process.stdout,process.stderr]});
process.on('SIGINT',()=>child.kill('SIGINT'));
child.on('close',(code)=>process.exit(code));`;
	const realKill = process.kill.bind(process);
	const signalTargets: { targetPid: number; signal: string | number | undefined }[] = [];
	let result;
	process.kill = (targetPid, signal) => {
		signalTargets.push({ targetPid, signal });
		return realKill(targetPid, signal);
	};
	try {
		result = await captureProcess({
			command: [process.execPath, '-e', wrapper],
			cwd: root,
			environment: { PATH: '/usr/bin:/bin', HOME: root },
			stem: path.join(root, 'capture'),
			stop: { signal: 'SIGINT', destination: 'group', expectation: 'cancel4' },
			expectedNativeSha: sha256(Buffer.from(source)),
		});
	} finally {
		process.kill = realKill;
	}
	assert.equal(
		gradeStop({
			result,
			expected: 'cancel4',
			reference,
			version: '0.0.6',
			configPath,
		}),
		'pass',
	);
	assert.ok(result.observation.pid);
	assert.ok(result.observation.stop);
	assert.deepEqual(signalTargets, [{ targetPid: -result.observation.pid, signal: 'SIGINT' }]);
	assert.equal(result.observation.stop.destination, 'group');
	assert.ok(
		result.observation.owned_tree_at_readiness.some(
			(member) => member.group !== result.observation.pid,
		),
	);
	assert.ok(
		result.observation.native_identity_at_readiness.every(
			(member) => member.pid !== result.observation.pid,
		),
	);
});

void test('an unrelated native with matching bytes cannot authorize a held request from another process', async (testContext) => {
	const root = fileSystem.mkdtempSync(
		path.join(operatingSystem.tmpdir(), 'sunstack-s4-unrelated-control-'),
	);
	const configPath = path.join(fileSystem.realpathSync(root), 'sunstack.lint.json');
	save(configPath, { decision_model: { endpoint: 'http://127.0.0.1:1' } });
	const binary = path.join(root, 'sunstack');
	const source = "process.stdout.write('decoy ready');setInterval(()=>{},1000);";
	fileSystem.writeFileSync(binary, source);
	const decoy = spawn(process.execPath, [binary], {
		detached: true,
		stdio: ['ignore', 'pipe', 'pipe'],
	});
	const decoyPid = decoy.pid;
	assert.ok(decoyPid);
	const closed = new Promise<void>((resolve) => decoy.once('close', () => resolve()));
	testContext.after(async () => {
		try {
			process.kill(-decoyPid, 'SIGKILL');
		} catch (error) {
			if (!(error instanceof Error && 'code' in error && error.code === 'ESRCH')) throw error;
		}
		await closed;
	});
	await new Promise<void>((resolve, reject) => {
		decoy.stdout.once('data', () => resolve());
		decoy.once('error', reject);
	});
	const stranger = `const fs=require('node:fs'),http=require('node:http');
const endpoint=JSON.parse(fs.readFileSync(${JSON.stringify(configPath)})).decision_model.endpoint;
const request=http.request(endpoint,{method:'POST'});request.on('error',()=>{});request.end(JSON.stringify({state:{contents:{}},questions:{question1:{question:'held'}}}));`;
	const result = await captureProcess({
		command: [process.execPath, '-e', stranger],
		cwd: root,
		environment: { PATH: '/usr/bin:/bin', HOME: root },
		stem: path.join(root, 'refused'),
		stop: { signal: 'SIGINT', destination: 'group', expectation: 'cancel4' },
		expectedNativeSha: sha256(Buffer.from(source)),
	});
	assert.ok(result.observation.error);
	assert.match(result.observation.error, /owned by the installed/);
	assert.equal(result.observation.signal_at_ms, null);
	assert.doesNotThrow(
		() => process.kill(decoyPid, 0),
		'unrelated native must not be cleaned by the observer',
	);
});

void test('ownership census after root exit retains its live same-group child and refuses proven absence', async (testContext) => {
	const root = fileSystem.mkdtempSync(
		path.join(operatingSystem.tmpdir(), 'sunstack-s4-missed-root-'),
	);
	const childPidPath = path.join(root, 'child.pid');
	const childSource = `require('node:fs').writeFileSync(${JSON.stringify(childPidPath)},String(process.pid));process.send('ready');setInterval(()=>{},1000);`;
	const wrapperSource = `const child=require('node:child_process').spawn(process.execPath,['-e',${JSON.stringify(childSource)}],{stdio:['ignore','ignore','ignore','ipc']});child.once('message',()=>{console.log('exact completed output');process.exit(0);});`;
	const wrapper = spawn(process.execPath, ['-e', wrapperSource], {
		detached: true,
		stdio: ['ignore', 'pipe', 'pipe'],
	});
	const wrapperPid = wrapper.pid;
	assert.ok(wrapperPid);
	testContext.after(() => {
		try {
			process.kill(-wrapperPid, 'SIGKILL');
		} catch (error) {
			if (!(error instanceof Error && 'code' in error && error.code === 'ESRCH')) throw error;
		}
	});
	let output = '';
	wrapper.stdout.on('data', (chunk) => {
		output += chunk;
	});
	const ending = await new Promise<{ code: number | null; signal: NodeJS.Signals | null }>(
		(resolve, reject) => {
			wrapper.once('close', (code, signal) => resolve({ code, signal }));
			wrapper.once('error', reject);
		},
	);
	assert.deepEqual(ending, { code: 0, signal: null });
	assert.equal(output, 'exact completed output\n');
	const childPid = Number(fileSystem.readFileSync(childPidPath, 'utf8'));
	const actualGroup = groupMembers(wrapperPid);
	assert.ok(
		!actualGroup.some((member) => member.pid === wrapperPid),
		'root has exited and been reaped before the first ownership census',
	);
	assert.ok(
		actualGroup.some((member) => member.pid === childPid),
		'synchronized child remains in caller group',
	);
	const observe = trackOwnedTree(wrapperPid);
	const members = observe();
	assert.ok(
		members.some((member) => member.pid === childPid),
		'missed root must not conceal its surviving caller-group child',
	);
	assert.ok(observe.ownership);
	assert.equal(observe.ownership().kind, 'unestablished');
	assert.equal(observe.ownership().initial_root, null);
});
