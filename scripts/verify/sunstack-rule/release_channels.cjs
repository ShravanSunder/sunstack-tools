#!/usr/bin/env node
/* eslint-disable no-await-in-loop -- Each fixture and process must settle before the next owned cell begins. */
// S4: install authenticated private artifacts, then compare real user routes.
const assert = require('node:assert/strict');
const fileSystem = require('node:fs');
const path = require('node:path');
const { sha256, save, startRegistry } = require('./release_registry.cjs');
const { captureProcess } = require('./release_process.cjs');
const { installHomebrew } = require('./release_homebrew.cjs');
const writeLine = (message) => process.stdout.write(message + '\n');
const writeError = (error) => process.stderr.write((error.stack || String(error)) + '\n');

function argumentsFrom(argv) {
	if (argv.includes('--help')) {
		writeLine(
			'S4 private installs: --commit DECLARED_PRODUCT_SHA --assets DIR (older/newer plus s4-artifacts.json) --native-root DIR (preflight older/newer/TARGET/sunstack) --fixtures DIR --evidence NEW_DIR --dependency-tar detect-libc-2.1.2.tgz --channel npm|shell|brew [--pnpm PATH]. npm runs local-bin, exec-script, global, npx and pnpm-dlx. No registry credentials or public publication.',
		);
		return null;
	}
	const allowed = [
		'commit',
		'assets',
		'native-root',
		'fixtures',
		'evidence',
		'dependency-tar',
		'channel',
		'pnpm',
	];
	const options = {};
	for (let index = 0; index < argv.length; index += 2) {
		const key = argv[index].replace(/^--/, '');
		assert.ok(
			allowed.includes(key) && argv[index + 1] && !options[key],
			'unknown/missing/duplicate option',
		);
		options[key] = argv[index + 1];
	}
	for (const key of allowed.filter((optionName) => optionName !== 'pnpm'))
		assert.ok(options[key], '--' + key + ' required');
	assert.ok(['npm', 'shell', 'brew'].includes(options.channel));
	assert.match(options.commit, /^[0-9a-f]{40}$/, 'explicit full declared product commit');
	if (options.channel === 'npm') assert.ok(options.pnpm, '--pnpm required for all npm routes');
	for (const key of allowed.filter((optionName) => !['channel', 'commit'].includes(optionName)))
		if (options[key]) options[key] = path.resolve(options[key]);
	return options;
}

function stopExpectation(route, signal, destination, platform = process.platform) {
	if (destination === 'group')
		return route === 'npx' && platform === 'linux' ? 'cancelled-native/npx-int' : 'cancel4';
	if (
		(route === 'npx' && platform === 'linux') ||
		(signal === 'SIGHUP' && ['exec-script', 'npx'].includes(route))
	)
		return 'no-cancelled-report';
	if (signal === 'SIGHUP')
		return route === 'pnpm-dlx' ? 'native-ended' : 'native-ended/caller-sighup';
	return 'cancel4';
}

function gradeStop(result, expected, reference, version, configPath) {
	const facts = result.observation;
	assert.equal(facts.error, null, facts.error || 'capture failed');
	if (expected === 'no-cancelled-report') {
		assert.doesNotMatch(result.stdout.toString(), /"kind"\s*:\s*"cancelled"/);
		return 'limit';
	}
	assert.deepEqual(facts.survivors_before_cleanup, [], 'survivors before cleanup');
	assert.equal(facts.sockets_before_cleanup, 0, 'held socket before cleanup');
	assert.ok(facts.root_close, 'caller did not close');
	if (expected === 'native-ended/caller-sighup') {
		assert.equal(facts.root_close.signal, 'SIGHUP');
		assert.equal(facts.root_close.code, null);
	} else if (expected !== 'native-ended') {
		if (expected === 'cancelled-native/npx-int')
			assert.ok(facts.root_close.signal === 'SIGINT' || facts.root_close.code === 130);
		else {
			assert.equal(facts.root_close.code, 4);
			assert.equal(facts.root_close.signal, null);
		}
		const actual = JSON.parse(result.stdout);
		assert.deepEqual(
			actual,
			{ ...reference, version, config: configPath },
			'full cancelled report',
		);
		assert.deepEqual(
			Object.values(actual.evidence_seen),
			[facts.admitted_request.state],
			'report state equals the actual sent state',
		);
	}
	return 'pass';
}

