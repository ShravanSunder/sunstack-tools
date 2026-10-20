// Controller controls exercise real HTTP/processes and a fake gh, never the real cancel API.
const test = require('node:test');
const assert = require('node:assert/strict');
const fileSystem = require('node:fs');
const operatingSystem = require('node:os');
const path = require('node:path');
const { spawn } = require('node:child_process');
const { startController, validateTarget, safeOutput } = require('./release_actions_control.cjs');
const { sha256, save } = require('./release_registry.cjs');
const { settleGroup } = require('./release_process.cjs');
const target = {
	repository: 'ShravanSunder/sunstack-release-rehearsal',
	run: '123456',
	head: 'a'.repeat(40),
	workflow: 'S4 cancellation observation',
};
const repository = { full_name: target.repository, private: true };
const run = {
	id: 123456,
	repository,
	head_sha: target.head,
	name: target.workflow,
	event: 'workflow_dispatch',
	status: 'in_progress',
};

test('self-cancel admission rejects the wrong repository, run, head, workflow and public state', () => {
	validateTarget(target, repository, run);
	for (const mutation of [
		{ id: 12 },
		{ head_sha: 'b'.repeat(40) },
		{ name: 'Release' },
		{ event: 'push' },
		{ repository: { ...repository, private: false } },
		{ repository: { ...repository, full_name: 'someone/else' } },
	])
		assert.throws(() => validateTarget(target, repository, { ...run, ...mutation }));
	assert.throws(() => validateTarget({ ...target, repository: 'someone/else' }, repository, run));
	assert.throws(() => validateTarget(target, { ...repository, private: false }, run));
	assert.throws(
		() => safeOutput('contains synthetic-controller-token', 'synthetic-controller-token'),
		/withheld/,
	);
});

