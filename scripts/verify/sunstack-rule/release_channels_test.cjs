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
const { captureProcess } = require('./release_process.cjs');
const {
	stopExpectation,
	gradeStop,
	argumentsFrom,
	channelCommand,
	npmRoutes,
	expectedCellCount,
} = require('./release_channels.cjs');
const { spawnSync } = require('node:child_process');

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
	const mismatchedState = {
		stdout: Buffer.from(JSON.stringify({ ...reference, version: '0.0.6', config: '/fixture' })),
		observation: {
			error: null,
			survivors_before_cleanup: [],
			sockets_before_cleanup: 0,
			root_close: { code: 4, signal: null },
			admitted_request: { state: { different: true } },
		},
	};
	assert.throws(
		() => gradeStop(mismatchedState, 'cancel4', reference, '0.0.6', '/fixture'),
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
	const configPath = path.join(root, 'sunstack.lint.json');
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
