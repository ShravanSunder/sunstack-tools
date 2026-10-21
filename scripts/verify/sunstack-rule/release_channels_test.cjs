/* eslint-disable no-await-in-loop -- Each fixture and process must settle before the next owned cell begins. */
// Permanent apparatus controls; synthetic packages are never product proof.
const test = require('node:test');
const assert = require('node:assert/strict');
const fileSystem = require('node:fs');
const operatingSystem = require('node:os');
const path = require('node:path');
const zlib = require('node:zlib');
const {
	unpackPackage,
	repackPackage,
	startRegistry,
	sha256,
	integrity,
	save,
} = require('./release_registry.cjs');
const { captureProcess, groupMembers, trackOwnedTree } = require('./release_process.cjs');
const {
	assertReportMatches,
	stopExpectation,
	gradeStop,
	argumentsFrom,
	channelCommand,
	npmRoutes,
	expectedCellCount,
} = require('./release_channels.cjs');
const { spawnSync } = require('node:child_process');

test('ordinary stream grading keeps native stderr lines whole and ordered with only manager extras', () => {
	const { compareStreams } = require('./release_streams.cjs');
	const native = {
		stdout: Buffer.from('literal output\n'),
		stderr: Buffer.from('native first\nnative second\n'),
	};
	const channel = (stderr) => ({
		stdout: Buffer.from(native.stdout),
		stderr: Buffer.from(stderr),
	});
	const comparison = compareStreams(
		native,
		channel('npm notice manager message\nnative first\nnative second\n'),
		'npx',
	);
	assert.deepEqual(comparison.native_line_positions, [1, 2]);
	assert.deepEqual(comparison.manager_extra_lines, ['npm notice manager message']);
	assert.doesNotThrow(() =>
		compareStreams(
			native,
			channel('native first\nDone in 0.8s using pnpm v12.14.0\nnative second\n'),
			'pnpm-dlx',
		),
	);
	for (const stderr of [
		'wrapped: native first extra\nnative second\n',
		'native first extra\nnative second\n',
		'native second\n',
		'native second\nnative first\n',
		'native first\nunattributed extra\nnative second\n',
	])
		assert.throws(() => compareStreams(native, channel(stderr), 'npx'));
	assert.throws(() =>
		compareStreams(
			native,
			channel('npm notice manager message\nnative first\nnative second\n'),
			'shell',
		),
	);
	assert.throws(
		() => compareStreams(native, { stdout: Buffer.from('changed'), stderr: native.stderr }, 'npx'),
		/stdout bytes/,
	);
});

test('exec npx preserves literal arguments through the actual inner shell and keeps every invocation', () => {
	const root = fileSystem.mkdtempSync(path.join(operatingSystem.tmpdir(), 'sunstack-s4-exec-npx-'));
	const binary = path.join(root, 'sunstack');
	fileSystem.writeFileSync(
		binary,
		'#!' + process.execPath + '\nprocess.stdout.write(JSON.stringify(process.argv.slice(2)));\n',
		{ mode: 0o755 },
	);
	const words = [
		'lint',
		'--config',
		"space ' quote $HOME $(touch forbidden) `touch forbidden` ; literal",
		'--format',
		'json',
		'',
	];
	const prefix = ['/node', '/npx-cli.js', '--yes', '--package=@sunstack/cli@0.0.6', '-c'];
	const command = channelCommand('exec-npx', prefix, words);
	assert.deepEqual(command.slice(0, -1), prefix);
	assert.match(command.at(-1), /^exec sunstack /);
	const result = spawnSync('/bin/sh', ['-c', command.at(-1)], {
		cwd: root,
		env: { PATH: root + ':/usr/bin:/bin' },
		encoding: 'utf8',
	});
	assert.equal(result.status, 0, result.stderr);
	assert.deepEqual(JSON.parse(result.stdout), words);
	assert.equal(fileSystem.existsSync(path.join(root, 'forbidden')), false);
	assert.deepEqual(
		channelCommand('npx', ['/node', '/npx-cli.js', '--yes', '@sunstack/cli@0.0.6'], words),
		['/node', '/npx-cli.js', '--yes', '@sunstack/cli@0.0.6', ...words],
	);
	assert.deepEqual(npmRoutes, [
		'local-bin',
		'exec-script',
		'global',
		'npx',
		'exec-npx',
		'pnpm-dlx',
	]);
	assert.equal(expectedCellCount('npm'), 132);
	assert.equal(expectedCellCount('shell'), 22);
	for (const platform of ['linux', 'darwin']) {
		for (const [signal, destination] of [
			['SIGINT', 'group'],
			['SIGINT', 'pid'],
			['SIGTERM', 'pid'],
		])
			assert.equal(stopExpectation('exec-npx', signal, destination, platform), 'cancel4');
		assert.equal(stopExpectation('exec-npx', 'SIGHUP', 'pid', platform), 'no-cancelled-report');
	}
});

