#!/usr/bin/env node
import assert from 'node:assert/strict';
import fileSystem from 'node:fs';
import path from 'node:path';
import { sha256, save, startRegistry } from './release-registry.ts';
import { captureProcess } from './release-process.ts';
import { installHomebrew } from './release-homebrew.ts';
import { compareStreams } from './release-streams.ts';
import { assertReportMatches, stopExpectation, gradeStop } from './release-report-checks.ts';
import { channelOptionsSchema, artifactDeclarationSchema, lintReportSchema, urlRepackProofSchema } from './release-channel-contracts.ts';
import type { ChannelOptions, ChannelRow, ChannelError, ChannelObservation } from './release-channel-contracts.ts';
import type { ProcessCapture } from './release-process.ts';
/* eslint-disable no-await-in-loop -- Each fixture and process must settle before the next owned cell begins. */
// S4: install authenticated private artifacts, then compare real user routes.







const writeLine = (message: string): void => { process.stdout.write(message + '\n'); };
const writeError = (error: unknown): void => { process.stderr.write((error instanceof Error ? error.stack || String(error) : String(error)) + '\n'); };
const npmRoutes = ['local-bin', 'exec-script', 'global', 'npx', 'exec-npx', 'pnpm-dlx'];
const expectedCellCount = (channel: string): number => 2 * (channel === 'npm' ? npmRoutes.length : 1) * 11;

function channelCommand(route: string, launch: readonly string[], words: readonly string[]): string[] {
	if (route === 'exec-npx') {
		const quoted = words.map((word) => "'" + word.replaceAll("'", "'\\''") + "'").join(' ');
		return [...launch, 'exec sunstack ' + quoted];
	}
	return [...launch, ...(route === 'exec-script' && words[0] === 'lint' ? words.slice(1) : words)];
}

