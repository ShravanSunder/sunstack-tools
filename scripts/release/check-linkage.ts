/** Reject non-standalone executables in the actual Sunstack release archives. */
import { execFileSync } from 'node:child_process';
import filesystem from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { parseArgs } from 'node:util';

const archiveNamePattern = /^sunstack-(aarch64|x86_64)-(apple-darwin|unknown-linux-musl)\.tar\.gz$/;
const releaseTargets: ReadonlySet<string> = new Set([
	'aarch64-apple-darwin',
	'x86_64-apple-darwin',
	'aarch64-unknown-linux-musl',
	'x86_64-unknown-linux-musl',
]);

export function checkElf(binary: Buffer): void {
	if (!binary.subarray(0, 6).equals(Buffer.from([0x7f, 0x45, 0x4c, 0x46, 2, 1])))
		throw new Error('expected a 64-bit little-endian ELF executable');
	const headerOffset = Number(binary.readBigUInt64LE(32));
	const headerSize = binary.readUInt16LE(54);
	const headerCount = binary.readUInt16LE(56);
	for (let index = 0; index < headerCount; index++) {
		const header = headerOffset + index * headerSize;
		const programType = binary.readUInt32LE(header);
		const offset = Number(binary.readBigUInt64LE(header + 8));
		const size = Number(binary.readBigUInt64LE(header + 32));
		if (programType === 3) throw new Error('Linux archive contains a PT_INTERP loader');
		if (programType === 2) {
			for (let entry = offset; entry < offset + size; entry += 16) {
				const tag = binary.readBigInt64LE(entry);
				if (tag === 0n) break;
				if (tag === 1n) throw new Error('Linux archive contains a DT_NEEDED shared library');
			}
		}
	}
}

export function checkMacosDependencies(otoolOutput: string): void {
	const dependencies = otoolOutput
		.split(/\r?\n/)
		.slice(1)
		.filter((line) => line.trim())
		.map((line) => line.trim().split(' (', 1)[0]);
	if (dependencies.length === 0) throw new Error('otool reported no macOS libraries');
	const forbidden = dependencies.filter(
		(dependency) =>
			!dependency?.startsWith('/usr/lib/') && !dependency?.startsWith('/System/Library/'),
	);
	if (forbidden.length)
		throw new Error(`macOS archive contains non-OS libraries: ${JSON.stringify(forbidden)}`);
}

function readArchivedBinary(archive: string): Buffer {
	// Use the same tar utility already used by release installation. Its listing
	// resolves PAX/GNU names; the type column keeps links and directories out.
	const options = { encoding: 'utf8', maxBuffer: Number.MAX_SAFE_INTEGER } satisfies Parameters<
		typeof execFileSync
	>[2];
	const names = execFileSync('tar', ['-tzf', archive], options).replace(/\n$/, '').split('\n');
	const members = execFileSync('tar', ['-tvzf', archive], options).replace(/\n$/, '').split('\n');
	if (names.length !== members.length) throw new Error(`${archive}: cannot read the executable`);
	const binaries = names.filter(
		(name, index) => members[index]?.startsWith('-') && path.posix.basename(name) === 'sunstack',
	);
	if (binaries.length !== 1)
		throw new Error(`${archive}: expected exactly one sunstack executable`);
	const memberName = binaries[0];
	if (!memberName) throw new Error(`${archive}: cannot read the executable`);
	return execFileSync('tar', ['-xOzf', archive, '--', memberName], {
		maxBuffer: Number.MAX_SAFE_INTEGER,
	});
}

export function checkArchive(archive: string): void {
	const binary = readArchivedBinary(archive);
	if (path.basename(archive).endsWith('linux-musl.tar.gz')) checkElf(binary);
	else {
		const directory = filesystem.mkdtempSync(path.join(tmpdir(), 'sunstack-linkage-'));
		try {
			const executable = path.join(directory, 'sunstack');
			filesystem.writeFileSync(executable, binary);
			checkMacosDependencies(execFileSync('otool', ['-L', executable], { encoding: 'utf8' }));
		} finally {
			filesystem.rmSync(directory, { recursive: true, force: true });
		}
	}
	process.stdout.write(`${path.basename(archive)}: standalone linkage verified\n`);
}

export function findArchives(directory: string, targets?: readonly string[]): string[] {
	const archives = targets?.length
		? targets.map((target) => path.join(directory, `sunstack-${target}.tar.gz`))
		: filesystem
				.readdirSync(directory)
				.filter((name) => archiveNamePattern.test(name))
				.toSorted()
				.map((name) => path.join(directory, name));
	if (
		archives.length === 0 ||
		archives.some(
			(archive) => !filesystem.existsSync(archive) || !filesystem.statSync(archive).isFile(),
		)
	)
		throw new Error('no Sunstack binary release archives found for the requested targets');
	return archives;
}

function main(): void {
	const { values, positionals } = parseArgs({
		options: { target: { type: 'string', multiple: true }, help: { type: 'boolean', short: 'h' } },
		allowPositionals: true,
	});
	if (values.help) {
		process.stdout.write(
			'Reject non-standalone executables in the actual Sunstack release archives.\nUsage: node check-linkage.ts DIRECTORY [--target TARGET ...]\n',
		);
		return;
	}
	const directory = positionals[0];
	if (
		positionals.length !== 1 ||
		!directory ||
		values.target?.some((target) => !releaseTargets.has(target))
	) {
		process.stderr.write('a directory and supported release targets are required\n');
		process.exitCode = 2;
		return;
	}
	for (const archive of findArchives(directory, values.target)) checkArchive(archive);
}

if (import.meta.main) {
	try {
		main();
	} catch (error) {
		process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
		process.exitCode =
			error instanceof TypeError &&
			'code' in error &&
			String(error.code).startsWith('ERR_PARSE_ARGS')
				? 2
				: 1;
	}
}
