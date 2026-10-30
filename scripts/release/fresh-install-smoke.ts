/** Install published release bytes through a local mirror, then run with no inherited environment. */
import { execFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { once } from 'node:events';
import filesystem from 'node:fs';
import { readFile } from 'node:fs/promises';
import http from 'node:http';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs, promisify } from 'node:util';

import { z } from 'zod';

const execute = promisify(execFile);
const repositorySchema = z.string().regex(/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/);
const argumentsSchema = z
	.object({
		tag: z.string().regex(/^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/),
		repository: repositorySchema.optional(),
		assets: z.string().min(1).optional(),
	})
	.refine((value): boolean => value.assets !== undefined || value.repository !== undefined, {
		message: '--repository is required when downloading artifacts',
	});
type SmokeArguments = z.infer<typeof argumentsSchema>;
const commandResultSchema = z.object({ code: z.number(), stdout: z.string(), stderr: z.string() });
type CommandResult = z.infer<typeof commandResultSchema>;
const checkSchema = z
	.array(z.object({ rule: z.literal('smoke_requirement'), marker: z.literal('K001') }))
	.length(1);
const reportSchema = z.object({
	status: z.object({ kind: z.string() }),
	findings: z.array(z.object({ marker: z.literal('K001') })),
	results: z.array(z.unknown()).length(1),
	usage: z.object({ model_requests: z.literal(0), checked_files_acquired: z.literal(1) }),
});
const archiveNames = [
	'sunstack-aarch64-apple-darwin.tar.gz',
	'sunstack-x86_64-apple-darwin.tar.gz',
	'sunstack-aarch64-unknown-linux-musl.tar.gz',
	'sunstack-x86_64-unknown-linux-musl.tar.gz',
];
const assetNames = [
	...archiveNames,
	...archiveNames.map((name): string => `${name}.sha256`),
	'sunstack-installer.sh',
];

interface Launch {
	readonly label: string;
	readonly command: readonly string[];
}
interface RuntimeCommand {
	readonly launch: Launch;
	readonly arguments: readonly string[];
	readonly fixture: string;
	readonly home: string;
}

async function runInOrder<TItem>(
	items: readonly TItem[],
	operation: (item: TItem) => Promise<void>,
): Promise<void> {
	await items.reduce(async (previous, item): Promise<void> => {
		await previous;
		await operation(item);
	}, Promise.resolve());
}