function argumentsFrom(argv: readonly string[]): ChannelOptions | null {
	if (argv.includes('--help')) {
		writeLine(
			'S4 private installs: --commit DECLARED_PRODUCT_SHA --assets DIR (older/newer plus s4-artifacts.json) --native-root DIR (preflight older/newer/TARGET/sunstack) --fixtures DIR --evidence NEW_DIR --dependency-tar detect-libc-2.1.2.tgz --channel npm|shell|brew [--pnpm PATH]. npm runs local-bin, exec-script, global, npx, exec-npx and pnpm-dlx. No registry credentials or public publication.',
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
	const options: Record<string, string> = {};
	for (let index = 0; index < argv.length; index += 2) {
		const argument = argv[index];
		assert.ok(argument);
		const key = argument.replace(/^--/, '');
		const value = argv[index + 1];
		assert.ok(
			allowed.includes(key) && value && !options[key],
			'unknown/missing/duplicate option',
		);
		options[key] = value;
	}
	for (const key of allowed.filter((optionName) => optionName !== 'pnpm'))
		assert.ok(options[key], '--' + key + ' required');
	const channel = options['channel'];
	const commit = options['commit'];
	assert.ok(channel && commit);
	assert.ok(['npm', 'shell', 'brew'].includes(channel));
	assert.match(commit, /^[0-9a-f]{40}$/, 'explicit full declared product commit');
	if (channel === 'npm') assert.ok(options['pnpm'], '--pnpm required for all npm routes');
	for (const key of allowed.filter((optionName) => !['channel', 'commit'].includes(optionName)))
		if (options[key]) options[key] = path.resolve(options[key]);
	return channelOptionsSchema.parse(options);
}

function walkFiles(root: string): string[] {
	const files: string[] = [];
	for (const entry of fileSystem.readdirSync(root, { withFileTypes: true })) {
		const full = path.join(root, entry.name);
		if (entry.isDirectory()) files.push(...walkFiles(full));
		else if (entry.isFile()) files.push(full);
	}
	return files;
}

async function observeChannels(options: ChannelOptions): Promise<ChannelObservation> {
	assert.ok(!fileSystem.existsSync(options.evidence), 'new evidence directory required');
	fileSystem.mkdirSync(options.evidence, { recursive: true });
	const declaration = artifactDeclarationSchema.parse(JSON.parse(
		fileSystem.readFileSync(path.join(options.assets, 's4-artifacts.json'), 'utf8'),
	));
	const target =
		(process.arch === 'arm64' ? 'aarch64' : 'x86_64') +
		(process.platform === 'darwin' ? '-apple-darwin' : '-unknown-linux-musl');
	const npm = path.resolve(
		path.dirname(process.execPath),
		'../lib/node_modules/npm/bin/npm-cli.js',
	);
	const npx = path.join(path.dirname(npm), 'npx-cli.js');
	assert.ok(fileSystem.existsSync(npm) && fileSystem.existsSync(npx));
	const rows: ChannelRow[] = [];
	const errors: ChannelError[] = [];
	const registry = await startRegistry({
		assets: options.assets,
		declaration,
		dependencyTar: options['dependency-tar'],
		evidence: path.join(options.evidence, 'registry'),
	});
	const reference = (name: string): Record<string, unknown> =>
		lintReportSchema.parse(JSON.parse(fileSystem.readFileSync(path.join(options.fixtures, name + '.expected.json'), 'utf8')));
	const recordRows = (): void =>
		save(path.join(options.evidence, 'receipt.json'), {
			target,
			channel: options.channel,
			rows,
			errors,
			product_source: options.commit,
		});
	async function raw(command: string[], cwd: string, environment: NodeJS.ProcessEnv, stem: string): Promise<ProcessCapture> {
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
		let pnpmVersion: ProcessCapture | undefined;
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
		const labels: readonly ('older' | 'newer')[] = ['older', 'newer'];
		for (const label of labels) {
			const version = declaration[label].version;
			const expectedSha = declaration[label].binary_sha256[target];
			assert.ok(expectedSha);
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
			const routes = options.channel === 'npm' ? npmRoutes : [options.channel];
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
						const row: ChannelRow = {
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
				let launch: string[] | undefined;
				let installedBinary: string | undefined;
				let channelCleanup: (() => Promise<void>) | undefined;
				const firstRequest = registry.requests.length;
				async function one(name: string, action: () => Promise<string | void>): Promise<boolean> {
					const row = planned[name];
					assert.ok(row);
					try {
						const status = await action();
						row.status = status || 'pass';
					} catch (error) {
						row.status = 'fail';
						row.error = error instanceof Error ? error.stack : undefined;
					}
					recordRows();
					return row.status !== 'fail';
				}
				const copyFixture = (name: string, template: string): string => {
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
						else if (route === 'exec-npx')
							launch = [process.execPath, npx, '--yes', '--package=' + spec, '-c'];
						else if (route === 'pnpm-dlx') {
							assert.ok(options.pnpm);
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
						}
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
						if (['npx', 'exec-npx', 'pnpm-dlx'].includes(route)) {
							assert.ok(launch);
							const cold = await raw(
								channelCommand(route, launch, ['--version']),
								consumer,
								env,
								path.join(cell, 'install'),
							);
							assert.equal(cold.stdout.toString(), 'sunstack ' + version + '\n');
						}
					});
					if (!installed) continue;
					assert.ok(launch);
					const installedLaunch = launch;
					const executionEnv = ['shell', 'brew'].includes(route) ? minimal : env;
					const commandFor = (words: readonly string[]): string[] => channelCommand(route, installedLaunch, words);
					const versionCommand =
						route === 'exec-script'
							? [process.execPath, npm, 'run', '--silent', 'version']
							: commandFor(['--version']);
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
							const launcherDigest = urlRepackProofSchema.parse(JSON.parse(
								fileSystem.readFileSync(
									path.join(options.evidence, 'registry', label + '-url-only.json'),
									'utf8',
								),
							)).before['package/run-sunstack.js'];
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
							commandFor(['lint', '--check']),
							fixture,
							executionEnv,
							path.join(cell, 'check'),
						);
					});
					const reportCases: readonly [string, string, number][] = [
						['clean', 'clean', 0],
						['finding', 'mechanical-error', 1],
						['invalid', 'clean', 2],
						['incomplete', 'error-plus-incomplete', 3],
					];
					for (const [name, template, exit] of reportCases) {
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
								command: commandFor(words),
								cwd: fixture,
								environment: executionEnv,
								stem: path.join(cell, name),
							});
							assert.equal(result.observation.error, null);
							assert.equal(native.observation.error, null);
							assert.equal(native.observation.root_close?.code, exit);
							assert.equal(result.observation.root_close?.code, exit);
							save(
								path.join(cell, name + '-stream-comparison.json'),
								compareStreams(native, result, route),
							);
							assert.deepEqual(result.observation.survivors_before_cleanup, []);
							if (name === 'invalid') {
								assert.equal(lintReportSchema.parse(JSON.parse(native.stdout.toString('utf8'))).status.kind, 'invalid');
								assert.match(native.stdout.toString(), /UNKNOWN_S4_FIELD|unknown field/);
							} else
								assertReportMatches(
									lintReportSchema.parse(JSON.parse(result.stdout.toString('utf8'))),
									reference(template),
									version,
									path.join(fixture, 'sunstack.lint.json'),
								);
						});
					}
					const stopCases: readonly [NodeJS.Signals, 'group' | 'pid'][] = [
						['SIGINT', 'group'],
						['SIGINT', 'pid'],
						['SIGTERM', 'pid'],
						['SIGHUP', 'pid'],
					];
					for (const [signal, destination] of stopCases) {
						const name = destination + '-' + signal;
						await one(name, async () => {
							const fixture = copyFixture(name, 'cancel');
							const expectation = stopExpectation(route, signal, destination);
							const result = await captureProcess({
								command: commandFor(['lint', '--all', '--format', 'json']),
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
							errors.push({ label, route, cleanup: error instanceof Error ? error.stack : undefined });
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
	const expectedCells = expectedCellCount(options.channel);
	if (rows.length !== expectedCells)
		errors.push({ matrix: 'incomplete', expected: expectedCells, actual: rows.length });
	recordRows();
	return { rows, errors };
}

if (import.meta.main) {
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
export {
	argumentsFrom,
	observeChannels,
	channelCommand,
	npmRoutes,
	expectedCellCount,
};
