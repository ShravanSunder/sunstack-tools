/** Reject non-standalone executables in the actual Sunstack release archives. */
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { readReleaseArchive } from './release-archive.ts';
import { parseReleaseArguments, printReleaseError } from './release-arguments.ts';

const archiveName = /^sunstack-(aarch64|x86_64)-(apple-darwin|unknown-linux-musl)\.tar\.gz$/;
export const targets = [
	'aarch64-apple-darwin',
	'x86_64-apple-darwin',
	'aarch64-unknown-linux-musl',
	'x86_64-unknown-linux-musl',
];

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
		if (programType === 2)
			for (let entry = offset; entry < offset + size; entry += 16) {
				const tag = binary.readBigInt64LE(entry);
				if (tag === 0n) break;
				if (tag === 1n) throw new Error('Linux archive contains a DT_NEEDED shared library');
			}
	}
}

export function checkMacosDependencies(output: string): void {
	const dependencies = output
		.split(/\r?\n/)
		.slice(1)
		.filter((line) => line.trim())
		.map((line) => line.trim().split(' (', 1)[0] ?? '');
	if (dependencies.length === 0) throw new Error('otool reported no macOS libraries');
	const forbidden = dependencies.filter(
		(dependency) =>
			!dependency.startsWith('/usr/lib/') && !dependency.startsWith('/System/Library/'),
	);
	if (forbidden.length)
		throw new Error(
			`macOS archive contains non-OS libraries: [${forbidden.map((dependency) => `'${dependency}'`).join(', ')}]`,
		);
}

export function findArchives(directory: string, selectedTargets?: string[]): string[] {
	const archives = selectedTargets?.length
		? selectedTargets.map((target) => path.join(directory, `sunstack-${target}.tar.gz`))
		: readdirSync(directory)
				.filter((name) => archiveName.test(name))
				.toSorted()
				.map((name) => path.join(directory, name));
	if (
		!archives.length ||
		archives.some((archive) => {
			try {
				return !statSync(archive).isFile();
			} catch {
				return true;
			}
		})
	)
		throw new Error('no Sunstack binary release archives found for the requested targets');
	return archives;
}

export function checkArchive(archive: string): void {
	const binaries = readReleaseArchive(archive).filter(
		(member) => member.regular && path.posix.basename(member.name) === 'sunstack',
	);
	if (binaries.length !== 1)
		throw new Error(`${archive}: expected exactly one sunstack executable`);
	const binary = binaries[0];
	if (!binary) throw new Error(`${archive}: cannot read the executable`);
	if (archive.endsWith('linux-musl.tar.gz')) checkElf(binary.data);
	else {
		const directory = mkdtempSync(path.join(tmpdir(), 'sunstack-linkage-'));
		try {
			const executable = path.join(directory, 'sunstack');
			writeFileSync(executable, binary.data);
			checkMacosDependencies(execFileSync('otool', ['-L', executable], { encoding: 'utf8' }));
		} finally {
			rmSync(directory, { recursive: true, force: true });
		}
	}
	process.stdout.write(`${path.basename(archive)}: standalone linkage verified\n`);
}

export function main(words: string[]): void {
	const parsed = parseReleaseArguments({
		words,
		flags: ['target'],
		positional: 'directory',
		choices: { target: targets },
	});
	if (parsed.help) {
		process.stdout.write(helpText);
		return;
	}
	const directory = parsed.positionals[0] ?? '';
	const selected = parsed.values['target'] ?? [];
	for (const archive of findArchives(directory, selected)) checkArchive(archive);
}

const helpText =
	"usage: check-linkage.ts [-h]\n                        [--target {aarch64-apple-darwin,x86_64-apple-darwin,aarch64-unknown-linux-musl,x86_64-unknown-linux-musl}]\n                        directory\n\nReject non-standalone executables in the actual Sunstack release archives.\n\npositional arguments:\n  directory             cargo-dist artifact directory\n\noptions:\n  -h, --help            show this help message and exit\n  --target {aarch64-apple-darwin,x86_64-apple-darwin,aarch64-unknown-linux-musl,x86_64-unknown-linux-musl}\n                        check only this build job's target; repeat for\n                        multiple targets\n";

if (import.meta.main) {
	try {
		main(process.argv.slice(2));
	} catch (error) {
		printReleaseError(error, helpText, 'check-linkage.ts');
	}
}
