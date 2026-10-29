import { execFile } from 'node:child_process';
/** Install actual release bytes from a local mirror and run with an empty environment. */
import { createHash } from 'node:crypto';
import {
	chmodSync,
	copyFileSync,
	mkdirSync,
	mkdtempSync,
	readFileSync,
	readdirSync,
	rmSync,
	writeFileSync,
} from 'node:fs';
import http from 'node:http';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { readReleaseArchive } from './release-archive.ts';
import { parseReleaseArguments, printReleaseError } from './release-arguments.ts';

const execute = promisify(execFile);

async function runBinary(
	command: string[],
	fixture: string,
): Promise<{ code: number; stdout: string; stderr: string }> {
	try {
		const output = await execute(command[0] ?? '', command.slice(1), {
			cwd: fixture,
			timeout: 30000,
		});
		return { code: 0, stdout: output.stdout, stderr: output.stderr };
	} catch (error) {
		if (
			error !== null &&
			typeof error === 'object' &&
			'code' in error &&
			typeof error.code === 'number' &&
			'stdout' in error &&
			typeof error.stdout === 'string' &&
			'stderr' in error &&
			typeof error.stderr === 'string'
		)
			return { code: error.code, stdout: error.stdout, stderr: error.stderr };
		throw error;
	}
}

export async function verifyInstall(assets: string, tag: string, scratch: string): Promise<void> {
	const names = readdirSync(assets);
	const archives = [
		...names.filter((name) => /^sunstack-.*-apple-darwin\.tar\.gz$/.test(name)),
		...names.filter((name) => /^sunstack-.*-unknown-linux-musl\.tar\.gz$/.test(name)),
	];
	if (archives.length !== 4)
		throw new Error('expected all four downloaded binary release archives');
	for (const archive of archives) {
		const expected =
			readFileSync(path.join(assets, `${archive}.sha256`), 'utf8')
				.trim()
				.split(/\s+/)[0] ?? '';
		const actual = createHash('sha256')
			.update(readFileSync(path.join(assets, archive)))
			.digest('hex');
		if (!/^[0-9a-f]{64}$/.test(expected) || actual !== expected)
			throw new Error(`release checksum mismatch: ${archive}`);
	}
	process.stdout.write('All four downloaded binary archive checksums match.\n');
	const workspace = fileURLToPath(new URL('../../', import.meta.url));
	const examples = path.join(workspace, 'docs/specs/2026-10-12-rust-rule-engine/examples');
	const fixture = path.join(scratch, 'fixture');
	mkdirSync(fixture);
	for (const name of [
		'mechanical.ruleset.json',
		'mechanical.rules.json',
		'mechanical-standards.md',
		'mechanical-skill.md',
	])
		copyFileSync(path.join(examples, name), path.join(fixture, name));
	copyFileSync(
		path.join(workspace, 'scripts/release/smoke-config.json'),
		path.join(fixture, 'sunstack.lint.json'),
	);
	const installHome = path.join(scratch, 'install-home');
	const runtimeHome = path.join(scratch, 'empty-home');
	const installed = path.join(scratch, 'installed');
	mkdirSync(installHome);
	mkdirSync(runtimeHome);
	const mirror = http.createServer((request, response) => {
		const pathname = decodeURIComponent(new URL(request.url ?? '', 'http://127.0.0.1').pathname);
		const name = path.basename(pathname);
		try {
			const bytes = readFileSync(path.join(assets, name));
			response.setHeader('Content-Length', bytes.length);
			response.end(request.method === 'HEAD' ? undefined : bytes);
		} catch {
			response.writeHead(404).end();
		}
	});
	await new Promise<void>((resolve) => mirror.listen(0, '127.0.0.1', resolve));
	const address = mirror.address();
	if (!address || typeof address === 'string') throw new Error('mirror did not bind');
	try {
		const installer = await execute('/bin/sh', [path.join(assets, 'sunstack-installer.sh')], {
			env: {
				PATH: '/usr/bin:/bin',
				HOME: installHome,
				SUNSTACK_DOWNLOAD_URL: `http://127.0.0.1:${address.port}`,
				SUNSTACK_UNMANAGED_INSTALL: installed,
			},
			timeout: 120000,
		});
		process.stdout.write(installer.stdout);
	} finally {
		mirror.closeAllConnections();
		await new Promise<void>((resolve, reject) =>
			mirror.close((error) => (error ? reject(error) : resolve())),
		);
	}
	const launches: [string, string[]][] = [['installed', [path.join(installed, 'sunstack')]]];
	if (process.platform === 'darwin') {
		const members = readReleaseArchive(
			path.join(assets, 'sunstack-x86_64-apple-darwin.tar.gz'),
		).filter((member) => member.regular && path.posix.basename(member.name) === 'sunstack');
		if (members.length !== 1 || !members[0])
			throw new Error('expected exactly one macOS x64 executable');
		const rosetta = path.join(scratch, 'sunstack-x64');
		writeFileSync(rosetta, members[0].data);
		chmodSync(rosetta, 0o755);
		launches.push(['rosetta-x64', ['/usr/bin/arch', '-x86_64', rosetta]]);
	}
	const cases: [string[], number, string][] = [
		[['--version'], 0, `sunstack ${tag.replace(/^v/, '')}\n`],
		[['lint', '--check'], 0, 'Config and rule sets are valid.'],
		[['lint', '--all'], 0, 'Clean: 0 error(s), 0 warning(s)'],
	];
	const scenarios = launches.flatMap(([label, launch]) =>
		cases.map(([arguments_, expectedExit, marker]) => ({
			label,
			launch,
			arguments_,
			expectedExit,
			marker,
		})),
	);
	await scenarios.reduce(
		async (previous, { label, launch, arguments_, expectedExit, marker }): Promise<void> => {
			await previous;
			const result = await runBinary(
				[
					'/usr/bin/env',
					'-i',
					'PATH=/usr/bin:/bin',
					`HOME=${runtimeHome}`,
					...launch,
					...arguments_,
				],
				fixture,
			);
			if (result.code !== expectedExit || !result.stdout.includes(marker))
				throw new Error(
					`${label} ${JSON.stringify(arguments_)}: exit ${result.code}; ${result.stdout}; ${result.stderr}`,
				);
			process.stdout.write(`${label} env -i ${arguments_.join(' ')}: exit ${result.code}\n`);
		},
		Promise.resolve(),
	);
	const target = path.join(fixture, 'mechanical-skill.md');
	const original = readFileSync(target, 'utf8');
	const expected = 'description: Use when demonstrating local skill checks.';
	if (!original.includes(expected)) throw new Error('the positive-control fixture changed');
	writeFileSync(target, original.replaceAll(expected, 'description: Wrong opening.'));
	await launches.reduce(async (previous, [label, launch]): Promise<void> => {
		await previous;
		const failed = await runBinary(
			[
				'/usr/bin/env',
				'-i',
				'PATH=/usr/bin:/bin',
				`HOME=${runtimeHome}`,
				...launch,
				'lint',
				'--all',
			],
			fixture,
		);
		if (failed.code !== 1 || !failed.stdout.includes('mechanical/skill.description'))
			throw new Error(
				`${label} negative lint control: exit ${failed.code}; ${failed.stdout}; ${failed.stderr}`,
			);
		process.stdout.write(`${label} env -i negative lint control: exit 1\n`);
	}, Promise.resolve());
	if (readdirSync(runtimeHome).length) throw new Error('runtime wrote into the empty HOME');
}