async function runBinary(options: RuntimeCommand): Promise<CommandResult> {
	try {
		const result = await execute(
			'/usr/bin/env',
			[
				'-i',
				'PATH=/usr/bin:/bin',
				`HOME=${options.home}`,
				'SUNSTACK_CREDENTIAL_STORE=off',
				...options.launch.command,
				...options.arguments,
			],
			{ cwd: options.fixture, timeout: 30000, maxBuffer: 16 * 1024 * 1024 },
		);
		return { code: 0, stdout: result.stdout, stderr: result.stderr };
	} catch (error) {
		const result = commandResultSchema.safeParse(error);
		if (!result.success) throw error;
		return result.data;
	}
}
function requireResult(result: CommandResult, expected: number, label: string): void {
	if (result.code !== expected)
		throw new Error(`${label}: exit ${result.code}; ${result.stdout}; ${result.stderr}`);
	process.stdout.write(`${label}: exit ${result.code}\n`);
}
async function downloadAssets(options: SmokeArguments, destination: string): Promise<void> {
	if (options.repository === undefined)
		throw new Error('--repository is required when downloading artifacts');
	await runInOrder(assetNames, async (name): Promise<void> => {
		const url = `https://github.com/${options.repository}/releases/download/${options.tag}/${name}`;
		const response = await fetch(url, { signal: AbortSignal.timeout(120000) });
		if (!response.ok) throw new Error(`Release asset ${name}: HTTP ${response.status}`);
		filesystem.writeFileSync(
			path.join(destination, name),
			Buffer.from(await response.arrayBuffer()),
		);
	});
}
function verifyChecksums(assets: string): void {
	for (const archive of archiveNames) {
		const expected = z
			.string()
			.regex(/^[0-9a-f]{64}$/)
			.parse(
				filesystem
					.readFileSync(path.join(assets, `${archive}.sha256`), 'utf8')
					.trim()
					.split(/\s+/)[0],
			);
		const actual = createHash('sha256')
			.update(filesystem.readFileSync(path.join(assets, archive)))
			.digest('hex');
		if (actual !== expected) throw new Error(`release checksum mismatch: ${archive}`);
	}
	process.stdout.write('All four downloaded binary archive checksums match.\n');
}
async function serveAsset(options: {
	request: http.IncomingMessage;
	response: http.ServerResponse;
	assets: string;
}): Promise<void> {
	const name = path.posix.basename(
		new URL(options.request.url ?? '/', 'http://127.0.0.1').pathname,
	);
	if (!assetNames.includes(name)) {
		options.response.writeHead(404).end();
		return;
	}
	try {
		const bytes = await readFile(path.join(options.assets, name));
		options.response.setHeader('Content-Length', bytes.length);
		options.response.end(options.request.method === 'HEAD' ? undefined : bytes);
	} catch {
		options.response.writeHead(404).end();
	}
}
async function installFromMirror(options: { assets: string; scratch: string }): Promise<string> {
	const home = path.join(options.scratch, 'install-home');
	const installed = path.join(options.scratch, 'installed');
	filesystem.mkdirSync(home);
	const mirror = http.createServer((request, response): void => {
		void serveAsset({ request, response, assets: options.assets }).catch((error: unknown): void => {
			response.destroy(error instanceof Error ? error : undefined);
		});
	});
	const listening = once(mirror, 'listening');
	mirror.listen(0, '127.0.0.1');
	await listening;
	try {
		const address = mirror.address();
		if (address === null || typeof address === 'string') throw new Error('mirror did not bind');
		const installer = await execute(
			'/bin/sh',
			[path.join(options.assets, 'sunstack-installer.sh')],
			{
				env: {
					PATH: '/usr/bin:/bin',
					HOME: home,
					SUNSTACK_DOWNLOAD_URL: `http://127.0.0.1:${address.port}`,
					SUNSTACK_UNMANAGED_INSTALL: installed,
				},
				timeout: 120000,
			},
		);
		process.stdout.write(installer.stdout);
	} finally {
		mirror.closeAllConnections();
		await new Promise<void>((resolve, reject): void => {
			mirror.close((error): void => {
				if (error) reject(error);
				else resolve();
			});
		});
	}
	return path.join(installed, 'sunstack');
}
async function executableLaunches(options: {
	binary: string;
	assets: string;
	scratch: string;
}): Promise<Launch[]> {
	const launches: Launch[] = [{ label: 'installed', command: [options.binary] }];
	if (process.platform === 'darwin') {
		const archive = path.join(options.assets, 'sunstack-x86_64-apple-darwin.tar.gz');
		const listing = await execute('/usr/bin/tar', ['-tzf', archive], { timeout: 30000 });
		const members = listing.stdout
			.trim()
			.split('\n')
			.filter((name): boolean => path.posix.basename(name) === 'sunstack');
		const member = z.array(z.string().min(1)).length(1).parse(members)[0];
		if (member === undefined) throw new Error('expected exactly one macOS x64 executable');
		const extracted = await execute('/usr/bin/tar', ['-xOzf', archive, member], {
			encoding: 'buffer',
			maxBuffer: 128 * 1024 * 1024,
			timeout: 30000,
		});
		const rosetta = path.join(options.scratch, 'sunstack-x64');
		filesystem.writeFileSync(rosetta, extracted.stdout, { mode: 0o755 });
		launches.push({ label: 'rosetta-x64', command: ['/usr/bin/arch', '-x86_64', rosetta] });
	}
	return launches;
}
async function verifyInstall(options: {
	assets: string;
	tag: string;
	scratch: string;
}): Promise<void> {
	verifyChecksums(options.assets);
	const workspace = fileURLToPath(new URL('../../', import.meta.url));
	const fixture = path.join(options.scratch, 'fixture');
	filesystem.cpSync(path.join(workspace, 'scripts/release/smoke-fixture'), fixture, {
		recursive: true,
	});
	filesystem.copyFileSync(
		path.join(workspace, 'scripts/release/smoke-config.json'),
		path.join(fixture, 'sunstack.lint.json'),
	);
	const home = path.join(options.scratch, 'empty-home');
	filesystem.mkdirSync(home);
	const binary = await installFromMirror(options);
	const launches = await executableLaunches({ ...options, binary });
	await runInOrder(launches, async (launch): Promise<void> => {
		const command = { launch, fixture, home };
		const version = await runBinary({ ...command, arguments: ['--version'] });
		requireResult(version, 0, `${launch.label} env -i --version`);
		if (version.stdout !== `sunstack ${options.tag.slice(1)}\n`)
			throw new Error(`unexpected installed version: ${version.stdout}`);
		const check = await runBinary({ ...command, arguments: ['lint', '--check'] });
		requireResult(check, 0, `${launch.label} env -i lint --check`);
		checkSchema.parse(JSON.parse(check.stdout));
		const lint = await runBinary({ ...command, arguments: ['lint', '--all', '--format', 'json'] });
		requireResult(lint, 0, `${launch.label} env -i lint --all`);
		const report = reportSchema.parse(JSON.parse(lint.stdout));
		if (report.status.kind !== 'clean' || report.findings.length !== 0)
			throw new Error('positive mechanical smoke did not pass');
	});
	const target = path.join(fixture, 'skill.md');
	const original = filesystem.readFileSync(target, 'utf8');
	if (original !== 'MUST validate the released installation.\n')
		throw new Error('positive-control fixture changed');
	filesystem.writeFileSync(target, original.replace('MUST', 'MAY'));
	await runInOrder(launches, async (launch): Promise<void> => {
		const result = await runBinary({
			launch,
			fixture,
			home,
			arguments: ['lint', '--all', '--format', 'json'],
		});
		requireResult(result, 1, `${launch.label} env -i negative lint control`);
		const report = reportSchema.parse(JSON.parse(result.stdout));
		if (report.status.kind !== 'findings' || report.findings.length !== 1)
			throw new Error('negative mechanical smoke failed to catch missing requirement');
	});
	if (filesystem.readdirSync(home).length !== 0)
		throw new Error('runtime wrote into the empty HOME');
	process.stdout.write(
		'Fresh-install smoke passed with no model requests and an untouched runtime HOME.\n',
	);
}
async function main(): Promise<void> {
	const parsed = parseArgs({
		options: {
			tag: { type: 'string' },
			repository: { type: 'string' },
			assets: { type: 'string' },
			help: { type: 'boolean', short: 'h' },
		},
	});
	if (parsed.values.help) {
		process.stdout.write(
			'usage: fresh-install-smoke.ts --tag TAG [--repository OWNER/REPO] [--assets DIRECTORY]\n',
		);
		return;
	}
	const options = argumentsSchema.parse({
		...parsed.values,
		repository: parsed.values.repository ?? process.env['GITHUB_REPOSITORY'],
	});
	const scratch = filesystem.mkdtempSync(path.join(tmpdir(), 'sunstack-fresh-install-'));
	try {
		const assets =
			options.assets === undefined ? path.join(scratch, 'assets') : path.resolve(options.assets);
		if (options.assets === undefined) {
			filesystem.mkdirSync(assets);
			await downloadAssets(options, assets);
		}
		await verifyInstall({ assets, tag: options.tag, scratch });
	} finally {
		filesystem.rmSync(scratch, { recursive: true, force: true });
	}
}
if (import.meta.main) {
	try {
		await main();
	} catch (error) {
		process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
		process.exitCode = 1;
	}
}