function walkFiles(root) {
	const files = [];
	for (const entry of fileSystem.readdirSync(root, { withFileTypes: true })) {
		const full = path.join(root, entry.name);
		if (entry.isDirectory()) files.push(...walkFiles(full));
		else if (entry.isFile()) files.push(full);
	}
	return files;
}

async function observeChannels(options) {
	assert.ok(!fileSystem.existsSync(options.evidence), 'new evidence directory required');
	fileSystem.mkdirSync(options.evidence, { recursive: true });
	const declaration = JSON.parse(
		fileSystem.readFileSync(path.join(options.assets, 's4-artifacts.json')),
	);
	const target =
		(process.arch === 'arm64' ? 'aarch64' : 'x86_64') +
		(process.platform === 'darwin' ? '-apple-darwin' : '-unknown-linux-musl');
	const npm = path.resolve(
		path.dirname(process.execPath),
		'../lib/node_modules/npm/bin/npm-cli.js',
	);
	const npx = path.join(path.dirname(npm), 'npx-cli.js');
	assert.ok(fileSystem.existsSync(npm) && fileSystem.existsSync(npx));
	const rows = [],
		errors = [];
	const registry = await startRegistry({
		assets: options.assets,
		declaration,
		dependencyTar: options['dependency-tar'],
		evidence: path.join(options.evidence, 'registry'),
	});
	const reference = (name) =>
		JSON.parse(fileSystem.readFileSync(path.join(options.fixtures, name + '.expected.json')));
	const recordRows = () =>
		save(path.join(options.evidence, 'receipt.json'), {
			target,
			channel: options.channel,
			rows,
			errors,
			product_source: options.commit,
		});
	async function raw(command, cwd, environment, stem) {
		const result = await captureProcess({ command, cwd, environment, stem });
		assert.equal(result.observation.error, null);
		assert.equal(result.observation.root_close?.code, 0, stem + ': ' + result.stderr);
		assert.deepEqual(result.observation.survivors_before_cleanup, []);
		return result;
	}
	try {
		const toolHome = path.join(options.evidence, 'tools-home');
		fileSystem.mkdirSync(toolHome);
		const toolEnvironment = {
			PATH: path.dirname(process.execPath) + ':/usr/bin:/bin',
			HOME: toolHome,
			npm_config_update_notifier: 'false',
		};
		const npmVersion = await raw(
			[process.execPath, npm, '--version'],
			toolHome,
			toolEnvironment,
			path.join(options.evidence, 'npm-version'),
		);
		let pnpmVersion;
		if (options.pnpm) {
			pnpmVersion = await raw(
				[options.pnpm, '--dir', toolHome, '--version'],
				toolHome,
				toolEnvironment,
				path.join(options.evidence, 'pnpm-version'),
			);
			assert.equal(pnpmVersion.stdout.toString().trim(), '12.14.0');
		}
		save(path.join(options.evidence, 'tools.json'), {
			node: process.version,
			node_path: process.execPath,
			npm: npmVersion.stdout.toString().trim(),
			pnpm: pnpmVersion?.stdout.toString().trim() || null,
			platform: process.platform,
			arch: process.arch,
			shell: fileSystem.realpathSync('/bin/sh'),
			shell_sha256: sha256(fileSystem.readFileSync('/bin/sh')),
		});
		for (const label of ['older', 'newer']) {
			const version = declaration[label].version;
			const expectedSha = declaration[label].binary_sha256[target];
			const baseline = path.join(options['native-root'], label, target, 'sunstack');
			assert.equal(sha256(fileSystem.readFileSync(baseline)), expectedSha);
			const served = await fetch(
				registry.base + '/archives/' + label + '/sunstack-' + target + '.tar.gz',
			);
			assert.equal(served.status, 200);
			assert.equal(
				sha256(Buffer.from(await served.arrayBuffer())),
				declaration[label].archive_sha256[target],
			);
			const routes =
				options.channel === 'npm'
					? ['local-bin', 'exec-script', 'global', 'npx', 'pnpm-dlx']
					: [options.channel];
			for (const route of routes) {
				const cell = path.join(options.evidence, label, route);
				fileSystem.mkdirSync(cell, { recursive: true });
				const caseNames = [
					'install',
					'version',
					'check',
					'clean',
					'finding',
					'invalid',
					'incomplete',
					'group-SIGINT',
					'pid-SIGINT',
					'pid-SIGTERM',
					'pid-SIGHUP',
				];
				const planned = Object.fromEntries(
					caseNames.map((name) => {
						const row = {
							id: label + '.' + route + '.' + name,
							version,
							route,
							case: name,
							status: 'unknown',
							transcript: path.join(cell, name),
						};
						rows.push(row);
						return [name, row];
					}),
				);
				const home = path.join(cell, 'home');
				fileSystem.mkdirSync(home);
				const minimal = { PATH: '/usr/bin:/bin', HOME: home };
				const env = {
					...minimal,
					PATH: path.dirname(process.execPath) + ':/usr/bin:/bin',
					npm_config_registry: registry.base,
					npm_config_cache: path.join(cell, 'npm-cache'),
					npm_config_prefix: path.join(cell, 'global-prefix'),
					npm_config_userconfig: path.join(cell, 'empty.npmrc'),
					npm_config_globalconfig: path.join(cell, 'empty-global.npmrc'),
					npm_config_audit: 'false',
					npm_config_fund: 'false',
					npm_config_update_notifier: 'false',
					XDG_CONFIG_HOME: path.join(cell, 'config'),
					XDG_CACHE_HOME: path.join(cell, 'cache'),
					XDG_DATA_HOME: path.join(cell, 'data'),
					XDG_STATE_HOME: path.join(cell, 'state'),
				};
				fileSystem.writeFileSync(env.npm_config_userconfig, '');
				fileSystem.writeFileSync(env.npm_config_globalconfig, '');
				const consumer = path.join(cell, 'consumer');
				fileSystem.mkdirSync(consumer);
				save(path.join(consumer, 'package.json'), {
					name: 's4-private-consumer',
					private: true,
					version: '1.0.0',
					scripts: { lint: 'exec sunstack lint', version: 'exec sunstack --version' },
				});
				let launch, installedBinary, channelCleanup;
				const firstRequest = registry.requests.length;
				async function one(name, action) {
					try {
						const status = await action();
						planned[name].status = status || 'pass';
					} catch (error) {
						planned[name].status = 'fail';
						planned[name].error = error.stack;
					}
					recordRows();
					return planned[name].status !== 'fail';
				}
				const copyFixture = (name, template) => {
					const destination = path.join(cell, 'fixtures', name);
					fileSystem.mkdirSync(path.dirname(destination), { recursive: true });
					fileSystem.cpSync(path.join(options.fixtures, template), destination, {
						recursive: true,
						filter: (source) => path.basename(source) !== '.git',
					});
					if (route === 'exec-script') {
						fileSystem.symlinkSync(
							path.join(consumer, 'node_modules'),
							path.join(destination, 'node_modules'),
							'dir',
						);
						save(path.join(destination, 'package.json'), {
							private: true,
							scripts: { lint: 'exec sunstack lint' },
						});
					}
					return destination;
				};
				try {
					const installed = await one('install', async () => {
						const spec = '@sunstack/cli@' + version;
						if (route === 'local-bin' || route === 'exec-script') {
							await raw(
								[process.execPath, npm, 'install', '-D', spec],
								consumer,
								env,
								path.join(cell, 'install'),
							);
							launch =
								route === 'local-bin'
									? [process.execPath, path.join(consumer, 'node_modules/.bin/sunstack')]
									: [process.execPath, npm, 'run', '--silent', 'lint', '--'];
						} else if (route === 'global') {
							await raw(
								[process.execPath, npm, 'install', '--global', spec],
								consumer,
								env,
								path.join(cell, 'install'),
							);
							launch = [path.join(env.npm_config_prefix, 'bin/sunstack')];
						} else if (route === 'npx') launch = [process.execPath, npx, '--yes', spec];
						else if (route === 'pnpm-dlx')
							launch = [
								options.pnpm,
								'--registry',
								registry.base,
								'--store-dir',
								path.join(cell, 'store'),
								'--state-dir',
								env.XDG_STATE_HOME,
								'--ignore-workspace',
								'--use-stderr',
								'dlx',
								'--allow-build',
								'@sunstack/cli',
								spec,
							];
						else if (route === 'shell') {
							const installer = path.join(options.assets, label, 'sunstack-installer.sh');
							const installerSource = fileSystem.readFileSync(installer, 'utf8');
							assert.ok(
								installerSource.includes('INSTALLER_DOWNLOAD_URL') &&
									installerSource.includes('SUNSTACK_UNMANAGED_INSTALL'),
							);
							await raw(
								['/bin/sh', installer],
								cell,
								{
									...minimal,
									INSTALLER_DOWNLOAD_URL: registry.base + '/archives/' + label,
									SUNSTACK_UNMANAGED_INSTALL: path.join(cell, 'installed'),
									SUNSTACK_NO_MODIFY_PATH: '1',
								},
								path.join(cell, 'install'),
							);
							assert.deepEqual(walkFiles(home), [], 'shell installer changed HOME/profile');
							installedBinary = path.join(cell, 'installed/sunstack');
							launch = [installedBinary];
						} else {
							const homebrewInstallation = await installHomebrew({
								assets: options.assets,
								label,
								version,
								cell,
								environment: env,
								mirrorBase: registry.base,
								raw,
							});
							installedBinary = homebrewInstallation.binary;
							launch = [installedBinary];
							channelCleanup = homebrewInstallation.cleanup;
						}
						if (['npx', 'pnpm-dlx'].includes(route)) {
							const cold = await raw(
								[...launch, '--version'],
								consumer,
								env,
								path.join(cell, 'install'),
							);
							assert.equal(cold.stdout.toString(), 'sunstack ' + version + '\n');
						}
					});
					if (!installed) continue;
					const executionEnv = ['shell', 'brew'].includes(route) ? minimal : env;
					const args = (words) =>
						route === 'exec-script' && words[0] === 'lint' ? words.slice(1) : words;
					const versionCommand =
						route === 'exec-script'
							? [process.execPath, npm, 'run', '--silent', 'version']
							: [...launch, '--version'];
					const versionOK = await one('version', async () => {
						const result = await raw(
							versionCommand,
							consumer,
							executionEnv,
							path.join(cell, 'version'),
						);
						assert.equal(result.stdout.toString(), 'sunstack ' + version + '\n');
						if (!installedBinary) {
							const nativeFiles = walkFiles(cell).filter(
								(file) =>
									path.basename(file) === 'sunstack' &&
									fileSystem.statSync(file).size > 1024 * 1024,
							);
							installedBinary = nativeFiles.find(
								(file) => sha256(fileSystem.readFileSync(file)) === expectedSha,
							);
							assert.ok(installedBinary, 'actual installed native bytes must match archive');
							const launcherDigest = JSON.parse(
								fileSystem.readFileSync(
									path.join(options.evidence, 'registry', label + '-url-only.json'),
								),
							).before['package/run-sunstack.js'];
							assert.ok(
								walkFiles(cell).some(
									(file) =>
										path.basename(file) === 'run-sunstack.js' &&
										sha256(fileSystem.readFileSync(file)) === launcherDigest,
								),
							);
							assert.ok(
								registry.requests
									.slice(firstRequest)
									.some(
										(request) =>
											request.status === 200 &&
											request.path === '/tarballs/sunstack-cli-' + version + '.tgz',
									),
								'name-based cold tar fetch missing',
							);
						}
						assert.equal(sha256(fileSystem.readFileSync(installedBinary)), expectedSha);
						save(path.join(cell, 'installed-identity.json'), {
							native: installedBinary,
							sha256: expectedSha,
							version,
							registry_requests_since_install: registry.requests.slice(firstRequest),
						});
					});
					if (!versionOK) continue;
					await one('check', async () => {
						const fixture = copyFixture('check', 'clean');
						await raw(
							[...launch, ...args(['lint', '--check'])],
							fixture,
							executionEnv,
							path.join(cell, 'check'),
						);
					});
					for (const [name, template, exit] of [
						['clean', 'clean', 0],
						['finding', 'mechanical-error', 1],
						['invalid', 'clean', 2],
						['incomplete', 'error-plus-incomplete', 3],
					]) {
						await one(name, async () => {
							const fixture = copyFixture(name, template);
							if (name === 'invalid')
								save(path.join(fixture, 'sunstack.lint.json'), { UNKNOWN_S4_FIELD: true });
							const words = ['lint', '--all', '--format', 'json'];
							const native = await captureProcess({
								command: [baseline, ...words],
								cwd: fixture,
								environment: minimal,
								stem: path.join(cell, name + '-native'),
							});
							const result = await captureProcess({
								command: [...launch, ...args(words)],
								cwd: fixture,
								environment: executionEnv,
								stem: path.join(cell, name),
							});
							assert.equal(result.observation.error, null);
							assert.equal(native.observation.error, null);
							assert.equal(native.observation.root_close?.code, exit);
							assert.equal(result.observation.root_close?.code, exit);
							assert.deepEqual(result.stdout, native.stdout, 'stdout bytes differ from native');
							for (const line of native.stderr.toString().split('\n').filter(Boolean))
								assert.ok(result.stderr.toString().includes(line), 'native stderr line missing');
							assert.deepEqual(result.observation.survivors_before_cleanup, []);
							if (name === 'invalid') {
								assert.equal(JSON.parse(native.stdout).status.kind, 'invalid');
								assert.match(native.stdout.toString(), /UNKNOWN_S4_FIELD|unknown field/);
							} else
								assert.deepEqual(JSON.parse(result.stdout), {
									...reference(template),
									config: path.join(fixture, 'sunstack.lint.json'),
									version,
								});
						});
					}
					for (const [signal, destination] of [
						['SIGINT', 'group'],
						['SIGINT', 'pid'],
						['SIGTERM', 'pid'],
						['SIGHUP', 'pid'],
					]) {
						const name = destination + '-' + signal;
						await one(name, async () => {
							const fixture = copyFixture(name, 'cancel');
							const expectation = stopExpectation(route, signal, destination);
							const result = await captureProcess({
								command: [...launch, ...args(['lint', '--all', '--format', 'json'])],
								cwd: fixture,
								environment: executionEnv,
								stem: path.join(cell, name),
								expectedNativeSha: expectedSha,
								stop: { signal, destination, expectation },
							});
							return gradeStop(
								result,
								expectation,
								reference('cancel'),
								version,
								path.join(fixture, 'sunstack.lint.json'),
							);
						});
					}
				} finally {
					if (channelCleanup) {
						try {
							await channelCleanup();
						} catch (error) {
							errors.push({ label, route, cleanup: error.stack });
						}
					}
					recordRows();
				}
			}
		}
	} finally {
		await registry.close();
		recordRows();
	}
	const expectedCells = 2 * (options.channel === 'npm' ? 5 : 1) * 11;
	if (rows.length !== expectedCells)
		errors.push({ matrix: 'incomplete', expected: expectedCells, actual: rows.length });
	recordRows();
	return { rows, errors };
}

if (require.main === module) {
	try {
		const options = argumentsFrom(process.argv.slice(2));
		if (options)
			observeChannels(options)
				.then((result) => {
					const failed = result.rows.filter((row) => !['pass', 'limit'].includes(row.status));
					writeLine(
						JSON.stringify(
							{ rows: result.rows.length, failed: failed.length, errors: result.errors },
							null,
							2,
						),
					);
					process.exitCode = failed.length || result.errors.length ? 2 : 0;
				})
				.catch((error) => {
					writeError(error);
					process.exitCode = 2;
				});
	} catch (error) {
		writeError(error);
		process.exitCode = 2;
	}
}
module.exports = { stopExpectation, gradeStop, argumentsFrom, observeChannels };