function archiveFrom(entries) {
	const pieces = [];
	for (const [name, text] of Object.entries(entries)) {
		const body = Buffer.from(text),
			header = Buffer.alloc(512);
		header.write(name);
		header.write('0000755\0', 100);
		header.write(body.length.toString(8).padStart(11, '0') + '\0', 124);
		header[156] = 48;
		header.fill(32, 148, 156);
		header.write(
			header
				.reduce((sum, byte) => sum + byte, 0)
				.toString(8)
				.padStart(6, '0') + '\0 ',
			148,
		);
		pieces.push(header, body, Buffer.alloc((512 - (body.length % 512)) % 512));
	}
	return zlib.gzipSync(Buffer.concat([...pieces, Buffer.alloc(1024)]));
}

test('URL-only repack preserves all other members, including dotfiles, and rejects wrong identity', () => {
	const input = archiveFrom({
		'package/package.json': JSON.stringify({
			name: '@sunstack/cli',
			version: '0.0.6',
			bin: { sunstack: 'run-sunstack.js' },
			artifactDownloadUrls: ['https://example.invalid'],
		}),
		'package/.gitignore': 'unchanged\n',
		'package/run-sunstack.js': 'unchanged launcher\n',
	});
	const output = repackPackage(input, '0.0.6', 'http://127.0.0.1:3/older');
	assert.deepEqual(output.proof.changed, ['package/package.json']);
	assert.equal(output.proof.before['package/.gitignore'], output.proof.after['package/.gitignore']);
	assert.throws(() => repackPackage(input, '0.0.7', 'http://127.0.0.1:3'), /AssertionError/);
	assert.throws(() => unpackPackage(archiveFrom({ 'package/../escape': 'bad' })), /AssertionError/);
	const broken = zlib.gunzipSync(input);
	broken[100] ^= 1;
	assert.throws(() => unpackPackage(zlib.gzipSync(broken)), /checksum/);
});

test('real loopback registry resolves both exact versions and refuses unknown names and writes', async () => {
	const root = fileSystem.mkdtempSync(
		path.join(operatingSystem.tmpdir(), 'sunstack-s4-registry-control-'),
	);
	const dependency = archiveFrom({
		'package/package.json': JSON.stringify({ name: 'detect-libc', version: '2.1.2' }),
	});
	const dependencyFile = path.join(root, 'dependency.tgz');
	fileSystem.writeFileSync(dependencyFile, dependency);
	const declaration = {};
	for (const [label, version] of [
		['older', '0.0.6'],
		['newer', '0.0.7'],
	]) {
		const folder = path.join(root, label);
		fileSystem.mkdirSync(folder);
		const payload = archiveFrom({
			'package/package.json': JSON.stringify({
				name: '@sunstack/cli',
				version,
				bin: { sunstack: 'run-sunstack.js' },
				artifactDownloadUrls: ['https://example.invalid'],
			}),
			'package/npm-shrinkwrap.json': JSON.stringify({
				packages: {
					'node_modules/detect-libc': { version: '2.1.2', integrity: integrity(dependency) },
				},
			}),
			'package/run-sunstack.js': 'unchanged\n',
		});
		fileSystem.writeFileSync(path.join(folder, 'sunstack-npm-package.tar.gz'), payload);
		fileSystem.writeFileSync(
			path.join(folder, 'sha256.sum'),
			sha256(payload) + ' *sunstack-npm-package.tar.gz\n',
		);
		declaration[label] = { version };
	}
	const service = await startRegistry({
		assets: root,
		declaration,
		dependencyTar: dependencyFile,
		evidence: path.join(root, 'proof'),
	});
	try {
		const metadata = await (await fetch(service.base + '/@sunstack%2fcli')).json();
		assert.equal(metadata['dist-tags'].latest, '0.0.7');
		for (const version of ['0.0.6', '0.0.7']) {
			const response = await fetch(metadata.versions[version].dist.tarball);
			const payload = Buffer.from(await response.arrayBuffer());
			assert.equal(integrity(payload), metadata.versions[version].dist.integrity);
			assert.equal(JSON.parse(unpackPackage(payload)[0].data).version, version);
		}
		assert.equal((await fetch(service.base + '/missing')).status, 404);
		assert.equal(
			(await fetch(service.base + '/@sunstack/cli', { method: 'PUT', body: '{}' })).status,
			405,
		);
	} finally {
		await service.close();
	}
	assert.equal(
		JSON.parse(fileSystem.readFileSync(path.join(root, 'proof/cleanup.json'))).listening,
		false,
	);
});