async function controllerCase(mode, testContext) {
	const root = fileSystem.mkdtempSync(
		path.join(operatingSystem.tmpdir(), 'sunstack-s4-actions-control-'),
	);
	fileSystem.mkdirSync(path.join(root, 'fixture'));
	fileSystem.mkdirSync(path.join(root, 'installed'));
	save(path.join(root, 'fixture/sunstack.lint.json'), {
		backend: { endpoint: 'http://127.0.0.1:1' },
	});
	const binary = path.join(root, 'installed/sunstack');
	fileSystem.symlinkSync(process.execPath, binary);
	const identity = path.join(root, 'identity.json');
	save(identity, {
		tag: 'v0.0.7',
		version: '0.0.7',
		binary_sha256: sha256(fileSystem.readFileSync(binary)),
		asset_sha256: {},
	});
	const fake = path.join(root, 'fake-gh.cjs');
	fileSystem.writeFileSync(
		fake,
		`const fs=require('node:fs');const args=process.argv.slice(2);fs.appendFileSync(${JSON.stringify(path.join(root, 'calls.jsonl'))},JSON.stringify(args)+'\\n');
if(args[0]==='api'){process.stdout.write(JSON.stringify(args[1].includes('/actions/runs/')?${JSON.stringify(run)}:${JSON.stringify(repository)}));}
else {const work=JSON.parse(fs.readFileSync(${JSON.stringify(path.join(root, 'work-step.json'))}));
if(${JSON.stringify(mode)}==='signal')process.kill(work.pid,'SIGINT');
process.stdout.write(${JSON.stringify(mode)}==='leak'?process.env.GH_TOKEN:'cancel accepted');}`,
	);
	const controller = await startController({
		root,
		assets: root,
		identity,
		target,
		token: 'synthetic-controller-token',
		ghCommand: [process.execPath, fake],
		bounds: [0, 100, 500],
	});
	let child;
	testContext.after(async () => {
		if (child) {
			try {
				process.kill(-child.pid, 'SIGKILL');
			} catch (error) {
				if (error.code !== 'ESRCH') throw error;
			}
			assert.deepEqual(await settleGroup(child.pid), []);
		}
		controller.server.closeAllConnections();
		await new Promise((resolve) => controller.server.close(resolve));
	});
	{
		// Health and malformed requests must not invoke cancellation.
		assert.equal((await fetch(controller.ready.base + '/health')).status, 200);
		assert.equal(
			(await fetch(controller.ready.base + '/systemone', { method: 'POST', body: '{}' })).status,
			400,
		);
		assert.equal(
			fileSystem.readFileSync(path.join(root, 'calls.jsonl'), 'utf8').includes('"cancel"'),
			false,
		);
		await assert.rejects(controller.finish(), /before frozen/);
		const source = `const http=require('node:http'),fs=require('node:fs');
if(process.env.GH_TOKEN)throw Error('controller token inherited');
process.on('SIGINT',()=>process.stdout.write('cancelled-control\\n',()=>process.exit(4)));
const endpoint=JSON.parse(fs.readFileSync(${JSON.stringify(path.join(root, 'fixture/sunstack.lint.json'))})).backend.endpoint;
const body=JSON.stringify({state:{contents:{content001:'synthetic actual HTTP state'}},questions:{q1:{question:'held'}}});
setTimeout(()=>{const request=http.request(endpoint,{method:'POST'});request.on('error',()=>{});request.end(body);},50);`;
		child = spawn(binary, ['-e', source], {
			detached: true,
			env: { PATH: '/usr/bin:/bin', HOME: root },
			stdio: ['ignore', 'pipe', 'pipe'],
		});
		save(path.join(root, 'work-step.json'), { pid: child.pid, group: child.pid, binary });
		const stdout = [];
		child.stdout.on('data', (chunk) => stdout.push(chunk));
		const closed = new Promise((resolve) =>
			child.once('close', (code, signal) => resolve({ code, signal })),
		);
		await new Promise((resolve, reject) => {
			const timer = setTimeout(() => {
				clearInterval(poll);
				reject(new Error('controller did not freeze observation'));
			}, 10000);
			const poll = setInterval(() => {
				if (controller.observation) {
					clearTimeout(timer);
					clearInterval(poll);
					resolve();
				}
			}, 10);
		});
		await controller.pending;
		const observation = controller.observation;
		assert.deepEqual(observation.admitted.body.state, {
			contents: { content001: 'synthetic actual HTTP state' },
		});
		const calls = fileSystem
			.readFileSync(path.join(root, 'calls.jsonl'), 'utf8')
			.trim()
			.split('\n')
			.map((line) => JSON.parse(line));
		assert.deepEqual(
			calls.filter((args) => args[0] === 'run'),
			[['run', 'cancel', target.run, '--repo', target.repository]],
		);
		assert.equal(
			JSON.parse(fileSystem.readFileSync(path.join(root, 'cancel-command.json')))
				.after_real_admission,
			true,
		);
		const before = fileSystem.readFileSync(path.join(root, 'observation.json'));
		if (mode === 'signal') {
			assert.deepEqual(await closed, { code: 4, signal: null });
			assert.equal(Buffer.concat(stdout).toString(), 'cancelled-control\n');
			assert.deepEqual(observation.survivors_before_cleanup, []);
			assert.equal(observation.sockets_before_cleanup, 0);
		} else {
			assert.ok(observation.survivors_before_cleanup.some((member) => member.pid === child.pid));
			assert.equal(observation.sockets_before_cleanup, 1);
		}
		if (mode === 'leak') {
			assert.notEqual(observation.cancellation.code, 0);
			for (const name of fileSystem
				.readdirSync(root)
				.filter((fileName) => /\.(json|jsonl|log)$/.test(fileName)))
				assert.equal(
					fileSystem
						.readFileSync(path.join(root, name), 'utf8')
						.includes('synthetic-controller-token'),
					false,
					name,
				);
		}
		await controller.finish();
		assert.deepEqual(
			fileSystem.readFileSync(path.join(root, 'observation.json')),
			before,
			'cleanup rewrote observed state',
		);
		assert.equal(
			JSON.parse(fileSystem.readFileSync(path.join(root, 'cleanup.json'))).listening,
			false,
		);
		await closed;
	}
}

test('actual HTTP admission precedes exact fake self-cancel and the token stays out of the native environment', (testContext) =>
	controllerCase('signal', testContext));
test('a live native and socket remain observed even when owned cleanup ends them later', (testContext) =>
	controllerCase('leave', testContext));
test('a synthetic token in cancellation output is withheld and cannot yield a successful receipt', (testContext) =>
	controllerCase('leak', testContext));