export async function main(words: string[]): Promise<void> {
	const parsed = parseReleaseArguments({
		words,
		flags: ['tag', 'repository', 'assets'],
		required: ['tag'],
	});
	if (parsed.help) {
		process.stdout.write(helpText);
		return;
	}
	const tag = parsed.values['tag']?.at(-1) ?? '';
	const assetsOption = parsed.values['assets']?.at(-1);
	const repositoryOption = parsed.values['repository']?.at(-1);
	if (!/^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(tag)) throw new Error('expected a version tag');
	const scratch = mkdtempSync(path.join(tmpdir(), 'sunstack-fresh-install-'));
	try {
		let assets: string;
		if (assetsOption === undefined) {
			const repository = repositoryOption ?? process.env['GITHUB_REPOSITORY'];
			if (repository === undefined)
				throw new Error('--repository is required when downloading artifacts');
			assets = path.join(scratch, 'assets');
			mkdirSync(assets);
			const downloaded = await execute('gh', [
				'release',
				'download',
				tag,
				'--repo',
				repository,
				'--dir',
				assets,
			]);
			process.stdout.write(downloaded.stdout);
			process.stderr.write(downloaded.stderr);
		} else assets = path.resolve(assetsOption);
		await verifyInstall(assets, tag, scratch);
	} finally {
		rmSync(scratch, { recursive: true, force: true });
	}
}

const helpText =
	'usage: fresh-install-smoke.ts [-h] --tag TAG [--repository REPOSITORY]\n                              [--assets ASSETS]\n\nInstall actual release bytes from a local mirror and run with an empty\nenvironment.\n\noptions:\n  -h, --help            show this help message and exit\n  --tag TAG\n  --repository REPOSITORY\n  --assets ASSETS       use previously downloaded real release artifacts\n';

if (import.meta.main) {
	try {
		await main(process.argv.slice(2));
	} catch (error) {
		printReleaseError(error, helpText, 'fresh-install-smoke.ts');
	}
}