test('oracle distinguishes no-report limits, Linux npx group exit and pnpm HUP', () => {
	assert.equal(stopExpectation('npx', 'SIGINT', 'group', 'linux'), 'cancelled-native/npx-int');
	assert.equal(stopExpectation('npx', 'SIGINT', 'pid', 'linux'), 'no-cancelled-report');
	assert.equal(stopExpectation('npx', 'SIGINT', 'pid', 'darwin'), 'cancel4');
	assert.equal(stopExpectation('pnpm-dlx', 'SIGHUP', 'pid'), 'native-ended');
	assert.equal(stopExpectation('global', 'SIGHUP', 'pid'), 'native-ended/caller-sighup');
	assert.equal(stopExpectation('exec-script', 'SIGHUP', 'pid'), 'no-cancelled-report');
	assert.throws(() => argumentsFrom(['--channel', 'npm']), /required/);
	const commit = 'a'.repeat(40);
	const options = argumentsFrom([
		'--commit',
		commit,
		'--assets',
		'.',
		'--native-root',
		'.',
		'--fixtures',
		'.',
		'--evidence',
		'./new-proof',
		'--dependency-tar',
		'./dependency.tgz',
		'--channel',
		'npm',
		'--pnpm',
		'./pnpm',
	]);
	assert.equal(options.commit, commit);
});

test('a cleanup result cannot hide pre-cleanup survivors or a cancelled report in a limit cell', () => {
	const result = {
		stdout: Buffer.from('{}'),
		observation: {
			error: null,
			survivors_before_cleanup: [{ pid: 12 }],
			sockets_before_cleanup: 0,
			root_close: { code: 4, signal: null },
		},
	};
	assert.throws(() => gradeStop(result, 'cancel4', {}, '0.0.6', '/fixture'), /survivors/);
	result.stdout = Buffer.from('{"status":{"kind":"cancelled"}}');
	assert.throws(() => gradeStop(result, 'no-cancelled-report', {}, '0.0.6', '/fixture'), /match/);
	result.stdout = Buffer.from('');
	assert.equal(gradeStop(result, 'no-cancelled-report', {}, '0.0.6', '/fixture'), 'limit');
	const state = { contents: { content1: 'literal' } };
	const reference = { status: { kind: 'cancelled' }, evidence_seen: { 0: state } };
	const literalConfig = fileSystem.realpathSync(__filename);
	const mismatchedState = {
		stdout: Buffer.from(JSON.stringify({ ...reference, version: '0.0.6', config: literalConfig })),
		observation: {
			error: null,
			survivors_before_cleanup: [],
			sockets_before_cleanup: 0,
			root_close: { code: 4, signal: null },
			admitted_request: { state: { different: true } },
		},
	};
	assert.throws(
		() => gradeStop(mismatchedState, 'cancel4', reference, '0.0.6', literalConfig),
		/actual sent state/,
	);
});

test('real child capture retains streams and exit; timeout records live group before cleanup', async () => {
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
	assert.match(timedOut.observation.error, /within bound/);
	assert.equal(timedOut.stdout.toString(), 'partial sentinel\n');
	assert.ok(timedOut.observation.survivors_before_cleanup.length > 0);
	assert.deepEqual(
		JSON.parse(fileSystem.readFileSync(path.join(root, 'timeout.cleanup.json')))
			.survivors_after_cleanup,
		[],
	);
});

