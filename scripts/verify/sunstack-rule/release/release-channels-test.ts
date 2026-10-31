import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fileSystem from 'node:fs';
import operatingSystem from 'node:os';
import path from 'node:path';
import test from 'node:test';
import zlib from 'node:zlib';

import { z } from 'zod';

import { argumentsFrom, channelCommand, npmRoutes, expectedCellCount } from './release-channels.ts';
import {
	unpackPackage,
	repackPackage,
	startRegistry,
	sha256,
	integrity,
	save,
} from './release-registry.ts';
import { assertReportMatches, stopExpectation, gradeStop } from './release-report-checks.ts';
import { compareStreams } from './release-streams.ts';
/* eslint-disable no-await-in-loop -- Each fixture and process must settle before the next owned cell begins. */
// Permanent apparatus controls; synthetic packages are never product proof.

void test('ordinary stream grading keeps native stderr lines whole and ordered with only manager extras', () => {
	const native = {
		stdout: Buffer.from('literal output\n'),
		stderr: Buffer.from('native first\nnative second\n'),
	};
	const channel = (stderr: string): { stdout: Buffer; stderr: Buffer } => ({
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

void test('exec npx preserves literal arguments through the actual inner shell and keeps every invocation', () => {
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
	const innerCommand = command.at(-1);
	assert.ok(innerCommand);
	assert.match(innerCommand, /^exec sunstack /);
	const result = spawnSync('/bin/sh', ['-c', innerCommand], {
		cwd: root,
		env: { PATH: root + ':/usr/bin:/bin' },
		encoding: 'utf8',
	});
	assert.equal(result.status, 0, result.stderr);
	assert.deepEqual(z.array(z.string()).parse(JSON.parse(result.stdout)), words);
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
		for (const { signal, destination } of [
			{ signal: 'SIGINT', destination: 'group' },
			{ signal: 'SIGINT', destination: 'pid' },
			{ signal: 'SIGTERM', destination: 'pid' },
		])
			assert.equal(
				stopExpectation({ route: 'exec-npx', signal, destination, platform }),
				'cancel4',
			);
		assert.equal(
			stopExpectation({ route: 'exec-npx', signal: 'SIGHUP', destination: 'pid', platform }),
			'no-cancelled-report',
		);
	}
});

function archiveFrom(entries: Readonly<Record<string, string>>): Buffer {
	const pieces: Buffer[] = [];
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

void test('URL-only repack preserves all other members, including dotfiles, and rejects wrong identity', () => {
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
	const originalByte = broken[100];
	assert.notEqual(originalByte, undefined);
	broken[100] = (originalByte ?? 0) ^ 1;
	assert.throws(() => unpackPackage(zlib.gzipSync(broken)), /checksum/);
});

void test('real loopback registry resolves both exact versions and refuses unknown names and writes', async () => {
	const root = fileSystem.mkdtempSync(
		path.join(operatingSystem.tmpdir(), 'sunstack-s4-registry-control-'),
	);
	const dependency = archiveFrom({
		'package/package.json': JSON.stringify({ name: 'detect-libc', version: '2.1.2' }),
	});
	const dependencyFile = path.join(root, 'dependency.tgz');
	fileSystem.writeFileSync(dependencyFile, dependency);
	const declaration: Record<string, { version: string }> = {};
	for (const { label, version } of [
		{ label: 'older', version: '0.0.6' },
		{ label: 'newer', version: '0.0.7' },
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
		const metadata = z
			.object({
				'dist-tags': z.object({ latest: z.string() }),
				versions: z.record(
					z.string(),
					z.object({ dist: z.object({ tarball: z.string(), integrity: z.string() }) }),
				),
			})
			.parse(await (await fetch(service.base + '/@sunstack%2fcli')).json());
		assert.equal(metadata['dist-tags'].latest, '0.0.7');
		for (const version of ['0.0.6', '0.0.7']) {
			const entry = metadata.versions[version];
			assert.ok(entry);
			const response = await fetch(entry.dist.tarball);
			const payload = Buffer.from(await response.arrayBuffer());
			assert.equal(integrity(payload), entry.dist.integrity);
			const member = unpackPackage(payload)[0];
			assert.ok(member);
			assert.equal(
				z.object({ version: z.string() }).parse(JSON.parse(member.data.toString('utf8'))).version,
				version,
			);
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
		z
			.object({ listening: z.boolean() })
			.parse(JSON.parse(fileSystem.readFileSync(path.join(root, 'proof/cleanup.json'), 'utf8')))
			.listening,
		false,
	);
});

void test('oracle distinguishes no-report limits, Linux npx group exit and pnpm HUP', () => {
	assert.equal(
		stopExpectation({ route: 'npx', signal: 'SIGINT', destination: 'group', platform: 'linux' }),
		'cancelled-native/npx-int',
	);
	assert.equal(
		stopExpectation({ route: 'npx', signal: 'SIGINT', destination: 'pid', platform: 'linux' }),
		'no-cancelled-report',
	);
	assert.equal(
		stopExpectation({ route: 'npx', signal: 'SIGINT', destination: 'pid', platform: 'darwin' }),
		'cancel4',
	);
	assert.equal(
		stopExpectation({ route: 'pnpm-dlx', signal: 'SIGHUP', destination: 'pid' }),
		'native-ended',
	);
	assert.equal(
		stopExpectation({ route: 'global', signal: 'SIGHUP', destination: 'pid' }),
		'native-ended/caller-sighup',
	);
	assert.equal(
		stopExpectation({ route: 'exec-script', signal: 'SIGHUP', destination: 'pid' }),
		'no-cancelled-report',
	);
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
	assert.ok(options);
	assert.equal(options.commit, commit);
});

void test('a cleanup result cannot hide pre-cleanup survivors or a cancelled report in a limit cell', () => {
	const result = {
		stdout: Buffer.from('{}'),
		observation: {
			error: null,
			survivors_before_cleanup: [{ pid: 12 }],
			sockets_before_cleanup: 0,
			root_close: { code: 4, signal: null },
		},
	};
	assert.throws(
		() =>
			gradeStop({
				result,
				expected: 'cancel4',
				reference: {},
				version: '0.0.6',
				configPath: '/fixture',
			}),
		/survivors/,
	);
	result.stdout = Buffer.from('{"status":{"kind":"cancelled"}}');
	assert.throws(
		() =>
			gradeStop({
				result,
				expected: 'no-cancelled-report',
				reference: {},
				version: '0.0.6',
				configPath: '/fixture',
			}),
		/match/,
	);
	result.stdout = Buffer.from('');
	assert.equal(
		gradeStop({
			result,
			expected: 'no-cancelled-report',
			reference: {},
			version: '0.0.6',
			configPath: '/fixture',
		}),
		'limit',
	);
	const state = { contents: { content1: 'literal' } };
	const reference = { status: { kind: 'cancelled' }, evidence_seen: { 0: state } };
	const literalConfig = fileSystem.realpathSync(import.meta.filename);
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
		() =>
			gradeStop({
				result: mismatchedState,
				expected: 'cancel4',
				reference,
				version: '0.0.6',
				configPath: literalConfig,
			}),
		/actual sent state/,
	);
});

void test('report comparison accepts canonical config paths behind a symlinked fixture directory', () => {
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
		assertReportMatches({
			actual,
			reference,
			version: '0.0.6',
			configPath: aliasConfig,
		});
		assert.throws(() =>
			assertReportMatches({
				actual: { ...actual, config: path.join(physical, 'wrong.json') },
				reference,
				version: '0.0.6',
				configPath: aliasConfig,
			}),
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
	assert.equal(
		gradeStop({
			result,
			expected: 'cancel4',
			reference,
			version: '0.0.6',
			configPath: aliasConfig,
		}),
		'pass',
	);
});