test('a real held request with a question map admits cancellation and retains its exact state', async () => {
	const root = fileSystem.mkdtempSync(
		path.join(operatingSystem.tmpdir(), 'sunstack-s4-held-control-'),
	);
	const configPath = path.join(fileSystem.realpathSync(root), 'sunstack.lint.json');
	save(configPath, { backend: { endpoint: 'http://127.0.0.1:1' } });
	const state = { contents: { content1: 'synthetic held state' }, evidence: {}, state_version: 1 };
	const reference = { status: { kind: 'cancelled' }, evidence_seen: { 0: state } };
	const source = `const fileSystem =require('node:fs'),http=require('node:http');
const state=${JSON.stringify(state)},reference=${JSON.stringify(reference)};
const config=JSON.parse(fileSystem.readFileSync(${JSON.stringify(configPath)}));
const cancelled=()=>{process.removeAllListeners('SIGINT');process.stdout.write(JSON.stringify({...reference,version:'0.0.6',config:${JSON.stringify(configPath)}})+'\\n',()=>process.exit(4));};
process.on('SIGINT',cancelled);process.on('SIGTERM',cancelled);
const body=JSON.stringify({state,questions:{q000001:{question:'synthetic'}}});
const request=http.request(config.backend.endpoint,{method:'POST',headers:{'content-length':Buffer.byteLength(body)}},()=>{});request.on('error',()=>{});request.end(body);`;
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
	assert.equal(gradeStop(result, 'cancel4', reference, '0.0.6', configPath), 'pass');
	assert.deepEqual(result.observation.admitted_request.state, state);
});

test('an owned native in a separate process group is admitted without widening the signal destination', async () => {
	const root = fileSystem.mkdtempSync(
		path.join(operatingSystem.tmpdir(), 'sunstack-s4-detached-control-'),
	);
	const configPath = path.join(fileSystem.realpathSync(root), 'sunstack.lint.json');
	save(configPath, { backend: { endpoint: 'http://127.0.0.1:1' } });
	const state = { contents: { content1: 'detached native held state' } };
	const reference = { status: { kind: 'cancelled' }, evidence_seen: { 0: state } };
	const binary = path.join(root, 'sunstack');
	const source = `const fs=require('node:fs'),http=require('node:http');
process.on('SIGINT',()=>process.stdout.write(JSON.stringify({...${JSON.stringify(reference)},version:'0.0.6',config:${JSON.stringify(configPath)}})+'\\n',()=>process.exit(4)));
const endpoint=JSON.parse(fs.readFileSync(${JSON.stringify(configPath)})).backend.endpoint;
const request=http.request(endpoint,{method:'POST'});request.on('error',()=>{});request.end(JSON.stringify({state:${JSON.stringify(state)},questions:{question1:{question:'held'}}}));`;
	fileSystem.writeFileSync(binary, source);
	const wrapper = `const {spawn}=require('node:child_process');
const child=spawn(process.execPath,[${JSON.stringify(binary)}],{detached:true,stdio:['ignore',process.stdout,process.stderr]});
process.on('SIGINT',()=>child.kill('SIGINT'));
child.on('close',(code)=>process.exit(code));`;
	const realKill = process.kill;
	const signalTargets = [];
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
	assert.equal(gradeStop(result, 'cancel4', reference, '0.0.6', configPath), 'pass');
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

test('an unrelated native with matching bytes cannot authorize a held request from another process', async (testContext) => {
	const { spawn } = require('node:child_process');
	const root = fileSystem.mkdtempSync(
		path.join(operatingSystem.tmpdir(), 'sunstack-s4-unrelated-control-'),
	);
	const configPath = path.join(fileSystem.realpathSync(root), 'sunstack.lint.json');
	save(configPath, { backend: { endpoint: 'http://127.0.0.1:1' } });
	const binary = path.join(root, 'sunstack');
	const source = "process.stdout.write('decoy ready');setInterval(()=>{},1000);";
	fileSystem.writeFileSync(binary, source);
	const decoy = spawn(process.execPath, [binary], {
		detached: true,
		stdio: ['ignore', 'pipe', 'pipe'],
	});
	const closed = new Promise((resolve) => decoy.once('close', resolve));
	testContext.after(async () => {
		try {
			process.kill(-decoy.pid, 'SIGKILL');
		} catch (error) {
			if (error.code !== 'ESRCH') throw error;
		}
		await closed;
	});
	await new Promise((resolve, reject) => {
		decoy.stdout.once('data', resolve);
		decoy.once('error', reject);
	});
	const stranger = `const fs=require('node:fs'),http=require('node:http');
const endpoint=JSON.parse(fs.readFileSync(${JSON.stringify(configPath)})).backend.endpoint;
const request=http.request(endpoint,{method:'POST'});request.on('error',()=>{});request.end(JSON.stringify({state:{contents:{}},questions:{question1:{question:'held'}}}));`;
	const result = await captureProcess({
		command: [process.execPath, '-e', stranger],
		cwd: root,
		environment: { PATH: '/usr/bin:/bin', HOME: root },
		stem: path.join(root, 'refused'),
		stop: { signal: 'SIGINT', destination: 'group', expectation: 'cancel4' },
		expectedNativeSha: sha256(Buffer.from(source)),
	});
	assert.match(result.observation.error, /owned by the installed/);
	assert.equal(result.observation.signal_at_ms, null);
	assert.doesNotThrow(
		() => process.kill(decoy.pid, 0),
		'unrelated native must not be cleaned by the observer',
	);
});

test('ownership census after root exit retains its live same-group child and refuses proven absence', async (testContext) => {
	const { spawn } = require('node:child_process');
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
	testContext.after(() => {
		try {
			process.kill(-wrapper.pid, 'SIGKILL');
		} catch (error) {
			if (error.code !== 'ESRCH') throw error;
		}
	});
	let output = '';
	wrapper.stdout.on('data', (chunk) => {
		output += chunk;
	});
	const ending = await new Promise((resolve, reject) => {
		wrapper.once('close', (code, signal) => resolve({ code, signal }));
		wrapper.once('error', reject);
	});
	assert.deepEqual(ending, { code: 0, signal: null });
	assert.equal(output, 'exact completed output\n');
	const childPid = Number(fileSystem.readFileSync(childPidPath, 'utf8'));
	const actualGroup = groupMembers(wrapper.pid);
	assert.ok(
		!actualGroup.some((member) => member.pid === wrapper.pid),
		'root has exited and been reaped before the first ownership census',
	);
	assert.ok(
		actualGroup.some((member) => member.pid === childPid),
		'synchronized child remains in caller group',
	);
	const observe = trackOwnedTree(wrapper.pid);
	const members = observe();
	assert.ok(
		members.some((member) => member.pid === childPid),
		'missed root must not conceal its surviving caller-group child',
	);
	assert.equal(observe.ownership().kind, 'unestablished');
	assert.equal(observe.ownership().initial_root, null);
});

test('report comparison accepts canonical config paths behind a symlinked fixture directory', () => {
	const root = fileSystem.mkdtempSync(
		path.join(operatingSystem.tmpdir(), 'sunstack-s4-config-alias-'),
	);
	const physical = path.join(fileSystem.realpathSync(root), 'physical');
	fileSystem.mkdirSync(physical);
	const canonicalConfig = path.join(physical, 'sunstack.lint.json');
	save(canonicalConfig, {});
	const alias = path.join(root, 'alias');
	fileSystem.symlinkSync(physical, alias, 'dir');
	const aliasConfig = path.join(alias, 'sunstack.lint.json');
	assert.notEqual(aliasConfig, canonicalConfig);
	for (const kind of ['clean', 'findings', 'incomplete']) {
		const reference = { status: { kind }, findings: [], counts: { error: 0, warn: 0 } };
		const actual = { ...reference, version: '0.0.6', config: canonicalConfig };
		assertReportMatches(actual, reference, '0.0.6', aliasConfig);
		assert.throws(() =>
			assertReportMatches(
				{ ...actual, config: path.join(physical, 'wrong.json') },
				reference,
				'0.0.6',
				aliasConfig,
			),
		);
	}
	const state = { contents: { content001: 'literal held state' } };
	const reference = { status: { kind: 'cancelled' }, evidence_seen: { 0: state } };
	const result = {
		stdout: Buffer.from(
			JSON.stringify({ ...reference, version: '0.0.6', config: canonicalConfig }),
		),
		observation: {
			error: null,
			survivors_before_cleanup: [],
			sockets_before_cleanup: 0,
			root_close: { code: 4, signal: null },
			admitted_request: { state },
		},
	};
	assert.equal(gradeStop(result, 'cancel4', reference, '0.0.6', aliasConfig), 'pass');
});
